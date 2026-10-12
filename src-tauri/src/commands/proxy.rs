use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

use checker_core::proxy::{
    parse_list, GeoIp, ParseError, ProxyChecker, ProxyCheckerConfig, ProxyProtocol,
};
use checker_core::storage::{ProxyListSummary, StoredProxy};
use checker_core::{CheckContext, JobRunner};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::error::CommandResult;
use crate::notify;
use crate::state::AppState;

pub const PROGRESS_EVENT: &str = "proxy-progress";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    count: usize,
    duplicates: usize,
    error_count: usize,
    errors: Vec<ParseError>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    list_id: i64,
    done: usize,
    total: usize,
    working: usize,
    running: bool,
}

#[tauri::command]
pub fn proxy_preview(text: String, protocol: ProxyProtocol) -> ImportPreview {
    let parsed = parse_list(&text, protocol);
    ImportPreview {
        count: parsed.entries.len(),
        duplicates: parsed.duplicates,
        error_count: parsed.errors.len(),
        errors: parsed.errors.into_iter().take(20).collect(),
    }
}

#[tauri::command]
pub async fn proxy_import(
    state: State<'_, AppState>,
    name: String,
    text: String,
    protocol: ProxyProtocol,
) -> CommandResult<ProxyListSummary> {
    let parsed = parse_list(&text, protocol);
    if parsed.entries.is_empty() {
        return Err("no valid proxies found".into());
    }
    Ok(state
        .storage
        .create_proxy_list(&name, &parsed.entries, &state.secrets)
        .await?)
}

#[tauri::command]
pub async fn proxy_lists(state: State<'_, AppState>) -> CommandResult<Vec<ProxyListSummary>> {
    Ok(state.storage.proxy_lists().await?)
}

#[tauri::command]
pub async fn proxy_items(
    state: State<'_, AppState>,
    list_id: i64,
) -> CommandResult<Vec<StoredProxy>> {
    Ok(state.storage.proxies(list_id).await?)
}

#[tauri::command]
pub async fn proxy_delete_list(state: State<'_, AppState>, list_id: i64) -> CommandResult<()> {
    if let Some(token) = state.proxy_jobs.lock().unwrap().remove(&list_id) {
        token.cancel();
    }
    Ok(state.storage.delete_proxy_list(list_id).await?)
}

#[tauri::command]
pub async fn proxy_remove_dead(state: State<'_, AppState>, list_id: i64) -> CommandResult<u64> {
    Ok(state.storage.remove_dead_proxies(list_id).await?)
}

#[tauri::command]
pub fn proxy_running(state: State<'_, AppState>) -> Vec<i64> {
    state.proxy_jobs.lock().unwrap().keys().copied().collect()
}

#[tauri::command]
pub fn proxy_cancel(state: State<'_, AppState>, list_id: i64) {
    if let Some(token) = state.proxy_jobs.lock().unwrap().get(&list_id) {
        token.cancel();
    }
}

/// Starts checking every proxy of a list in the background.
#[tauri::command]
pub async fn proxy_check(
    app: AppHandle,
    state: State<'_, AppState>,
    list_id: i64,
) -> CommandResult<()> {
    let token = CancellationToken::new();
    {
        let mut jobs = state.proxy_jobs.lock().unwrap();
        if jobs.contains_key(&list_id) {
            return Err("this list is already being checked".into());
        }
        jobs.insert(list_id, token.clone());
    }

    let entries = match state.storage.proxy_entries(list_id, &state.secrets).await {
        Ok(entries) => entries,
        Err(err) => {
            state.proxy_jobs.lock().unwrap().remove(&list_id);
            return Err(err.into());
        }
    };

    tauri::async_runtime::spawn(async move {
        run_check(&app, list_id, entries, token).await;
        app.state::<AppState>()
            .proxy_jobs
            .lock()
            .unwrap()
            .remove(&list_id);
    });
    Ok(())
}

