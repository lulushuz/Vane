use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeasurementTargetId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementProtocol {
    TcpConnect,
    Http,
    Https,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementTarget {
    pub id: MeasurementTargetId,
    pub host: String,
    pub port: u16,
    pub path: Option<String>,
    pub protocol: MeasurementProtocol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedMeasurementTarget {
    pub target: MeasurementTarget,
    pub resolved_addrs: Vec<SocketAddr>,
}

pub fn default_measurement_targets() -> Vec<MeasurementTarget> {
    vec![
        MeasurementTarget {
            id: MeasurementTargetId("youtube_target".into()),
            host: "www.youtube.com".into(),
            port: 443,
            path: Some("/".into()),
            protocol: MeasurementProtocol::Https,
        },
        MeasurementTarget {
            id: MeasurementTargetId("discord_target".into()),
            host: "discord.com".into(),
            port: 443,
            path: Some("/".into()),
            protocol: MeasurementProtocol::Https,
        },
        MeasurementTarget {
            id: MeasurementTargetId("x_target".into()),
            host: "x.com".into(),
            port: 443,
            path: Some("/".into()),
            protocol: MeasurementProtocol::Https,
        },
    ]
}
