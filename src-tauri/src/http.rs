use crate::ipc::IpcError;
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Instant;

const DNS_DIAGNOSIS_TARGETS: &[(&str, &str, u16)] = &[
    ("discord.com", "162.159.135.232", 443),
    ("www.discord.com", "162.159.135.232", 443),
    ("x.com", "104.244.42.65", 443),
    ("twitter.com", "104.244.42.65", 443),
    ("youtube.com", "142.250.185.14", 443),
    ("www.youtube.com", "142.250.185.14", 443),
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResult {
    pub success: bool,
    pub latency_ms: u64,
    pub status_code: Option<u16>,
    pub error: Option<String>,
}

/// Advanced result structure that independently tests DNS resolution and HTTP connection.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsCheckResult {
    /// Was DNS resolution successful via system DNS?
    pub system_dns_ok: bool,
    /// Was DNS resolution successful via Cloudflare DoH (DNS-over-HTTPS)?
    pub doh_dns_ok: bool,
    /// Summary indicating whether the issue is DNS or DPI.
    pub diagnosis: String,
    /// Recommended solution to show the user.
    pub recommendation: String,
}

#[tauri::command]
pub async fn check_url_health(url: String) -> Result<PingResult, IpcError> {
    const OPERATION: &str = "check_url_health";
    let (target_url, hostname) = parse_public_https_target(&url, OPERATION)?;
    let client = build_restricted_public_client(&hostname, OPERATION).await?;

    let start = Instant::now();

    match client.head(target_url).send().await {
        Ok(response) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let status_code = response.status().as_u16();
            let success = response.status().is_success()
                || response.status().is_redirection()
                || status_code == 405;

            Ok(PingResult {
                success,
                latency_ms,
                status_code: Some(status_code),
                error: if success {
                    None
                } else {
                    Some(format!("HTTP {}", status_code))
                },
            })
        }
        Err(e) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let error_msg = if e.is_connect() {
                format!("Connection error (likely DNS block): {}", e)
            } else if e.is_timeout() {
                "Timeout — possibly DPI block or slow connection.".to_string()
            } else {
                e.to_string()
            };

            tracing::warn!(
                "Public URL health check failed for '{}': {}",
                hostname,
                error_msg
            );

            Ok(PingResult {
                success: false,
                latency_ms,
                status_code: e.status().map(|s| s.as_u16()),
                error: Some(error_msg),
            })
        }
    }
}

async fn build_restricted_public_client(
    hostname: &str,
    operation: &'static str,
) -> Result<reqwest::Client, IpcError> {
    let mut addresses: Vec<SocketAddr> = tokio::net::lookup_host((hostname, 443))
        .await
        .map_err(|error| {
            IpcError::runtime(
                operation,
                "HEALTH_CHECK_DNS_FAILED",
                format!("The health-check hostname could not be resolved: {error}"),
            )
        })?
        .collect();
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err(IpcError::validation(
            operation,
            "HEALTH_CHECK_TARGET_NOT_PUBLIC",
            "Health checks are restricted to hostnames that resolve only to public addresses.",
        ));
    }
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .resolve_to_addrs(hostname, &addresses)
        .build()
        .map_err(|error| {
            IpcError::runtime(
                operation,
                "HEALTH_CHECK_CLIENT_FAILED",
                format!("The restricted health-check client could not be created: {error}"),
            )
        })
}

fn parse_public_https_target(
    input: &str,
    operation: &'static str,
) -> Result<(reqwest::Url, String), IpcError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(IpcError::validation(
            operation,
            "HEALTH_CHECK_TARGET_EMPTY",
            "A health-check hostname is required.",
        ));
    }
    let candidate = if input.contains("://") {
        input.to_string()
    } else {
        format!("https://{input}")
    };
    let url = reqwest::Url::parse(&candidate).map_err(|_| {
        IpcError::validation(
            operation,
            "HEALTH_CHECK_TARGET_INVALID",
            "The health-check target is not a valid HTTPS URL.",
        )
    })?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443)
    {
        return Err(IpcError::validation(
            operation,
            "HEALTH_CHECK_TARGET_INVALID",
            "Health checks require HTTPS on port 443 without embedded credentials.",
        ));
    }
    let hostname = url
        .host_str()
        .ok_or_else(|| {
            IpcError::validation(
                operation,
                "HEALTH_CHECK_TARGET_INVALID",
                "The health-check target has no hostname.",
            )
        })?
        .to_string();
    if hostname
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse::<IpAddr>()
        .is_ok()
    {
        return Err(IpcError::validation(
            operation,
            "HEALTH_CHECK_IP_LITERAL_REJECTED",
            "Health checks require a public hostname; IP literals are not accepted.",
        ));
    }
    Ok((url, hostname))
}

fn is_public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_ipv4(address),
        IpAddr::V6(address) => is_public_ipv6(address),
    }
}

fn is_public_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, c, _] = address.octets();
    !matches!(
        (a, b, c),
        (0, _, _)
            | (10, _, _)
            | (100, 64..=127, _)
            | (127, _, _)
            | (169, 254, _)
            | (172, 16..=31, _)
            | (192, 0, 0)
            | (192, 0, 2)
            | (192, 88, 99)
            | (192, 168, _)
            | (198, 18..=19, _)
            | (198, 51, 100)
            | (203, 0, 113)
            | (224..=255, _, _)
    )
}

