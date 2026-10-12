//! Periodic system sampling, history storage and threshold alerts.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use checker_core::system::{self, AlertMetric, ServiceInfo, Snapshot};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::notify;
use crate::state::AppState;

pub const SNAPSHOT_EVENT: &str = "system-snapshot";
const TICK: Duration = Duration::from_secs(5);
/// One stored sample per minute.
const SAMPLE_EVERY: u32 = 12;
const SERVICES_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiringAlert {
    pub rule_id: i64,
    pub message: String,
}

/// Takes a fresh snapshot on a blocking thread.
pub async fn take_snapshot(app: &AppHandle) -> Snapshot {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let snapshot = state.system.lock().unwrap().snapshot();
        *state.last_snapshot.write().unwrap() = Some(snapshot.clone());
        snapshot
    })
    .await
    .expect("snapshot task panicked")
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut services_cache: Option<(Instant, Vec<ServiceInfo>)> = None;
        let mut tick: u32 = 0;
        loop {
            let snapshot = take_snapshot(&app).await;
            let _ = app.emit(SNAPSHOT_EVENT, &snapshot);
            let state = app.state::<AppState>();
            if tick.is_multiple_of(SAMPLE_EVERY) {
                if let Err(err) = state.storage.insert_system_sample(&snapshot).await {
                    tracing::warn!("storing system sample failed: {err}");
                }
            }
            if let Err(err) = check_alerts(&app, &snapshot, &mut services_cache).await {
                tracing::warn!("evaluating alerts failed: {err}");
            }
            tick = tick.wrapping_add(1);
            tokio::time::sleep(TICK).await;
        }
    });
}

async fn check_alerts(
    app: &AppHandle,
    snapshot: &Snapshot,
    services_cache: &mut Option<(Instant, Vec<ServiceInfo>)>,
) -> checker_core::Result<()> {
    let state = app.state::<AppState>();
    let rules: Vec<_> = state
        .storage
        .alert_rules()
        .await?
        .into_iter()
        .filter(|r| r.rule.enabled)
        .collect();

    let needs = |metric| rules.iter().any(|r| r.rule.metric == metric);
    let services = if needs(AlertMetric::Service) {
        let stale = services_cache
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() > SERVICES_TTL);
        if stale {
            let list = tauri::async_runtime::spawn_blocking(system::list_services)
                .await
                .map_err(|e| checker_core::CoreError::Invalid(e.to_string()))??;
            *services_cache = Some((Instant::now(), list));
        }
        services_cache
            .as_ref()
            .map(|(_, s)| s.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let ports: Vec<u16> = if needs(AlertMetric::Port) {
        tauri::async_runtime::spawn_blocking(system::listening_ports)
            .await
            .map_err(|e| checker_core::CoreError::Invalid(e.to_string()))??
            .into_iter()
            .filter(|p| p.protocol == "tcp")
            .map(|p| p.port)
            .collect()
    } else {
        Vec::new()
    };

    let mut now_firing = HashMap::new();
    for rule in &rules {
        if let Some(message) = system::evaluate(&rule.rule, snapshot, &services, &ports) {
            now_firing.insert(rule.id, message);
        }
    }

    let newly: Vec<String> = {
        let mut firing = state.alerts_firing.lock().unwrap();
        let newly = now_firing
            .iter()
            .filter(|(id, _)| !firing.contains_key(*id))
            .map(|(_, message)| message.clone())
            .collect();
        *firing = now_firing;
        newly
    };
    if !newly.is_empty() || state.alerts_firing.lock().unwrap().is_empty() {
        crate::tray::refresh_status(app).await;
    }
    for message in newly {
        let title = notify::tr_en(app, "Sistem uyarısı", "System alert");
        notify::send(app, &title, &message);
    }
    Ok(())
}
