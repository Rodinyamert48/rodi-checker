use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::Storage;
use crate::checker::CheckResult;
use crate::error::{CoreError, Result};
use crate::proxy::{ProxyEntry, ProxyProtocol};
use crate::secret::SecretBox;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProxyListSummary {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub total: i64,
    pub checked: i64,
    pub working: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct StoredProxy {
    pub id: i64,
    pub list_id: i64,
    pub protocol: String,
    pub host: String,
    pub port: i64,
    pub username: Option<String>,
    pub has_password: bool,
    pub status: Option<String>,
    pub latency_ms: Option<i64>,
    pub anonymity: Option<String>,
    pub exit_ip: Option<String>,
    pub country: Option<String>,
    pub message: Option<String>,
    pub checked_at: Option<DateTime<Utc>>,
}

#[derive(FromRow)]
struct EntryRow {
    id: i64,
    protocol: String,
    host: String,
    port: i64,
    username: Option<String>,
    password_enc: Option<String>,
}

impl Storage {
    /// Creates a list with the given proxies. Passwords are encrypted with `secrets`.
    pub async fn create_proxy_list(
        &self,
        name: &str,
        entries: &[ProxyEntry],
        secrets: &SecretBox,
    ) -> Result<ProxyListSummary> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Invalid("list name is empty".into()));
        }
        let mut tx = self.pool.begin().await?;
        let list_id = sqlx::query("INSERT INTO proxy_lists (name) VALUES (?)")
            .bind(name)
            .execute(&mut *tx)
            .await?
            .last_insert_rowid();
        for entry in entries {
            sqlx::query(
                "INSERT OR IGNORE INTO proxies (list_id, protocol, host, port, username, has_password, password_enc)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(list_id)
            .bind(entry.protocol.as_str())
            .bind(&entry.host)
            .bind(entry.port as i64)
            .bind(&entry.username)
            .bind(entry.password.is_some())
            .bind(entry.password.as_deref().map(|p| secrets.encrypt(p)))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        self.proxy_list(list_id).await
    }

    const SUMMARY_SQL: &'static str = "SELECT l.id, l.name, l.created_at,
            COUNT(p.id) AS total,
            COALESCE(SUM(p.status IS NOT NULL), 0) AS checked,
            COALESCE(SUM(p.status IN ('up', 'degraded')), 0) AS working
         FROM proxy_lists l LEFT JOIN proxies p ON p.list_id = l.id";

    pub async fn proxy_list(&self, id: i64) -> Result<ProxyListSummary> {
        sqlx::query_as(&format!(
            "{} WHERE l.id = ? GROUP BY l.id",
            Self::SUMMARY_SQL
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CoreError::NotFound(format!("proxy list {id}")))
    }

    pub async fn proxy_lists(&self) -> Result<Vec<ProxyListSummary>> {
        Ok(sqlx::query_as(&format!(
            "{} GROUP BY l.id ORDER BY l.created_at DESC, l.id DESC",
            Self::SUMMARY_SQL
        ))
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn delete_proxy_list(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM proxy_lists WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn proxies(&self, list_id: i64) -> Result<Vec<StoredProxy>> {
        Ok(sqlx::query_as(
            "SELECT id, list_id, protocol, host, port, username, has_password, status, latency_ms,
                    anonymity, exit_ip, country, message, checked_at
             FROM proxies WHERE list_id = ? ORDER BY id",
        )
        .bind(list_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Proxies of a list with decrypted passwords, ready to be checked.
    pub async fn proxy_entries(
        &self,
        list_id: i64,
        secrets: &SecretBox,
    ) -> Result<Vec<(i64, ProxyEntry)>> {
        let rows: Vec<EntryRow> = sqlx::query_as(
            "SELECT id, protocol, host, port, username, password_enc FROM proxies WHERE list_id = ? ORDER BY id",
        )
        .bind(list_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let password = match row.password_enc.as_deref() {
                    Some(enc) => Some(secrets.decrypt(enc)?),
                    None => None,
                };
                Ok((
                    row.id,
                    ProxyEntry {
                        protocol: row.protocol.parse::<ProxyProtocol>()?,
                        host: row.host,
                        port: row.port as u16,
                        username: row.username,
                        password,
                    },
                ))
            })
            .collect()
    }

    pub async fn save_proxy_result(&self, proxy_id: i64, result: &CheckResult) -> Result<()> {
        let detail = |key: &str| {
            result
                .details
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        };
        sqlx::query(
            "UPDATE proxies SET status = ?, latency_ms = ?, anonymity = ?, exit_ip = ?, country = ?,
                    message = ?, checked_at = ?
             WHERE id = ?",
        )
        .bind(result.status.as_str())
        .bind(result.latency_ms.map(|ms| ms as i64))
        .bind(detail("anonymity"))
        .bind(detail("exitIp"))
        .bind(detail("country"))
        .bind(&result.message)
        .bind(result.checked_at)
        .bind(proxy_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes proxies whose last check failed. Returns how many were removed.
    pub async fn remove_dead_proxies(&self, list_id: i64) -> Result<u64> {
        Ok(
            sqlx::query("DELETE FROM proxies WHERE list_id = ? AND status IN ('down', 'error')")
                .bind(list_id)
                .execute(&self.pool)
                .await?
                .rows_affected(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker::{CheckKind, Status};
    use crate::proxy::parse_list;

    #[tokio::test]
    async fn proxy_list_lifecycle() {
        let storage = Storage::in_memory().await.unwrap();
        let secrets = SecretBox::new(&SecretBox::generate_key());
        let parsed = parse_list(
            "1.1.1.1:80\nsocks5://bob:secret@2.2.2.2:1080\n3.3.3.3:3128",
            ProxyProtocol::Http,
        );
        let list = storage
            .create_proxy_list("Liste", &parsed.entries, &secrets)
            .await
            .unwrap();
        assert_eq!((list.total, list.checked, list.working), (3, 0, 0));

        // The password is encrypted at rest.
        let raw: (String,) =
            sqlx::query_as("SELECT password_enc FROM proxies WHERE username = 'bob'")
                .fetch_one(storage.pool())
                .await
                .unwrap();
        assert!(!raw.0.contains("secret"));

        let entries = storage.proxy_entries(list.id, &secrets).await.unwrap();
        assert_eq!(entries[1].1.password.as_deref(), Some("secret"));

        let up = CheckResult::new(CheckKind::Proxy, Status::Up).with_details(
            serde_json::json!({ "anonymity": "elite", "exitIp": "9.9.9.9", "country": "TR" }),
        );
        storage.save_proxy_result(entries[0].0, &up).await.unwrap();
        let down = CheckResult::new(CheckKind::Proxy, Status::Down).with_message("timeout");
        storage
            .save_proxy_result(entries[1].0, &down)
            .await
            .unwrap();

        let proxies = storage.proxies(list.id).await.unwrap();
        assert_eq!(proxies[0].anonymity.as_deref(), Some("elite"));
        assert_eq!(proxies[0].country.as_deref(), Some("TR"));
        assert!(proxies[1].has_password);
        assert_eq!(proxies[1].message.as_deref(), Some("timeout"));

        let summary = storage.proxy_list(list.id).await.unwrap();
        assert_eq!((summary.total, summary.checked, summary.working), (3, 2, 1));

        assert_eq!(storage.remove_dead_proxies(list.id).await.unwrap(), 1);
        assert_eq!(storage.proxy_lists().await.unwrap()[0].total, 2);

        storage.delete_proxy_list(list.id).await.unwrap();
        assert!(storage.proxy_lists().await.unwrap().is_empty());
        assert!(storage.proxies(list.id).await.unwrap().is_empty());
    }
}
