-- Key/value application settings (JSON values).
CREATE TABLE settings (
    key        TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL
);

-- Scheduled web/API, SSL, DNS, TCP and system monitors.
CREATE TABLE monitors (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    kind         TEXT    NOT NULL,
    name         TEXT    NOT NULL,
    config_json  TEXT    NOT NULL DEFAULT '{}',
    interval_sec INTEGER NOT NULL DEFAULT 60,
    enabled      INTEGER NOT NULL DEFAULT 1,
    tags         TEXT    NOT NULL DEFAULT '',
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE check_results (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    monitor_id   INTEGER NOT NULL REFERENCES monitors(id) ON DELETE CASCADE,
    status       TEXT    NOT NULL,
    latency_ms   INTEGER,
    message      TEXT,
    details_json TEXT    NOT NULL DEFAULT 'null',
    checked_at   TEXT    NOT NULL
);
CREATE INDEX idx_check_results_monitor_time ON check_results(monitor_id, checked_at);

CREATE TABLE incidents (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    monitor_id  INTEGER NOT NULL REFERENCES monitors(id) ON DELETE CASCADE,
    started_at  TEXT    NOT NULL,
    resolved_at TEXT,
    cause       TEXT
);
CREATE INDEX idx_incidents_monitor ON incidents(monitor_id, started_at);

CREATE TABLE proxy_lists (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE proxies (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    list_id      INTEGER NOT NULL REFERENCES proxy_lists(id) ON DELETE CASCADE,
    protocol     TEXT    NOT NULL,
    host         TEXT    NOT NULL,
    port         INTEGER NOT NULL,
    username     TEXT,
    has_password INTEGER NOT NULL DEFAULT 0,
    status       TEXT,
    latency_ms   INTEGER,
    anonymity    TEXT,
    exit_ip      TEXT,
    country      TEXT,
    checked_at   TEXT,
    UNIQUE (list_id, protocol, host, port, username)
);

CREATE TABLE file_baselines (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL UNIQUE,
    root_path  TEXT NOT NULL,
    algorithm  TEXT NOT NULL,
    excludes   TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE file_entries (
    baseline_id INTEGER NOT NULL REFERENCES file_baselines(id) ON DELETE CASCADE,
    path        TEXT    NOT NULL,
    size        INTEGER NOT NULL,
    modified_at TEXT,
    hash        TEXT    NOT NULL,
    PRIMARY KEY (baseline_id, path)
);

CREATE TABLE system_samples (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    ts        TEXT NOT NULL,
    cpu       REAL NOT NULL,
    mem_used  INTEGER NOT NULL,
    mem_total INTEGER NOT NULL,
    disk_json TEXT NOT NULL DEFAULT '[]',
    net_json  TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_system_samples_ts ON system_samples(ts);

CREATE TABLE alert_rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    target_kind TEXT    NOT NULL,
    target_id   INTEGER,
    metric      TEXT    NOT NULL,
    op          TEXT    NOT NULL,
    threshold   REAL    NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1
);
