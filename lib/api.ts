// Typed wrappers around the Tauri commands in `src-tauri/src/commands`.
//
// When the UI runs in a plain browser (`npm run dev` without Tauri) a small
// localStorage-backed fallback keeps the pages usable for UI work.

import { invoke, isTauri } from "@tauri-apps/api/core";

import { DEFAULT_SETTINGS, type AppInfo, type AppSettings, type CheckKind, type Monitor } from "./types";

const BROWSER_SETTINGS_KEY = "rodi-checker:settings";

function browserSettings(): AppSettings {
  try {
    const raw = window.localStorage.getItem(BROWSER_SETTINGS_KEY);
    return raw ? { ...DEFAULT_SETTINGS, ...JSON.parse(raw) } : DEFAULT_SETTINGS;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export const api = {
  appInfo(): Promise<AppInfo> {
    if (!isTauri()) {
      return Promise.resolve({ name: "Rodi Checker", version: "dev", dataDir: "-", databasePath: "-" });
    }
    return invoke("app_info");
  },

  getSettings(): Promise<AppSettings> {
    if (!isTauri()) return Promise.resolve(browserSettings());
    return invoke("get_settings");
  },

  saveSettings(settings: AppSettings): Promise<AppSettings> {
    if (!isTauri()) {
      try {
        window.localStorage.setItem(BROWSER_SETTINGS_KEY, JSON.stringify(settings));
      } catch {
        // Storage unavailable (private mode); keep the in-memory value.
      }
      return Promise.resolve(settings);
    }
    return invoke("save_settings", { settings });
  },

  listMonitors(kind?: CheckKind): Promise<Monitor[]> {
    if (!isTauri()) return Promise.resolve([]);
    return invoke("list_monitors", { kind: kind ?? null });
  },
};
