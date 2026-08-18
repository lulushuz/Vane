use crate::dns::runtime_config::{DnsConfigFingerprint, DnsConfigRevision};
use hickory_resolver::proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_resolver::proto::rr::{Name, RecordType};
use hickory_resolver::proto::serialize::binary::{BinDecodable, BinEncodable};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessProtocol {
    Udp,
    Tcp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionalReadinessEvidence {
    pub protocol: ReadinessProtocol,
    pub transaction_id: u16,
    pub answer_count: usize,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionalReadinessError {
    pub protocol: ReadinessProtocol,
    pub stage: &'static str,
    pub detail: String,
}

impl std::fmt::Display for FunctionalReadinessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DNS {:?} functional readiness failed at {}: {}",
            self.protocol, self.stage, self.detail
        )
    }
}

static NEXT_READINESS_ID: AtomicU16 = AtomicU16::new(0x5100);

pub async fn verify_local_socket_readiness(endpoint: SocketAddr) -> bool {
    tokio::net::TcpStream::connect(endpoint).await.is_ok()
}

fn readiness_query(target: &str) -> Result<(Message, Vec<u8>), String> {
    let name = Name::from_ascii(target.trim_end_matches('.')).map_err(|error| error.to_string())?;
    let mut message = Message::new(
        NEXT_READINESS_ID.fetch_add(1, Ordering::Relaxed),
        MessageType::Query,
        OpCode::Query,
    );
    message.metadata.recursion_desired = true;
    message.add_query(Query::query(name, RecordType::A));
    let wire = message.to_bytes().map_err(|error| error.to_string())?;
    Ok((message, wire))
}

fn validate_readiness_response(query: &Message, wire: &[u8]) -> Result<usize, String> {
    let response =
        Message::from_bytes(wire).map_err(|error| format!("malformed DNS response: {error}"))?;
    if response.metadata.message_type != MessageType::Response {
        return Err("packet is not a DNS response".into());
    }
    if response.metadata.id != query.metadata.id {
        return Err("transaction ID mismatch".into());
    }
    let questions_match = response.queries.len() == query.queries.len()
        && response
            .queries
            .iter()
            .zip(&query.queries)
            .all(|(left, right)| {
                left.name().to_ascii().trim_end_matches('.')
                    == right.name().to_ascii().trim_end_matches('.')
                    && left.query_type() == right.query_type()
                    && left.query_class() == right.query_class()
            });
    if !questions_match {
        return Err("question mismatch".into());
    }
    match response.metadata.response_code {
        ResponseCode::NoError => {
            if response.answers.is_empty() {
                return Err("NOERROR response contains no answers".into());
            }
        }
        ResponseCode::ServFail => return Err("upstream returned SERVFAIL".into()),
        ResponseCode::Refused => return Err("upstream returned REFUSED".into()),
        code => return Err(format!("upstream returned DNS response code: {code:?}")),
    }
    Ok(response.answers.len())
}

pub async fn verify_forwarder_functional_readiness(
    endpoint: SocketAddr,
    protocol: ReadinessProtocol,
    target: &str,
    timeout: Duration,
) -> Result<FunctionalReadinessEvidence, FunctionalReadinessError> {
    let started = Instant::now();
    let (query, wire) = readiness_query(target).map_err(|detail| FunctionalReadinessError {
        protocol,
        stage: "query_build",
        detail,
    })?;
    let transaction_id = query.metadata.id;
    let operation = async {
        match protocol {
            ReadinessProtocol::Udp => {
                let bind = if endpoint.is_ipv6() {
                    "[::1]:0"
                } else {
                    "127.0.0.1:0"
                };
                let socket = tokio::net::UdpSocket::bind(bind)
                    .await
                    .map_err(|e| ("bind", e.to_string()))?;
                socket
                    .send_to(&wire, endpoint)
                    .await
                    .map_err(|e| ("send", e.to_string()))?;
                let mut response = vec![0u8; 65_535];
                let (length, _) = socket
                    .recv_from(&mut response)
                    .await
                    .map_err(|e| ("receive", e.to_string()))?;
                validate_readiness_response(&query, &response[..length])
                    .map_err(|e| ("validate", e))
            }
            ReadinessProtocol::Tcp => {
                let mut stream = tokio::net::TcpStream::connect(endpoint)
                    .await
                    .map_err(|e| ("connect", e.to_string()))?;
                stream
                    .write_u16(wire.len() as u16)
                    .await
                    .map_err(|e| ("write_length", e.to_string()))?;
                stream
                    .write_all(&wire)
                    .await
                    .map_err(|e| ("write_query", e.to_string()))?;
                let length = stream
                    .read_u16()
                    .await
                    .map_err(|e| ("read_length", e.to_string()))?
                    as usize;
                if length == 0 {
                    return Err(("read_length", "zero-length DNS response".into()));
                }
                let mut response = vec![0u8; length];
                stream
                    .read_exact(&mut response)
                    .await
                    .map_err(|e| ("read_body", e.to_string()))?;
                validate_readiness_response(&query, &response).map_err(|e| ("validate", e))
            }
        }
    };
    let answer_count = tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| FunctionalReadinessError {
            protocol,
            stage: "timeout",
            detail: format!("no valid response within {timeout:?}"),
        })?
        .map_err(|(stage, detail)| FunctionalReadinessError {
            protocol,
            stage,
            detail,
        })?;
    Ok(FunctionalReadinessEvidence {
        protocol,
        transaction_id,
        answer_count,
        elapsed: started.elapsed(),
    })
}