async fn run_check(
    app: &AppHandle,
    list_id: i64,
    entries: Vec<(i64, checker_core::proxy::ProxyEntry)>,
    token: CancellationToken,
) {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let timeout = Duration::from_secs(settings.default_timeout_secs);

    let geoip = settings
        .geoip_db_path
        .as_deref()
        .and_then(|path| match GeoIp::open(path) {
            Ok(geoip) => Some(geoip),
            Err(err) => {
                tracing::warn!("{err}");
                None
            }
        });
    let real_ip = match settings.proxy_judge_url.as_deref() {
        Some(judge) => ProxyChecker::detect_real_ip(judge, timeout).await,
        None => None,
    };
    let checker = Arc::new(ProxyChecker::new(ProxyCheckerConfig {
        judge_url: settings.proxy_judge_url.clone(),
        test_url: settings.proxy_test_url.clone(),
        real_ip,
        geoip,
    }));

    let ids: Vec<i64> = entries.iter().map(|(id, _)| *id).collect();
    let targets: Vec<_> = entries.into_iter().map(|(_, entry)| entry).collect();
    let total = targets.len();
    let mut ctx = CheckContext::new(timeout);
    ctx.cancel = token;

    let emit = |done, working, running| {
        let _ = app.emit(
            PROGRESS_EVENT,
            ProgressEvent {
                list_id,
                done,
                total,
                working,
                running,
            },
        );
    };
    emit(0, 0, true);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let storage = state.storage.clone();
    // Persist results off the runner's callback, which must stay synchronous.
    let writer = tokio::spawn(async move {
        while let Some((id, result)) = rx.recv().await {
            if let Err(err) = storage.save_proxy_result(id, &result).await {
                tracing::error!("saving proxy result failed: {err}");
            }
        }
    });

    let mut working = 0;
    let results = JobRunner::new(settings.default_concurrency)
        .run(checker, targets, &ctx, |progress, index, result| {
            if result.status.is_ok() {
                working += 1;
            }
            let _ = tx.send((ids[index], result.clone()));
            emit(progress.done, working, true);
        })
        .await;
    drop(tx);
    let _ = writer.await;

    emit(results.len(), working, false);
    let title = notify::tr_en(app, "Proxy testi tamamlandı", "Proxy check finished");
    let body = notify::tr_en(
        app,
        format!("{working}/{total} proxy çalışıyor"),
        format!("{working}/{total} proxies working"),
    );
    notify::send(app, &title, &body);
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Txt,
    Csv,
}

/// Writes a list to `path`. Returns the number of exported proxies.
#[tauri::command]
pub async fn proxy_export(
    state: State<'_, AppState>,
    list_id: i64,
    path: String,
    format: ExportFormat,
    only_working: bool,
    include_passwords: bool,
) -> CommandResult<usize> {
    let stored = state.storage.proxies(list_id).await?;
    let entries = state.storage.proxy_entries(list_id, &state.secrets).await?;
    let mut out = String::new();
    if matches!(format, ExportFormat::Csv) {
        out.push_str(
            "protocol,host,port,username,password,status,latency_ms,anonymity,exit_ip,country\n",
        );
    }
    let mut count = 0;
    for (proxy, (_, entry)) in stored.iter().zip(entries.iter()) {
        let ok = matches!(proxy.status.as_deref(), Some("up" | "degraded"));
        if only_working && !ok {
            continue;
        }
        count += 1;
        match format {
            ExportFormat::Txt => {
                let _ = writeln!(out, "{}", entry.to_line(include_passwords));
            }
            ExportFormat::Csv => {
                let password = if include_passwords {
                    entry.password.clone()
                } else {
                    None
                };
                let fields = [
                    Some(entry.protocol.as_str().to_owned()),
                    Some(entry.host.clone()),
                    Some(entry.port.to_string()),
                    entry.username.clone(),
                    password,
                    proxy.status.clone(),
                    proxy.latency_ms.map(|v| v.to_string()),
                    proxy.anonymity.clone(),
                    proxy.exit_ip.clone(),
                    proxy.country.clone(),
                ];
                let line: Vec<String> = fields
                    .iter()
                    .map(|f| csv_field(f.as_deref().unwrap_or("")))
                    .collect();
                let _ = writeln!(out, "{}", line.join(","));
            }
        }
    }
    tokio::fs::write(&path, out).await?;
    Ok(count)
}

pub fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
