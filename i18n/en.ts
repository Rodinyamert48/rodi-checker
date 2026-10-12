import type { Dictionary } from "./tr";

const en: Dictionary = {
  app: { name: "Rodi Checker", tagline: "All-in-one checking tool" },
  nav: {
    dashboard: "Dashboard",
    web: "Web / API Monitoring",
    proxy: "Proxy Checker",
    files: "File Integrity",
    system: "System Health",
    reports: "Reports",
    settings: "Settings",
  },
  status: { up: "Up", degraded: "Degraded", down: "Down", error: "Error" },
  common: {
    save: "Save",
    saved: "Saved",
    saving: "Saving…",
    reset: "Reset to defaults",
    loading: "Loading…",
    comingSoon: "Coming soon",
    phase: "Phase",
    plannedFeatures: "Planned features",
    monitors: "monitors",
    open: "Open",
  },
  dashboard: {
    title: "Dashboard",
    subtitle: "Overview of every checking module",
    version: "Version",
    database: "Database",
    recentEvents: "Recent events",
    noEvents: "No events yet. Add a check in any module to get started.",
  },
  modules: {
    web: {
      description: "Monitors availability, response time and SSL/DNS health of websites and APIs.",
      features: [
        "HTTP status code, response time and keyword checks",
        "SSL certificate expiry and chain validation",
        "DNS records (A, AAAA, MX, TXT, CNAME)",
        "TCP port reachability",
        "Uptime percentage, charts and incident history",
      ],
    },
    proxy: {
      description: "Bulk-tests your own proxy lists.",
      features: [
        "HTTP / HTTPS / SOCKS4 / SOCKS5 support",
        "Latency, exit IP and country",
        "Anonymity level: Transparent / Anonymous / Elite",
        "Paste, .txt and .csv import",
        "Filtering and exporting working proxies",
      ],
    },
    files: {
      description: "Hashes files and folders and detects changes.",
      features: [
        "Parallel SHA-256 and BLAKE3 hashing",
        "Save and compare baselines (added / removed / modified)",
        "Single file hash verification",
        "Live folder watching with notifications",
      ],
    },
    system: {
      description: "Monitors your computer's hardware and services.",
      features: [
        "Live CPU, RAM, disk and network usage",
        "Process list and listening ports",
        "Windows service status",
        "Threshold notifications and history charts",
      ],
    },
    reports: {
      description: "Exports check results.",
      features: ["CSV, JSON and HTML reports", "Date range and module filters"],
    },
  },
  settings: {
    title: "Settings",
    subtitle: "Application preferences",
    appearance: "Appearance",
    language: "Language",
    theme: "Theme",
    themes: { system: "System", light: "Light", dark: "Dark" },
    checks: "Checks",
    timeout: "Default timeout (s)",
    concurrency: "Default concurrency",
    retention: "Keep history (days)",
    proxyJudge: "Proxy anonymity judge URL",
    proxyJudgeHint: "An endpoint that echoes request headers. Leave empty to skip anonymity checks.",
    behavior: "Behavior",
    notifications: "Desktop notifications",
    minimizeToTray: "Minimize to system tray on close",
    startMinimized: "Start minimized",
    autostart: "Start with Windows",
  },
};

export default en;
