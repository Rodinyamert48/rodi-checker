use std::path::PathBuf;
use std::sync::RwLock;

use checker_core::web::WebChecker;
use checker_core::{AppSettings, Scheduler, Storage};

/// Shared application state managed by Tauri.
pub struct AppState {
    pub storage: Storage,
    pub scheduler: Scheduler,
    pub web: WebChecker,
    pub data_dir: PathBuf,
    pub database_path: PathBuf,
    /// Cached copy of the persisted settings for synchronous access
    /// (window events, tray).
    pub settings: RwLock<AppSettings>,
}

impl AppState {
    pub fn settings(&self) -> AppSettings {
        self.settings
            .read()
            .expect("settings lock poisoned")
            .clone()
    }

    pub fn set_settings(&self, settings: AppSettings) {
        *self.settings.write().expect("settings lock poisoned") = settings;
    }
}
