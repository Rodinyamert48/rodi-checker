use checker_core::storage::SystemSample;
use checker_core::system::{
    self, AlertRule, ListeningPort, NewAlertRule, ProcessInfo, ProcessSort, ServiceInfo, Snapshot,
};
use chrono::{Duration, Utc};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::CommandResult;
use crate::state::AppState;
use crate::system_runtime::{self, FiringAlert};

#[tauri::command]
pub async fn system_snapshot(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Snapshot> {
    let cached = state.last_snapshot.read().unwrap().clone();
    match cached {
        Some(snapshot) => Ok(snapshot),
        None => Ok(system_runtime::take_snapshot(&app).await),
    }
}

#[tauri::command]
pub fn system_processes(
    state: State<'_, AppState>,
    sort: ProcessSort,
    limit: usize,
) -> Vec<ProcessInfo> {
    state
        .system
        .lock()
        .unwrap()
        .processes(sort, limit.clamp(1, 500))
}

#[tauri::command]
pub fn system_kill(state: State<'_, AppState>, pid: u32) -> CommandResult<()> {
    if pid == std::process::id() {
        return Err("refusing to kill Rodi Checker itself".into());
    }
    if state.system.lock().unwrap().kill(pid) {
        Ok(())
    } else {
        Err(format!("cannot end process {pid}").into())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortInfo {
    #[serde(flatten)]
    port: ListeningPort,
    process: Option<String>,
}

#[tauri::command]
pub async fn system_ports(state: State<'_, AppState>) -> CommandResult<Vec<PortInfo>> {
    let ports = tauri::async_runtime::spawn_blocking(system::listening_ports).await??;
    let monitor = state.system.lock().unwrap();
    Ok(ports
        .into_iter()
        .map(|port| PortInfo {
            process: port.pids.first().and_then(|pid| monitor.process_name(*pid)),
            port,
        })
        .collect())
}

#[tauri::command]
pub async fn system_services() -> CommandResult<Vec<ServiceInfo>> {
    Ok(tauri::async_runtime::spawn_blocking(system::list_services).await??)
}

#[tauri::command]
pub async fn system_history(
    state: State<'_, AppState>,
    hours: i64,
) -> CommandResult<Vec<SystemSample>> {
    let since = Utc::now() - Duration::hours(hours.clamp(1, 24 * 90));
    Ok(state.storage.system_samples(since).await?)
}

#[tauri::command]
pub async fn system_alert_rules(state: State<'_, AppState>) -> CommandResult<Vec<AlertRule>> {
    Ok(state.storage.alert_rules().await?)
}

#[tauri::command]
pub async fn system_create_alert_rule(
    state: State<'_, AppState>,
    rule: NewAlertRule,
) -> CommandResult<AlertRule> {
    Ok(state.storage.create_alert_rule(&rule).await?)
}

#[tauri::command]
pub async fn system_toggle_alert_rule(
    state: State<'_, AppState>,
    id: i64,
    enabled: bool,
) -> CommandResult<()> {
    state.alerts_firing.lock().unwrap().remove(&id);
    Ok(state.storage.set_alert_rule_enabled(id, enabled).await?)
}

#[tauri::command]
pub async fn system_delete_alert_rule(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    state.alerts_firing.lock().unwrap().remove(&id);
    Ok(state.storage.delete_alert_rule(id).await?)
}

#[tauri::command]
pub fn system_firing(state: State<'_, AppState>) -> Vec<FiringAlert> {
    state
        .alerts_firing
        .lock()
        .unwrap()
        .iter()
        .map(|(rule_id, message)| FiringAlert {
            rule_id: *rule_id,
            message: message.clone(),
        })
        .collect()
}
