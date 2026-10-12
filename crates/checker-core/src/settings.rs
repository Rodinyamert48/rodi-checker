use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::storage::Storage;

const SETTINGS_KEY: &str = "app";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Tr,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// User preferences, stored as a single JSON document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub language: Language,
    pub theme: Theme,
    /// Check results older than this are pruned.
    pub retention_days: u32,
    pub default_timeout_secs: u64,
    pub default_concurrency: usize,
    pub notifications: bool,
    pub start_minimized: bool,
    pub autostart: bool,
    pub minimize_to_tray: bool,
    /// Endpoint that echoes request headers, used for proxy anonymity checks.
    pub proxy_judge_url: Option<String>,
    /// Connectivity test URL used when no judge is configured.
    pub proxy_test_url: Option<String>,
    /// Optional MaxMind GeoLite2 .mmdb file for proxy country lookup.
    pub geoip_db_path: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Tr,
            theme: Theme::System,
            retention_days: 30,
            default_timeout_secs: 10,
            default_concurrency: 50,
            notifications: true,
            start_minimized: false,
            autostart: false,
            minimize_to_tray: true,
            proxy_judge_url: None,
            proxy_test_url: None,
            geoip_db_path: None,
        }
    }
}

impl AppSettings {
    /// Clamps values into sane ranges.
    pub fn normalized(mut self) -> Self {
        self.retention_days = self.retention_days.clamp(1, 3650);
        self.default_timeout_secs = self.default_timeout_secs.clamp(1, 300);
        self.default_concurrency = self.default_concurrency.clamp(1, 1000);
        let clean =
            |value: Option<String>| value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        self.proxy_judge_url = clean(self.proxy_judge_url);
        self.proxy_test_url = clean(self.proxy_test_url);
        self.geoip_db_path = clean(self.geoip_db_path);
        self
    }

    pub async fn load(storage: &Storage) -> Result<Self> {
        Ok(storage
            .get_setting::<Self>(SETTINGS_KEY)
            .await?
            .unwrap_or_default()
            .normalized())
    }

    pub async fn save(self, storage: &Storage) -> Result<Self> {
        let settings = self.normalized();
        storage.set_setting(SETTINGS_KEY, &settings).await?;
        Ok(settings)
    }
}
