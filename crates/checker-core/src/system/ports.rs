use std::collections::BTreeSet;

use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningPort {
    pub protocol: String,
    pub port: u16,
    pub address: String,
    pub pids: Vec<u32>,
}

/// TCP sockets in LISTEN state and bound UDP sockets.
pub fn listening_ports() -> Result<Vec<ListeningPort>> {
    let sockets = get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP | ProtocolFlags::UDP,
    )
    .map_err(|err| CoreError::Invalid(format!("cannot list sockets: {err}")))?;

    let ports: BTreeSet<ListeningPort> = sockets
        .into_iter()
        .filter_map(|socket| match socket.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp) if tcp.state == TcpState::Listen => Some(ListeningPort {
                protocol: "tcp".into(),
                port: tcp.local_port,
                address: tcp.local_addr.to_string(),
                pids: socket.associated_pids,
            }),
            ProtocolSocketInfo::Udp(udp) => Some(ListeningPort {
                protocol: "udp".into(),
                port: udp.local_port,
                address: udp.local_addr.to_string(),
                pids: socket.associated_pids,
            }),
            _ => None,
        })
        .collect();
    let mut ports: Vec<_> = ports.into_iter().collect();
    ports.sort_by(|a, b| (a.port, &a.protocol, &a.address).cmp(&(b.port, &b.protocol, &b.address)));
    Ok(ports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_our_listener() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let ports = listening_ports().unwrap();
        assert!(
            ports.iter().any(|p| p.protocol == "tcp" && p.port == port),
            "{port} not in {ports:?}"
        );
    }
}
