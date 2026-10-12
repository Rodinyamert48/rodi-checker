// Typed wrappers around the Tauri commands in `src-tauri/src/commands`.
//
// When the UI runs in a plain browser (`npm run dev` without Tauri) reads
// return empty data and settings use localStorage, so pages stay usable for
// UI work; actions that need the backend reject with a clear message.

import { invoke, isTauri } from "@tauri-apps/api/core";

import {
  DEFAULT_SETTINGS,
  type AlertRule,
  type FiringAlert,
  type NewAlertRule,
  type PortInfo,
  type ProcessInfo,
  type ServiceInfo,
  type SystemSample,
  type SystemSnapshot,
  type AppInfo,
  type AppSettings,
  type CheckKind,
  type Baseline,
  type CheckResult,
  type CompareResult,
  type FileHashes,
  type HashAlgorithm,
  type ImportPreview,
  type ProxyListSummary,
  type ProxyProtocol,
  type StoredProxy,
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

  readTextFile: (path: string) => call<string>("read_text_file", { path }),

  // ---- proxies ----
  proxyPreview: (text: string, protocol: ProxyProtocol) => call<ImportPreview>("proxy_preview", { text, protocol }),
  proxyImport: (name: string, text: string, protocol: ProxyProtocol) =>
    call<ProxyListSummary>("proxy_import", { name, text, protocol }),
  proxyLists: () => call<ProxyListSummary[]>("proxy_lists", undefined, []),
  proxyItems: (listId: number) => call<StoredProxy[]>("proxy_items", { listId }, []),
  proxyDeleteList: (listId: number) => call<void>("proxy_delete_list", { listId }),
  proxyRemoveDead: (listId: number) => call<number>("proxy_remove_dead", { listId }),
  proxyRunning: () => call<number[]>("proxy_running", undefined, []),
  proxyCheck: (listId: number) => call<void>("proxy_check", { listId }),
  proxyCancel: (listId: number) => call<void>("proxy_cancel", { listId }),
  proxyExport: (listId: number, path: string, format: "txt" | "csv", onlyWorking: boolean, includePasswords: boolean) =>
    call<number>("proxy_export", { listId, path, format, onlyWorking, includePasswords }),

  // ---- file integrity ----
  filesHash: (path: string) => call<FileHashes>("files_hash", { path }),
  filesBaselines: () => call<Baseline[]>("files_baselines", undefined, []),
  filesCancel: () => call<void>("files_cancel"),
  filesCreateBaseline: (name: string, root: string, algorithm: HashAlgorithm, excludes: string[]) =>
    call<Baseline>("files_create_baseline", { name, root, algorithm, excludes }),
  filesCompare: (baselineId: number) => call<CompareResult>("files_compare", { baselineId }),
  filesDeleteBaseline: (baselineId: number) => call<void>("files_delete_baseline", { baselineId }),
  filesWatching: () => call<number[]>("files_watching", undefined, []),
  filesWatch: (baselineId: number, enabled: boolean) => call<void>("files_watch", { baselineId, enabled }),

  // ---- system health ----
  systemSnapshot: () => call<SystemSnapshot>("system_snapshot"),
  systemProcesses: (sort: "cpu" | "memory", limit = 30) => call<ProcessInfo[]>("system_processes", { sort, limit }, []),
  systemKill: (pid: number) => call<void>("system_kill", { pid }),
  systemPorts: () => call<PortInfo[]>("system_ports", undefined, []),
  systemServices: () => call<ServiceInfo[]>("system_services", undefined, []),
  systemHistory: (hours: number) => call<SystemSample[]>("system_history", { hours }, []),
  alertRules: () => call<AlertRule[]>("system_alert_rules", undefined, []),
  createAlertRule: (rule: NewAlertRule) => call<AlertRule>("system_create_alert_rule", { rule }),
  toggleAlertRule: (id: number, enabled: boolean) => call<void>("system_toggle_alert_rule", { id, enabled }),
  deleteAlertRule: (id: number) => call<void>("system_delete_alert_rule", { id }),
  firingAlerts: () => call<FiringAlert[]>("system_firing", undefined, []),
};
