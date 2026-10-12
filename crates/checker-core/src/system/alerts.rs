//! Threshold alert rules for system health.

use serde::{Deserialize, Serialize};

use super::{ServiceInfo, ServiceState, Snapshot};
use crate::error::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertMetric {
    /// Total CPU usage percent above threshold.
    Cpu,
    /// Memory usage percent above threshold.
    Memory,
    /// Usage percent of the disk mounted at `target` (or any disk) above threshold.
    Disk,
    /// Highest temperature (°C) above threshold.
    Temperature,
    /// Service named `target` not running.
    Service,
    /// Nothing listening on TCP port `target`.
    Port,
}

impl AlertMetric {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Disk => "disk",
            Self::Temperature => "temperature",
            Self::Service => "service",
            Self::Port => "port",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        serde_json::from_value(serde_json::Value::String(value.to_owned()))
            .map_err(|_| CoreError::Invalid(format!("unknown alert metric: {value}")))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAlertRule {
    pub metric: AlertMetric,
    #[serde(default)]
    pub threshold: f64,
    /// Disk mount point, service name or port, depending on the metric.
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl NewAlertRule {
    pub fn validate(&self) -> Result<()> {
        let target = self.target.as_deref().map(str::trim).unwrap_or_default();
        match self.metric {
            AlertMetric::Cpu | AlertMetric::Memory | AlertMetric::Disk
                if !(1.0..=100.0).contains(&self.threshold) =>
            {
                Err(CoreError::Invalid(
                    "threshold must be between 1 and 100".into(),
                ))
            }
            AlertMetric::Temperature if !(1.0..=150.0).contains(&self.threshold) => Err(
                CoreError::Invalid("threshold must be between 1 and 150 °C".into()),
            ),
            AlertMetric::Service if target.is_empty() => {
                Err(CoreError::Invalid("service name is required".into()))
            }
            AlertMetric::Port if target.parse::<u16>().map_or(true, |p| p == 0) => {
                Err(CoreError::Invalid("a valid port is required".into()))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRule {
    pub id: i64,
    #[serde(flatten)]
    pub rule: NewAlertRule,
}

/// Returns `Some(description)` when the rule is violated.
///
/// `services` / `ports` may be empty when the rule set does not need them.
pub fn evaluate(
    rule: &NewAlertRule,
    snapshot: &Snapshot,
    services: &[ServiceInfo],
    ports: &[u16],
) -> Option<String> {
    let target = rule
        .target
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());
    match rule.metric {
        AlertMetric::Cpu => (f64::from(snapshot.cpu) > rule.threshold)
            .then(|| format!("CPU %{:.0} > %{}", snapshot.cpu, rule.threshold)),
        AlertMetric::Memory => {
            let used = snapshot.mem_percent();
            (used > rule.threshold).then(|| format!("RAM %{used:.0} > %{}", rule.threshold))
        }
        AlertMetric::Disk => snapshot
            .disks
            .iter()
            .filter(|d| target.is_none_or(|t| d.mount_point.eq_ignore_ascii_case(t)))
            .find(|d| d.used_percent() > rule.threshold)
            .map(|d| {
                format!(
                    "{} %{:.0} > %{}",
                    d.mount_point,
                    d.used_percent(),
                    rule.threshold
                )
            }),
        AlertMetric::Temperature => snapshot
            .temperatures
            .iter()
            .filter(|t| f64::from(t.celsius) > rule.threshold)
            .max_by(|a, b| a.celsius.total_cmp(&b.celsius))
            .map(|t| format!("{} {:.0}°C > {}°C", t.label, t.celsius, rule.threshold)),
        AlertMetric::Service => {
            let name = target?;
            let service = services.iter().find(|s| {
                s.name.eq_ignore_ascii_case(name) || s.display_name.eq_ignore_ascii_case(name)
            });
            match service {
                Some(s) if s.state == ServiceState::Running => None,
                Some(s) => Some(format!("{} ({:?})", s.display_name, s.state)),
                None => Some(format!("{name} (not found)")),
            }
        }
        AlertMetric::Port => {
            let port: u16 = target?.parse().ok()?;
            (!ports.contains(&port)).then(|| format!("port {port} closed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::system::{DiskInfo, NetworkRates, Temperature};

    fn snapshot() -> Snapshot {
        Snapshot {
            ts: Utc::now(),
            hostname: None,
            os: None,
            uptime_secs: 0,
            cpu: 95.0,
            cpu_per_core: vec![95.0],
            cpu_name: String::new(),
            mem_used: 40,
            mem_total: 100,
            swap_used: 0,
            swap_total: 0,
            disks: vec![
                DiskInfo {
                    name: "C".into(),
                    mount_point: "C:\\".into(),
                    file_system: "NTFS".into(),
                    total: 100,
                    available: 5,
                    removable: false,
                },
                DiskInfo {
                    name: "D".into(),
                    mount_point: "D:\\".into(),
                    file_system: "NTFS".into(),
                    total: 100,
                    available: 90,
                    removable: false,
                },
            ],
            net: NetworkRates::default(),
            temperatures: vec![Temperature {
                label: "CPU".into(),
                celsius: 81.0,
            }],
            process_count: 1,
        }
    }

    fn rule(metric: AlertMetric, threshold: f64, target: Option<&str>) -> NewAlertRule {
        NewAlertRule {
            metric,
            threshold,
            target: target.map(Into::into),
            enabled: true,
        }
    }

    #[test]
    fn evaluates_thresholds() {
        let s = snapshot();
        assert!(evaluate(&rule(AlertMetric::Cpu, 90.0, None), &s, &[], &[]).is_some());
        assert!(evaluate(&rule(AlertMetric::Memory, 50.0, None), &s, &[], &[]).is_none());
        assert!(evaluate(&rule(AlertMetric::Disk, 90.0, None), &s, &[], &[])
            .unwrap()
            .starts_with("C:"));
        assert!(evaluate(&rule(AlertMetric::Disk, 90.0, Some("d:\\")), &s, &[], &[]).is_none());
        assert!(evaluate(&rule(AlertMetric::Temperature, 80.0, None), &s, &[], &[]).is_some());
    }

    #[test]
    fn evaluates_services_and_ports() {
        let s = snapshot();
        let services = vec![ServiceInfo {
            name: "Spooler".into(),
            display_name: "Print Spooler".into(),
            state: ServiceState::Stopped,
        }];
        assert!(evaluate(
            &rule(AlertMetric::Service, 0.0, Some("spooler")),
            &s,
            &services,
            &[]
        )
        .is_some());
        assert!(evaluate(
            &rule(AlertMetric::Service, 0.0, Some("missing")),
            &s,
            &services,
            &[]
        )
        .is_some());
        assert!(evaluate(
            &rule(AlertMetric::Port, 0.0, Some("443")),
            &s,
            &[],
            &[80, 443]
        )
        .is_none());
        assert!(evaluate(&rule(AlertMetric::Port, 0.0, Some("22")), &s, &[], &[80]).is_some());
    }

    #[test]
    fn validates_rules() {
        assert!(rule(AlertMetric::Cpu, 0.0, None).validate().is_err());
        assert!(rule(AlertMetric::Cpu, 90.0, None).validate().is_ok());
        assert!(rule(AlertMetric::Service, 0.0, Some(" "))
            .validate()
            .is_err());
        assert!(rule(AlertMetric::Port, 0.0, Some("http"))
            .validate()
            .is_err());
        assert!(rule(AlertMetric::Port, 0.0, Some("8080"))
            .validate()
            .is_ok());
        assert_eq!(AlertMetric::parse("disk").unwrap(), AlertMetric::Disk);
    }
}
