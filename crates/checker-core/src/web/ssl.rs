use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use rustls::pki_types::ServerName;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::checker::{CheckKind, CheckResult, Status};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SslTarget {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// Certificates expiring within this many days are reported as degraded.
    #[serde(default = "default_warn_days")]
    pub warn_days: i64,
}

fn default_port() -> u16 {
    443
}

fn default_warn_days() -> i64 {
    14
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub days_left: i64,
    pub sans: Vec<String>,
    pub serial: String,
}

impl CertInfo {
    pub fn from_der(der: &[u8]) -> Result<Self, String> {
        let (_, cert) = X509Certificate::from_der(der).map_err(|err| err.to_string())?;
        let to_utc = |t: x509_parser::time::ASN1Time| {
            DateTime::<Utc>::from_timestamp(t.timestamp(), 0).unwrap_or_default()
        };
        let not_after = to_utc(cert.validity().not_after);
        let sans = cert
            .subject_alternative_name()
            .ok()
            .flatten()
            .map(|ext| {
                ext.value
                    .general_names
                    .iter()
                    .filter_map(|name| match name {
                        GeneralName::DNSName(dns) => Some(dns.to_string()),
                        GeneralName::IPAddress(ip) => Some(format!("{ip:?}")),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(CertInfo {
            subject: cert.subject().to_string(),
            issuer: cert.issuer().to_string(),
            not_before: to_utc(cert.validity().not_before),
            not_after,
            days_left: (not_after - Utc::now()).num_days(),
            sans,
            serial: cert.raw_serial_as_string(),
        })
    }
}

fn connector() -> TlsConnector {
    static CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    let config = CONFIG.get_or_init(|| {
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .expect("tls versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
        Arc::new(config)
    });
    TlsConnector::from(config.clone())
}

pub(super) async fn check(target: &SslTarget, timeout: Duration) -> CheckResult {
    let kind = CheckKind::Ssl;
    let host = target.host.trim().to_owned();
    let started = Instant::now();

    let server_name = match ServerName::try_from(host.clone()) {
        Ok(name) => name,
        Err(err) => return CheckResult::error(kind, format!("invalid host: {err}")),
    };

    let handshake = async {
        let tcp = TcpStream::connect((host.as_str(), target.port))
            .await
            .map_err(|err| format!("connection failed: {err}"))?;
        let tls = connector()
            .connect(server_name, tcp)
            .await
            .map_err(|err| format!("TLS error: {err}"))?;
        let (_, session) = tls.get_ref();
        let protocol = session.protocol_version().map(|v| format!("{v:?}"));
        let leaf = session
            .peer_certificates()
            .and_then(|certs| certs.first())
            .ok_or_else(|| "server sent no certificate".to_owned())?;
        let info = CertInfo::from_der(leaf.as_ref())?;
        Ok::<_, String>((info, protocol))
    };

    let (info, protocol) = match tokio::time::timeout(timeout, handshake).await {
        Ok(Ok(value)) => value,
        Ok(Err(message)) => {
            return CheckResult::new(kind, Status::Down)
                .with_latency(started.elapsed())
                .with_message(message)
        }
        Err(_) => {
            return CheckResult::new(kind, Status::Down)
                .with_latency(started.elapsed())
                .with_message("timeout")
        }
    };

    let (status, message) = classify(&info, target.warn_days);
    let mut details = serde_json::to_value(&info).unwrap_or_default();
    details["protocol"] = protocol.into();
    CheckResult::new(kind, status)
        .with_latency(started.elapsed())
        .with_message(message)
        .with_details(details)
}

fn classify(info: &CertInfo, warn_days: i64) -> (Status, String) {
    if info.days_left < 0 {
        (Status::Down, "certificate expired".into())
    } else if info.days_left < warn_days {
        (
            Status::Degraded,
            format!("certificate expires in {} days", info.days_left),
        )
    } else {
        (Status::Up, format!("{} days left", info.days_left))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cert_der(not_before: (i32, u8, u8), not_after: (i32, u8, u8)) -> Vec<u8> {
        let mut params = rcgen::CertificateParams::new(vec!["example.test".into()]).unwrap();
        params.not_before = rcgen::date_time_ymd(not_before.0, not_before.1, not_before.2);
        params.not_after = rcgen::date_time_ymd(not_after.0, not_after.1, not_after.2);
        let key = rcgen::KeyPair::generate().unwrap();
        params.self_signed(&key).unwrap().der().to_vec()
    }

    #[test]
    fn parses_certificate_details() {
        let info = CertInfo::from_der(&cert_der((2020, 1, 1), (2099, 1, 1))).unwrap();
        assert_eq!(info.sans, vec!["example.test"]);
        assert!(info.days_left > 365);
        assert_eq!(info.not_after.format("%Y-%m-%d").to_string(), "2099-01-01");
        assert_eq!(classify(&info, 14).0, Status::Up);
    }

    #[test]
    fn classifies_expiry() {
        let mut info = CertInfo::from_der(&cert_der((2000, 1, 1), (2001, 1, 1))).unwrap();
        assert!(info.days_left < 0);
        assert_eq!(classify(&info, 14).0, Status::Down);
        info.days_left = 5;
        assert_eq!(classify(&info, 14).0, Status::Degraded);
    }

    #[tokio::test]
    async fn unreachable_host_is_down() {
        let target = SslTarget {
            host: "127.0.0.1".into(),
            port: 1,
            warn_days: 14,
        };
        let result = check(&target, Duration::from_secs(2)).await;
        assert_eq!(result.status, Status::Down);
    }
}
