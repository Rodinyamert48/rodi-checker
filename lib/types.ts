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
  autoUpdate: boolean;
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
  autoUpdate: true,
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

export type HashAlgorithm = "sha256" | "blake3";

export interface Baseline {
  id: number;
  name: string;
  rootPath: string;
  algorithm: HashAlgorithm;
  excludes: string;
  createdAt: string;
  fileCount: number;
  totalSize: number;
}

export interface FileEntry {
  path: string;
  size: number;
  modifiedAt: string | null;
  hash: string;
}

export interface ModifiedFile {
  path: string;
  oldHash: string;
  newHash: string;
  oldSize: number;
  newSize: number;
}

export interface CompareResult {
  baseline: Baseline;
  diff: { added: FileEntry[]; removed: FileEntry[]; modified: ModifiedFile[]; unchanged: number };
  errors: [string, string][];
  cancelled: boolean;
}

export interface FileHashes {
  sha256: string;
  blake3: string;
  size: number;
}

export interface FilesProgressEvent {
  done: number;
  total: number;
  running: boolean;
}

export interface FilesChangeEvent {
  baselineId: number;
  baselineName: string;
  paths: string[];
}

export interface DiskInfo {
  name: string;
  mountPoint: string;
  fileSystem: string;
  total: number;
  available: number;
  removable: boolean;
}

export interface SystemSnapshot {
  ts: string;
  hostname: string | null;
  os: string | null;
  uptimeSecs: number;
  cpu: number;
  cpuPerCore: number[];
  cpuName: string;
  memUsed: number;
  memTotal: number;
  swapUsed: number;
  swapTotal: number;
  disks: DiskInfo[];
  net: { rxPerSec: number; txPerSec: number; totalRx: number; totalTx: number };
  temperatures: { label: string; celsius: number }[];
  processCount: number;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  cpu: number;
  memory: number;
  exe: string | null;
}

export interface PortInfo {
  protocol: "tcp" | "udp";
  port: number;
  address: string;
  pids: number[];
  process: string | null;
}

export type ServiceState = "running" | "stopped" | "starting" | "stopping" | "paused" | "unknown";

export interface ServiceInfo {
  name: string;
  displayName: string;
  state: ServiceState;
}

export interface SystemSample {
  ts: string;
  cpu: number;
  memUsed: number;
  memTotal: number;
}

export type AlertMetric = "cpu" | "memory" | "disk" | "temperature" | "service" | "port";

export interface NewAlertRule {
  metric: AlertMetric;
  threshold: number;
  target: string | null;
  enabled: boolean;
}

export interface AlertRule extends NewAlertRule {
  id: number;
}

export interface FiringAlert {
  ruleId: number;
  message: string;
}

export interface DashboardSummary {
  monitors: { total: number; up: number; degraded: number; down: number; paused: number; pending: number };
  proxyLists: number;
  proxiesTotal: number;
  proxiesWorking: number;
  baselines: number;
  watching: number;
  cpu: number | null;
  memPercent: number | null;
  firing: FiringAlert[];
  incidents: Incident[];
}

export type ReportKind = "monitor_results" | "incidents" | "proxies" | "baselines" | "system_samples";
export type ReportFormat = "csv" | "json" | "html";

export interface ReportRequest {
  kind: ReportKind;
  hours: number;
  targetId: number | null;
}

export interface ReportPreview {
  table: { title: string; headers: string[]; rows: string[][] };
  totalRows: number;
}

export interface UpdateInfo {
  enabled: boolean;
  currentVersion: string;
  available: string | null;
  notes: string | null;
}
