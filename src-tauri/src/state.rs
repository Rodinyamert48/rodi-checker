use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use checker_core::secret::SecretBox;
use checker_core::web::WebChecker;
use checker_core::{AppSettings, Scheduler, Storage};
use tokio_util::sync::CancellationToken;

/// Shared application state managed by Tauri.
pub struct AppState {
    pub storage: Storage,
    pub scheduler: Scheduler,
    pub web: WebChecker,
    pub secrets: SecretBox,
    /// Running proxy list checks, by list id.
    pub proxy_jobs: Mutex<HashMap<i64, CancellationToken>>,
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
