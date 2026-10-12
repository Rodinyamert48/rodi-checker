use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::Storage;
use crate::error::Result;
use crate::system::{AlertMetric, AlertRule, NewAlertRule, Snapshot};

/// Stored system sample used for history charts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SystemSample {
    pub ts: DateTime<Utc>,
    pub cpu: f64,
    pub mem_used: i64,
    pub mem_total: i64,
}

#[derive(FromRow)]
struct RuleRow {
    id: i64,
    metric: String,
    threshold: f64,
    target: Option<String>,
    enabled: bool,
}

impl Storage {
    pub async fn insert_system_sample(&self, snapshot: &Snapshot) -> Result<()> {
        sqlx::query(
            "INSERT INTO system_samples (ts, cpu, mem_used, mem_total, disk_json, net_json) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(snapshot.ts)
        .bind(f64::from(snapshot.cpu))
        .bind(snapshot.mem_used as i64)
        .bind(snapshot.mem_total as i64)
        .bind(serde_json::to_string(&snapshot.disks)?)
        .bind(serde_json::to_string(&snapshot.net)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn system_samples(&self, since: DateTime<Utc>) -> Result<Vec<SystemSample>> {
        Ok(sqlx::query_as(
            "SELECT ts, cpu, mem_used, mem_total FROM system_samples WHERE ts >= ? ORDER BY ts",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn alert_rules(&self) -> Result<Vec<AlertRule>> {
        let rows: Vec<RuleRow> =
            sqlx::query_as("SELECT id, metric, threshold, target, enabled FROM alert_rules WHERE target_kind = 'system' ORDER BY id")
                .fetch_all(&self.pool)
                .await?;
        rows.into_iter()
            .map(|row| {
                Ok(AlertRule {
                    id: row.id,
                    rule: NewAlertRule {
                        metric: AlertMetric::parse(&row.metric)?,
                        threshold: row.threshold,
                        target: row.target,
                        enabled: row.enabled,
                    },
                })
            })
            .collect()
    }

    pub async fn create_alert_rule(&self, rule: &NewAlertRule) -> Result<AlertRule> {
        rule.validate()?;
        let target = rule
            .target
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty());
        let id = sqlx::query(
            "INSERT INTO alert_rules (target_kind, metric, op, threshold, target, enabled) VALUES ('system', ?, '>', ?, ?, ?)",
        )
        .bind(rule.metric.as_str())
        .bind(rule.threshold)
        .bind(target)
        .bind(rule.enabled)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(AlertRule {
            id,
            rule: NewAlertRule {
                target: target.map(str::to_owned),
                ..rule.clone()
            },
        })
    }

    pub async fn set_alert_rule_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE alert_rules SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_alert_rule(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM alert_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::SystemMonitor;

    #[tokio::test]
    async fn samples_and_rules() {
        let storage = Storage::in_memory().await.unwrap();
        let snapshot = SystemMonitor::new().snapshot();
        storage.insert_system_sample(&snapshot).await.unwrap();
        let samples = storage
            .system_samples(Utc::now() - chrono::Duration::hours(1))
            .await
            .unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].mem_total as u64, snapshot.mem_total);

        let rule = storage
            .create_alert_rule(&NewAlertRule {
                metric: AlertMetric::Service,
                threshold: 0.0,
                target: Some(" Spooler ".into()),
                enabled: true,
            })
            .await
            .unwrap();
        assert_eq!(rule.rule.target.as_deref(), Some("Spooler"));
        storage
            .set_alert_rule_enabled(rule.id, false)
            .await
            .unwrap();
        let rules = storage.alert_rules().await.unwrap();
        assert!(!rules[0].rule.enabled);

        assert!(storage
            .create_alert_rule(&NewAlertRule {
                metric: AlertMetric::Cpu,
                threshold: 500.0,
                target: None,
                enabled: true,
            })
            .await
            .is_err());

        storage.delete_alert_rule(rule.id).await.unwrap();
        assert!(storage.alert_rules().await.unwrap().is_empty());
    }
}
