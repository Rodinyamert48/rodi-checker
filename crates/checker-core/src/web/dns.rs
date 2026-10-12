use std::time::{Duration, Instant};

use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::proto::rr::RecordType as HickoryType;
use hickory_resolver::TokioAsyncResolver;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::checker::{CheckKind, CheckResult, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum RecordType {
    #[default]
    A,
    Aaaa,
    Mx,
    Txt,
    Cname,
    Ns,
}

impl RecordType {
    fn to_hickory(self) -> HickoryType {
        match self {
            RecordType::A => HickoryType::A,
            RecordType::Aaaa => HickoryType::AAAA,
            RecordType::Mx => HickoryType::MX,
            RecordType::Txt => HickoryType::TXT,
            RecordType::Cname => HickoryType::CNAME,
            RecordType::Ns => HickoryType::NS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsTarget {
    pub domain: String,
    #[serde(default)]
    pub record_type: RecordType,
    /// When set, one of the returned records must contain this value.
    #[serde(default)]
    pub expected: Option<String>,
}

fn resolver(timeout: Duration) -> TokioAsyncResolver {
    let mut opts = ResolverOpts::default();
    opts.timeout = timeout;
    opts.attempts = 1;
    opts.cache_size = 0;
    match hickory_resolver::system_conf::read_system_conf() {
        Ok((config, mut system_opts)) => {
            system_opts.timeout = opts.timeout;
            system_opts.attempts = opts.attempts;
            system_opts.cache_size = 0;
            TokioAsyncResolver::tokio(config, system_opts)
        }
        Err(_) => TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), opts),
    }
}

pub(super) async fn check(target: &DnsTarget, timeout: Duration) -> CheckResult {
    let kind = CheckKind::Dns;
    let started = Instant::now();
    let domain = target.domain.trim();
    let lookup = resolver(timeout)
        .lookup(domain, target.record_type.to_hickory())
        .await;
    let latency = started.elapsed();

    let records: Vec<String> = match lookup {
        Ok(lookup) => lookup
            .record_iter()
            .filter(|r| r.record_type() == target.record_type.to_hickory())
            .filter_map(|r| r.data().map(|d| d.to_string()))
            .collect(),
        Err(err) => {
            return CheckResult::new(kind, Status::Down)
                .with_latency(latency)
                .with_message(err.to_string());
        }
    };

    let details = json!({ "recordType": target.record_type, "records": records });
    let result = CheckResult::new(kind, Status::Up)
        .with_latency(latency)
        .with_details(details);

    if records.is_empty() {
        return CheckResult {
            status: Status::Down,
            ..result
        }
        .with_message("no records");
    }
    if let Some(expected) = target
        .expected
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
    {
        let expected = expected.to_lowercase();
        if !records.iter().any(|r| r.to_lowercase().contains(&expected)) {
            return CheckResult {
                status: Status::Down,
                ..result
            }
            .with_message(format!("expected value {expected:?} not found"));
        }
    }
    let count = records.len();
    result.with_message(format!("{count} record(s)"))
}
