//! Operating system services: `sc.exe` on Windows, `systemctl` elsewhere.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Running,
    Stopped,
    Starting,
    Stopping,
    Paused,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: ServiceState,
}

/// Parses `sc query type= service state= all` output. Field names are not
/// localized; the state is read from its numeric code.
pub fn parse_sc_query(output: &str) -> Vec<ServiceInfo> {
    let mut services = Vec::new();
    let mut current: Option<ServiceInfo> = None;
    for line in output.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "SERVICE_NAME" => {
                services.extend(current.take());
                current = Some(ServiceInfo {
                    name: value.to_owned(),
                    display_name: value.to_owned(),
                    state: ServiceState::Unknown,
                });
            }
            "DISPLAY_NAME" => {
                if let Some(service) = current.as_mut() {
                    service.display_name = value.to_owned();
                }
            }
            "STATE" => {
                if let Some(service) = current.as_mut() {
                    service.state = match value.split_whitespace().next() {
                        Some("1") => ServiceState::Stopped,
                        Some("2") => ServiceState::Starting,
                        Some("3") => ServiceState::Stopping,
                        Some("4") => ServiceState::Running,
                        Some("5" | "6") => ServiceState::Starting,
                        Some("7") => ServiceState::Paused,
                        _ => ServiceState::Unknown,
                    };
                }
            }
            _ => {}
        }
    }
    services.extend(current);
    services.sort_by_key(|s| s.display_name.to_lowercase());
    services
}

/// Parses `systemctl list-units --type=service --all --no-legend --plain` output.
pub fn parse_systemctl(output: &str) -> Vec<ServiceInfo> {
    let mut services: Vec<ServiceInfo> = output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let unit = parts.next()?.trim_start_matches('●');
            let name = unit.strip_suffix(".service")?.to_owned();
            let _load = parts.next()?;
            let active = parts.next()?;
            let sub = parts.next()?;
            let description = parts.collect::<Vec<_>>().join(" ");
            let state = match (active, sub) {
                ("active", "running") => ServiceState::Running,
                ("activating", _) => ServiceState::Starting,
                ("deactivating", _) => ServiceState::Stopping,
                ("inactive" | "failed", _) | ("active", "exited") => ServiceState::Stopped,
                _ => ServiceState::Unknown,
            };
            Some(ServiceInfo {
                display_name: if description.is_empty() {
                    name.clone()
                } else {
                    description
                },
                name,
                state,
            })
        })
        .collect();
    services.sort_by_key(|s| s.display_name.to_lowercase());
    services
}

fn run(program: &str, args: &[&str]) -> Result<String> {
    let mut command = std::process::Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .output()
        .map_err(|err| CoreError::Invalid(format!("{program}: {err}")))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Lists the services of this machine (empty when unsupported).
pub fn list_services() -> Result<Vec<ServiceInfo>> {
    if cfg!(windows) {
        Ok(parse_sc_query(&run(
            "sc.exe",
            &["query", "type=", "service", "state=", "all"],
        )?))
    } else {
        match run(
            "systemctl",
            &[
                "list-units",
                "--type=service",
                "--all",
                "--no-legend",
                "--plain",
            ],
        ) {
            Ok(output) => Ok(parse_systemctl(&output)),
            Err(_) => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sc_output() {
        let output = "
SERVICE_NAME: Spooler
DISPLAY_NAME: Yazdırma Biriktiricisi
        TYPE               : 110  WIN32_OWN_PROCESS  (interactive)
        STATE              : 4  RUNNING
                                (STOPPABLE, NOT_PAUSABLE, ACCEPTS_SHUTDOWN)

SERVICE_NAME: wuauserv
DISPLAY_NAME: Windows Update
        TYPE               : 20  WIN32_SHARE_PROCESS
        STATE              : 1  STOPPED
";
        let services = parse_sc_query(output);
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "wuauserv");
        assert_eq!(services[0].state, ServiceState::Stopped);
        assert_eq!(services[1].display_name, "Yazdırma Biriktiricisi");
        assert_eq!(services[1].state, ServiceState::Running);
    }

    #[test]
    fn parses_systemctl_output() {
        let output = "ssh.service loaded active running OpenBSD Secure Shell server\n\
                      cron.service loaded inactive dead Regular background program processing daemon\n\
                      foo.mount loaded active mounted Foo\n";
        let services = parse_systemctl(output);
        assert_eq!(services.len(), 2);
        // Sorted by description.
        assert_eq!(services[0].name, "ssh");
        assert_eq!(services[0].state, ServiceState::Running);
        assert_eq!(services[1].state, ServiceState::Stopped);
    }
}