fn is_public_ipv6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    let segments = address.segments();
    segments[0] & 0xe000 == 0x2000
        && !address.is_unspecified()
        && !address.is_loopback()
        && segments[0] & 0xfe00 != 0xfc00
        && segments[0] & 0xffc0 != 0xfe80
        && segments[0] & 0xffc0 != 0xfec0
        && segments[0] & 0xff00 != 0xff00
        && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
}

fn dns_diagnosis_target(hostname: &str) -> Option<(&'static str, u16)> {
    DNS_DIAGNOSIS_TARGETS
        .iter()
        .find(|(target, _, _)| *target == hostname)
        .map(|(_, address, port)| (*address, *port))
}

/// Determines whether the system is affected by a DNS block or a DPI block.
///
/// Method:
/// 1. Try to connect to target using System DNS.
/// 2. Try to connect directly to known IPs (bypassing DNS entirely).
/// The difference between the two results reveals the source of the issue.
#[tauri::command]
pub async fn check_dns_block(domain: String) -> Result<DnsCheckResult, IpcError> {
    const OPERATION: &str = "check_dns_block";

    let (target_url, hostname) = parse_public_https_target(&domain, OPERATION)?;
    let (direct_ip, direct_port) = dns_diagnosis_target(&hostname).ok_or_else(|| {
        IpcError::validation(
            OPERATION,
            "DNS_DIAGNOSIS_TARGET_UNSUPPORTED",
            "DNS diagnosis is restricted to the built-in public test targets.",
        )
    })?;
    let client = build_restricted_public_client(&hostname, OPERATION).await?;

    // --- Step 1: System DNS test ---
    let system_dns_ok = client
        .head(target_url)
        .send()
        .await
        .map(|r| r.status().as_u16() < 500)
        .unwrap_or(false);

    // --- Step 2: Direct IP test (Bypasses DNS) ---
    let addr: SocketAddr = format!("{direct_ip}:{direct_port}")
        .parse()
        .map_err(|error| {
            IpcError::runtime(
                OPERATION,
                "DNS_DIAGNOSIS_ADDRESS_INVALID",
                format!("A built-in DNS diagnosis address is invalid: {error}"),
            )
        })?;

    // Create a fresh client with resolve override at the builder-level
    let direct_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .pool_max_idle_per_host(0)
        .resolve(&hostname, addr)
        .build();

    let doh_dns_ok = match direct_client {
        Ok(c) => c
            .head(format!("https://{hostname}/"))
            .send()
            .await
            .map(|r| r.status().as_u16() < 500)
            .unwrap_or(false),
        Err(_) => false,
    };

    let (diagnosis, recommendation) = match (system_dns_ok, doh_dns_ok) {
        (true, _) => (
            "Your connection is working perfectly! Neither DPI nor DNS is blocking.".to_string(),
            "No changes needed.".to_string(),
        ),
        (false, true) => (
            "🔍 DNS Block Detected! IP connection works but your system DNS is poisoned."
                .to_string(),
            "Change your DNS server to Cloudflare (1.1.1.1) or Google (8.8.8.8). \
            Windows Settings > Network & Internet > Ethernet/Wi-Fi > DNS server assignment."
                .to_string(),
        ),
        (false, false) => (
            "⚠️ DPI + DNS Block: Both system DNS and direct IP access are blocked.".to_string(),
            "1) First change your DNS to 1.1.1.1. \
            2) Then find the best DPI bypass method using Smart Scan. \
            3) When both are working, discord.com will be accessible."
                .to_string(),
        ),
    };

    Ok(DnsCheckResult {
        system_dns_ok,
        doh_dns_ok,
        diagnosis,
        recommendation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_target_requires_public_https_hostname() {
        assert!(parse_public_https_target("example.com", "test").is_ok());
        assert!(parse_public_https_target("https://example.com/path", "test").is_ok());
        for target in [
            "http://example.com",
            "https://user@example.com",
            "https://example.com:8443",
            "https://127.0.0.1",
            "https://[::1]",
        ] {
            assert!(
                parse_public_https_target(target, "test").is_err(),
                "accepted {target}"
            );
        }
    }

    #[test]
    fn private_reserved_and_documentation_addresses_are_rejected() {
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "172.16.0.1",
            "192.168.1.1",
            "192.0.2.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "64:ff9b::7f00:1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(
                !is_public_ip(address.parse().expect("test IP")),
                "accepted {address}"
            );
        }
        assert!(is_public_ip("1.1.1.1".parse().expect("public IPv4")));
        assert!(is_public_ip(
            "2606:4700:4700::1111".parse().expect("public IPv6")
        ));
    }

    #[test]
    fn dns_diagnosis_accepts_only_built_in_public_targets() {
        assert!(dns_diagnosis_target("discord.com").is_some());
        assert!(dns_diagnosis_target("youtube.com").is_some());
        assert!(dns_diagnosis_target("localhost").is_none());
        assert!(dns_diagnosis_target("metadata.google.internal").is_none());
    }
}
