//! Report generation: tables built from stored data, rendered as CSV, JSON or HTML.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::settings::Language;
use crate::storage::Storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    MonitorResults,
    Incidents,
    Proxies,
    Baselines,
    SystemSamples,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Csv,
    Json,
    Html,
}

impl ReportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Html => "html",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRequest {
    pub kind: ReportKind,
    /// Only rows newer than this many hours (time based reports).
    #[serde(default = "default_hours")]
    pub hours: i64,
    /// Restrict to one monitor or proxy list.
    #[serde(default)]
    pub target_id: Option<i64>,
}

fn default_hours() -> i64 {
    24
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

fn t(lang: Language, tr: &str, en: &str) -> String {
    match lang {
        Language::Tr => tr.to_owned(),
        Language::En => en.to_owned(),
    }
}

fn opt<T: ToString>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

fn time(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub async fn build(storage: &Storage, request: &ReportRequest, lang: Language) -> Result<Table> {
    let since = Utc::now() - chrono::Duration::hours(request.hours.clamp(1, 24 * 3650));
    let h = |tr: &str, en: &str| t(lang, tr, en);
    match request.kind {
        ReportKind::MonitorResults => {
            let monitors = storage.list_monitors(None).await?;
            let mut rows = Vec::new();
            for monitor in monitors
                .iter()
                .filter(|m| request.target_id.is_none_or(|id| id == m.id))
            {
                for stored in storage.results_since(monitor.id, since).await? {
                    let r = stored.result;
                    rows.push(vec![
                        time(r.checked_at),
                        monitor.name.clone(),
                        monitor.kind.to_string(),
                        r.status.as_str().to_owned(),
                        opt(r.latency_ms),
                        r.message.unwrap_or_default(),
                    ]);
                }
            }
            rows.sort_by(|a, b| b[0].cmp(&a[0]));
            Ok(Table {
                title: h("İzleyici sonuçları", "Monitor results"),
                headers: vec![
                    h("Zaman", "Time"),
                    h("İzleyici", "Monitor"),
                    h("Tür", "Type"),
                    h("Durum", "Status"),
                    h("Gecikme (ms)", "Latency (ms)"),
                    h("Mesaj", "Message"),
                ],
                rows,
            })
        }
        ReportKind::Incidents => {
            let rows = storage
                .list_incidents(request.target_id, 10_000)
                .await?
                .into_iter()
                .filter(|i| i.resolved_at.is_none_or(|r| r >= since) || i.started_at >= since)
                .map(|i| {
                    let minutes = i.resolved_at.map(|r| (r - i.started_at).num_minutes());
                    vec![
                        i.monitor_name,
                        time(i.started_at),
                        i.resolved_at.map(time).unwrap_or_default(),
                        opt(minutes),
                        i.cause.unwrap_or_default(),
                    ]
                })
                .collect();
            Ok(Table {
                title: h("Olaylar", "Incidents"),
                headers: vec![
                    h("İzleyici", "Monitor"),
                    h("Başlangıç", "Started"),
                    h("Çözülme", "Resolved"),
                    h("Süre (dk)", "Duration (min)"),
                    h("Neden", "Cause"),
                ],
                rows,
            })
        }
        ReportKind::Proxies => {
            let lists = storage.proxy_lists().await?;
            let mut rows = Vec::new();
            for list in lists
                .iter()
                .filter(|l| request.target_id.is_none_or(|id| id == l.id))
            {
                for p in storage.proxies(list.id).await? {
                    rows.push(vec![
                        list.name.clone(),
                        p.protocol,
                        format!("{}:{}", p.host, p.port),
                        p.status.unwrap_or_default(),
                        opt(p.latency_ms),
                        p.anonymity.unwrap_or_default(),
                        p.exit_ip.unwrap_or_default(),
                        p.country.unwrap_or_default(),
                        p.checked_at.map(time).unwrap_or_default(),
                    ]);
                }
            }
            Ok(Table {
                title: h("Proxy sonuçları", "Proxy results"),
                headers: vec![
                    h("Liste", "List"),
                    h("Protokol", "Protocol"),
                    "Proxy".into(),
                    h("Durum", "Status"),
                    h("Gecikme (ms)", "Latency (ms)"),
                    h("Anonimlik", "Anonymity"),
                    h("Çıkış IP", "Exit IP"),
                    h("Ülke", "Country"),
                    h("Test zamanı", "Checked at"),
                ],
                rows,
            })
        }
        ReportKind::Baselines => {
            let rows = storage
                .baselines()
                .await?
                .into_iter()
                .map(|b| {
                    vec![
                        b.name,
                        b.root_path,
                        b.algorithm,
                        b.file_count.to_string(),
                        b.total_size.to_string(),
                        b.created_at,
                    ]
                })
                .collect();
            Ok(Table {
                title: h("Dosya baseline'ları", "File baselines"),
                headers: vec![
                    h("Ad", "Name"),
                    h("Klasör", "Folder"),
                    h("Algoritma", "Algorithm"),
                    h("Dosya", "Files"),
                    h("Boyut (bayt)", "Size (bytes)"),
                    h("Oluşturulma", "Created"),
                ],
                rows,
            })
        }
        ReportKind::SystemSamples => {
            let rows = storage
                .system_samples(since)
                .await?
                .into_iter()
                .map(|s| {
                    let mem = if s.mem_total > 0 {
                        s.mem_used as f64 * 100.0 / s.mem_total as f64
                    } else {
                        0.0
                    };
                    vec![
                        time(s.ts),
                        format!("{:.1}", s.cpu),
                        format!("{mem:.1}"),
                        s.mem_used.to_string(),
                    ]
                })
                .collect();
            Ok(Table {
                title: h("Sistem geçmişi", "System history"),
                headers: vec![
                    h("Zaman", "Time"),
                    h("CPU %", "CPU %"),
                    h("Bellek %", "Memory %"),
                    h("Kullanılan bellek (bayt)", "Memory used (bytes)"),
                ],
                rows,
            })
        }
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl Table {
    /// CSV with a UTF-8 BOM so Excel opens Turkish characters correctly.
    pub fn to_csv(&self) -> String {
        let mut out = String::from("\u{feff}");
        let line = |cells: &[String]| {
            cells
                .iter()
                .map(|c| csv_field(c))
                .collect::<Vec<_>>()
                .join(",")
        };
        out.push_str(&line(&self.headers));
        out.push('\n');
        for row in &self.rows {
            out.push_str(&line(row));
            out.push('\n');
        }
        out
    }

    pub fn to_json(&self) -> String {
        let rows: Vec<serde_json::Map<String, serde_json::Value>> = self
            .rows
            .iter()
            .map(|row| {
                self.headers
                    .iter()
                    .cloned()
                    .zip(row.iter().map(|c| serde_json::Value::String(c.clone())))
                    .collect()
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({ "title": self.title, "rows": rows }))
            .expect("serializable")
    }

    /// Self-contained HTML page (inline CSS, no scripts).
    pub fn to_html(&self, lang: Language) -> String {
        let generated = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let mut rows = String::new();
        for row in &self.rows {
            rows.push_str("<tr>");
            for cell in row {
                let class = match cell.as_str() {
                    "up" => " class=\"up\"",
                    "degraded" => " class=\"degraded\"",
                    "down" | "error" => " class=\"down\"",
                    _ => "",
                };
                rows.push_str(&format!("<td{class}>{}</td>", html_escape(cell)));
            }
            rows.push_str("</tr>\n");
        }
        let headers: String = self
            .headers
            .iter()
            .map(|h| format!("<th>{}</th>", html_escape(h)))
            .collect();
        let (lang_code, generated_label, count_label) = match lang {
            Language::Tr => ("tr", "Oluşturulma", "satır"),
            Language::En => ("en", "Generated", "rows"),
        };
        format!(
            r#"<!doctype html>
<html lang="{lang_code}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · Rodi Checker</title>
<style>
  :root {{ color-scheme: light dark; --bg:#fff; --fg:#111827; --muted:#6b7280; --border:#e5e7eb; --head:#f3f4f6; }}
  @media (prefers-color-scheme: dark) {{ :root {{ --bg:#0b0f17; --fg:#e5e7eb; --muted:#9ca3af; --border:#1f2937; --head:#111827; }} }}
  body {{ margin: 0; padding: 24px; background: var(--bg); color: var(--fg); font: 14px/1.5 "Segoe UI", system-ui, sans-serif; }}
  h1 {{ margin: 0 0 4px; font-size: 22px; }}
  p {{ margin: 0 0 16px; color: var(--muted); }}
  .wrap {{ overflow-x: auto; }}
  table {{ border-collapse: collapse; width: 100%; }}
  th, td {{ border-bottom: 1px solid var(--border); padding: 6px 10px; text-align: left; white-space: nowrap; }}
  th {{ background: var(--head); position: sticky; top: 0; font-size: 12px; text-transform: uppercase; letter-spacing: .03em; }}
  .up {{ color: #16a34a; font-weight: 600; }} .degraded {{ color: #d97706; font-weight: 600; }} .down {{ color: #dc2626; font-weight: 600; }}
</style>
</head>
<body>
<h1>{title}</h1>
<p>Rodi Checker · {generated_label}: {generated} · {count} {count_label}</p>
<div class="wrap"><table>
<thead><tr>{headers}</tr></thead>
<tbody>
{rows}</tbody>
</table></div>
</body>
</html>
"#,
            title = html_escape(&self.title),
            count = self.rows.len(),
        )
    }

    pub fn render(&self, format: ReportFormat, lang: Language) -> String {
        match format {
            ReportFormat::Csv => self.to_csv(),
            ReportFormat::Json => self.to_json(),
            ReportFormat::Html => self.to_html(lang),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::checker::{CheckKind, CheckResult, Status};
    use crate::monitoring;
    use crate::storage::NewMonitor;

    fn table() -> Table {
        Table {
            title: "Test <1>".into(),
            headers: vec!["Ad".into(), "Durum".into()],
            rows: vec![
                vec!["a, \"b\"".into(), "up".into()],
                vec!["ç".into(), "down".into()],
            ],
        }
    }

    #[test]
    fn renders_formats() {
        let csv = table().to_csv();
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("\"a, \"\"b\"\"\",up"));

        let json: serde_json::Value = serde_json::from_str(&table().to_json()).unwrap();
        assert_eq!(json["rows"][1]["Ad"], "ç");

        let html = table().to_html(Language::Tr);
        assert!(html.contains("<title>Test &lt;1&gt; · Rodi Checker</title>"));
        assert!(html.contains("<td class=\"down\">down</td>"));
        assert!(html.contains("2 satır"));
    }

    #[tokio::test]
    async fn builds_monitor_and_incident_reports() {
        let storage = Storage::in_memory().await.unwrap();
        let monitor = storage
            .create_monitor(&NewMonitor {
                kind: CheckKind::Http,
                name: "site".into(),
                config: json!({ "url": "https://example.com" }),
                interval_sec: 60,
                enabled: true,
                tags: vec![],
            })
            .await
            .unwrap();
        for status in [Status::Up, Status::Down, Status::Up] {
            let result = CheckResult::new(CheckKind::Http, status).with_message("m");
            monitoring::record(&storage, monitor.id, &result)
                .await
                .unwrap();
        }

        let request = |kind| ReportRequest {
            kind,
            hours: 24,
            target_id: None,
        };
        let results = build(&storage, &request(ReportKind::MonitorResults), Language::En)
            .await
            .unwrap();
        assert_eq!(results.rows.len(), 3);
        assert_eq!(results.headers[0], "Time");

        let incidents = build(&storage, &request(ReportKind::Incidents), Language::Tr)
            .await
            .unwrap();
        assert_eq!(incidents.rows.len(), 1);
        assert_eq!(incidents.rows[0][0], "site");

        for kind in [
            ReportKind::Proxies,
            ReportKind::Baselines,
            ReportKind::SystemSamples,
        ] {
            assert!(build(&storage, &request(kind), Language::Tr)
                .await
                .unwrap()
                .rows
                .is_empty());
        }
    }
}
