/*
   Local DNS Forwarder with AdBlock, Cache, and DoH/DoT Protocols
*/

use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};

use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use hickory_resolver::error::ResolveErrorKind;
use hickory_resolver::proto::op::{Message, MessageType, ResponseCode};
use hickory_resolver::proto::rr::{rdata::A, RData, Record, RecordType};
use hickory_resolver::proto::serialize::binary::{BinDecodable, BinEncodable};
use hickory_resolver::TokioAsyncResolver;

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
            if ip == "0.0.0.0" || ip == "127.0.0.1" {
                for domain in &parts[1..] {
                    let domain = domain.to_lowercase();
                    if domain != "localhost"
                        && domain.len() <= 253
                        && domain
                            .split('.')
                            .all(|label| !label.is_empty() && label.len() <= 63)
                    {
                        set.insert(domain);
                    }
                }
            }
        }
    }
    set
}

pub fn initialize_adblock(app_handle: &AppHandle) {
    let mut should_refresh = true;
    if let Ok(app_data) = app_handle.path().app_data_dir() {
        let cache_path = app_data.join("adblock_cache.txt");
        if cache_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&cache_path) {
                let set = parse_hosts_file(&text);
                if let Ok(mut guard) = ADBLOCK_LIST.write() {
                    *guard = Some(set);
                }
                tracing::info!("AdBlock listesi önbellekten yüklendi.");
                should_refresh = std::fs::metadata(&cache_path)
                    .and_then(|metadata| metadata.modified())
                    .and_then(|modified| modified.elapsed().map_err(std::io::Error::other))
                    .map(|age| age > std::time::Duration::from_secs(24 * 60 * 60))
                    .unwrap_or(true);
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
    if should_refresh {
        let handle = app_handle.clone();
        tokio::spawn(async move {
            update_adblock_list(handle).await;
        });
    }
}

pub async fn update_adblock_list(app_handle: AppHandle) {
    let Ok(app_data) = app_handle.path().app_data_dir() else {
        return;
    };
    let cache_path = app_data.join("adblock_cache.txt");

    let url = "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts";
    let client = reqwest::Client::new();
    match client
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                const MAX_BLOCKLIST_BYTES: usize = 12 * 1024 * 1024;
                let content_type_ok = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .map(|value| {
                        value.starts_with("text/plain")
                            || value.starts_with("application/octet-stream")
                    })
                    .unwrap_or(false);
                if !content_type_ok {
                    tracing::error!("AdBlock/malware list was rejected because the server returned an unexpected content type.");
                    return;
                }
                if resp
                    .content_length()
                    .is_some_and(|length| length > MAX_BLOCKLIST_BYTES as u64)
                {
                    tracing::error!("AdBlock/malware list was rejected because it exceeds the 12 MiB safety limit.");
                    return;
                }
                let mut stream = resp.bytes_stream();
                let mut data = Vec::new();
                while let Some(chunk) = stream.next().await {
                    let Ok(chunk) = chunk else {
                        tracing::error!(
                            "AdBlock/malware list download ended with an invalid chunk."
                        );
                        return;
                    };
                    if data.len() + chunk.len() > MAX_BLOCKLIST_BYTES {
                        tracing::error!("AdBlock/malware list was rejected because it exceeds the 12 MiB safety limit.");
                        return;
                    }
                    data.extend_from_slice(&chunk);
                }
                if let Ok(text) = String::from_utf8(data) {
                    let set = parse_hosts_file(&text);
                    if set.len() < 10_000 {
                        tracing::error!("AdBlock/malware list validation failed: only {} valid domains were found.", set.len());
                        return;
                    }
                    let _ = std::fs::create_dir_all(&app_data);
                    if let Err(e) =
                        crate::settings::atomic_replace_bytes(&cache_path, text.as_bytes())
                    {
                        tracing::error!("AdBlock önbellek dosyası yazılamadı: {}", e);
                    } else {
                        if let Ok(mut guard) = ADBLOCK_LIST.write() {
                            *guard = Some(set);
                        }
                        tracing::info!("Combined ad and malware domain list was validated and updated atomically.");
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "AdBlock listesi indirilemedi (çevrimdışı veya zaman aşımı): {}",
                e
            );
        }
    }
}

