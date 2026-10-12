//! `rodi-checker` command line interface. See PLAN.md section 10.

mod output;

use std::io::Write as _;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use checker_core::files::{self, HashAlgorithm, ScanOptions};
use checker_core::proxy::{parse_list, GeoIp, ProxyChecker, ProxyCheckerConfig, ProxyProtocol};
use checker_core::web::{
    DnsTarget, HttpTarget, RecordType, SslTarget, TcpTarget, WebChecker, WebTarget,
};
use checker_core::{CheckContext, CheckKind, Checker, JobRunner, Storage};
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
    /// Check a proxy list file (one proxy per line).
    Proxy {
        file: String,
        /// Protocol for lines without a scheme.
        #[arg(long, value_enum, default_value_t = Proto::Http)]
        protocol: Proto,
        #[arg(long, default_value_t = 50)]
        concurrency: usize,
        /// Judge URL echoing request headers (enables anonymity / exit IP).
        #[arg(long)]
        judge: Option<String>,
        /// Connectivity test URL when no judge is given.
        #[arg(long)]
        test_url: Option<String>,
        /// MaxMind GeoLite2 .mmdb file for country lookup.
        #[arg(long)]
        geoip: Option<String>,
        /// Write working proxies to this file.
        #[arg(long)]
        out: Option<String>,
    },
    /// Hash a file or folder; save or compare baselines (shared with the app).
    Hash {
        path: String,
        #[arg(long, value_enum, default_value_t = Algo::Sha256)]
        algo: Algo,
        /// Glob pattern to skip (repeatable), e.g. --exclude "*.log".
        #[arg(long = "exclude")]
        excludes: Vec<String>,
        /// Save the result as a baseline with this name.
        #[arg(long, conflicts_with = "compare")]
        save_baseline: Option<String>,
        /// Compare against the baseline with this name.
        #[arg(long)]
        compare: Option<String>,
        /// Database file (defaults to the desktop app's database).
        #[arg(long)]
        db: Option<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Algo {
    Sha256,
    Blake3,
}

impl From<Algo> for HashAlgorithm {
    fn from(value: Algo) -> Self {
        match value {
            Algo::Sha256 => HashAlgorithm::Sha256,
            Algo::Blake3 => HashAlgorithm::Blake3,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Proto {
    Http,
    Https,
    Socks4,
    Socks5,
}

impl From<Proto> for ProxyProtocol {
    fn from(value: Proto) -> Self {
        match value {
            Proto::Http => ProxyProtocol::Http,
            Proto::Https => ProxyProtocol::Https,
            Proto::Socks4 => ProxyProtocol::Socks4,
            Proto::Socks5 => ProxyProtocol::Socks5,
        }
    }
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
        Command::Proxy {
            file,
            protocol,
            concurrency,
            judge,
            test_url,
            geoip,
            out,
        } => {
            let options = ProxyOptions {
                file,
                protocol: (*protocol).into(),
                concurrency: *concurrency,
                judge: judge.clone(),
                test_url: test_url.clone(),
                geoip: geoip.as_deref(),
                out: out.as_deref(),
            };
            match run_proxy(options, &ctx, &cli).await {
                Ok(code) => code,
                Err(err) => {
                    eprintln!("{err}");
                    2
                }
            }
        }
        Command::Hash {
            path,
            algo,
            excludes,
            save_baseline,
            compare,
            db,
        } => {
            let options = HashOptions {
                path,
                scan: ScanOptions {
                    algorithm: (*algo).into(),
                    excludes: excludes.clone(),
                },
                save_baseline: save_baseline.as_deref(),
                compare: compare.as_deref(),
                db: db.as_deref(),
            };
            match run_hash(options, &cli).await {
                Ok(code) => code,
                Err(err) => {
                    eprintln!("{err}");
                    2
                }
            }
        }
    };
    ExitCode::from(code)
}

struct HashOptions<'a> {
    path: &'a str,
    scan: ScanOptions,
    save_baseline: Option<&'a str>,
    compare: Option<&'a str>,
    db: Option<&'a str>,
}

/// The desktop app's database: `%APPDATA%\\com.rodi.checker\\rodi-checker.db` on Windows.
fn default_db_path() -> anyhow::Result<std::path::PathBuf> {
    let dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot find the data directory"))?
        .join("com.rodi.checker");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("rodi-checker.db"))
}

async fn run_hash(options: HashOptions<'_>, cli: &Cli) -> anyhow::Result<u8> {
    let root = std::path::PathBuf::from(options.path);
    let root = std::fs::canonicalize(&root).unwrap_or(root);

    let storage = if options.save_baseline.is_some() || options.compare.is_some() {
        let db = match options.db {
            Some(db) => std::path::PathBuf::from(db),
            None => default_db_path()?,
        };
        Some(Storage::open(db).await?)
    } else {
        None
    };

    // When comparing, scan with the baseline's own settings.
    let (baseline, scan_options) = match (options.compare, &storage) {
        (Some(name), Some(storage)) => {
            let baseline = storage.baseline_by_name(name).await?;
            let scan_options = baseline.scan_options()?;
            (Some(baseline), scan_options)
        }
        _ => (None, options.scan.clone()),
    };

    let show_progress = !cli.json && root.is_dir();
    let scan_root = root.clone();
    let report = tokio::task::spawn_blocking(move || {
        files::scan(
            &scan_root,
            &scan_options,
            &Default::default(),
            |done, total| {
                if show_progress {
                    eprint!("\r{done}/{total}");
                }
            },
        )
    })
    .await??;
    if show_progress {
        eprintln!();
    }
    for (path, reason) in &report.errors {
        eprintln!("{path}: {reason}");
    }

    if let (Some(baseline), Some(storage)) = (baseline, &storage) {
        let previous = storage.baseline_entries(baseline.id).await?;
        let diff = files::compare(&previous, &report.entries);
        if cli.json {
            output::print_json(&diff);
        } else {
            for entry in &diff.added {
                println!("+ {}", entry.path);
            }
            for entry in &diff.removed {
                println!("- {}", entry.path);
            }
            for entry in &diff.modified {
                println!("~ {}", entry.path);
            }
            match cli.lang {
                Lang::Tr => println!(
                    "{} eklenen, {} silinen, {} değişen, {} aynı",
                    diff.added.len(),
                    diff.removed.len(),
                    diff.modified.len(),
                    diff.unchanged
                ),
                Lang::En => println!(
                    "{} added, {} removed, {} modified, {} unchanged",
                    diff.added.len(),
                    diff.removed.len(),
                    diff.modified.len(),
                    diff.unchanged
                ),
            }
        }
        return Ok(if diff.is_clean() { 0 } else { 1 });
    }

    if cli.json {
        output::print_json(&report.entries);
    } else {
        for entry in &report.entries {
            println!("{}  {}", entry.hash, entry.path);
        }
    }

    if let (Some(name), Some(storage)) = (options.save_baseline, &storage) {
        let saved = storage
            .save_baseline(
                name,
                &root.display().to_string(),
                &options.scan,
                &report.entries,
            )
            .await?;
        if !cli.json {
            match cli.lang {
                Lang::Tr => eprintln!(
                    "baseline \"{}\" kaydedildi ({} dosya)",
                    saved.name, saved.file_count
                ),
                Lang::En => eprintln!(
                    "baseline \"{}\" saved ({} files)",
                    saved.name, saved.file_count
                ),
            }
        }
    }
    Ok(if report.errors.is_empty() { 0 } else { 1 })
}

struct ProxyOptions<'a> {
    file: &'a str,
    protocol: ProxyProtocol,
    concurrency: usize,
    judge: Option<String>,
    test_url: Option<String>,
    geoip: Option<&'a str>,
    out: Option<&'a str>,
}

