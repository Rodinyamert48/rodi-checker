//! Self-update through tauri-plugin-updater.
//!
//! The updater is only compiled into release builds that were configured with
//! a signing key (see `tauri.updater.conf.json` and the release workflow);
//! otherwise the commands report it as unavailable.

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::error::CommandResult;
use crate::notify;

/// Whether this build was configured with an update endpoint and key.
pub struct UpdaterEnabled(pub bool);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    enabled: bool,
    current_version: String,
    available: Option<String>,
    notes: Option<String>,
}

fn enabled(app: &AppHandle) -> bool {
    app.try_state::<UpdaterEnabled>().is_some_and(|s| s.0)
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> CommandResult<UpdateInfo> {
    let current_version = app.package_info().version.to_string();
    if !enabled(&app) {
        return Ok(UpdateInfo {
            enabled: false,
            current_version,
            available: None,
            notes: None,
        });
    }
    let update = app.updater()?.check().await?;
    Ok(UpdateInfo {
        enabled: true,
        current_version,
        available: update.as_ref().map(|u| u.version.clone()),
        notes: update.and_then(|u| u.body),
    })
}

/// Downloads and installs the update, then restarts the app.
#[tauri::command]
pub async fn update_install(app: AppHandle) -> CommandResult<()> {
    if !enabled(&app) {
        return Err("updates are not enabled in this build".into());
    }
    let Some(update) = app.updater()?.check().await? else {
        return Err("no update available".into());
    };
    update.download_and_install(|_, _| {}, || {}).await?;
    app.restart();
}

/// Checks once on startup and notifies when a new version exists.
pub async fn check_on_startup(app: &AppHandle) {
    if !enabled(app) {
        return;
    }
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(err) => {
            tracing::warn!("updater unavailable: {err}");
            return;
        }
    };
    match updater.check().await {
        Ok(Some(update)) => {
            let title = notify::tr_en(app, "Güncelleme mevcut", "Update available");
            let body = notify::tr_en(
                app,
                format!(
                    "Rodi Checker {} yüklenebilir (Ayarlar → Güncellemeler)",
                    update.version
                ),
                format!(
                    "Rodi Checker {} can be installed (Settings → Updates)",
                    update.version
                ),
            );
            notify::send(app, &title, &body);
        }
        Ok(None) => {}
        Err(err) => tracing::warn!("update check failed: {err}"),
    }
}
