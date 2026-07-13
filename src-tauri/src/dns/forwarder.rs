/* 
   Local DNS Forwarder with AdBlock, Cache, and DoH/DoT Protocols
*/

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::collections::{HashSet, HashMap};
use tokio::net::UdpSocket;
use tauri::{AppHandle, Manager, Emitter};

use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::{ResolverConfig, ResolverOpts, NameServerConfig, Protocol};
use hickory_resolver::proto::op::{Message, MessageType, ResponseCode};
use hickory_resolver::proto::rr::{Record, RecordType, RData, rdata::A};
use hickory_resolver::proto::serialize::binary::{BinDecodable, BinEncodable};

pub const DOH_FORWARDER_DEFAULT_PORT: u16 = 53;

// Endpoint options for the DoH/DoT upstream resolver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoHEndpoint {
    Cloudflare,
    Google,
}

impl DoHEndpoint {
    pub fn url(&self) -> &'static str {
        match self {
            Self::Cloudflare => "https://cloudflare-dns.com/dns-query",
            Self::Google => "https://dns.google/dns-query",
        }
    }
}

// ─── ADBLOCK (HOSTS FILTERING) ───
static ADBLOCK_LIST: RwLock<Option<HashSet<String>>> = RwLock::new(None);

pub fn parse_hosts_file(content: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let ip = parts[0];
            let domain = parts[1].to_lowercase();
            if ip == "0.0.0.0" || ip == "127.0.0.1" {
                if domain != "localhost" {
                    set.insert(domain);
                }
            }
        }
    }
    set
}

pub fn initialize_adblock(app_handle: &AppHandle) {
    if let Ok(app_data) = app_handle.path().app_data_dir() {
        let cache_path = app_data.join("adblock_cache.txt");
        if cache_path.exists() {
            if let Ok(text) = std::fs::read_to_string(cache_path) {
                let set = parse_hosts_file(&text);
                if let Ok(mut guard) = ADBLOCK_LIST.write() {
                    *guard = Some(set);
                }
                tracing::info!("AdBlock listesi önbellekten yüklendi.");
                return;
            }
        }
    }
    
    // List is empty initially
    if let Ok(mut guard) = ADBLOCK_LIST.write() {
        if guard.is_none() {
            *guard = Some(HashSet::new());
        }
    }

    // Spawn download in background
    let handle = app_handle.clone();
    tokio::spawn(async move {
        update_adblock_list(handle).await;
    });
}

pub async fn update_adblock_list(app_handle: AppHandle) {
    let Ok(app_data) = app_handle.path().app_data_dir() else { return; };
    let cache_path = app_data.join("adblock_cache.txt");
    
    let url = "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts";
    let client = reqwest::Client::new();
    match client.get(url).timeout(std::time::Duration::from_secs(15)).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    let _ = std::fs::create_dir_all(&app_data);
                    if let Err(e) = std::fs::write(&cache_path, &text) {
                        tracing::error!("AdBlock önbellek dosyası yazılamadı: {}", e);
                    } else {
                        let set = parse_hosts_file(&text);
                        if let Ok(mut guard) = ADBLOCK_LIST.write() {
                            *guard = Some(set);
                        }
                        tracing::info!("AdBlock listesi başarıyla güncellendi.");
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!("AdBlock listesi indirilemedi (çevrimdışı veya zaman aşımı): {}", e);
        }
    }
}

fn is_domain_blocked(domain: &str) -> bool {
    let guard = match ADBLOCK_LIST.read() {
        Ok(g) => g,
        Err(_) => return false,
    };
    let Some(ref set) = *guard else { return false; };
    if set.contains(domain) {
        return true;
    }
    // Check wildcard subdomains
    let parts: Vec<&str> = domain.split('.').collect();
    for i in 1..parts.len() {
        let parent = parts[i..].join(".");
        if set.contains(&parent) {
            return true;
        }
    }
    false
}

// ─── DNS CACHE ───
struct CacheEntry {
    response_bytes: bytes::Bytes,
    expires_at: std::time::Instant,
}

static DNS_CACHE: RwLock<Option<HashMap<String, CacheEntry>>> = RwLock::new(None);

pub fn init_dns_cache() {
    if let Ok(mut guard) = DNS_CACHE.write() {
        if guard.is_none() {
            *guard = Some(HashMap::new());
        }
    }
}

pub fn clear_dns_cache() {
    if let Ok(mut guard) = DNS_CACHE.write() {
        if let Some(cache) = guard.as_mut() {
            cache.clear();
        }
    }
}