async fn run_proxy(options: ProxyOptions<'_>, ctx: &CheckContext, cli: &Cli) -> anyhow::Result<u8> {
    let text = std::fs::read_to_string(options.file)?;
    let parsed = parse_list(&text, options.protocol);
    for error in &parsed.errors {
        eprintln!("line {}: {} ({})", error.line, error.text, error.reason);
    }
    if parsed.entries.is_empty() {
        anyhow::bail!("no valid proxies in {}", options.file);
    }

    let geoip = options.geoip.map(GeoIp::open).transpose()?;
    let real_ip = match options.judge.as_deref() {
        Some(judge) => ProxyChecker::detect_real_ip(judge, ctx.timeout).await,
        None => None,
    };
    let checker = Arc::new(ProxyChecker::new(ProxyCheckerConfig {
        judge_url: options.judge,
        test_url: options.test_url,
        real_ip,
        geoip,
    }));

    let entries = parsed.entries;
    let results = JobRunner::new(options.concurrency)
        .run(checker, entries.clone(), ctx, |progress, _, _| {
            if !cli.json {
                eprint!("\r{}/{}", progress.done, progress.total);
                let _ = std::io::stderr().flush();
            }
        })
        .await;
    if !cli.json {
        eprintln!();
    }

    let working: Vec<_> = results
        .iter()
        .filter(|(_, r)| r.status.is_ok())
        .map(|(i, _)| &entries[*i])
        .collect();

    if cli.json {
        let rows: Vec<_> = results
            .iter()
            .map(|(i, r)| serde_json::json!({ "proxy": entries[*i].display(), "result": r }))
            .collect();
        output::print_json(&rows);
    } else {
        for (i, result) in &results {
            output::print_result(&entries[*i].display(), result, false, cli.lang);
        }
        match cli.lang {
            Lang::Tr => println!("\n{}/{} proxy çalışıyor", working.len(), entries.len()),
            Lang::En => println!("\n{}/{} proxies working", working.len(), entries.len()),
        }
    }

    if let Some(out) = options.out {
        let lines: Vec<String> = working.iter().map(|e| e.to_line(true)).collect();
        std::fs::write(out, lines.join("\n") + "\n")?;
    }
    Ok(if working.is_empty() { 1 } else { 0 })
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
