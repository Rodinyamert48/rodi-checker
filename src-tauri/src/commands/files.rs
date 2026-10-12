use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use checker_core::files::{self, Diff, HashAlgorithm, ScanOptions, ScanReport};
use checker_core::storage::Baseline;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::CommandResult;
use crate::notify;
use crate::state::AppState;

pub const PROGRESS_EVENT: &str = "files-progress";
pub const CHANGE_EVENT: &str = "files-change";
const WATCHED_KEY: &str = "watched_baselines";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    done: usize,
    total: usize,
    running: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHashes {
    sha256: String,
    blake3: String,
    size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareResult {
    baseline: Baseline,
    diff: Diff,
    errors: Vec<(String, String)>,
    cancelled: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangeEvent {
    baseline_id: i64,
    baseline_name: String,
    paths: Vec<String>,
}

#[tauri::command]
pub async fn files_hash(path: String) -> CommandResult<FileHashes> {
    let result =
        tauri::async_runtime::spawn_blocking(move || -> checker_core::Result<FileHashes> {
            Ok(FileHashes {
                sha256: files::hash_file(&path, HashAlgorithm::Sha256)?,
                blake3: files::hash_file(&path, HashAlgorithm::Blake3)?,
                size: std::fs::metadata(&path)?.len(),
            })
        })
        .await?;
    Ok(result?)
}

#[tauri::command]
pub async fn files_baselines(state: State<'_, AppState>) -> CommandResult<Vec<Baseline>> {
    Ok(state.storage.baselines().await?)
}

#[tauri::command]
pub fn files_cancel(state: State<'_, AppState>) {
    state.files_cancel.store(true, Ordering::SeqCst);
}

/// Scans on a blocking thread, emitting throttled progress events.
async fn run_scan(
    app: &AppHandle,
    root: PathBuf,
    options: ScanOptions,
) -> CommandResult<ScanReport> {
    let state = app.state::<AppState>();
    if state.files_busy.swap(true, Ordering::SeqCst) {
        return Err("another scan is running".into());
    }
    state.files_cancel.store(false, Ordering::SeqCst);
    let cancel = state.files_cancel.clone();
    let emitter = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        files::scan(&root, &options, &cancel, |done, total| {
            // Every file for small scans, every 1% for big ones.
            if total < 200 || done % (total / 100).max(1) == 0 || done == total {
                let _ = emitter.emit(
                    PROGRESS_EVENT,
                    ProgressEvent {
                        done,
                        total,
                        running: true,
                    },
                );
            }
        })
    })
    .await;
    state.files_busy.store(false, Ordering::SeqCst);
    let _ = app.emit(
        PROGRESS_EVENT,
        ProgressEvent {
            done: 0,
            total: 0,
            running: false,
        },
    );
    Ok(result??)
}

#[tauri::command]
pub async fn files_create_baseline(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    root: String,
    algorithm: HashAlgorithm,
    excludes: Vec<String>,
) -> CommandResult<Baseline> {
    let options = ScanOptions {
        algorithm,
        excludes,
    };
    files::exclude_matcher(&options.excludes)?;
    let report = run_scan(&app, PathBuf::from(&root), options.clone()).await?;
    if report.cancelled {
        return Err("cancelled".into());
    }
    let baseline = state
        .storage
        .save_baseline(&name, &root, &options, &report.entries)
        .await?;
    restart_watch(&app, baseline.id).await;
    Ok(baseline)
}

#[tauri::command]
pub async fn files_compare(
    app: AppHandle,
    state: State<'_, AppState>,
    baseline_id: i64,
) -> CommandResult<CompareResult> {
    let baseline = state.storage.baseline(baseline_id).await?;
    let options = baseline.scan_options()?;
    let report = run_scan(&app, PathBuf::from(&baseline.root_path), options).await?;
    let previous = state.storage.baseline_entries(baseline_id).await?;
    let diff = files::compare(&previous, &report.entries);
    Ok(CompareResult {
        baseline,
        diff,
        errors: report.errors,
        cancelled: report.cancelled,
    })
}

