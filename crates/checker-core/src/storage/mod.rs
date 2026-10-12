//! SQLite persistence.

mod models;

use std::path::Path;
use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

pub use models::{Incident, Monitor, MonitorStats, NewMonitor, StoredResult};

use crate::checker::{CheckKind, CheckResult};
use crate::error::{CoreError, Result};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone)]
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    /// Opens (creating if needed) the database file and runs migrations.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        Self::with_pool(pool).await
    }

    /// In-memory database, used by tests and the CLI.
    pub async fn in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        // A single connection: every in-memory connection is its own database.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Self::with_pool(pool).await
    }

    async fn with_pool(pool: SqlitePool) -> Result<Self> {
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    // ---- settings -------------------------------------------------------

    pub async fn get_setting<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value_json FROM settings WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        row.map(|(json,)| serde_json::from_str(&json).map_err(Into::into))
            .transpose()
    }

    pub async fn set_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value_json) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
        )
        .bind(key)
        .bind(serde_json::to_string(value)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- monitors -------------------------------------------------------

    pub async fn create_monitor(&self, new: &NewMonitor) -> Result<Monitor> {
        new.validate()?;
        let id = sqlx::query(
            "INSERT INTO monitors (kind, name, config_json, interval_sec, enabled, tags)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(new.kind.as_str())
        .bind(new.name.trim())
        .bind(serde_json::to_string(&new.config)?)
        .bind(new.interval_sec as i64)
        .bind(new.enabled)
        .bind(new.tags.join(","))
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        self.get_monitor(id).await
    }

    pub async fn update_monitor(&self, id: i64, new: &NewMonitor) -> Result<Monitor> {
        new.validate()?;
        let affected = sqlx::query(
            "UPDATE monitors SET kind = ?, name = ?, config_json = ?, interval_sec = ?, enabled = ?, tags = ?
             WHERE id = ?",
        )
        .bind(new.kind.as_str())
        .bind(new.name.trim())
        .bind(serde_json::to_string(&new.config)?)
        .bind(new.interval_sec as i64)
        .bind(new.enabled)
        .bind(new.tags.join(","))
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if affected == 0 {
            return Err(CoreError::NotFound(format!("monitor {id}")));
        }
        self.get_monitor(id).await
    }

    pub async fn get_monitor(&self, id: i64) -> Result<Monitor> {
        sqlx::query_as::<_, models::MonitorRow>("SELECT * FROM monitors WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("monitor {id}")))?
            .try_into()
    }

    pub async fn list_monitors(&self, kind: Option<CheckKind>) -> Result<Vec<Monitor>> {
        let rows = match kind {
            Some(kind) => {
                sqlx::query_as::<_, models::MonitorRow>(
                    "SELECT * FROM monitors WHERE kind = ? ORDER BY name",
                )
                .bind(kind.as_str())
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, models::MonitorRow>("SELECT * FROM monitors ORDER BY name")
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn delete_monitor(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM monitors WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- results --------------------------------------------------------

    pub async fn insert_result(&self, monitor_id: i64, result: &CheckResult) -> Result<i64> {
        let id = sqlx::query(
            "INSERT INTO check_results (monitor_id, status, latency_ms, message, details_json, checked_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(monitor_id)
        .bind(result.status.as_str())
        .bind(result.latency_ms.map(|ms| ms as i64))
        .bind(&result.message)
        .bind(serde_json::to_string(&result.details)?)
        .bind(result.checked_at)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    /// Most recent results first.
    pub async fn recent_results(&self, monitor_id: i64, limit: u32) -> Result<Vec<StoredResult>> {
        sqlx::query_as::<_, models::ResultRow>(
            "SELECT r.*, m.kind FROM check_results r JOIN monitors m ON m.id = r.monitor_id
             WHERE r.monitor_id = ? ORDER BY r.checked_at DESC LIMIT ?",
        )
        .bind(monitor_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }

    /// Results of a monitor since `since`, oldest first (for charts).
    pub async fn results_since(
        &self,
        monitor_id: i64,
        since: DateTime<Utc>,
    ) -> Result<Vec<StoredResult>> {
        sqlx::query_as::<_, models::ResultRow>(
            "SELECT r.*, m.kind FROM check_results r JOIN monitors m ON m.id = r.monitor_id
             WHERE r.monitor_id = ? AND r.checked_at >= ? ORDER BY r.checked_at",
        )
        .bind(monitor_id)
        .bind(since)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }

    /// The newest result of every monitor.
    pub async fn latest_results(&self) -> Result<Vec<StoredResult>> {
        sqlx::query_as::<_, models::ResultRow>(
            "SELECT r.*, m.kind FROM check_results r JOIN monitors m ON m.id = r.monitor_id
             WHERE r.id IN (SELECT MAX(id) FROM check_results GROUP BY monitor_id)",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }

    /// Uptime and average latency of a monitor since `since`.
    pub async fn monitor_stats(
        &self,
        monitor_id: i64,
        since: DateTime<Utc>,
    ) -> Result<MonitorStats> {
        let (total, ok, avg): (i64, i64, Option<f64>) = sqlx::query_as(
            "SELECT COUNT(*),
                    COALESCE(SUM(status IN ('up', 'degraded')), 0),
                    AVG(latency_ms)
             FROM check_results WHERE monitor_id = ? AND checked_at >= ?",
        )
        .bind(monitor_id)
        .bind(since)
        .fetch_one(&self.pool)
        .await?;
        Ok(MonitorStats {
            monitor_id,
            total,
            ok,
            uptime: (total > 0).then(|| ok as f64 * 100.0 / total as f64),
            avg_latency_ms: avg,
        })
    }

    // ---- incidents ------------------------------------------------------

    pub async fn open_incident(&self, monitor_id: i64) -> Result<Option<Incident>> {
        Ok(sqlx::query_as::<_, Incident>(
            "SELECT i.*, m.name AS monitor_name FROM incidents i JOIN monitors m ON m.id = i.monitor_id
             WHERE i.monitor_id = ? AND i.resolved_at IS NULL ORDER BY i.started_at DESC LIMIT 1",
        )
        .bind(monitor_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn start_incident(
        &self,
        monitor_id: i64,
        at: DateTime<Utc>,
        cause: Option<&str>,
    ) -> Result<i64> {
        Ok(
            sqlx::query("INSERT INTO incidents (monitor_id, started_at, cause) VALUES (?, ?, ?)")
                .bind(monitor_id)
                .bind(at)
                .bind(cause)
                .execute(&self.pool)
                .await?
                .last_insert_rowid(),
        )
    }

    pub async fn resolve_incident(&self, incident_id: i64, at: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE incidents SET resolved_at = ? WHERE id = ?")
            .bind(at)
            .bind(incident_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Most recent incidents across all monitors (or one monitor).
    pub async fn list_incidents(
        &self,
        monitor_id: Option<i64>,
        limit: u32,
    ) -> Result<Vec<Incident>> {
        Ok(sqlx::query_as::<_, Incident>(
            "SELECT i.*, m.name AS monitor_name FROM incidents i JOIN monitors m ON m.id = i.monitor_id
             WHERE (?1 IS NULL OR i.monitor_id = ?1) ORDER BY i.started_at DESC LIMIT ?2",
        )
        .bind(monitor_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Deletes history older than `retention_days`. Returns removed rows.
    pub async fn prune(&self, retention_days: u32) -> Result<u64> {
        let cutoff = Utc::now() - Duration::days(retention_days as i64);
        let mut removed = sqlx::query("DELETE FROM check_results WHERE checked_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?
            .rows_affected();
        removed += sqlx::query("DELETE FROM system_samples WHERE ts < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::checker::Status;
    use crate::settings::{AppSettings, Language};

    fn http_monitor(name: &str) -> NewMonitor {
        NewMonitor {
            kind: CheckKind::Http,
            name: name.into(),
            config: json!({ "url": "https://example.com" }),
            interval_sec: 60,
            enabled: true,
            tags: vec!["prod".into(), "web".into()],
        }
    }

    #[tokio::test]
    async fn settings_round_trip_with_defaults() {
        let storage = Storage::in_memory().await.unwrap();
        assert_eq!(
            AppSettings::load(&storage).await.unwrap(),
            AppSettings::default()
        );

        let saved = AppSettings {
            language: Language::En,
            default_concurrency: 0, // clamped to 1
            proxy_judge_url: Some("  ".into()),
            ..Default::default()
        }
        .save(&storage)
        .await
        .unwrap();
        assert_eq!(saved.default_concurrency, 1);
        assert_eq!(saved.proxy_judge_url, None);
        assert_eq!(AppSettings::load(&storage).await.unwrap(), saved);
    }

    #[tokio::test]
    async fn monitor_crud() {
        let storage = Storage::in_memory().await.unwrap();
        let b = storage
            .create_monitor(&http_monitor("b site"))
            .await
            .unwrap();
        let a = storage
            .create_monitor(&http_monitor("a site"))
            .await
            .unwrap();
        assert_eq!(b.tags, vec!["prod", "web"]);
        assert_eq!(b.config["url"], "https://example.com");

        let all = storage.list_monitors(None).await.unwrap();
        assert_eq!(
            all.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
            ["a site", "b site"]
        );
        assert!(storage
            .list_monitors(Some(CheckKind::Dns))
            .await
            .unwrap()
            .is_empty());

        let mut changed = http_monitor("renamed");
        changed.enabled = false;
        let updated = storage.update_monitor(a.id, &changed).await.unwrap();
        assert_eq!(updated.name, "renamed");
        assert!(!updated.enabled);

        storage.delete_monitor(a.id).await.unwrap();
        assert!(matches!(
            storage.get_monitor(a.id).await,
            Err(CoreError::NotFound(_))
        ));
        assert!(matches!(
            storage.update_monitor(a.id, &changed).await,
            Err(CoreError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn rejects_invalid_monitor() {
        let storage = Storage::in_memory().await.unwrap();
        let mut bad = http_monitor("  ");
        assert!(matches!(
            storage.create_monitor(&bad).await,
            Err(CoreError::Invalid(_))
        ));
        bad.name = "ok".into();
        bad.interval_sec = 0;
        assert!(matches!(
            storage.create_monitor(&bad).await,
            Err(CoreError::Invalid(_))
        ));
    }

    #[tokio::test]
    async fn results_history_and_prune() {
        let storage = Storage::in_memory().await.unwrap();
        let monitor = storage.create_monitor(&http_monitor("site")).await.unwrap();

        let mut old = CheckResult::new(CheckKind::Http, Status::Down).with_message("timeout");
        old.checked_at = Utc::now() - Duration::days(40);
        storage.insert_result(monitor.id, &old).await.unwrap();
        let fresh = CheckResult::new(CheckKind::Http, Status::Up)
            .with_latency(std::time::Duration::from_millis(120))
            .with_details(json!({ "code": 200 }));
        storage.insert_result(monitor.id, &fresh).await.unwrap();

        let recent = storage.recent_results(monitor.id, 10).await.unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].result.status, Status::Up);
        assert_eq!(recent[0].result.latency_ms, Some(120));
        assert_eq!(recent[0].result.details["code"], 200);
        assert_eq!(recent[1].result.message.as_deref(), Some("timeout"));

        let week = storage
            .results_since(monitor.id, Utc::now() - Duration::days(7))
            .await
            .unwrap();
        assert_eq!(week.len(), 1);

        assert_eq!(storage.prune(30).await.unwrap(), 1);
        assert_eq!(
            storage.recent_results(monitor.id, 10).await.unwrap().len(),
            1
        );

        // Deleting the monitor cascades to its results.
        storage.delete_monitor(monitor.id).await.unwrap();
        assert!(storage
            .recent_results(monitor.id, 10)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn opens_file_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rodi.db");
        let storage = Storage::open(&path).await.unwrap();
        storage.set_setting("k", &42).await.unwrap();
        storage.close().await;

        let reopened = Storage::open(&path).await.unwrap();
        assert_eq!(reopened.get_setting::<i32>("k").await.unwrap(), Some(42));
    }
}
