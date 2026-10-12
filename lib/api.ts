// Typed wrappers around the Tauri commands in `src-tauri/src/commands`.
//
// When the UI runs in a plain browser (`npm run dev` without Tauri) reads
// return empty data and settings use localStorage, so pages stay usable for
// UI work; actions that need the backend reject with a clear message.

import { invoke, isTauri } from "@tauri-apps/api/core";

import {
  DEFAULT_SETTINGS,
  type AppInfo,
  type AppSettings,
  type CheckKind,
  type CheckResult,
  type Incident,
  type Monitor,
  type MonitorStats,
  type NewMonitor,
  type StoredResult,
} from "./types";

const BROWSER_SETTINGS_KEY = "rodi-checker:settings";

export const inTauri = (): boolean => typeof window !== "undefined" && isTauri();

/** Invokes a command, or resolves `fallback` / rejects outside Tauri. */
export function call<T>(command: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (inTauri()) return invoke<T>(command, args);
  if (fallback !== undefined) return Promise.resolve(fallback);
  return Promise.reject(new Error("Bu işlem masaüstü uygulamasında çalışır / Requires the desktop app"));
}

function browserSettings(): AppSettings {
  try {
    const raw = window.localStorage.getItem(BROWSER_SETTINGS_KEY);
    return raw ? { ...DEFAULT_SETTINGS, ...JSON.parse(raw) } : DEFAULT_SETTINGS;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export const api = {
  appInfo: () =>
    call<AppInfo>("app_info", undefined, { name: "Rodi Checker", version: "dev", dataDir: "-", databasePath: "-" }),

  getSettings: (): Promise<AppSettings> => (inTauri() ? invoke("get_settings") : Promise.resolve(browserSettings())),

  saveSettings(settings: AppSettings): Promise<AppSettings> {
    if (inTauri()) return invoke("save_settings", { settings });
    try {
      window.localStorage.setItem(BROWSER_SETTINGS_KEY, JSON.stringify(settings));
    } catch {
      // Storage unavailable (private mode); keep the in-memory value.
    }
    return Promise.resolve(settings);
  },

  // ---- web monitors ----
  listMonitors: (kind?: CheckKind) => call<Monitor[]>("list_monitors", { kind: kind ?? null }, []),
  createMonitor: (monitor: NewMonitor) => call<Monitor>("create_monitor", { monitor }),
  updateMonitor: (id: number, monitor: NewMonitor) => call<Monitor>("update_monitor", { id, monitor }),
  deleteMonitor: (id: number) => call<void>("delete_monitor", { id }),
  runMonitorNow: (id: number) => call<CheckResult>("run_monitor_now", { id }),
  monitorResults: (id: number, hours: number) => call<StoredResult[]>("monitor_results", { id, hours }, []),
  monitorStats: (id: number, hours: number) =>
    call<MonitorStats>("monitor_stats", { id, hours }, { monitorId: id, total: 0, ok: 0, uptime: null, avgLatencyMs: null }),
  latestResults: () => call<StoredResult[]>("latest_results", undefined, []),
  listIncidents: (monitorId?: number, limit = 50) =>
    call<Incident[]>("list_incidents", { monitorId: monitorId ?? null, limit }, []),
};
