//! Proxy checking: connectivity, latency, exit IP, anonymity and country.

mod anonymity;
mod parser;

use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub use anonymity::{classify_anonymity, extract_ip, Anonymity};
pub use parser::{parse_line, parse_list, ParseError, ParsedList};

use crate::checker::{CheckContext, CheckKind, CheckResult, Checker, Status};
use crate::error::{CoreError, Result};
use crate::web::http::describe_error;

/// URL used for plain connectivity tests when no judge is configured.
pub const DEFAULT_TEST_URL: &str = "http://www.gstatic.com/generate_204";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocol {
    #[default]
    Http,
    Https,
    Socks4,
    Socks5,
}

impl ProxyProtocol {
    pub const ALL: [ProxyProtocol; 4] = [Self::Http, Self::Https, Self::Socks4, Self::Socks5];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks4 => "socks4",
            Self::Socks5 => "socks5",
        }
    }

    pub fn from_scheme(scheme: &str) -> std::result::Result<Self, String> {
        match scheme.trim().to_ascii_lowercase().as_str() {
            "http" => Ok(Self::Http),
            "https" => Ok(Self::Https),
            "socks4" | "socks4a" => Ok(Self::Socks4),
            "socks5" | "socks5h" | "socks" => Ok(Self::Socks5),
            other => Err(format!("unsupported protocol: {other}")),
        }
    }
}

impl std::str::FromStr for ProxyProtocol {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_scheme(s).map_err(CoreError::Invalid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyEntry {
    pub protocol: ProxyProtocol,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default, skip_serializing)]
    pub password: Option<String>,
}

impl ProxyEntry {
    /// Identity used for de-duplication.
    pub fn key(&self) -> (ProxyProtocol, String, u16, Option<String>) {
        (
            self.protocol,
            self.host.to_ascii_lowercase(),
            self.port,
            self.username.clone(),
        )
    }

    /// `host:port`, with brackets for IPv6.
    pub fn address(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// Display form without the password, e.g. `socks5://user@1.2.3.4:1080`.
    pub fn display(&self) -> String {
        match &self.username {
            Some(user) => format!("{}://{user}@{}", self.protocol.as_str(), self.address()),
            None => format!("{}://{}", self.protocol.as_str(), self.address()),
        }
    }

    /// Line format used for exports: `scheme://[user:pass@]host:port`.
    pub fn to_line(&self, include_password: bool) -> String {
        match (&self.username, &self.password) {
            (Some(user), Some(pass)) if include_password => {
                format!(
                    "{}://{user}:{pass}@{}",
                    self.protocol.as_str(),
                    self.address()
                )
            }
            _ => self.display(),
        }
    }

    fn reqwest_proxy(&self) -> Result<reqwest::Proxy> {
        // socks5h resolves target hostnames through the proxy.
        let scheme = match self.protocol {
            ProxyProtocol::Http => "http",
            ProxyProtocol::Https => "https",
            ProxyProtocol::Socks4 => "socks4a",
            ProxyProtocol::Socks5 => "socks5h",
        };
        let url = format!("{scheme}://{}", self.address());
        let mut proxy = reqwest::Proxy::all(&url)
            .map_err(|err| CoreError::Invalid(format!("invalid proxy: {err}")))?;
        if self.protocol != ProxyProtocol::Socks4 {
            if let Some(user) = &self.username {
                proxy = proxy.basic_auth(user, self.password.as_deref().unwrap_or_default());
            }
        }
        Ok(proxy)
    }
}

/// Optional offline GeoIP database (MaxMind GeoLite2-Country / City .mmdb).
#[derive(Clone)]
pub struct GeoIp(Arc<maxminddb::Reader<Vec<u8>>>);

impl GeoIp {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let reader = maxminddb::Reader::open_readfile(path.as_ref())
            .map_err(|err| CoreError::Invalid(format!("GeoIP database: {err}")))?;
        Ok(Self(Arc::new(reader)))
    }

    /// ISO country code of an IP, if known.
    pub fn country(&self, ip: IpAddr) -> Option<String> {
        let country: maxminddb::geoip2::Country = self.0.lookup(ip).ok()?;
        country.country?.iso_code.map(str::to_owned)
    }
}

#[derive(Clone, Default)]
pub struct ProxyCheckerConfig {
    /// Endpoint echoing request headers and client IP; enables anonymity detection.
    pub judge_url: Option<String>,
    /// Plain connectivity test URL used when no judge is configured.
    pub test_url: Option<String>,
    /// The machine's public IP as seen by the judge, used to detect transparent proxies.
    pub real_ip: Option<String>,
    pub geoip: Option<GeoIp>,
}

pub struct ProxyChecker {
    config: ProxyCheckerConfig,
}

impl ProxyChecker {
    pub fn new(config: ProxyCheckerConfig) -> Self {
        Self { config }
    }

    /// Looks up the machine's public IP through the judge without a proxy.
    pub async fn detect_real_ip(judge_url: &str, timeout: Duration) -> Option<String> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(timeout)
            .build()
            .ok()?;
        let body = client.get(judge_url).send().await.ok()?.text().await.ok()?;
        extract_ip(&body).map(|ip| ip.to_string())
    }

    fn target_url(&self) -> &str {
        self.config
            .judge_url
            .as_deref()
            .or(self.config.test_url.as_deref())
            .unwrap_or(DEFAULT_TEST_URL)
    }
}

