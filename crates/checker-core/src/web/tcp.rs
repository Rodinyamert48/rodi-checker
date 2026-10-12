use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpStream;

use crate::checker::{CheckKind, CheckResult, Status};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpTarget {
    pub host: String,
    pub port: u16,
}

pub(super) async fn check(target: &TcpTarget, timeout: Duration) -> CheckResult {
    let kind = CheckKind::Tcp;
    let started = Instant::now();
    let address = (target.host.trim(), target.port);
    match tokio::time::timeout(timeout, TcpStream::connect(address)).await {
        Ok(Ok(stream)) => {
            let peer = stream.peer_addr().map(|a| a.to_string()).ok();
            CheckResult::new(kind, Status::Up)
                .with_latency(started.elapsed())
                .with_message(format!("port {} open", target.port))
                .with_details(json!({ "peer": peer }))
        }
        Ok(Err(err)) => CheckResult::new(kind, Status::Down)
            .with_latency(started.elapsed())
            .with_message(err.to_string()),
        Err(_) => CheckResult::new(kind, Status::Down)
            .with_latency(started.elapsed())
            .with_message("timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn open_and_closed_ports() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let open = check(
            &TcpTarget {
                host: "127.0.0.1".into(),
                port,
            },
            Duration::from_secs(2),
        )
        .await;
        assert_eq!(open.status, Status::Up);

        drop(listener);
        let closed = check(
            &TcpTarget {
                host: "127.0.0.1".into(),
                port,
            },
            Duration::from_secs(2),
        )
        .await;
        assert_eq!(closed.status, Status::Down);
    }
}