fn get_cached_dns(qname: &str, qtype: u16) -> Option<bytes::Bytes> {
    let guard = DNS_CACHE.read().ok()?;
    let cache = guard.as_ref()?;
    let entry = cache.get(&format!("{}:{}", qname, qtype))?;
    if std::time::Instant::now() < entry.expires_at {
        Some(entry.response_bytes.clone())
    } else {
        None
    }
}

fn set_cached_dns(qname: &str, qtype: u16, response_bytes: bytes::Bytes, ttl: u32) {
    if let Ok(mut guard) = DNS_CACHE.write() {
        if let Some(ref mut cache) = *guard {
            if cache.len() >= 5000 {
                cache.clear();
            }
            let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(ttl as u64);
            cache.insert(
                format!("{}:{}", qname, qtype),
                CacheEntry { response_bytes, expires_at }
            );
        }
    }
}

// ─── RESOLVERS POOL FOR DoT ───
static DOT_RESOLVER_CLOUDFLARE: RwLock<Option<TokioAsyncResolver>> = RwLock::new(None);
static DOT_RESOLVER_GOOGLE: RwLock<Option<TokioAsyncResolver>> = RwLock::new(None);

fn build_dot_resolver(endpoint: DoHEndpoint) -> Result<TokioAsyncResolver, String> {
    let mut config = ResolverConfig::new();
    let (ip, name) = match endpoint {
        DoHEndpoint::Cloudflare => (std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)), "cloudflare-dns.com"),
        DoHEndpoint::Google => (std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)), "dns.google"),
    };
    let socket_addr = std::net::SocketAddr::new(ip, 853);
    let ns = NameServerConfig {
        socket_addr,
        protocol: Protocol::Tls,
        tls_dns_name: Some(name.to_string()),
        trust_negative_responses: true,
        bind_addr: None,
        tls_config: None,
    };
    config.add_name_server(ns);
    let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());
    Ok(resolver)
}

fn get_or_create_dot_resolver(endpoint: DoHEndpoint) -> Option<TokioAsyncResolver> {
    let lock = match endpoint {
        DoHEndpoint::Cloudflare => &DOT_RESOLVER_CLOUDFLARE,
        DoHEndpoint::Google => &DOT_RESOLVER_GOOGLE,
    };
    
    if let Ok(guard) = lock.read() {
        if let Some(ref r) = *guard {
            return Some(r.clone());
        }
    }
    
    let resolver = build_dot_resolver(endpoint).ok()?;
    if let Ok(mut guard) = lock.write() {
        *guard = Some(resolver.clone());
    }
    Some(resolver)
}

// ─── SETTINGS READING & CACHING ───
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsSettings {
    pub protocol: String,
    pub adblock: bool,
    pub cache: bool,
    pub socks5_proxy: String,
}

static DNS_SETTINGS_CACHE: RwLock<Option<DnsSettings>> = RwLock::new(None);

pub fn update_dns_settings_cache(settings: DnsSettings) {
    if !settings.cache {
        clear_dns_cache();
    }
    if let Ok(mut guard) = DNS_SETTINGS_CACHE.write() {
        *guard = Some(settings);
    }
}

