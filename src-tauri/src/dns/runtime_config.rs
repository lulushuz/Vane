use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigCandidate {
    pub enabled: bool,
    pub protocol: String,
    pub provider: Option<String>,
    pub adblock: bool,
    pub cache_enabled: bool,
    pub socks5: Option<DnsSocksCandidate>,
    pub kill_switch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsSocksCandidate {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsProtocol {
    Doh,
    Dot,
}

impl DnsProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Doh => "doh",
            Self::Dot => "dot",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsProvider {
    Cloudflare,
    Google,
}

impl DnsProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cloudflare => "cloudflare",
            Self::Google => "google",
        }
    }

    pub fn doh_url(&self) -> &'static str {
        match self {
            Self::Cloudflare => "https://cloudflare-dns.com/dns-query",
            Self::Google => "https://dns.google/dns-query",
        }
    }

    pub fn dot_host(&self) -> &'static str {
        match self {
            Self::Cloudflare => "1.1.1.1",
            Self::Google => "8.8.8.8",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedDnsSocks {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl fmt::Debug for VerifiedDnsSocks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifiedDnsSocks")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username.as_ref().map(|_| "[REDACTED]"))
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DnsConfigRevision(pub u64);

impl DnsConfigRevision {
    pub fn new(rev: u64) -> Self {
        Self(rev)
    }
    pub fn get(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DnsConfigFingerprint(pub String);

impl DnsConfigFingerprint {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn compute(
        enabled: bool,
        protocol: DnsProtocol,
        provider: DnsProvider,
        adblock: bool,
        cache_enabled: bool,
        socks5: Option<&VerifiedDnsSocks>,
        kill_switch: bool,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"vane-dns-v1\n");
        hasher.update(format!("enabled={}\n", enabled).as_bytes());
        hasher.update(format!("protocol={}\n", protocol.as_str()).as_bytes());
        hasher.update(format!("provider={}\n", provider.as_str()).as_bytes());
        hasher.update(format!("adblock={}\n", adblock).as_bytes());
        hasher.update(format!("cache={}\n", cache_enabled).as_bytes());
        if let Some(socks) = socks5 {
            hasher.update(
                format!(
                    "socks_enabled=true\nsocks_host={}\nsocks_port={}\n",
                    socks.host, socks.port
                )
                .as_bytes(),
            );
        } else {
            hasher.update(b"socks_enabled=false\n");
        }
        hasher.update(format!("kill_switch={}\n", kill_switch).as_bytes());

        let digest = hasher.finalize();
        let hex_str: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        Self(hex_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedDnsConfig {
    pub revision: DnsConfigRevision,
    pub fingerprint: DnsConfigFingerprint,
    pub enabled: bool,
    pub protocol: DnsProtocol,
    pub provider: DnsProvider,
    pub adblock: bool,
    pub cache_enabled: bool,
    pub socks5: Option<VerifiedDnsSocks>,
    pub kill_switch: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DnsValidationError {
    #[error("DoQ is not supported")]
    UnsupportedProtocolDoQ,
    #[error("Unsupported DNS protocol: {0}")]
    UnsupportedProtocol(String),
    #[error("Unsupported DNS provider: {0}")]
    UnsupportedProvider(String),
    #[error("Invalid SOCKS5 host: {0}")]
    InvalidSocksHost(String),
    #[error("Invalid SOCKS5 port: {0}")]
    InvalidSocksPort(u16),
    #[error("DoT protocol cannot be combined with SOCKS5 proxy to prevent DNS leaks")]
    DotWithSocks5NotAllowed,
}

pub fn verify_dns_config(
    candidate: DnsConfigCandidate,
    revision: DnsConfigRevision,
) -> Result<VerifiedDnsConfig, DnsValidationError> {
    let protocol_str = candidate.protocol.to_lowercase();
    if protocol_str == "doq" {
        return Err(DnsValidationError::UnsupportedProtocolDoQ);
    }
    let protocol = match protocol_str.as_str() {
        "doh" => DnsProtocol::Doh,
        "dot" => DnsProtocol::Dot,
        other => return Err(DnsValidationError::UnsupportedProtocol(other.to_string())),
    };

    let provider = match candidate
        .provider
        .as_deref()
        .unwrap_or("cloudflare")
        .to_lowercase()
        .as_str()
    {
        "cloudflare" => DnsProvider::Cloudflare,
        "google" => DnsProvider::Google,
        other => return Err(DnsValidationError::UnsupportedProvider(other.to_string())),
    };

    let socks5 = if let Some(socks_cand) = candidate.socks5 {
        let host = socks_cand.host.trim().to_string();
        if host.is_empty() || host.contains(' ') || host.contains('\r') || host.contains('\n') {
            return Err(DnsValidationError::InvalidSocksHost(host));
        }
        if socks_cand.port == 0 {
            return Err(DnsValidationError::InvalidSocksPort(0));
        }
        Some(VerifiedDnsSocks {
            host,
            port: socks_cand.port,
            username: socks_cand.username.filter(|s| !s.trim().is_empty()),
            password: socks_cand.password.filter(|s| !s.trim().is_empty()),
        })
    } else {
        None
    };

    if protocol == DnsProtocol::Dot && socks5.is_some() {
        return Err(DnsValidationError::DotWithSocks5NotAllowed);
    }

    let fingerprint = DnsConfigFingerprint::compute(
        candidate.enabled,
        protocol,
        provider,
        candidate.adblock,
        candidate.cache_enabled,
        socks5.as_ref(),
        candidate.kill_switch,
    );

    Ok(VerifiedDnsConfig {
        revision,
        fingerprint,
        enabled: candidate.enabled,
        protocol,
        provider,
        adblock: candidate.adblock,
        cache_enabled: candidate.cache_enabled,
        socks5,
        kill_switch: candidate.kill_switch,
    })
}
