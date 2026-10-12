//! `rodi-checker` command line interface. See PLAN.md section 10.

mod output;

use std::process::ExitCode;
use std::time::Duration;

use checker_core::web::{
    DnsTarget, HttpTarget, RecordType, SslTarget, TcpTarget, WebChecker, WebTarget,
};
use checker_core::{CheckContext, CheckKind, Checker};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "rodi-checker", version, about = "Rodi Checker CLI")]
struct Cli {
    /// Print machine readable JSON output.
    #[arg(long, global = true)]
    json: bool,

    /// Output language.
    #[arg(long, global = true, value_enum, default_value_t = Lang::Tr)]
    lang: Lang,

    /// Timeout in seconds for each check.
    #[arg(long, global = true, default_value_t = 10)]
    timeout: u64,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Lang {
    Tr,
    En,
}

#[derive(Clone, Copy, ValueEnum)]
enum DnsType {
    A,
    Aaaa,
    Mx,
    Txt,
    Cname,
    Ns,
}

impl From<DnsType> for RecordType {
    fn from(value: DnsType) -> Self {
        match value {
            DnsType::A => RecordType::A,
            DnsType::Aaaa => RecordType::Aaaa,
            DnsType::Mx => RecordType::Mx,
            DnsType::Txt => RecordType::Txt,
            DnsType::Cname => RecordType::Cname,
            DnsType::Ns => RecordType::Ns,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// List the supported check kinds.
    Kinds,
    /// Check a website or API endpoint.
    Web {
        url: String,
        /// HTTP method.
        #[arg(long, default_value = "GET")]
        method: String,
        /// Accepted status code (repeatable). Default: any 2xx/3xx.
        #[arg(long = "expect")]
        expect: Vec<u16>,
        /// Text that must appear in the response body.
        #[arg(long)]
        keyword: Option<String>,
        /// Do not follow redirects.
        #[arg(long)]
        no_redirects: bool,
    },
    /// Check a TLS certificate.
    Ssl {
        /// host or host:port
        host: String,
        /// Warn when the certificate expires within this many days.
        #[arg(long, default_value_t = 14)]
        warn_days: i64,
    },
    /// Resolve DNS records.
    Dns {
        domain: String,
        #[arg(long = "type", value_enum, default_value_t = DnsType::A)]
        record_type: DnsType,
        /// A value one of the records must contain.
        #[arg(long)]
        expect: Option<String>,
    },
    /// Check whether a TCP port is open.
    Port { host: String, port: u16 },
}

fn split_host_port(value: &str, default_port: u16) -> (String, u16) {
    match value.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => match port.parse() {
            Ok(port) => (host.to_owned(), port),
            Err(_) => (value.to_owned(), default_port),
        },
        _ => (value.to_owned(), default_port),
    }
}

async fn run_web(target: WebTarget, ctx: &CheckContext, cli: &Cli) -> u8 {
    let label = match &target {
        WebTarget::Http(t) => t.url.clone(),
        WebTarget::Ssl(t) => format!("{}:{}", t.host, t.port),
        WebTarget::Dns(t) => t.domain.clone(),
        WebTarget::Tcp(t) => format!("{}:{}", t.host, t.port),
    };
    if let Err(err) = target.validate() {
        eprintln!("{err}");
        return 2;
    }
    let result = WebChecker::new().check(&target, ctx).await;
    output::print_result(&label, &result, cli.json, cli.lang);
    output::exit_code([&result])
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let ctx = CheckContext::new(Duration::from_secs(cli.timeout.max(1)));

    let code = match &cli.command {
        Command::Kinds => {
            let kinds: Vec<_> = CheckKind::ALL.iter().map(|k| k.as_str()).collect();
            if cli.json {
                output::print_json(&kinds);
            } else {
                println!(
                    "{}",
                    match cli.lang {
                        Lang::Tr => "Desteklenen kontrol türleri:",
                        Lang::En => "Supported check kinds:",
                    }
                );
                for kind in kinds {
                    println!("  - {kind}");
                }
            }
            0
        }
        Command::Web {
            url,
            method,
            expect,
            keyword,
            no_redirects,
        } => {
            let mut target = HttpTarget::new(url.clone());
            target.method = method.clone();
            target.expected_status = expect.clone();
            target.keyword = keyword.clone();
            target.follow_redirects = !no_redirects;
            run_web(WebTarget::Http(target), &ctx, &cli).await
        }
        Command::Ssl { host, warn_days } => {
            let (host, port) = split_host_port(host, 443);
            run_web(
                WebTarget::Ssl(SslTarget {
                    host,
                    port,
                    warn_days: *warn_days,
                }),
                &ctx,
                &cli,
            )
            .await
        }
        Command::Dns {
            domain,
            record_type,
            expect,
        } => {
            let target = DnsTarget {
                domain: domain.clone(),
                record_type: (*record_type).into(),
                expected: expect.clone(),
            };
            run_web(WebTarget::Dns(target), &ctx, &cli).await
        }
        Command::Port { host, port } => {
            let target = TcpTarget {
                host: host.clone(),
                port: *port,
            };
            run_web(WebTarget::Tcp(target), &ctx, &cli).await
        }
    };
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_host_and_port() {
        assert_eq!(
            split_host_port("example.com", 443),
            ("example.com".into(), 443)
        );
        assert_eq!(
            split_host_port("example.com:8443", 443),
            ("example.com".into(), 8443)
        );
        assert_eq!(
            split_host_port("example.com:x", 443),
            ("example.com:x".into(), 443)
        );
    }

    #[test]
    fn cli_parses() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
