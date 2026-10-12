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
}

export interface CheckResult {
  kind: CheckKind;
  status: Status;
  latency_ms: number | null;
  message: string | null;
  details: unknown;
  checked_at: string;
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
};