#[async_trait]
impl Checker for ProxyChecker {
    type Target = ProxyEntry;

    fn kind(&self) -> CheckKind {
        CheckKind::Proxy
    }

    async fn check(&self, proxy: &ProxyEntry, ctx: &CheckContext) -> CheckResult {
        let kind = CheckKind::Proxy;
        let client = match proxy.reqwest_proxy().and_then(|p| {
            reqwest::Client::builder()
                .proxy(p)
                .timeout(ctx.timeout)
                .danger_accept_invalid_certs(false)
                .user_agent(concat!("RodiChecker/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|err| CoreError::Invalid(err.to_string()))
        }) {
            Ok(client) => client,
            Err(err) => return CheckResult::error(kind, err.to_string()),
        };

        let started = Instant::now();
        let response = match client.get(self.target_url()).send().await {
            Ok(response) => response,
            Err(err) => {
                return CheckResult::new(kind, Status::Down)
                    .with_latency(started.elapsed())
                    .with_message(describe_error(&err))
                    .with_details(json!({ "protocol": proxy.protocol }))
            }
        };
        let code = response.status();
        let latency = started.elapsed();
        if !(code.is_success() || code.is_redirection()) {
            return CheckResult::new(kind, Status::Down)
                .with_latency(latency)
                .with_message(format!("HTTP {}", code.as_u16()))
                .with_details(json!({ "protocol": proxy.protocol }));
        }

        let mut details = json!({ "protocol": proxy.protocol });
        let mut message = format!("HTTP {}", code.as_u16());
        if self.config.judge_url.is_some() {
            let body = response.text().await.unwrap_or_default();
            let exit_ip = extract_ip(&body);
            let anonymity = classify_anonymity(&body, self.config.real_ip.as_deref());
            details["anonymity"] = json!(anonymity);
            details["exitIp"] = json!(exit_ip.map(|ip| ip.to_string()));
            if let (Some(ip), Some(geoip)) = (exit_ip, &self.config.geoip) {
                details["country"] = json!(geoip.country(ip));
            }
            message = format!("{anonymity:?}");
        } else if let (Some(geoip), Ok(ip)) = (&self.config.geoip, proxy.host.parse::<IpAddr>()) {
            details["country"] = json!(geoip.country(ip));
        }

        CheckResult::new(kind, Status::Up)
            .with_latency(latency)
            .with_message(message)
            .with_details(details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_hides_password() {
        let entry = parse_line("socks5://bob:secret@1.2.3.4:1080", ProxyProtocol::Http).unwrap();
        assert_eq!(entry.display(), "socks5://bob@1.2.3.4:1080");
        assert_eq!(entry.to_line(true), "socks5://bob:secret@1.2.3.4:1080");
        assert!(!serde_json::to_string(&entry).unwrap().contains("secret"));
        let v6 = parse_line("[::1]:8080", ProxyProtocol::Http).unwrap();
        assert_eq!(v6.address(), "[::1]:8080");
    }

    /// Minimal HTTP forward proxy: answers every request itself, echoing the
    /// request headers like a judge would (adds X-Forwarded-For when `leak`).
    async fn fake_proxy(leak: Option<&'static str>) -> u16 {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]).to_string();
                    let mut body = format!("{{\"origin\": \"203.0.113.9\", \"raw\": {:?}", request);
                    if let Some(ip) = leak {
                        body.push_str(&format!(", \"X-Forwarded-For\": \"{ip}\""));
                    }
                    body.push('}');
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });
        port
    }

    #[tokio::test]
    async fn checks_http_proxy_with_judge() {
        let port = fake_proxy(None).await;
        let checker = ProxyChecker::new(ProxyCheckerConfig {
            judge_url: Some("http://judge.invalid/headers".into()),
            real_ip: Some("198.51.100.7".into()),
            ..Default::default()
        });
        let entry = parse_line(&format!("127.0.0.1:{port}"), ProxyProtocol::Http).unwrap();
        let result = checker.check(&entry, &CheckContext::default()).await;
        assert_eq!(result.status, Status::Up, "{result:?}");
        assert_eq!(result.details["exitIp"], "203.0.113.9");
        assert_eq!(result.details["anonymity"], "elite");

        let leaky = fake_proxy(Some("198.51.100.7")).await;
        let entry = parse_line(&format!("127.0.0.1:{leaky}"), ProxyProtocol::Http).unwrap();
        let result = checker.check(&entry, &CheckContext::default()).await;
        assert_eq!(result.details["anonymity"], "transparent");
    }

    #[tokio::test]
    async fn dead_proxy_is_down() {
        let checker = ProxyChecker::new(ProxyCheckerConfig::default());
        let entry = parse_line("127.0.0.1:1", ProxyProtocol::Socks5).unwrap();
        let result = checker
            .check(&entry, &CheckContext::new(Duration::from_secs(2)))
            .await;
        assert_eq!(result.status, Status::Down);
    }
}
