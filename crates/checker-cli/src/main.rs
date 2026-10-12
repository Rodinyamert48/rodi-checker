//! `rodi-checker` command line interface.
//!
//! Check subcommands (web, ssl, dns, port, proxy, hash, system) are added as
//! their modules land in `checker-core`; see PLAN.md section 10.

use std::process::ExitCode;

use checker_core::CheckKind;
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

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum Lang {
    Tr,
    En,
}

#[derive(Subcommand)]
enum Command {
    /// List the supported check kinds.
    Kinds,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Kinds => {
            let kinds: Vec<_> = CheckKind::ALL.iter().map(|k| k.as_str()).collect();
            if cli.json {
                println!("{}", serde_json::to_string(&kinds).expect("serializable"));
            } else {
                let header = match cli.lang {
                    Lang::Tr => "Desteklenen kontrol türleri:",
                    Lang::En => "Supported check kinds:",
                };
                println!("{header}");
                for kind in kinds {
                    println!("  - {kind}");
                }
            }
        }
    }
    ExitCode::SUCCESS
}
