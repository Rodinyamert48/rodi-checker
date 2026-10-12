use checker_core::storage::Incident;
use checker_core::Status;
use serde::Serialize;
use tauri::State;

use crate::error::CommandResult;
use crate::monitor_runtime;
use crate::state::AppState;
use crate::system_runtime::FiringAlert;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MonitorCounts {
    total: usize,
    up: usize,
    degraded: usize,
    down: usize,
    paused: usize,
    pending: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    monitors: MonitorCounts,
    proxy_lists: usize,
    proxies_total: i64,
    proxies_working: i64,
    baselines: usize,
    watching: usize,
    cpu: Option<f32>,
    mem_percent: Option<f64>,
    firing: Vec<FiringAlert>,
    incidents: Vec<Incident>,
}

#[tauri::command]
pub async fn dashboard_summary(state: State<'_, AppState>) -> CommandResult<DashboardSummary> {
    let monitors = state.storage.list_monitors(None).await?;
    let latest = state.storage.latest_results().await?;
    let mut counts = MonitorCounts::default();
    for monitor in monitors.iter().filter(|m| monitor_runtime::is_web(m.kind)) {
        counts.total += 1;
        if !monitor.enabled {
            counts.paused += 1;
            continue;
        }
        match latest
            .iter()
            .find(|r| r.monitor_id == monitor.id)
            .map(|r| r.result.status)
        {
            Some(Status::Up) => counts.up += 1,
            Some(Status::Degraded) => counts.degraded += 1,
            Some(Status::Down | Status::Error) => counts.down += 1,
            None => counts.pending += 1,
        }
    }

    let lists = state.storage.proxy_lists().await?;
    let snapshot = state.last_snapshot.read().unwrap().clone();
    let firing = state
        .alerts_firing
        .lock()
        .unwrap()
        .iter()
        .map(|(rule_id, message)| FiringAlert {
            rule_id: *rule_id,
            message: message.clone(),
        })
        .collect();

    let baselines = state.storage.baselines().await?.len();
    let incidents = state.storage.list_incidents(None, 10).await?;
    let watching = state.watchers.lock().unwrap().len();

    Ok(DashboardSummary {
        monitors: counts,
        proxy_lists: lists.len(),
        proxies_total: lists.iter().map(|l| l.total).sum(),
        proxies_working: lists.iter().map(|l| l.working).sum(),
        baselines,
        watching,
        cpu: snapshot.as_ref().map(|s| s.cpu),
        mem_percent: snapshot.as_ref().map(|s| s.mem_percent()),
        firing,
        incidents,
    })
}
