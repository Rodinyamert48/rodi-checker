use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::checker::{CheckKind, CheckResult, Status};
use crate::error::{CoreError, Result};

/// Maximum response body read for keyword matching.
const MAX_BODY: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpTarget {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    /// Accepted status codes; empty means any 2xx/3xx.
    #[serde(default)]
    pub expected_status: Vec<u16>,
    /// Text that must appear in the body (or must not, with `keyword_absent`).
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub keyword_absent: bool,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    /// Responses slower than this are reported as degraded.
    #[serde(default)]
    pub degraded_latency_ms: Option<u64>,
}

fn default_method() -> String {
    "GET".into()
}

fn default_true() -> bool {
    true
}

impl HttpTarget {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: default_method(),
            headers: BTreeMap::new(),
            body: None,
            expected_status: Vec::new(),
            keyword: None,
            keyword_absent: false,
            follow_redirects: true,
            degraded_latency_ms: None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let url = url::Url::parse(self.url.trim())
            .map_err(|err| CoreError::Invalid(format!("invalid url: {err}")))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(CoreError::Invalid(
                "url must start with http:// or https://".into(),
            ));
        }
        reqwest::Method::from_bytes(self.method.trim().to_uppercase().as_bytes())
            .map_err(|_| CoreError::Invalid(format!("invalid method: {}", self.method)))?;
        Ok(())
    }

    fn status_ok(&self, code: u16) -> bool {
        if self.expected_status.is_empty() {
            (200..400).contains(&code)
        } else {
            self.expected_status.contains(&code)
        }
    }
}

pub(super) async fn check(
    client: &reqwest::Client,
    target: &HttpTarget,
    timeout: Duration,
) -> CheckResult {
    let kind = CheckKind::Http;
    if let Err(err) = target.validate() {
        return CheckResult::error(kind, err.to_string());
    }
    let method = reqwest::Method::from_bytes(target.method.trim().to_uppercase().as_bytes())
        .expect("validated");
    let mut request = client.request(method, target.url.trim()).timeout(timeout);
    for (name, value) in &target.headers {
        request = request.header(name, value);
    }
    if let Some(body) = &target.body {
        request = request.body(body.clone());
    }

    let started = Instant::now();
    let response = match request.send().await {
        Ok(response) => response,
        Err(err) => {
            return CheckResult::new(kind, Status::Down)
                .with_latency(started.elapsed())
                .with_message(describe_error(&err));
        }
    };
    let latency = started.elapsed();
    let code = response.status().as_u16();
    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    let mut details = json!({
        "statusCode": code,
        "finalUrl": final_url,
        "contentType": content_type,
    });

    let mut status = Status::Up;
    let mut message = format!("HTTP {code}");

    if !target.status_ok(code) {
        status = Status::Down;
        message = format!("unexpected status {code}");
    }

    if let Some(keyword) = target.keyword.as_deref().filter(|k| !k.is_empty()) {
        let body = read_limited(response).await;
        details["bodyBytes"] = json!(body.len());
        let found = body.contains(keyword);
        if status == Status::Up && found == target.keyword_absent {
            status = Status::Down;
            message = if target.keyword_absent {
                format!("keyword {keyword:?} found")
            } else {
                format!("keyword {keyword:?} not found")
            };
        }
    }

    if status == Status::Up {
        if let Some(limit) = target.degraded_latency_ms {
            if latency.as_millis() as u64 > limit {
                status = Status::Degraded;
                message = format!("slow response ({} ms > {limit} ms)", latency.as_millis());
            }
        }
    }

    CheckResult::new(kind, status)
        .with_latency(latency)
        .with_message(message)
        .with_details(details)
}

async fn read_limited(mut response: reqwest::Response) -> String {
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        body.extend_from_slice(&chunk);
        if body.len() >= MAX_BODY {
            body.truncate(MAX_BODY);
            break;
        }
    }
    String::from_utf8_lossy(&body).into_owned()
}

pub(crate) fn describe_error(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        "timeout".into()
    } else if err.is_connect() {
        format!("connection failed: {}", root_cause(err))
    } else {
        root_cause(err)
    }
}

fn root_cause(err: &dyn std::error::Error) -> String {
    let mut current = err;
    while let Some(source) = current.source() {
        current = source;
    }
    current.to_string()
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn test_client() -> reqwest::Client {
        reqwest::Client::builder().no_proxy().build().unwrap()
    }

    async fn run(target: &HttpTarget) -> CheckResult {
        check(&test_client(), target, Duration::from_secs(5)).await
    }

    #[tokio::test]
    async fn up_down_keyword_and_latency() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ok"))
            .respond_with(ResponseTemplate::new(200).set_body_string("hello world"))
            .mount(&server)
            .await;
        Mock::given(path("/fail"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        Mock::given(path("/slow"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(150)))
            .mount(&server)
            .await;

        let ok = run(&HttpTarget::new(format!("{}/ok", server.uri()))).await;
        assert_eq!(ok.status, Status::Up, "{ok:?}");
        assert_eq!(ok.details["statusCode"], 200);
        assert!(ok.latency_ms.is_some());

        let fail = run(&HttpTarget::new(format!("{}/fail", server.uri()))).await;
        assert_eq!(fail.status, Status::Down);

        let mut expect_503 = HttpTarget::new(format!("{}/fail", server.uri()));
        expect_503.expected_status = vec![503];
        assert_eq!(run(&expect_503).await.status, Status::Up);

        let mut keyword = HttpTarget::new(format!("{}/ok", server.uri()));
        keyword.keyword = Some("world".into());
        assert_eq!(run(&keyword).await.status, Status::Up);
        keyword.keyword = Some("missing".into());
        assert_eq!(run(&keyword).await.status, Status::Down);
        keyword.keyword_absent = true;
        assert_eq!(run(&keyword).await.status, Status::Up);

        let mut slow = HttpTarget::new(format!("{}/slow", server.uri()));
        slow.degraded_latency_ms = Some(50);
        assert_eq!(run(&slow).await.status, Status::Degraded);
    }

    #[tokio::test]
    async fn timeout_and_unreachable_are_down() {
        let server = MockServer::start().await;
        Mock::given(path("/hang"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&server)
            .await;
        let result = check(
            &test_client(),
            &HttpTarget::new(format!("{}/hang", server.uri())),
            Duration::from_millis(100),
        )
        .await;
        assert_eq!(result.status, Status::Down);
        assert_eq!(result.message.as_deref(), Some("timeout"));

        let unreachable = run(&HttpTarget::new("http://127.0.0.1:1/")).await;
        assert_eq!(unreachable.status, Status::Down);
    }

    #[test]
    fn validates_url_and_method() {
        assert!(HttpTarget::new("https://example.com").validate().is_ok());
        assert!(HttpTarget::new("example.com").validate().is_err());
        let mut bad = HttpTarget::new("https://example.com");
        bad.method = "GE T".into();
        assert!(bad.validate().is_err());
    }
}