#[tauri::command]
pub async fn files_delete_baseline(
    app: AppHandle,
    state: State<'_, AppState>,
    baseline_id: i64,
) -> CommandResult<()> {
    set_watch(&app, baseline_id, false).await?;
    Ok(state.storage.delete_baseline(baseline_id).await?)
}

#[tauri::command]
pub fn files_watching(state: State<'_, AppState>) -> Vec<i64> {
    state.watchers.lock().unwrap().keys().copied().collect()
}

#[tauri::command]
pub async fn files_watch(app: AppHandle, baseline_id: i64, enabled: bool) -> CommandResult<()> {
    set_watch(&app, baseline_id, enabled).await
}

async fn set_watch(app: &AppHandle, baseline_id: i64, enabled: bool) -> CommandResult<()> {
    let state = app.state::<AppState>();
    if enabled {
        let baseline = state.storage.baseline(baseline_id).await?;
        let watcher = start_watcher(app, &baseline)?;
        state.watchers.lock().unwrap().insert(baseline_id, watcher);
    } else {
        state.watchers.lock().unwrap().remove(&baseline_id);
    }
    let ids = files_watching(state.clone());
    state.storage.set_setting(WATCHED_KEY, &ids).await?;
    Ok(())
}

/// Re-creates the watcher of a baseline whose root may have changed.
async fn restart_watch(app: &AppHandle, baseline_id: i64) {
    let watching = app
        .state::<AppState>()
        .watchers
        .lock()
        .unwrap()
        .contains_key(&baseline_id);
    if watching {
        if let Err(err) = set_watch(app, baseline_id, true).await {
            tracing::warn!("restarting watcher failed: {err:?}");
        }
    }
}

/// Restores the watchers that were active when the app was closed.
pub async fn restore_watchers(app: &AppHandle) {
    let state = app.state::<AppState>();
    let ids: Vec<i64> = state
        .storage
        .get_setting(WATCHED_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    for id in ids {
        if let Err(err) = set_watch(app, id, true).await {
            tracing::warn!("restoring watcher for baseline {id} failed: {err:?}");
        }
    }
}

fn start_watcher(
    app: &AppHandle,
    baseline: &Baseline,
) -> CommandResult<::notify::RecommendedWatcher> {
    use ::notify::{RecursiveMode, Watcher};

    let root = PathBuf::from(&baseline.root_path);
    let excludes = files::exclude_matcher(&baseline.scan_options()?.excludes)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<PathBuf>();

    let mut watcher =
        ::notify::recommended_watcher(move |event: ::notify::Result<::notify::Event>| {
            if let Ok(event) = event {
                if event.kind.is_create() || event.kind.is_modify() || event.kind.is_remove() {
                    for path in event.paths {
                        let _ = tx.send(path);
                    }
                }
            }
        })?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let app = app.clone();
    let (baseline_id, baseline_name) = (baseline.id, baseline.name.clone());
    let base = if root.is_file() {
        root.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        root
    };
    // Batches changes for two seconds before notifying. The task ends when the
    // watcher (and with it the sender) is dropped.
    tauri::async_runtime::spawn(async move {
        while let Some(first) = rx.recv().await {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let mut paths = vec![first];
            while let Ok(path) = rx.try_recv() {
                paths.push(path);
            }
            let mut relative: Vec<String> = paths
                .iter()
                .map(|p| files::relative(&base, p))
                .filter(|rel| !rel.is_empty() && !excludes.is_match(rel))
                .collect();
            relative.sort();
            relative.dedup();
            if relative.is_empty() {
                continue;
            }
            let title = notify::tr_en(
                &app,
                format!("{baseline_name}: {} dosya değişti", relative.len()),
                format!("{baseline_name}: {} file(s) changed", relative.len()),
            );
            notify::send(
                &app,
                &title,
                &relative
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            let _ = app.emit(
                CHANGE_EVENT,
                ChangeEvent {
                    baseline_id,
                    baseline_name: baseline_name.clone(),
                    paths: relative,
                },
            );
        }
    });
    Ok(watcher)
}
