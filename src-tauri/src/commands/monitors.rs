use checker_core::storage::{Incident, Monitor, MonitorStats, NewMonitor, StoredResult};
use checker_core::web::WebTarget;
use checker_core::{CheckKind, CheckResult};
use chrono::{Duration, Utc};
use tauri::{AppHandle, State};

use crate::error::CommandResult;
use crate::monitor_runtime;
use crate::state::AppState;

fn validate(new: &NewMonitor) -> CommandResult<()> {
    if !monitor_runtime::is_web(new.kind) {
        return Err(format!("{} monitors are not supported", new.kind).into());
    }
    WebTarget::from_config(new.kind, &new.config)?;
    Ok(())
}

#[tauri::command]
pub async fn list_monitors(
    state: State<'_, AppState>,
    kind: Option<CheckKind>,
) -> CommandResult<Vec<Monitor>> {
    Ok(state.storage.list_monitors(kind).await?)
}

#[tauri::command]
pub async fn create_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
    monitor: NewMonitor,
) -> CommandResult<Monitor> {
    validate(&monitor)?;
    let created = state.storage.create_monitor(&monitor).await?;
    monitor_runtime::schedule(&app, &created);
    Ok(created)
}

#[tauri::command]
pub async fn update_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    monitor: NewMonitor,
) -> CommandResult<Monitor> {
    validate(&monitor)?;
    let updated = state.storage.update_monitor(id, &monitor).await?;
    monitor_runtime::schedule(&app, &updated);
    Ok(updated)
}

#[tauri::command]
pub async fn delete_monitor(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    state.scheduler.unschedule(id);
    Ok(state.storage.delete_monitor(id).await?)
}

#[tauri::command]
pub async fn run_monitor_now(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> CommandResult<CheckResult> {
    let monitor = state.storage.get_monitor(id).await?;
    Ok(monitor_runtime::run_once(&app, &monitor).await)
}

#[tauri::command]
pub async fn monitor_results(
    state: State<'_, AppState>,
    id: i64,
    hours: i64,
) -> CommandResult<Vec<StoredResult>> {
    let since = Utc::now() - Duration::hours(hours.clamp(1, 24 * 365));
    Ok(state.storage.results_since(id, since).await?)
}

#[tauri::command]
pub async fn monitor_stats(
    state: State<'_, AppState>,
    id: i64,
    hours: i64,
) -> CommandResult<MonitorStats> {
    let since = Utc::now() - Duration::hours(hours.clamp(1, 24 * 365));
    Ok(state.storage.monitor_stats(id, since).await?)
}

#[tauri::command]
pub async fn latest_results(state: State<'_, AppState>) -> CommandResult<Vec<StoredResult>> {
    Ok(state.storage.latest_results().await?)
}

#[tauri::command]
pub async fn list_incidents(
    state: State<'_, AppState>,
    monitor_id: Option<i64>,
    limit: Option<u32>,
) -> CommandResult<Vec<Incident>> {
    Ok(state
        .storage
        .list_incidents(monitor_id, limit.unwrap_or(50).min(500))
        .await?)
}
