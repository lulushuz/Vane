use hickory_resolver::proto::{
    op::{Message, MessageType, OpCode, Query},
    rr::{Name, RecordType},
    serialize::binary::{BinDecodable, BinEncodable},
};
use serde::Serialize;

/// Known DoH endpoints. Both support RFC 8484 (application/dns-message).
pub const DOH_CLOUDFLARE: &str = "https://cloudflare-dns.com/dns-query";
pub const DOH_GOOGLE: &str = "https://dns.google/dns-query";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DohResult {
    pub endpoint: String,
    pub domain: String,
    pub addresses: Vec<String>,
    pub latency_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Resolves a domain using DNS-over-HTTPS (RFC 8484).
///
/// Sends DNS wire-format queries via HTTPS POST to bypass ISP DNS interception.
/// The query is indistinguishable from regular HTTPS traffic on port 443.
pub async fn resolve_doh(client: &reqwest::Client, endpoint: &str, domain: &str) -> DohResult {
    let start = std::time::Instant::now();

    let result = resolve_doh_inner(client, endpoint, domain).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(addresses) => DohResult {
            endpoint: endpoint.to_string(),
            domain: domain.to_string(),
            addresses,
            latency_ms,
            success: true,
            error: None,
        },
        Err(e) => DohResult {
            endpoint: endpoint.to_string(),
            domain: domain.to_string(),
            addresses: vec![],
            latency_ms,
            success: false,
            error: Some(e),
        },
    }
}

async fn resolve_doh_inner(
    client: &reqwest::Client,
    endpoint: &str,
    domain: &str,
) -> Result<Vec<String>, String> {
    // Build the DNS query in wire-format (RFC 1035).
    let name = Name::from_utf8(domain).map_err(|e| format!("Geçersiz domain adı: {}", e))?;

    let mut query = Query::new();
    query.set_name(name);
    query.set_query_type(RecordType::A);

    let mut message = Message::new(rand_id(), MessageType::Query, OpCode::Query);
    message.metadata.recursion_desired = true;
    message.add_query(query);

    let wire_bytes = message
        .to_bytes()
        .map_err(|e| format!("DNS encode hatası: {}", e))?;

    // RFC 8484: POST with Content-Type: application/dns-message
    let response = client
        .post(endpoint)
        .header("Content-Type", "application/dns-message")
        .header("Accept", "application/dns-message")
        .body(wire_bytes)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("DoH istek hatası: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("DoH HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("DoH yanıt okunamadı: {}", e))?;

    let reply = Message::from_bytes(&bytes).map_err(|e| format!("DNS decode hatası: {}", e))?;

    let addresses: Vec<String> = reply
        .answers
        .iter()
        .filter_map(|rr| {
            // Extract A (IPv4) records as strings.
            if let hickory_resolver::proto::rr::RData::A(ip) = &rr.data {
                Some(ip.to_string())
            } else {
                None
            }
        })
        .collect();

    if addresses.is_empty() {
        return Err("DNS yanıtı boş (NXDOMAIN veya engellenmiş olabilir).".into());
    }

    Ok(addresses)
}

/// Generates a pseudo-random DNS message ID using a secure RNG.
fn rand_id() -> u16 {
    rand::random::<u16>()
}
