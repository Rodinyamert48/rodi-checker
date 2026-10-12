//! Running scheduled monitors and tracking incidents.

use serde::Serialize;

use crate::checker::{CheckContext, CheckResult, Checker};
use crate::error::Result;
use crate::storage::{Monitor, Storage};
use crate::web::{WebChecker, WebTarget};

/// Status change caused by a check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Transition {
    /// The monitor started failing; an incident was opened.
    WentDown,
    /// The monitor recovered; the open incident was resolved.
    Recovered,
}

/// Runs a single web monitor once (without storing the result).
pub async fn run_web_monitor(
    checker: &WebChecker,
    monitor: &Monitor,
    ctx: &CheckContext,
) -> CheckResult {
    match WebTarget::from_config(monitor.kind, &monitor.config) {
        Ok(target) => checker.check(&target, ctx).await,
        Err(err) => CheckResult::error(monitor.kind, err.to_string()),
    }
}

/// Stores `result` and opens/resolves incidents. Returns the transition, if any.
pub async fn record(
    storage: &Storage,
    monitor_id: i64,
    result: &CheckResult,
) -> Result<Option<Transition>> {
    storage.insert_result(monitor_id, result).await?;
    let open = storage.open_incident(monitor_id).await?;
    match (result.status.is_ok(), open) {
        (false, None) => {
            storage
                .start_incident(monitor_id, result.checked_at, result.message.as_deref())
                .await?;
            Ok(Some(Transition::WentDown))
        }
        (true, Some(incident)) => {
            storage
                .resolve_incident(incident.id, result.checked_at)
                .await?;
            Ok(Some(Transition::Recovered))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use serde_json::json;

    use super::*;
    use crate::checker::{CheckKind, Status};
    use crate::storage::NewMonitor;

    #[tokio::test]
    async fn incidents_follow_status_changes() {
        let storage = Storage::in_memory().await.unwrap();
        let monitor = storage
            .create_monitor(&NewMonitor {
                kind: CheckKind::Tcp,
                name: "db".into(),
                config: json!({ "host": "127.0.0.1", "port": 5432 }),
                interval_sec: 60,
                enabled: true,
                tags: vec![],
            })
            .await
            .unwrap();
        let result = |status| CheckResult::new(CheckKind::Tcp, status).with_message("x");

        assert_eq!(
            record(&storage, monitor.id, &result(Status::Up))
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            record(&storage, monitor.id, &result(Status::Down))
                .await
                .unwrap(),
            Some(Transition::WentDown)
        );
        assert_eq!(
            record(&storage, monitor.id, &result(Status::Error))
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            record(&storage, monitor.id, &result(Status::Degraded))
                .await
                .unwrap(),
            Some(Transition::Recovered)
        );

        let incidents = storage.list_incidents(None, 10).await.unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].monitor_name, "db");
        assert!(incidents[0].resolved_at.is_some());

        let stats = storage
            .monitor_stats(monitor.id, Utc::now() - Duration::hours(1))
            .await
            .unwrap();
        assert_eq!((stats.total, stats.ok), (4, 2));
        assert_eq!(stats.uptime, Some(50.0));

        let latest = storage.latest_results().await.unwrap();
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].result.status, Status::Degraded);
    }

    #[tokio::test]
    async fn invalid_config_yields_error_result() {
        let monitor = Monitor {
            id: 1,
            kind: CheckKind::Http,
            name: "bad".into(),
            config: json!({ "url": "nope" }),
            interval_sec: 60,
            enabled: true,
            tags: vec![],
            created_at: String::new(),
        };
        let result = run_web_monitor(&WebChecker::new(), &monitor, &CheckContext::default()).await;
        assert_eq!(result.status, Status::Error);
    }
}
