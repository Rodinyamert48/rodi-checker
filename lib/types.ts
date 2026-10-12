// Mirrors the serde types in `crates/checker-core`. Keep in sync.

export type CheckKind = "http" | "ssl" | "dns" | "tcp" | "proxy" | "file_integrity" | "system";
export type Status = "up" | "degraded" | "down" | "error";
export type Language = "tr" | "en";
export type Theme = "system" | "light" | "dark";

export interface AppSettings {
  language: Language;
  theme: Theme;
  retentionDays: number;
  defaultTimeoutSecs: number;
  defaultConcurrency: number;
  notifications: boolean;
  startMinimized: boolean;
  autostart: boolean;
  minimizeToTray: boolean;
  proxyJudgeUrl: string | null;
  proxyTestUrl: string | null;
  geoipDbPath: string | null;
}

export interface CheckResult {
  kind: CheckKind;
  status: Status;
  latencyMs: number | null;
  message: string | null;
  details: Record<string, unknown> | null;
  checkedAt: string;
}

export interface StoredResult extends CheckResult {
  id: number;
  monitorId: number;
}

export type WebKind = "http" | "ssl" | "dns" | "tcp";

export interface NewMonitor {
  kind: CheckKind;
  name: string;
  config: Record<string, unknown>;
  intervalSec: number;
  enabled: boolean;
  tags: string[];
}

export interface MonitorStats {
  monitorId: number;
  total: number;
  ok: number;
  uptime: number | null;
  avgLatencyMs: number | null;
}

export interface Incident {
  id: number;
  monitorId: number;
  monitorName: string;
  startedAt: string;
  resolvedAt: string | null;
  cause: string | null;
}

export interface MonitorResultEvent {
  monitorId: number;
  result: CheckResult;
  transition: "went_down" | "recovered" | null;
}

export interface Monitor {
  id: number;
  kind: CheckKind;
  name: string;
  config: Record<string, unknown>;
  intervalSec: number;
  enabled: boolean;
  tags: string[];
  createdAt: string;
}

export interface AppInfo {
  name: string;
  version: string;
  dataDir: string;
  databasePath: string;
}

export const DEFAULT_SETTINGS: AppSettings = {
  language: "tr",
  theme: "system",
  retentionDays: 30,
  defaultTimeoutSecs: 10,
  defaultConcurrency: 50,
  notifications: true,
  startMinimized: false,
  autostart: false,
  minimizeToTray: true,
  proxyJudgeUrl: null,
  proxyTestUrl: null,
  geoipDbPath: null,
};

export type ProxyProtocol = "http" | "https" | "socks4" | "socks5";
export type Anonymity = "transparent" | "anonymous" | "elite";

export interface ProxyListSummary {
  id: number;
  name: string;
  createdAt: string;
  total: number;
  checked: number;
  working: number;
}

export interface StoredProxy {
  id: number;
  listId: number;
  protocol: ProxyProtocol;
  host: string;
  port: number;
  username: string | null;
  hasPassword: boolean;
  status: Status | null;
  latencyMs: number | null;
  anonymity: Anonymity | null;
  exitIp: string | null;
  country: string | null;
  message: string | null;
  checkedAt: string | null;
}

export interface ImportPreview {
  count: number;
  duplicates: number;
  errorCount: number;
  errors: { line: number; text: string; reason: string }[];
}

export interface ProxyProgressEvent {
  listId: number;
  done: number;
  total: number;
  working: number;
  running: boolean;
}
