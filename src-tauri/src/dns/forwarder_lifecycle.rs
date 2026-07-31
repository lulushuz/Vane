use crate::dns::runtime_config::{DnsConfigFingerprint, DnsConfigRevision};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsForwarderIdentity {
    pub installation_id: String,
    pub instance_id: String,
    pub generation: u64,
    pub revision: DnsConfigRevision,
    pub fingerprint: DnsConfigFingerprint,
    pub process_id: Option<u32>,
    pub local_endpoint: SocketAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DnsForwarderState {
    Stopped,
    Preparing,
    Starting,
    WaitingForLocalReadiness,
    Ready,
    Stopping,
    Failed,
}

pub async fn verify_local_readiness(endpoint: SocketAddr) -> bool {
    let bind_addr = if endpoint.is_ipv6() {
        "[::1]:0"
    } else {
        "127.0.0.1:0"
    };
    let socket = match tokio::net::UdpSocket::bind(bind_addr).await {
        Ok(s) => s,
        Err(_) => return false,
    };
    if socket.connect(endpoint).await.is_err() {
        return false;
    }
    true
}