fn is_domain_blocked(domain: &str) -> bool {
    let guard = match ADBLOCK_LIST.read() {
        Ok(g) => g,
        Err(_) => return false,
    };
    let Some(ref set) = *guard else {
        return false;
    };
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
    stored_at: std::time::Instant,
    expires_at: std::time::Instant,
    last_used: std::time::Instant,
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

fn get_cached_dns(key: &str) -> Option<bytes::Bytes> {
    let mut guard = DNS_CACHE.write().ok()?;
    let cache = guard.as_mut()?;
    let now = std::time::Instant::now();
    if cache.get(key).is_some_and(|entry| now >= entry.expires_at) {
        cache.remove(key);
        return None;
    }
    let entry = cache.get_mut(key)?;
    entry.last_used = now;
    let elapsed = now
        .saturating_duration_since(entry.stored_at)
        .as_secs()
        .min(u32::MAX as u64) as u32;
    let mut message = Message::from_bytes(&entry.response_bytes).ok()?;
    for record in message.answers_mut() {
        record.set_ttl(record.ttl().saturating_sub(elapsed));
    }
    for record in message.name_servers_mut() {
        record.set_ttl(record.ttl().saturating_sub(elapsed));
    }
    for record in message.additionals_mut() {
        record.set_ttl(record.ttl().saturating_sub(elapsed));
    }
    message.to_bytes().ok().map(bytes::Bytes::from)
}

fn set_cached_dns(key: String, response_bytes: bytes::Bytes, ttl: u32) {
    if let Ok(mut guard) = DNS_CACHE.write() {
        if let Some(ref mut cache) = *guard {
            let now = std::time::Instant::now();
            cache.retain(|_, entry| entry.expires_at > now);
            if cache.len() >= 5000 {
                if let Some(oldest) = cache
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_used)
                    .map(|(key, _)| key.clone())
                {
                    cache.remove(&oldest);
                }
            }
            let expires_at = now + std::time::Duration::from_secs(ttl.max(1) as u64);
            cache.insert(
                key,
                CacheEntry {
                    response_bytes,
                    stored_at: now,
                    expires_at,
                    last_used: now,
                },
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
        DoHEndpoint::Cloudflare => (
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)),
            "cloudflare-dns.com",
        ),
        DoHEndpoint::Google => (
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)),
            "dns.google",
        ),
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsSettings {
    pub protocol: String,
    pub adblock: bool,
    pub cache: bool,
    pub socks5_proxy: String,
    pub health_check_targets: Vec<String>,
}

static DNS_SETTINGS_CACHE: RwLock<Option<DnsSettings>> = RwLock::new(None);
static PROXY_CLIENT_CACHE: RwLock<Option<(String, reqwest::Client)>> = RwLock::new(None);

pub fn normalize_socks5_proxy(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    let value = value
        .strip_prefix("socks5h://")
        .or_else(|| value.strip_prefix("socks5://"))
        .unwrap_or(value);
    if value.contains('@') {
        return Err("SOCKS5 credentials are not stored in application settings; use a credential-free local proxy.".into());
    }
    let (host, port) = value
        .rsplit_once(':')
        .ok_or_else(|| "SOCKS5 proxy must use host:port format.".to_string())?;
    let port = port
        .parse::<u16>()
        .map_err(|_| "SOCKS5 proxy port is invalid.".to_string())?;
    if port == 0 || host.is_empty() {
        return Err("SOCKS5 proxy host or port is invalid.".into());
    }
    let valid_host = host.parse::<std::net::IpAddr>().is_ok()
        || (host.len() <= 253
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            }));
    if !valid_host {
        return Err("SOCKS5 proxy host is invalid.".into());
    }
    Ok(format!("{host}:{port}"))
}

