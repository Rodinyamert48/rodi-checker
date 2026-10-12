use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::error::CoreError;

/// Every kind of check the application knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Http,
    Ssl,
    Dns,
    Tcp,
    Proxy,
    FileIntegrity,
    System,
}

impl CheckKind {
    pub const ALL: [CheckKind; 7] = [
        CheckKind::Http,
        CheckKind::Ssl,
        CheckKind::Dns,
        CheckKind::Tcp,
        CheckKind::Proxy,
        CheckKind::FileIntegrity,
        CheckKind::System,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            CheckKind::Http => "http",
            CheckKind::Ssl => "ssl",
            CheckKind::Dns => "dns",
            CheckKind::Tcp => "tcp",
            CheckKind::Proxy => "proxy",
            CheckKind::FileIntegrity => "file_integrity",
            CheckKind::System => "system",
        }
    }
}

impl fmt::Display for CheckKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CheckKind {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CheckKind::ALL
            .into_iter()
            .find(|k| k.as_str() == s)
            .ok_or_else(|| CoreError::Invalid(format!("unknown check kind: {s}")))
    }
}

/// Outcome of a single check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Target is healthy.
    Up,
    /// Target answered but something is off (slow, cert expiring, threshold crossed).
    Degraded,
    /// Target is unreachable or failed its expectation.
    Down,
    /// The check itself could not run (bad input, cancelled, internal error).
    Error,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Up => "up",
            Status::Degraded => "degraded",
            Status::Down => "down",
            Status::Error => "error",
        }
    }

    pub fn is_ok(self) -> bool {
        matches!(self, Status::Up | Status::Degraded)
    }
}

impl FromStr for Status {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "up" => Ok(Status::Up),
            "degraded" => Ok(Status::Degraded),
            "down" => Ok(Status::Down),
            "error" => Ok(Status::Error),
            _ => Err(CoreError::Invalid(format!("unknown status: {s}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub kind: CheckKind,
    pub status: Status,
    pub latency_ms: Option<u64>,
    /// Short human readable summary.
    pub message: Option<String>,
    /// Kind specific structured data (cert info, dns records, ...).
    pub details: serde_json::Value,
    pub checked_at: DateTime<Utc>,
}

impl CheckResult {
    pub fn new(kind: CheckKind, status: Status) -> Self {
        Self {
            kind,
            status,
            latency_ms: None,
            message: None,
            details: serde_json::Value::Null,
            checked_at: Utc::now(),
        }
    }

    pub fn error(kind: CheckKind, message: impl Into<String>) -> Self {
        Self::new(kind, Status::Error).with_message(message)
    }

    pub fn with_latency(mut self, latency: Duration) -> Self {
        self.latency_ms = Some(latency.as_millis() as u64);
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }
}

/// Shared parameters for a check run.
#[derive(Debug, Clone)]
pub struct CheckContext {
    pub timeout: Duration,
    pub cancel: CancellationToken,
}

impl CheckContext {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            cancel: CancellationToken::new(),
        }
    }
}

impl Default for CheckContext {
    fn default() -> Self {
        Self::new(Duration::from_secs(10))
    }
}

/// A single kind of check (HTTP, proxy, hash, ...).
///
/// Implementations never return `Err`: failures are reported through
/// [`Status::Down`] / [`Status::Error`] so batch runs always yield one result
/// per target.
#[async_trait]
pub trait Checker: Send + Sync {
    type Target: Send + Sync + 'static;

    fn kind(&self) -> CheckKind;

    async fn check(&self, target: &Self::Target, ctx: &CheckContext) -> CheckResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_round_trips_through_str() {
        for kind in CheckKind::ALL {
            assert_eq!(kind.as_str().parse::<CheckKind>().unwrap(), kind);
        }
        assert!("nope".parse::<CheckKind>().is_err());
    }

    #[test]
    fn status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&Status::Degraded).unwrap(),
            "\"degraded\""
        );
        assert_eq!("down".parse::<Status>().unwrap(), Status::Down);
    }
}