pub fn read_dns_settings(app: &AppHandle) -> DnsSettings {
    if let Ok(guard) = DNS_SETTINGS_CACHE.read() {
        if let Some(ref cached) = *guard {
            return cached.clone();
        }
    }

    let default_settings = DnsSettings {
        protocol: "doh".to_string(),
        adblock: false,
        cache: true,
        socks5_proxy: "".to_string(),
    };
    let Ok(app_data) = app.path().app_data_dir() else { return default_settings; };
    let settings_path = app_data.join("settings.json");
    if !settings_path.exists() {
        return default_settings;
    }

    let mut content = String::new();
    for _ in 0..3 {
        if let Ok(c) = std::fs::read_to_string(&settings_path) {
            content = c;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    if content.is_empty() {
        return default_settings;
    }

    let Ok(file_json) = serde_json::from_str::<serde_json::Value>(&content) else { return default_settings; };
    let Some(zustand_raw) = file_json.get("vane-settings") else { return default_settings; };
    let Ok(zustand_json) = (match zustand_raw {
        serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(&s),
        obj => Ok(obj.clone()),
    }) else { return default_settings; };

    let state = zustand_json.get("state");
    let protocol = state
        .and_then(|s| s.get("dnsProtocol"))
        .and_then(|p| p.as_str())
        .unwrap_or("doh")
        .to_string();
    let protocol = if protocol == "dot" { "dot".to_string() } else { "doh".to_string() };
    let adblock = state
        .and_then(|s| s.get("dnsAdBlock"))
        .and_then(|a| a.as_bool())
        .unwrap_or(false);
    let cache = state
        .and_then(|s| s.get("dnsCache"))
        .and_then(|c| c.as_bool())
        .unwrap_or(true);
    let socks5_proxy = state
        .and_then(|s| s.get("proxySocks5"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();

    let res = DnsSettings { protocol, adblock, cache, socks5_proxy };
    if let Ok(mut guard) = DNS_SETTINGS_CACHE.write() {
        *guard = Some(res.clone());
    }
    res
}

pub struct ForwarderHandle {
    pub port: u16,
    pub endpoint: DoHEndpoint,
    pub shutdown: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}

impl ForwarderHandle {
    pub async fn stop(self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.task.abort();
        let _ = self.task.await;
        tracing::info!("DoH/DoT Forwarder durduruldu (port {}).", self.port);
    }
}

pub async fn spawn_doh_forwarder(
    app: AppHandle,
    client: reqwest::Client,
    port: u16,
    endpoint: DoHEndpoint,
) -> Result<ForwarderHandle, String> {
    let addr = format!("127.0.0.1:{}", port);
    let socket = UdpSocket::bind(&addr)
        .await
        .map_err(|e| format!("DNS Forwarder port {} bağlanamadı: {}", port, e))?;

    let mut fallback_dns = crate::dns::get_active_adapters()
        .into_iter()
        .find_map(|a| a.current_primary_dns)
        .unwrap_or_else(|| "1.1.1.1".to_string());

    if fallback_dns == "127.0.0.1" || fallback_dns == "localhost" || fallback_dns.is_empty() {
        fallback_dns = "1.1.1.1".to_string();
    }

    // Initialize DNS tools
    initialize_adblock(&app);
    init_dns_cache();

    tracing::info!(
        "DNS Forwarder başlatıldı: {} (Fallback DNS: {})",
        addr,
        fallback_dns
    );

    let socket = Arc::new(socket);
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);

    let app_clone = app.clone();
    let task = tokio::spawn(async move {
        run_forwarder_loop(app_clone, socket, client, endpoint, fallback_dns, shutdown_clone).await;
    });

    Ok(ForwarderHandle { port, endpoint, shutdown, task })
}

async fn run_forwarder_loop(
    app: AppHandle,
    socket: Arc<UdpSocket>,
    client: reqwest::Client,
    endpoint: DoHEndpoint,
    fallback_dns: String,
    shutdown: Arc<AtomicBool>,
) {
    let mut buf = vec![0u8; 4096];
    const MAX_CONCURRENT_DNS_REQUESTS: usize = 100;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_DNS_REQUESTS));

    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        let recv_result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            socket.recv_from(&mut buf),
        ).await;

        let (len, client_addr) = match recv_result {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                tracing::warn!("DNS Forwarder recv hatası: {}", e);
                continue;
            }
            Err(_) => continue,
        };

        let query_bytes = bytes::Bytes::copy_from_slice(&buf[..len]);
        let socket_clone = Arc::clone(&socket);
        let client_clone = client.clone();
        let semaphore_clone = Arc::clone(&semaphore);
        let fallback_dns_clone = fallback_dns.clone();
        let app_clone = app.clone();

        let Ok(permit) = semaphore_clone.try_acquire_owned() else {
            tracing::warn!("DNS forwarder: concurrency limit reached, dropping query from {}", client_addr);
            continue;
        };

        tokio::spawn(async move {
            let _permit = permit;
            if let Some(response) = proxy_dns_query(app_clone, &client_clone, endpoint, &fallback_dns_clone, query_bytes).await {
                if let Err(e) = socket_clone.send_to(&response, client_addr).await {
                    tracing::warn!("DNS Forwarder send hatası → {}: {}", client_addr, e);
                }
            }
        });
    }
}

async fn query_fallback_dns(fallback_dns: &str, query_bytes: &[u8]) -> Option<bytes::Bytes> {
    let socket = UdpSocket::bind("0.0.0.0:0").await.ok()?;
    let target = format!("{}:53", fallback_dns);
    socket.send_to(query_bytes, &target).await.ok()?;
    
    let mut buf = vec![0u8; 4096];
    let recv_result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        socket.recv_from(&mut buf),
    ).await;
    
    match recv_result {
        Ok(Ok((len, _))) => Some(bytes::Bytes::copy_from_slice(&buf[..len])),
        _ => None,
    }
}

