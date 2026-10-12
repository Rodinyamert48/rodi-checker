use checker_core::{CheckResult, Status};
use serde::Serialize;

use crate::Lang;

pub fn status_label(status: Status, lang: Lang) -> &'static str {
    match (status, lang) {
        (Status::Up, Lang::Tr) => "ÇALIŞIYOR",
        (Status::Degraded, Lang::Tr) => "SORUNLU",
        (Status::Down, Lang::Tr) => "ERİŞİLEMİYOR",
        (Status::Error, Lang::Tr) => "HATA",
        (Status::Up, Lang::En) => "UP",
        (Status::Degraded, Lang::En) => "DEGRADED",
        (Status::Down, Lang::En) => "DOWN",
        (Status::Error, Lang::En) => "ERROR",
    }
}

pub fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("serializable")
    );
}

/// Prints a single check result in human or JSON form.
pub fn print_result(label: &str, result: &CheckResult, json: bool, lang: Lang) {
    if json {
        print_json(result);
        return;
    }
    let latency = result
        .latency_ms
        .map(|ms| format!(" ({ms} ms)"))
        .unwrap_or_default();
    println!("{label}: {}{latency}", status_label(result.status, lang));
    if let Some(message) = &result.message {
        println!("  {message}");
    }
    if let serde_json::Value::Object(details) = &result.details {
        for (key, value) in details {
            if value.is_null() {
                continue;
            }
            let value = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Array(items) => items
                    .iter()
                    .map(|i| {
                        i.as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| i.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                other => other.to_string(),
            };
            println!("  {key}: {value}");
        }
    }
}

/// Exit code for a set of results: 0 when all are ok, 1 otherwise.
pub fn exit_code<'a>(results: impl IntoIterator<Item = &'a CheckResult>) -> u8 {
    if results.into_iter().all(|r| r.status.is_ok()) {
        0
    } else {
        1
    }
}
