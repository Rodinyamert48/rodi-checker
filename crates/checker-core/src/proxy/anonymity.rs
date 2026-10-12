use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// How much a proxy reveals about the client, judged from the headers the
/// target received.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anonymity {
    /// The client's real IP is visible.
    Transparent,
    /// The real IP is hidden but the request reveals a proxy is used.
    Anonymous,
    /// No sign of the real IP or of a proxy.
    Elite,
}

/// Headers that reveal a proxy in the request.
const PROXY_HEADERS: [&str; 9] = [
    "via",
    "x-forwarded-for",
    "x-forwarded",
    "forwarded-for",
    "forwarded",
    "x-real-ip",
    "proxy-connection",
    "x-proxy-id",
    "client-ip",
];

/// Classifies a judge response body (which echoes the received headers).
pub fn classify_anonymity(judge_body: &str, real_ip: Option<&str>) -> Anonymity {
    if let Some(ip) = real_ip.map(str::trim).filter(|ip| !ip.is_empty()) {
        if contains_ip(judge_body, ip) {
            return Anonymity::Transparent;
        }
    }
    let lower = judge_body.to_ascii_lowercase();
    if PROXY_HEADERS
        .iter()
        .any(|header| contains_header(&lower, header))
    {
        Anonymity::Anonymous
    } else {
        Anonymity::Elite
    }
}

fn contains_header(lower_body: &str, header: &str) -> bool {
    // Match `"header"` (JSON) or `header:` / `header=` (plain text, PHP style).
    let underscored = header.replace('-', "_");
    [header, underscored.as_str()].iter().any(|name| {
        lower_body.match_indices(name).any(|(index, _)| {
            let before = lower_body[..index].chars().next_back();
            let after = lower_body[index + name.len()..].chars().next();
            let boundary_before =
                !matches!(before, Some(c) if c.is_ascii_alphanumeric() || c == '-' || c == '_');
            let boundary_after = matches!(after, Some('"' | ':' | '=' | '\''));
            boundary_before && boundary_after
        })
    })
}

fn contains_ip(body: &str, ip: &str) -> bool {
    body.match_indices(ip).any(|(index, _)| {
        let before = body[..index].chars().next_back();
        let after = body[index + ip.len()..].chars().next();
        let is_ip_char =
            |c: Option<char>| matches!(c, Some(c) if c.is_ascii_hexdigit() || c == '.' || c == ':');
        !is_ip_char(before) && !is_ip_char(after)
    })
}

/// Finds the client IP in a judge response: JSON `origin`/`ip` fields first,
/// then the first IP-looking token.
pub fn extract_ip(body: &str) -> Option<IpAddr> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        for key in ["origin", "ip", "client_ip", "clientIp", "REMOTE_ADDR"] {
            if let Some(ip) = value
                .get(key)
                .and_then(|v| v.as_str())
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim().parse().ok())
            {
                return Some(ip);
            }
        }
    }
    body.split(|c: char| !(c.is_ascii_hexdigit() || c == '.' || c == ':'))
        .filter(|token| token.contains('.') || token.matches(':').count() >= 2)
        .find_map(|token| token.trim_matches(|c| c == '.' || c == ':').parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_levels() {
        let elite = r#"{"headers": {"Host": "judge", "User-Agent": "x"}, "origin": "203.0.113.9"}"#;
        assert_eq!(
            classify_anonymity(elite, Some("198.51.100.7")),
            Anonymity::Elite
        );

        let anonymous = r#"{"headers": {"Via": "1.1 squid"}, "origin": "203.0.113.9"}"#;
        assert_eq!(
            classify_anonymity(anonymous, Some("198.51.100.7")),
            Anonymity::Anonymous
        );

        let transparent = "HTTP_X_FORWARDED_FOR = 198.51.100.7\nREMOTE_ADDR = 203.0.113.9";
        assert_eq!(
            classify_anonymity(transparent, Some("198.51.100.7")),
            Anonymity::Transparent
        );

        // A longer IP containing the real one must not match.
        assert_eq!(
            classify_anonymity("ip: 198.51.100.77", Some("198.51.100.7")),
            Anonymity::Elite
        );
        // Header names inside other words do not count.
        assert_eq!(
            classify_anonymity(r#"{"previa": 1}"#, None),
            Anonymity::Elite
        );
    }

    #[test]
    fn extracts_ip() {
        assert_eq!(
            extract_ip(r#"{"origin": "203.0.113.9, 10.0.0.1"}"#),
            "203.0.113.9".parse().ok()
        );
        assert_eq!(
            extract_ip(r#"{"ip": "2001:db8::1"}"#),
            "2001:db8::1".parse().ok()
        );
        assert_eq!(
            extract_ip("REMOTE_ADDR = 203.0.113.9\n"),
            "203.0.113.9".parse().ok()
        );
        assert_eq!(extract_ip("no address here."), None);
    }
}
