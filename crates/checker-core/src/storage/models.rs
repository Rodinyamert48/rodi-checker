use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::checker::{CheckKind, CheckResult};
use crate::error::{CoreError, Result};

/// Input for creating or updating a monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewMonitor {
    pub kind: CheckKind,
    pub name: String,
    /// Kind specific configuration (url, host, record type, ...).
    #[serde(default)]
    pub config: serde_json::Value,
    pub interval_sec: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl NewMonitor {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(CoreError::Invalid("monitor name is empty".into()));
        }
        if !(5..=86_400).contains(&self.interval_sec) {
            return Err(CoreError::Invalid(
                "interval must be between 5 seconds and 1 day".into(),
            ));
        }
        if self.tags.iter().any(|tag| tag.contains(',')) {
            return Err(CoreError::Invalid("tags cannot contain commas".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: i64,
    pub kind: CheckKind,
    pub name: String,
    pub config: serde_json::Value,
    pub interval_sec: u32,
    pub enabled: bool,
    pub tags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredResult {
    pub id: i64,
    pub monitor_id: i64,
    #[serde(flatten)]
    pub result: CheckResult,
}

#[derive(FromRow)]
pub(super) struct MonitorRow {
    id: i64,
    kind: String,
    name: String,
    config_json: String,
    interval_sec: i64,
    enabled: bool,
    tags: String,
    created_at: String,
}

impl TryFrom<MonitorRow> for Monitor {
    type Error = CoreError;

    fn try_from(row: MonitorRow) -> Result<Self> {
        Ok(Monitor {
            id: row.id,
            kind: row.kind.parse()?,
            name: row.name,
            config: serde_json::from_str(&row.config_json)?,
            interval_sec: row.interval_sec as u32,
            enabled: row.enabled,
            tags: row
                .tags
                .split(',')
                .filter(|tag| !tag.is_empty())
                .map(str::to_owned)
                .collect(),
            created_at: row.created_at,
        })
    }
}

#[derive(FromRow)]
pub(super) struct ResultRow {
    id: i64,
    monitor_id: i64,
    status: String,
    latency_ms: Option<i64>,
    message: Option<String>,
    details_json: String,
    checked_at: DateTime<Utc>,
    kind: String,
}

impl TryFrom<ResultRow> for StoredResult {
    type Error = CoreError;

    fn try_from(s: ResultRow) -> Result<Self> {
        Ok(StoredResult {
            id: s.id,
            monitor_id: s.monitor_id,
            result: CheckResult {
                kind: s.kind.parse()?,
                status: s.status.parse()?,
                latency_ms: s.latency_ms.map(|ms| ms as u64),
                message: s.message,
                details: serde_json::from_str(&s.details_json)?,
                checked_at: s.checked_at,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Incident {
    pub id: i64,
    pub monitor_id: i64,
    pub monitor_name: String,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub cause: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorStats {
    pub monitor_id: i64,
    pub total: i64,
    pub ok: i64,
    /// Percentage of successful checks, `None` when there is no data.
    pub uptime: Option<f64>,
    pub avg_latency_ms: Option<f64>,
}