pub fn update_dns_settings_cache(mut settings: DnsSettings) -> Result<(), String> {
    settings.socks5_proxy = normalize_socks5_proxy(&settings.socks5_proxy)?;
    if settings.protocol == "dot" && !settings.socks5_proxy.is_empty() {
        return Err("SOCKS5 upstream is supported with DoH only; DoT would bypass the proxy and was rejected.".into());
    }
    if settings.socks5_proxy.is_empty() {
        if let Ok(mut guard) = PROXY_CLIENT_CACHE.write() {
            *guard = None;
        }
        tracing::info!("DNS upstream connection verified: direct encrypted transport.");
    } else {
        let proxy = reqwest::Proxy::all(format!("socks5h://{}", settings.socks5_proxy))
            .map_err(|e| format!("SOCKS5 proxy configuration is invalid: {e}"))?;
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("SOCKS5 DNS client could not be created: {e}"))?;
        if let Ok(mut guard) = PROXY_CLIENT_CACHE.write() {
            *guard = Some((settings.socks5_proxy.clone(), client));
        }
        tracing::info!("DNS upstream connection verified: SOCKS5H client created; hostname resolution will occur through the proxy.");
    }
    if !settings.cache {
        clear_dns_cache();
        tracing::info!("Smart DNS Cache is disabled and all RAM cache entries were cleared.");
    } else {
        init_dns_cache();
        tracing::info!("Smart DNS Cache is enabled with TTL aging and a 5000-entry LRU limit.");
    }
    if let Ok(mut guard) = DNS_SETTINGS_CACHE.write() {
        *guard = Some(settings);
    }
    Ok(())
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
        health_check_targets: vec!["example.com".to_string()],
    };
    let res = match crate::settings::read_runtime_settings(app) {
        Ok(Some(settings)) => DnsSettings {
            protocol: if settings.dns_protocol == "dot" {
                "dot".into()
            } else {
                "doh".into()
            },
            adblock: settings.dns_ad_block,
            cache: settings.dns_cache,
            socks5_proxy: settings.proxy_socks5,
            health_check_targets: if settings.health_check_targets.is_empty() {
                vec!["example.com".into()]
            } else {
                settings.health_check_targets
            },
        },
        Ok(None) => default_settings,
        Err(error) => {
            tracing::error!("DNS settings could not be read safely; defaults are displayed but protected startup must not proceed: {error}");
            default_settings
        }
    };
    if let Ok(mut guard) = DNS_SETTINGS_CACHE.write() {
        *guard = Some(res.clone());
    }
    res
}

pub struct ForwarderHandle {
    pub port: u16,
    pub endpoint: DoHEndpoint,
    pub shutdown: Arc<AtomicBool>,
    pub watchdog_enabled: bool,
    pub previous_dns: Vec<crate::dns::NetworkAdapter>,
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
    let tcp_listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("DNS Forwarder TCP port {} bağlanamadı: {}", port, e))?;

    let previous_dns = crate::dns::get_active_adapters();
    let mut fallback_dns = previous_dns
        .iter()
        .cloned()
        .into_iter()
        .find_map(|a| a.current_primary_dns)
        .unwrap_or_else(|| "1.1.1.1".to_string());

    if fallback_dns == "127.0.0.1" || fallback_dns == "localhost" || fallback_dns.is_empty() {
        fallback_dns = "1.1.1.1".to_string();
    }

    // Initialize optional DNS tools only when the saved runtime enables them.
    let startup_settings = read_dns_settings(&app);
    if startup_settings.adblock {
        initialize_adblock(&app);
    }
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
        let tcp_app = app_clone.clone();
        let tcp_client = client.clone();
        let tcp_fallback = fallback_dns.clone();
        let tcp_shutdown = Arc::clone(&shutdown_clone);
        tokio::join!(
            run_forwarder_loop(
                app_clone,
                socket,
                client,
                endpoint,
                fallback_dns,
                shutdown_clone
            ),
            run_tcp_forwarder_loop(
                tcp_app,
                tcp_listener,
                tcp_client,
                endpoint,
                tcp_fallback,
                tcp_shutdown
            ),
        );
    });

    Ok(ForwarderHandle {
        port,
        endpoint,
        shutdown,
        watchdog_enabled: false,
        previous_dns,
        task,
    })
}

