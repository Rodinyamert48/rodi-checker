//! Parsing proxy lists in the common text formats.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{ProxyEntry, ProxyProtocol};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseError {
    pub line: usize,
    pub text: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedList {
    pub entries: Vec<ProxyEntry>,
    pub errors: Vec<ParseError>,
    pub duplicates: usize,
}

/// Parses one proxy per line. Supported forms:
///
/// - `host:port`
/// - `host:port:user:pass`
/// - `user:pass@host:port`
/// - `scheme://[user:pass@]host:port` (http, https, socks4, socks5, socks5h)
/// - CSV: `host,port[,protocol[,user,pass]]`
///
/// Empty lines and lines starting with `#` are skipped; duplicates are dropped.
pub fn parse_list(text: &str, default_protocol: ProxyProtocol) -> ParsedList {
    let mut list = ParsedList::default();
    let mut seen = HashSet::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') || is_csv_header(line) {
            continue;
        }
        match parse_line(line, default_protocol) {
            Ok(entry) => {
                if seen.insert(entry.key()) {
                    list.entries.push(entry);
                } else {
                    list.duplicates += 1;
                }
            }
            Err(reason) => list.errors.push(ParseError {
                line: index + 1,
                text: line.chars().take(200).collect(),
                reason,
            }),
        }
    }
    list
}

fn is_csv_header(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("host,") || lower.starts_with("ip,")
}

pub fn parse_line(line: &str, default_protocol: ProxyProtocol) -> Result<ProxyEntry, String> {
    let line = line.trim();
    if line.contains(',') {
        return parse_csv(line, default_protocol);
    }

    let (protocol, rest) = match line.split_once("://") {
        Some((scheme, rest)) => (ProxyProtocol::from_scheme(scheme)?, rest),
        None => (default_protocol, line),
    };
    let rest = rest.trim_end_matches('/');

    let (credentials, address) = match rest.rsplit_once('@') {
        Some((creds, address)) => (Some(creds), address),
        None => (None, rest),
    };

    let creds = credentials.map(|creds| creds.split_once(':').unwrap_or((creds, "")));

    // Bracketed IPv6: [::1]:8080
    if let Some((host, port)) = address.strip_prefix('[').and_then(|a| a.split_once("]:")) {
        return build(protocol, host, port, creds.map(|c| c.0), creds.map(|c| c.1));
    }

    let parts: Vec<&str> = address.split(':').collect();
    let (host, port, username, password) = match (parts.as_slice(), creds) {
        ([host, port], Some((user, pass))) => (*host, *port, Some(user), Some(pass)),
        ([host, port], None) => (*host, *port, None, None),
        ([host, port, user, pass], None) => (*host, *port, Some(*user), Some(*pass)),
        _ => return Err("expected host:port".into()),
    };
    build(protocol, host, port, username, password)
}

fn parse_csv(line: &str, default_protocol: ProxyProtocol) -> Result<ProxyEntry, String> {
    let fields: Vec<&str> = line
        .split(',')
        .map(|f| f.trim().trim_matches('"'))
        .collect();
    let protocol = match fields.get(2) {
        Some(p) if !p.is_empty() => ProxyProtocol::from_scheme(p)?,
        _ => default_protocol,
    };
    match fields.as_slice() {
        [host, port] | [host, port, _] => build(protocol, host, port, None, None),
        [host, port, _, user, pass, ..] => build(protocol, host, port, Some(user), Some(pass)),
        [host, port, _, user] => build(protocol, host, port, Some(user), None),
        _ => Err("expected host,port[,protocol[,user,pass]]".into()),
    }
}

fn build(
    protocol: ProxyProtocol,
    host: &str,
    port: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<ProxyEntry, String> {
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() || host.contains(char::is_whitespace) || host.contains('/') {
        return Err("invalid host".into());
    }
    let port: u16 = port
        .trim()
        .parse()
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| "invalid port".to_owned())?;
    let non_empty = |v: Option<&str>| {
        v.map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    };
    Ok(ProxyEntry {
        protocol,
        host: host.to_owned(),
        port,
        username: non_empty(username),
        password: non_empty(password),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        protocol: ProxyProtocol,
        host: &str,
        port: u16,
        user: Option<&str>,
        pass: Option<&str>,
    ) -> ProxyEntry {
        ProxyEntry {
            protocol,
            host: host.into(),
            port,
            username: user.map(Into::into),
            password: pass.map(Into::into),
        }
    }

    #[test]
    fn parses_supported_formats() {
        use ProxyProtocol::*;
        let cases = [
            ("1.2.3.4:8080", entry(Http, "1.2.3.4", 8080, None, None)),
            (
                "1.2.3.4:8080:bob:secret",
                entry(Http, "1.2.3.4", 8080, Some("bob"), Some("secret")),
            ),
            (
                "bob:secret@1.2.3.4:8080",
                entry(Http, "1.2.3.4", 8080, Some("bob"), Some("secret")),
            ),
            (
                "socks5://bob:p@ss@proxy.local:1080",
                entry(Socks5, "proxy.local", 1080, Some("bob"), Some("p@ss")),
            ),
            (
                "socks4://10.0.0.1:1080/",
                entry(Socks4, "10.0.0.1", 1080, None, None),
            ),
            ("https://[::1]:443", entry(Https, "::1", 443, None, None)),
            (
                "1.2.3.4,3128,socks5,u,p",
                entry(Socks5, "1.2.3.4", 3128, Some("u"), Some("p")),
            ),
            ("\"1.2.3.4\",3128", entry(Http, "1.2.3.4", 3128, None, None)),
        ];
        for (line, expected) in cases {
            assert_eq!(parse_line(line, Http).unwrap(), expected, "{line}");
        }
    }

    #[test]
    fn rejects_invalid_lines() {
        for line in [
            "1.2.3.4",
            "1.2.3.4:0",
            "1.2.3.4:99999",
            "ftp://1.2.3.4:21",
            "a b:80",
            "1:2:3",
        ] {
            assert!(parse_line(line, ProxyProtocol::Http).is_err(), "{line}");
        }
    }

    #[test]
    fn parses_list_with_comments_duplicates_and_errors() {
        let text = "\u{feff}# my proxies\nhost,port,protocol\n1.1.1.1:80\n\n1.1.1.1:80\nbad line\nsocks5://2.2.2.2:1080\n";
        let list = parse_list(text, ProxyProtocol::Http);
        assert_eq!(list.entries.len(), 2);
        assert_eq!(list.duplicates, 1);
        assert_eq!(list.errors.len(), 1);
        assert_eq!(list.errors[0].line, 6);
    }
}