fn build_blocked_response(parsed: &Message, query: &hickory_resolver::proto::op::Query) -> bytes::Bytes {
    let mut response = Message::new();
    response.set_id(parsed.id());
    response.set_message_type(MessageType::Response);
    response.set_op_code(parsed.op_code());
    response.set_recursion_desired(parsed.recursion_desired());
    response.set_recursion_available(true);
    response.set_response_code(ResponseCode::NoError);
    response.add_query(query.clone());

    if query.query_type() == RecordType::A {
        let mut record = Record::new();
        record.set_name(query.name().clone());
        record.set_record_type(RecordType::A);
        record.set_dns_class(hickory_resolver::proto::rr::DNSClass::IN);
        record.set_ttl(3600);
        record.set_data(Some(RData::A(A(std::net::Ipv4Addr::new(0, 0, 0, 0)))));
        response.add_answer(record);
    }
    
    response.to_bytes().unwrap_or_default().into()
}

async fn proxy_dns_query(
    app: AppHandle,
    client: &reqwest::Client,
    endpoint: DoHEndpoint,
    fallback_dns: &str,
    query_bytes: bytes::Bytes,
) -> Option<bytes::Bytes> {
    let parsed = Message::from_bytes(&query_bytes)
        .map_err(|e| tracing::warn!("Geçersiz DNS sorgusu: {}", e))
        .ok()?;

    // Emit live activity for UI graph
    let _ = app.emit("dns_activity", ());

    let is_local = parsed.queries().iter().any(|q| {
        let name = q.name().to_string();
        name.ends_with(".local.") || name.ends_with(".lan.") || name.ends_with(".home.") || name.ends_with(".arpa.")
    });

    if is_local {
        if let Some(resp) = query_fallback_dns(fallback_dns, &query_bytes).await {
            return Some(resp);
        }
    }

    if let Some(query) = parsed.queries().first() {
        let qname = query.name().to_string();
        let qname_clean = qname.trim_end_matches('.').to_lowercase();
        let qtype = query.query_type();
        
        let settings = read_dns_settings(&app);
        
        // 1. AdBlock Filter
        if settings.adblock && is_domain_blocked(&qname_clean) {
            tracing::info!("AdBlock: Engellendi -> {}", qname_clean);
            return Some(build_blocked_response(&parsed, query));
        }

        // 2. RAM Cache Lookup
        if settings.cache {
            if let Some(cached) = get_cached_dns(&qname_clean, u16::from(qtype)) {
                let mut resp = cached.to_vec();
                if resp.len() >= 2 {
                    let tx_id = parsed.id().to_be_bytes();
                    resp[0] = tx_id[0];
                    resp[1] = tx_id[1];
                }
                return Some(bytes::Bytes::from(resp));
            }
        }

        // 3. Resolve via Protocols
        let response_bytes = if settings.protocol == "dot" {
            let resolver = get_or_create_dot_resolver(endpoint)?;
            let name = hickory_resolver::Name::from_utf8(&qname_clean).ok()?;
            let lookup = resolver.lookup(name, qtype).await.ok()?;
            
            let mut response = Message::new();
            response.set_id(parsed.id());
            response.set_message_type(MessageType::Response);
            response.set_op_code(parsed.op_code());
            response.set_recursion_desired(parsed.recursion_desired());
            response.set_recursion_available(true);
            response.set_response_code(ResponseCode::NoError);
            response.add_query(query.clone());
            
            for record in lookup.records() {
                response.add_answer(record.clone());
            }
            
            response.to_bytes().ok()?
        } else {
            // DoH
            let doh_client = if !settings.socks5_proxy.is_empty() {
                if let Ok(proxy) = reqwest::Proxy::all(format!("socks5://{}", settings.socks5_proxy)) {
                    reqwest::Client::builder()
                        .proxy(proxy)
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .unwrap_or_else(|_| client.clone())
                } else {
                    client.clone()
                }
            } else {
                client.clone()
            };

            let response = doh_client
                .post(endpoint.url())
                .header("Content-Type", "application/dns-message")
                .header("Accept", "application/dns-message")
                .body(query_bytes)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .ok()?;
            
            if !response.status().is_success() {
                return None;
            }
            
            let bytes = response.bytes().await.ok()?;
            bytes.to_vec()
        };

        // Cache response if active
        if settings.cache {
            if let Ok(resp_msg) = Message::from_bytes(&response_bytes) {
                let ttl = resp_msg.answers().iter().map(|a: &Record| a.ttl()).min().unwrap_or(60);
                set_cached_dns(&qname_clean, u16::from(qtype), bytes::Bytes::copy_from_slice(&response_bytes), ttl);
            }
        }

        return Some(bytes::Bytes::from(response_bytes));
    }
    None
}