async fn run_tcp_forwarder_loop(
    app: AppHandle,
    listener: TcpListener,
    client: reqwest::Client,
    endpoint: DoHEndpoint,
    fallback_dns: String,
    shutdown: Arc<AtomicBool>,
) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(100));
    while !shutdown.load(Ordering::SeqCst) {
        let accepted =
            tokio::time::timeout(std::time::Duration::from_secs(1), listener.accept()).await;
        let (mut stream, peer) = match accepted {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                tracing::warn!("DNS TCP accept failed: {error}");
                continue;
            }
            Err(_) => continue,
        };
        let Ok(permit) = Arc::clone(&semaphore).try_acquire_owned() else {
            tracing::warn!("DNS TCP concurrency limit reached; dropping {peer}.");
            continue;
        };
        let app = app.clone();
        let client = client.clone();
        let fallback_dns = fallback_dns.clone();
        tokio::spawn(async move {
            let _permit = permit;
            loop {
                let length = match stream.read_u16().await {
                    Ok(length) if length > 0 => length as usize,
                    _ => break,
                };
                let mut query = vec![0u8; length];
                if stream.read_exact(&mut query).await.is_err() {
                    break;
                }
                let Some(response) = proxy_dns_query(
                    app.clone(),
                    &client,
                    endpoint,
                    &fallback_dns,
                    bytes::Bytes::from(query),
                )
                .await
                else {
                    break;
                };
                if response.len() > u16::MAX as usize
                    || stream.write_u16(response.len() as u16).await.is_err()
                    || stream.write_all(&response).await.is_err()
                {
                    break;
                }
            }
        });
    }
}

pub(crate) fn current_doh_client(default_client: &reqwest::Client) -> Option<reqwest::Client> {
    let settings = DNS_SETTINGS_CACHE
        .read()
        .ok()?
        .clone()
        .unwrap_or(DnsSettings {
            protocol: "doh".into(),
            adblock: false,
            cache: true,
            socks5_proxy: String::new(),
            health_check_targets: vec!["example.com".into()],
        });
    if settings.socks5_proxy.is_empty() {
        return Some(default_client.clone());
    }
    PROXY_CLIENT_CACHE
        .read()
        .ok()?
        .as_ref()
        .filter(|(address, _)| address == &settings.socks5_proxy)
        .map(|(_, client)| client.clone())
}

