//! Background execution of scheduled web monitors.

use std::sync::Arc;
use std::time::Duration;

use checker_core::monitoring::{self, Transition};
use checker_core::storage::Monitor;
use checker_core::{CheckContext, CheckKind, CheckResult};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::notify;
use crate::state::AppState;

pub const RESULT_EVENT: &str = "monitor-result";

const WEB_KINDS: [CheckKind; 4] = [
    CheckKind::Http,
    CheckKind::Ssl,
    CheckKind::Dns,
    CheckKind::Tcp,
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultEvent<'a> {
    monitor_id: i64,
    result: &'a CheckResult,
    transition: Option<Transition>,
}

pub fn is_web(kind: CheckKind) -> bool {
    WEB_KINDS.contains(&kind)
}

/// Runs the monitor once, stores the result and notifies on status changes.
pub async fn run_once(app: &AppHandle, monitor: &Monitor) -> CheckResult {
    let state = app.state::<AppState>();
    let ctx = CheckContext::new(Duration::from_secs(state.settings().default_timeout_secs));
    let result = monitoring::run_web_monitor(&state.web, monitor, &ctx).await;

    let transition = match monitoring::record(&state.storage, monitor.id, &result).await {
        Ok(transition) => transition,
        Err(err) => {
            tracing::error!("storing result of monitor {} failed: {err}", monitor.id);
            None
        }
    };

    match transition {
        Some(Transition::WentDown) => {
            let reason = result.message.clone().unwrap_or_default();
            let title = notify::tr_en(
                app,
                format!("{} erişilemiyor", monitor.name),
                format!("{} is down", monitor.name),
            );
            notify::send(app, &title, &reason);
        }
        Some(Transition::Recovered) => {
            let title = notify::tr_en(
                app,
                format!("{} tekrar çalışıyor", monitor.name),
                format!("{} recovered", monitor.name),
            );
            notify::send(app, &title, result.message.as_deref().unwrap_or_default());
        }
        None => {}
    }

    crate::tray::refresh_status(app).await;
    let _ = app.emit(
        RESULT_EVENT,
        ResultEvent {
            monitor_id: monitor.id,
            result: &result,
            transition,
        },
    );
    result
}

/// (Re)schedules a monitor, or unschedules it when disabled.
pub fn schedule(app: &AppHandle, monitor: &Monitor) {
    let state = app.state::<AppState>();
    if !monitor.enabled || !is_web(monitor.kind) {
        state.scheduler.unschedule(monitor.id);
        return;
    }
    let app = app.clone();
    let monitor = Arc::new(monitor.clone());
    let interval = Duration::from_secs(monitor.interval_sec.into());
    state.scheduler.schedule(
        monitor.id,
        interval,
        Arc::new(move || {
            let app = app.clone();
            let monitor = monitor.clone();
            Box::pin(async move {
                run_once(&app, &monitor).await;
            })
        }),
    );
}

/// Schedules every enabled monitor; called on startup.
pub async fn schedule_all(app: &AppHandle) {
    let monitors = match app.state::<AppState>().storage.list_monitors(None).await {
        Ok(monitors) => monitors,
        Err(err) => {
            tracing::error!("loading monitors failed: {err}");
            return;
        }
    };
    for monitor in &monitors {
        schedule(app, monitor);
    }
}
