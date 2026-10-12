//! System health: CPU, memory, disks, network, processes, ports and services.

mod alerts;
mod ports;
mod services;

use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::{Components, Disks, Networks, ProcessesToUpdate, System};

pub use alerts::{evaluate, AlertMetric, AlertRule, NewAlertRule};
pub use ports::{listening_ports, ListeningPort};
pub use services::{list_services, parse_sc_query, ServiceInfo, ServiceState};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total: u64,
    pub available: u64,
    pub removable: bool,
}

impl DiskInfo {
    pub fn used_percent(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.total - self.available) as f64 * 100.0 / self.total as f64
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRates {
    /// Bytes per second.
    pub rx_per_sec: f64,
    pub tx_per_sec: f64,
    pub total_rx: u64,
    pub total_tx: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Temperature {
    pub label: String,
    pub celsius: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub ts: DateTime<Utc>,
    pub hostname: Option<String>,
    pub os: Option<String>,
    pub uptime_secs: u64,
    pub cpu: f32,
    pub cpu_per_core: Vec<f32>,
    pub cpu_name: String,
    pub mem_used: u64,
    pub mem_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub disks: Vec<DiskInfo>,
    pub net: NetworkRates,
    pub temperatures: Vec<Temperature>,
    pub process_count: usize,
}

impl Snapshot {
    pub fn mem_percent(&self) -> f64 {
        if self.mem_total == 0 {
            0.0
        } else {
            self.mem_used as f64 * 100.0 / self.mem_total as f64
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory: u64,
    pub exe: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessSort {
    Cpu,
    Memory,
}

/// Keeps sysinfo state between refreshes (CPU usage and network rates are
/// computed from the difference between two refreshes).
pub struct SystemMonitor {
    system: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    last_refresh: Instant,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_all();
        system.refresh_memory();
        Self {
            system,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last_refresh: Instant::now(),
        }
    }

    /// Refreshes everything and returns a snapshot. Call at most every second
    /// or so; CPU usage needs time between refreshes to be meaningful.
    pub fn snapshot(&mut self) -> Snapshot {
        let elapsed = self.last_refresh.elapsed().as_secs_f64().max(0.001);
        self.last_refresh = Instant::now();
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        self.disks.refresh_list();
        self.networks.refresh_list();
        self.components.refresh();

        let (rx, tx, total_rx, total_tx) =
            self.networks.iter().fold((0, 0, 0, 0), |acc, (_, data)| {
                (
                    acc.0 + data.received(),
                    acc.1 + data.transmitted(),
                    acc.2 + data.total_received(),
                    acc.3 + data.total_transmitted(),
                )
            });

        Snapshot {
            ts: Utc::now(),
            hostname: System::host_name(),
            os: System::long_os_version(),
            uptime_secs: System::uptime(),
            cpu: self.system.global_cpu_usage(),
            cpu_per_core: self.system.cpus().iter().map(|c| c.cpu_usage()).collect(),
            cpu_name: self
                .system
                .cpus()
                .first()
                .map(|c| c.brand().trim().to_owned())
                .unwrap_or_default(),
            mem_used: self.system.used_memory(),
            mem_total: self.system.total_memory(),
            swap_used: self.system.used_swap(),
            swap_total: self.system.total_swap(),
            disks: self
                .disks
                .iter()
                .filter(|d| d.total_space() > 0)
                .map(|d| DiskInfo {
                    name: d.name().to_string_lossy().into_owned(),
                    mount_point: d.mount_point().display().to_string(),
                    file_system: d.file_system().to_string_lossy().into_owned(),
                    total: d.total_space(),
                    available: d.available_space(),
                    removable: d.is_removable(),
                })
                .collect(),
            net: NetworkRates {
                rx_per_sec: rx as f64 / elapsed,
                tx_per_sec: tx as f64 / elapsed,
                total_rx,
                total_tx,
            },
            temperatures: self
                .components
                .iter()
                .filter(|c| c.temperature().is_finite() && c.temperature() > 0.0)
                .map(|c| Temperature {
                    label: c.label().to_owned(),
                    celsius: c.temperature(),
                })
                .collect(),
            process_count: self
                .system
                .processes()
                .values()
                .filter(|p| p.thread_kind().is_none())
                .count(),
        }
    }

    /// Top processes from the last refresh.
    pub fn processes(&self, sort: ProcessSort, limit: usize) -> Vec<ProcessInfo> {
        let mut processes: Vec<ProcessInfo> = self
            .system
            .processes()
            .values()
            // On Linux sysinfo also lists threads as tasks; keep real processes.
            .filter(|p| p.thread_kind().is_none())
            .map(|p| ProcessInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                cpu: p.cpu_usage(),
                memory: p.memory(),
                exe: p.exe().map(|e| e.display().to_string()),
            })
            .collect();
        match sort {
            ProcessSort::Cpu => processes.sort_by(|a, b| b.cpu.total_cmp(&a.cpu)),
            ProcessSort::Memory => processes.sort_by_key(|p| std::cmp::Reverse(p.memory)),
        }
        processes.truncate(limit);
        processes
    }

    /// Process name by pid, from the last refresh.
    pub fn process_name(&self, pid: u32) -> Option<String> {
        self.system
            .process(sysinfo::Pid::from_u32(pid))
            .map(|p| p.name().to_string_lossy().into_owned())
    }

    /// Kills a process. Returns false when it does not exist or cannot be killed.
    pub fn kill(&self, pid: u32) -> bool {
        self.system
            .process(sysinfo::Pid::from_u32(pid))
            .is_some_and(|p| p.kill())
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_snapshots() {
        let mut monitor = SystemMonitor::new();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let snapshot = monitor.snapshot();
        assert!(snapshot.mem_total > 0);
        assert!(!snapshot.cpu_per_core.is_empty());
        assert!((0.0..=100.0).contains(&snapshot.mem_percent()));
        assert!(snapshot.process_count > 0);

        let me = std::process::id();
        assert!(monitor.process_name(me).is_some());
        let top = monitor.processes(ProcessSort::Memory, 5);
        assert!(top.len() <= 5);
        assert!(top.windows(2).all(|w| w[0].memory >= w[1].memory));
    }

    #[test]
    fn disk_percent() {
        let disk = DiskInfo {
            name: "C:".into(),
            mount_point: "C:\\".into(),
            file_system: "NTFS".into(),
            total: 200,
            available: 50,
            removable: false,
        };
        assert_eq!(disk.used_percent(), 75.0);
    }
}
