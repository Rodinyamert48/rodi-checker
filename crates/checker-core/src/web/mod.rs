//! Web / API monitoring: HTTP, SSL certificate, DNS and TCP port checks.

mod dns;
mod http;
mod ssl;
mod tcp;

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use dns::{DnsTarget, RecordType};
pub use http::HttpTarget;
pub use ssl::{CertInfo, SslTarget};
pub use tcp::TcpTarget;

use crate::checker::{CheckContext, CheckKind, CheckResult, Checker};
use crate::error::{CoreError, Result};

/// One web check target. The JSON form is `{"kind": "http", ...config}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WebTarget {
    Http(HttpTarget),
    Ssl(SslTarget),
    Dns(DnsTarget),
    Tcp(TcpTarget),
}

impl WebTarget {
    /// Builds a target from a monitor's kind and config JSON.
    pub fn from_config(kind: CheckKind, config: &serde_json::Value) -> Result<Self> {
        let mut object = match config {
            serde_json::Value::Object(map) => map.clone(),
            serde_json::Value::Null => serde_json::Map::new(),
            _ => {
                return Err(CoreError::Invalid(
                    "monitor config must be an object".into(),
                ))
            }
        };
        if !matches!(
            kind,
            CheckKind::Http | CheckKind::Ssl | CheckKind::Dns | CheckKind::Tcp
        ) {
            return Err(CoreError::Invalid(format!("{kind} is not a web check")));
        }
        object.insert("kind".into(), kind.as_str().into());
        let target: WebTarget = serde_json::from_value(object.into())
            .map_err(|err| CoreError::Invalid(format!("invalid {kind} config: {err}")))?;
        target.validate()?;
        Ok(target)
    }

    pub fn kind(&self) -> CheckKind {
        match self {
            WebTarget::Http(_) => CheckKind::Http,
            WebTarget::Ssl(_) => CheckKind::Ssl,
            WebTarget::Dns(_) => CheckKind::Dns,
            WebTarget::Tcp(_) => CheckKind::Tcp,
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            WebTarget::Http(t) => t.validate(),
            WebTarget::Ssl(t) => validate_host(&t.host),
            WebTarget::Dns(t) => validate_host(&t.domain),
            WebTarget::Tcp(t) => {
                validate_host(&t.host)?;
                if t.port == 0 {
                    return Err(CoreError::Invalid(
                        "port must be between 1 and 65535".into(),
                    ));
                }
                Ok(())
            }
        }
    }
}

pub(crate) fn validate_host(host: &str) -> Result<()> {
    let host = host.trim();
    if host.is_empty()
        || host.len() > 253
        || host.contains(char::is_whitespace)
        || host.contains('/')
    {
        return Err(CoreError::Invalid(format!("invalid host: {host:?}")));
    }
    Ok(())
}

/// Runs every [`WebTarget`] kind; cheap to clone and share.
#[derive(Clone)]
pub struct WebChecker {
    client: reqwest::Client,
    no_redirect_client: reqwest::Client,
}

impl WebChecker {
    pub fn new() -> Self {
        let builder = || {
            reqwest::Client::builder()
                .user_agent(concat!("RodiChecker/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(Duration::from_secs(10))
        };
        Self {
            client: builder().build().expect("http client"),
            no_redirect_client: builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("http client"),
        }
    }
}

impl Default for WebChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Checker for WebChecker {
    type Target = WebTarget;

    fn kind(&self) -> CheckKind {
        CheckKind::Http
    }

    async fn check(&self, target: &WebTarget, ctx: &CheckContext) -> CheckResult {
        let timeout = ctx.timeout;
        match target {
            WebTarget::Http(t) => {
                let client = if t.follow_redirects {
                    &self.client
                } else {
                    &self.no_redirect_client
                };
                http::check(client, t, timeout).await
            }
            WebTarget::Ssl(t) => ssl::check(t, timeout).await,
            WebTarget::Dns(t) => dns::check(t, timeout).await,
            WebTarget::Tcp(t) => tcp::check(t, timeout).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn builds_targets_from_monitor_config() {
        let http =
            WebTarget::from_config(CheckKind::Http, &json!({ "url": "https://example.com" }))
                .unwrap();
        let WebTarget::Http(http) = http else {
            panic!("expected http")
        };
        assert_eq!(http.method, "GET");
        assert!(http.follow_redirects);

        let tcp = WebTarget::from_config(
            CheckKind::Tcp,
            &json!({ "host": "example.com", "port": 22 }),
        )
        .unwrap();
        assert_eq!(tcp.kind(), CheckKind::Tcp);

        let dns =
            WebTarget::from_config(CheckKind::Dns, &json!({ "domain": "example.com" })).unwrap();
        assert!(matches!(
            dns,
            WebTarget::Dns(DnsTarget {
                record_type: RecordType::A,
                ..
            })
        ));
    }

    #[test]
    fn rejects_bad_configs() {
        assert!(WebTarget::from_config(CheckKind::Http, &json!({ "url": "ftp://x" })).is_err());
        assert!(WebTarget::from_config(CheckKind::Http, &json!({})).is_err());
        assert!(
            WebTarget::from_config(CheckKind::Tcp, &json!({ "host": "a b", "port": 1 })).is_err()
        );
        assert!(
            WebTarget::from_config(CheckKind::Tcp, &json!({ "host": "a", "port": 0 })).is_err()
        );
        assert!(WebTarget::from_config(CheckKind::Proxy, &json!({})).is_err());
        assert!(WebTarget::from_config(CheckKind::Ssl, &json!([])).is_err());
    }
}