pub async fn verify_forwarder_udp_and_tcp_readiness(
    endpoint: SocketAddr,
    target: &str,
    timeout: Duration,
) -> Result<[FunctionalReadinessEvidence; 2], FunctionalReadinessError> {
    let udp =
        verify_forwarder_functional_readiness(endpoint, ReadinessProtocol::Udp, target, timeout)
            .await?;
    let tcp =
        verify_forwarder_functional_readiness(endpoint, ReadinessProtocol::Tcp, target, timeout)
            .await?;
    Ok([udp, tcp])
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_resolver::proto::rr::{rdata::A, RData, Record};
    use std::net::Ipv4Addr;

    fn response_for(query_wire: &[u8], code: ResponseCode, wrong_id: bool) -> Vec<u8> {
        let query = Message::from_bytes(query_wire).unwrap();
        let mut response = Message::response(
            query.metadata.id.wrapping_add(u16::from(wrong_id)),
            query.metadata.op_code,
        );
        response.metadata.recursion_desired = query.metadata.recursion_desired;
        response.metadata.recursion_available = true;
        response.metadata.response_code = code;
        response.add_query(query.queries[0].clone());
        if code == ResponseCode::NoError {
            response.add_answer(Record::from_rdata(
                query.queries[0].name().clone(),
                30,
                RData::A(A(Ipv4Addr::new(192, 0, 2, 1))),
            ));
        }
        response.to_bytes().unwrap()
    }

    async fn udp_server(mode: &'static str) -> SocketAddr {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        tokio::spawn(async move {
            let mut buffer = [0u8; 512];
            let (length, peer) = socket.recv_from(&mut buffer).await.unwrap();
            if mode == "silent" {
                return;
            }
            let response = match mode {
                "valid" => response_for(&buffer[..length], ResponseCode::NoError, false),
                "wrong_id" => response_for(&buffer[..length], ResponseCode::NoError, true),
                "servfail" => response_for(&buffer[..length], ResponseCode::ServFail, false),
                _ => vec![0, 1, 2],
            };
            socket.send_to(&response, peer).await.unwrap();
        });
        address
    }

    async fn tcp_server(mode: &'static str) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            if mode == "silent" {
                tokio::time::sleep(Duration::from_secs(1)).await;
                return;
            }
            let length = stream.read_u16().await.unwrap() as usize;
            let mut query = vec![0; length];
            stream.read_exact(&mut query).await.unwrap();
            if mode == "invalid_length" {
                stream.write_u16(0).await.unwrap();
                return;
            }
            let response = response_for(&query, ResponseCode::NoError, false);
            stream.write_u16(response.len() as u16).await.unwrap();
            if mode == "partial" {
                stream.write_all(&response[..3]).await.unwrap();
                return;
            }
            stream.write_all(&response).await.unwrap();
        });
        address
    }

    async fn check(
        address: SocketAddr,
        protocol: ReadinessProtocol,
    ) -> Result<FunctionalReadinessEvidence, FunctionalReadinessError> {
        verify_forwarder_functional_readiness(
            address,
            protocol,
            "example.com",
            Duration::from_millis(150),
        )
        .await
    }

    #[tokio::test]
    async fn udp_socket_only_without_response_is_not_ready() {
        assert!(check(udp_server("silent").await, ReadinessProtocol::Udp)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn udp_valid_dns_response_is_ready() {
        let result = check(udp_server("valid").await, ReadinessProtocol::Udp).await;
        assert!(result.is_ok(), "{result:?}");
    }
    #[tokio::test]
    async fn udp_wrong_transaction_id_is_rejected() {
        assert!(check(udp_server("wrong_id").await, ReadinessProtocol::Udp)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn udp_malformed_response_is_rejected() {
        assert!(check(udp_server("malformed").await, ReadinessProtocol::Udp)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn udp_servfail_is_rejected() {
        assert!(check(udp_server("servfail").await, ReadinessProtocol::Udp)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn udp_timeout_is_rejected() {
        assert!(check(udp_server("silent").await, ReadinessProtocol::Udp)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn tcp_listener_only_without_response_is_not_ready() {
        assert!(check(tcp_server("silent").await, ReadinessProtocol::Tcp)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn tcp_valid_dns_response_is_ready() {
        let result = check(tcp_server("valid").await, ReadinessProtocol::Tcp).await;
        assert!(result.is_ok(), "{result:?}");
    }
    #[tokio::test]
    async fn tcp_invalid_length_is_rejected() {
        assert!(
            check(tcp_server("invalid_length").await, ReadinessProtocol::Tcp)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn tcp_partial_response_is_rejected() {
        assert!(check(tcp_server("partial").await, ReadinessProtocol::Tcp)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn tcp_timeout_is_rejected() {
        assert!(check(tcp_server("silent").await, ReadinessProtocol::Tcp)
            .await
            .is_err());
    }
}
