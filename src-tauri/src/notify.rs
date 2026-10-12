//! Desktop notifications, localized with the language setting.

use checker_core::settings::Language;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::AppState;

/// Picks the Turkish or English text according to the current settings.
pub fn tr_en(app: &AppHandle, tr: impl Into<String>, en: impl Into<String>) -> String {
    match app.state::<AppState>().settings().language {
        Language::Tr => tr.into(),
        Language::En => en.into(),
    }
}

/// Shows a desktop notification if notifications are enabled.
pub fn send(app: &AppHandle, title: &str, body: &str) {
    if !app.state::<AppState>().settings().notifications {
        return;
    }
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("notification failed: {err}");
    }
}
