use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use checker_core::secret::SecretBox;
use checker_core::system::{Snapshot, SystemMonitor};
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
    /// Set while a folder scan runs; only one scan at a time.
    pub files_busy: AtomicBool,
    pub files_cancel: Arc<AtomicBool>,
    /// Live folder watchers, by baseline id.
    pub watchers: Mutex<HashMap<i64, ::notify::RecommendedWatcher>>,
    pub system: Mutex<SystemMonitor>,
    pub last_snapshot: RwLock<Option<Snapshot>>,
    /// Currently violated alert rules and their messages.
    pub alerts_firing: Mutex<HashMap<i64, String>>,
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