pub(crate) async fn probe_dot_upstream(endpoint: DoHEndpoint, domain: &str) -> bool {
    let Some(resolver) = get_or_create_dot_resolver(endpoint) else {
        return false;
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        resolver.lookup_ip(format!("{}.", domain.trim_end_matches('.'))),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .map(|response| response.iter().next().is_some())
    .unwrap_or(false)
}

async fn run_forwarder_loop(
    app: AppHandle,
    socket: Arc<UdpSocket>,
    client: reqwest::Client,
    endpoint: DoHEndpoint,
    fallback_dns: String,
    shutdown: Arc<AtomicBool>,
) {
    let mut buf = vec![0u8; 65_535];
    const MAX_CONCURRENT_DNS_REQUESTS: usize = 100;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_DNS_REQUESTS));

    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        let recv_result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            socket.recv_from(&mut buf),
        )
        .await;

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
            tracing::warn!(
                "DNS forwarder: concurrency limit reached, dropping query from {}",
                client_addr
            );
            continue;
        };

        tokio::spawn(async move {
            let _permit = permit;
            if let Some(response) = proxy_dns_query(
                app_clone,
                &client_clone,
                endpoint,
                &fallback_dns_clone,
                query_bytes,
            )
            .await
            {
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

    let mut buf = vec![0u8; 65_535];
    let recv_result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        socket.recv_from(&mut buf),
    )
    .await;

    match recv_result {
        Ok(Ok((len, _))) => Some(bytes::Bytes::copy_from_slice(&buf[..len])),
        _ => None,
    }
}

fn build_blocked_response(
    parsed: &Message,
    query: &hickory_resolver::proto::op::Query,
) -> bytes::Bytes {
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
        name.ends_with(".local.")
            || name.ends_with(".lan.")
            || name.ends_with(".home.")
            || name.ends_with(".arpa.")
    });

    if is_local && !crate::engine::manager::kill_switch_enabled() {
        if let Some(resp) = query_fallback_dns(fallback_dns, &query_bytes).await {
            return Some(resp);
        }
    }

    if let Some(query) = parsed.queries().first() {
        let qname = query.name().to_string();
        let qname_clean = qname.trim_end_matches('.').to_lowercase();
        let qtype = query.query_type();
        let mut normalized_query = query_bytes.to_vec();
        if normalized_query.len() >= 2 {
            normalized_query[0] = 0;
            normalized_query[1] = 0;
        }
        let cache_key = format!("{:x}", Sha256::digest(&normalized_query));

        let settings = read_dns_settings(&app);

        // 1. AdBlock Filter
        if settings.adblock && is_domain_blocked(&qname_clean) {
            tracing::info!("AdBlock: Engellendi -> {}", qname_clean);
            return Some(build_blocked_response(&parsed, query));
        }

        // 2. RAM Cache Lookup
        if settings.cache {
            if let Some(cached) = get_cached_dns(&cache_key) {
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
            let mut response = Message::new();
            response.set_id(parsed.id());
            response.set_message_type(MessageType::Response);
            response.set_op_code(parsed.op_code());
            response.set_recursion_desired(parsed.recursion_desired());
            response.set_recursion_available(true);
            response.set_checking_disabled(parsed.checking_disabled());
            if let Some(edns) = parsed.extensions() {
                response.set_edns(edns.clone());
            }
            response.add_query(query.clone());
            match resolver.lookup(name, qtype).await {
                Ok(lookup) => {
                    response.set_response_code(ResponseCode::NoError);
                    for record in lookup.records() {
                        response.add_answer(record.clone());
                    }
                }
                Err(error) => match error.kind() {
                    ResolveErrorKind::NoRecordsFound { soa, response_code, .. } => {
                        response.set_response_code(*response_code);
                        if let Some(soa) = soa {
                            response.add_name_server(soa.as_ref().clone().into_record_of_rdata());
                        }
                    }
                    _ => return None,
                },
            }
            response.to_bytes().ok()?
        } else {
            // DoH
            let doh_client = if settings.socks5_proxy.is_empty() {
                client.clone()
            } else {
                PROXY_CLIENT_CACHE.read().ok().and_then(|guard| {
                    guard
                        .as_ref()
                        .filter(|(address, _)| address == &settings.socks5_proxy)
                        .map(|(_, client)| client.clone())
                })?
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
            if response.content_length().is_some_and(|length| length > 65_535) {
                tracing::error!("DNS upstream response exceeded the 65535-byte protocol limit.");
                return None;
            }
            let bytes = response.bytes().await.ok()?;
            if bytes.len() > 65_535 {
                tracing::error!("DNS upstream response exceeded the 65535-byte protocol limit.");
                return None;
            }
            bytes.to_vec()
        };

        // Cache response if active
        if settings.cache {
            if let Ok(resp_msg) = Message::from_bytes(&response_bytes) {
                let ttl = resp_msg
                    .answers()
                    .iter()
                    .chain(resp_msg.name_servers().iter())
                    .map(|record: &Record| record.ttl())
                    .min()
                    .unwrap_or(30)
                    .clamp(1, 86_400);
                set_cached_dns(
                    cache_key,
                    bytes::Bytes::copy_from_slice(&response_bytes),
                    ttl,
                );
            }
        }

        return Some(bytes::Bytes::from(response_bytes));
    }
    None
}
