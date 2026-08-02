use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxFilterIntent {
    pub tcp_ports: Vec<PortRange>,
    pub udp_ports: Vec<PortRange>,
    pub requires_quic: bool,
    pub hostlist_mode: String,
}

impl LinuxFilterIntent {
    pub fn from_specs(
        declared_tcp: Option<&str>,
        declared_udp: Option<&str>,
        hostlist_mode: &str,
    ) -> Self {
        let tcp_ports = parse_port_spec(declared_tcp.unwrap_or(""));
        let udp_ports = parse_port_spec(declared_udp.unwrap_or(""));

        let requires_quic = udp_ports.iter().any(|r| r.start <= 443 && 443 <= r.end);

        Self {
            tcp_ports,
            udp_ports,
            requires_quic,
            hostlist_mode: hostlist_mode.to_string(),
        }
    }
}

pub fn parse_port_spec(spec: &str) -> Vec<PortRange> {
    let mut ranges = Vec::new();
    let spec = spec.trim();
    if spec.is_empty() {
        return ranges;
    }
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start_str, end_str)) = part.split_once('-') {
            if let (Ok(start), Ok(end)) = (
                start_str.trim().parse::<u16>(),
                end_str.trim().parse::<u16>(),
            ) {
                if start > 0 && end >= start {
                    ranges.push(PortRange { start, end });
                }
            }
        } else if let Ok(port) = part.parse::<u16>() {
            if port > 0 {
                ranges.push(PortRange {
                    start: port,
                    end: port,
                });
            }
        }
    }
    ranges.sort_by_key(|r| (r.start, r.end));
    ranges.dedup();
    ranges
}
