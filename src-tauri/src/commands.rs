use tauri::{AppHandle, Manager, State, Emitter};
use std::time::Duration;
use std::sync::Arc;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use crate::AppState;
use crate::config::preset::Preset;
use crate::engine::{EngineError, EngineStatus};
use crate::dns::{
    DnsProvider, NetworkAdapter, ApplyDnsResult,
    builtin_providers, get_active_adapters, apply_dns,
    reset_dns_to_dhcp, is_using_trusted_dns,
    resolve_doh, DohResult, DOH_CLOUDFLARE, DOH_GOOGLE,
    DoHEndpoint, DOH_FORWARDER_DEFAULT_PORT,
    spawn_doh_forwarder,
};
use crate::privilege::checker::is_elevated;

// ─── Macro: Map Mutex lock error to EngineError ────────────────────────────
macro_rules! lock_or_err {
    ($mutex:expr) => {
        $mutex.lock().map_err(|_| EngineError::IoError("Config lock poisoned".into()))
    };
}

#[tauri::command]
pub async fn start_engine(
    preset_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EngineStatus, EngineError> {
    let preset = {
        let loader = lock_or_err!(state.config_loader)?;
        loader
            .find_preset(&preset_id)
            .ok_or(EngineError::InvalidPreset(preset_id))?
    };
    state.engine_manager.start(&preset, &app).await?;
    Ok(state.engine_manager.current_status())
}

#[tauri::command]
pub async fn stop_engine(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), EngineError> {
    state.engine_manager.stop(&app)
}

#[tauri::command]
pub fn get_engine_status(state: State<'_, AppState>) -> EngineStatus {
    state.engine_manager.current_status()
}

#[tauri::command]
pub fn list_presets(state: State<'_, AppState>) -> Result<Vec<Preset>, EngineError> {
    let loader = lock_or_err!(state.config_loader)?;
    Ok(loader.all_presets())
}

#[tauri::command]
pub fn save_custom_preset(
    preset: Preset,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), EngineError> {
    let app_data = app.path().app_data_dir().map_err(|e| EngineError::IoError(e.to_string()))?;
    let custom_dir = app_data.join("presets");
    std::fs::create_dir_all(&custom_dir).map_err(|e| EngineError::IoError(e.to_string()))?;

    lock_or_err!(state.config_loader)?
        .save_custom_preset(preset, &custom_dir)
        .map_err(|e| EngineError::IoError(e.to_string()))
}

#[tauri::command]
pub fn delete_custom_preset(
    preset_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), EngineError> {
    let custom_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| EngineError::IoError(e.to_string()))?
        .join("presets");

    lock_or_err!(state.config_loader)?
        .delete_custom_preset(&preset_id, &custom_dir)
        .map_err(|e| EngineError::IoError(e.to_string()))
}

use crate::engine::optimizer::{Optimizer, OptimizePayload};

#[tauri::command]
pub async fn start_auto_optimize(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Preset, EngineError> {
    let optimizer = Optimizer::new(app.clone());

    let _ = app.emit("optimize_progress", OptimizePayload {
        step: "Starting...".into(),
        preset_name: "Preparation".into(),
        progress_pct: 0,
    });

    let best_preset = optimizer
        .run_heuristic_scan()
        .await
        .map_err(|e| EngineError::SpawnFailed(e.to_string()))?;

    state.engine_manager.start(&best_preset, &app).await?;
    Ok(best_preset)
}

#[tauri::command]
pub fn get_last_optimized_preset(app: AppHandle) -> Option<Preset> {
    Optimizer::load_state(&app)
}

#[tauri::command]
pub fn list_dns_providers() -> Vec<DnsProvider> {
    builtin_providers()
}

#[tauri::command]
pub fn get_network_adapters() -> Vec<NetworkAdapter> {
    get_active_adapters()
}

#[tauri::command]
pub fn apply_dns_settings(primary: String, secondary: String, app: AppHandle) -> ApplyDnsResult {
    let res = apply_dns(&primary, &secondary);
    if res.success {
        let _ = app.emit("dns_status_changed", ());
    }
    res
}

#[tauri::command]
pub fn reset_dns_settings(app: AppHandle) -> ApplyDnsResult {
    let res = reset_dns_to_dhcp();
    if res.success {
        let _ = app.emit("dns_status_changed", ());
    }
    res
}

#[tauri::command]
pub fn check_trusted_dns() -> bool {
    is_using_trusted_dns()
}

#[tauri::command]
pub async fn start_engine_with_dns_guard(
    preset_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EngineStatus, EngineError> {
    let dns_ok = is_using_trusted_dns();
    if !dns_ok {
        let result = apply_dns("1.1.1.1", "1.0.0.1");
        if result.success {
            tracing::info!("DNS Guard: Cloudflare automatically applied.");
            let _ = app.emit("dns_auto_applied", "Cloudflare DNS (1.1.1.1) automatically applied.");
        } else {
            tracing::warn!("DNS Guard: Cloudflare could not be applied — {:?}", result.error);
        }
    }

    let preset = {
        let loader = lock_or_err!(state.config_loader)?;
        loader
            .find_preset(&preset_id)
            .ok_or(EngineError::InvalidPreset(preset_id))?
    };
    state.engine_manager.start(&preset, &app).await?;

    let _ = app.emit("dns_status_changed", ());

    Ok(state.engine_manager.current_status())
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.set_focus();
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("Vane - Settings")
        .inner_size(750.0, 550.0)
        .center()
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .skip_taskbar(false)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_system_info() -> serde_json::Value {
    let os = std::env::consts::OS;
    let device_model = std::env::var("COMPUTERNAME")
        .unwrap_or_else(|_| "Windows Desktop".into());

    serde_json::json!({
        "os": os,
        "device_model": device_model,
    })
}

#[tauri::command]
pub fn export_preset(file_path: String, content: String) -> Result<(), String> {
    if file_path.is_empty() {
        return Err("File path cannot be empty".into());
    }
    std::fs::write(&file_path, content).map_err(|e| format!("Could not export preset: {}", e))
}

#[tauri::command]
pub fn check_is_elevated() -> bool {
    is_elevated()
}

#[tauri::command]
pub fn set_autostart(enabled: bool, _app: AppHandle) -> Result<(), String> {
    if !is_elevated() {
        return Err("Administrator privileges are required for task scheduler registration.".into());
    }

    if enabled {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Could not get application path: {}", e))?;
        let exe_str = exe_path
            .to_str()
            .ok_or("Application path contains invalid unicode.")?;

        crate::autostart::enable_autostart(exe_str)
    } else {
        crate::autostart::disable_autostart()
    }
}

#[tauri::command]
pub fn get_autostart_status() -> bool {
    crate::autostart::is_autostart_enabled()
}

#[tauri::command]
pub async fn refresh_remote_presets(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use crate::presets::{fetch_remote_presets, RemoteFetchOutcome};

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    let cached_version = crate::presets::load_cached_presets(&app_data)
        .map(|m| m.version);

    match fetch_remote_presets(&state.http_client, cached_version.as_deref()).await {
        RemoteFetchOutcome::Updated(manifest, sig_text, raw_json) => {
            let version = manifest.version.clone();
            let _ = crate::presets::save_cached_presets_with_sig(&manifest, &raw_json, &sig_text, &app_data).await;

            lock_or_err!(state.config_loader)
                .map_err(|e| e.to_string())?
                .load_remote_presets(manifest.presets);

            let _ = app.emit("remote_presets_updated", &version);
            Ok(version)
        }
        RemoteFetchOutcome::VersionUnchanged => Ok("unchanged".into()),
        RemoteFetchOutcome::Offline => {
            let _ = app.emit("remote_presets_offline", ());
            Err("Offline: Remote presets are unreachable.".into())
        }
        RemoteFetchOutcome::ParseError(e) => Err(format!("Parse error: {}", e)),
        RemoteFetchOutcome::SignatureInvalid => Err("CRITICAL: Security Signature invalid! (CVE-5 Protection)".into()),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwarderStatus {
    pub active: bool,
    pub port: u16,
    pub endpoint: String,
    pub protocol: String,
    pub adblock: bool,
    pub cache: bool,
    pub watchdog_enabled: bool,
}

fn forwarder_status(
    active: bool,
    port: u16,
    endpoint: String,
    watchdog_enabled: bool,
    app: &AppHandle,
) -> ForwarderStatus {
    let settings = crate::dns::forwarder::read_dns_settings(app);
    ForwarderStatus {
        active,
        port,
        endpoint,
        protocol: settings.protocol,
        adblock: settings.adblock,
        cache: settings.cache,
        watchdog_enabled,
    }
}

#[tauri::command]
pub async fn start_doh_forwarder(
    watchdog: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ForwarderStatus, String> {
    {
        let guard = state.forwarder.lock()
            .map_err(|_| "Forwarder lock poisoned.".to_string())?;
        if guard.is_some() {
            return Err("DoH Forwarder is already running.".into());
        }
    }

    let mut handle = spawn_doh_forwarder(
        app.clone(),
        state.http_client.clone(),
        DOH_FORWARDER_DEFAULT_PORT,
        DoHEndpoint::Cloudflare,
    ).await?;

    let dns_applied = apply_dns("127.0.0.1", "127.0.0.1");
    if !dns_applied.success {
        let _ = handle.stop().await;
        return Err(format!("Sistem DNS'i 127.0.0.1 olarak ayarlanamadı: {:?}", dns_applied.error));
    }

    let shutdown_clone = Arc::clone(&handle.shutdown);
    let client_clone = state.http_client.clone();
    let endpoint_url = handle.endpoint.url().to_string();
    let app_clone = app.clone();

    if watchdog {
        crate::dns::spawn_dns_watchdog(client_clone, endpoint_url, shutdown_clone, app_clone);
        handle.watchdog_enabled = true;
        tracing::info!("DNS watchdog was enabled and its health-check task was started.");
    } else {
        tracing::info!("DNS watchdog was disabled; no health-check task was started.");
    }

    let status = {
        let mut guard = state.forwarder.lock()
            .map_err(|_| "Forwarder lock poisoned.".to_string())?;

        if guard.is_some() {
            tauri::async_runtime::spawn(async move { handle.stop().await; });
            let _ = reset_dns_to_dhcp();
            return Err("DoH Forwarder is already running.".into());
        }

        let port = handle.port;
        let endpoint = handle.endpoint.url().to_string();
        *guard = Some(handle);

        forwarder_status(true, port, endpoint, watchdog, &app)
    };

    tracing::info!(
        "DNS forwarder is running and verified: protocol={}, cache={}, adblock={}, port={}",
        status.protocol.to_uppercase(), status.cache, status.adblock, status.port
    );
    Ok(status)
}

#[tauri::command]
pub async fn stop_doh_forwarder(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let handle = {
        let mut guard = state.forwarder.lock()
            .map_err(|_| "Forwarder lock poisoned.".to_string())?;
        guard.take()
    };

    if let Some(h) = handle {
        h.stop().await;
        let reset = reset_dns_to_dhcp();
        if !reset.success {
            return Err(format!("Forwarder stopped but automatic DNS restore failed: {:?}", reset.error));
        }
        tracing::info!("DNS forwarder stopped and system DNS was restored automatically.");
        Ok(())
    } else {
        Err("DoH Forwarder is already stopped.".into())
    }
}

#[tauri::command]
pub fn get_doh_forwarder_status(app: AppHandle, state: State<'_, AppState>) -> ForwarderStatus {
    let guard = state.forwarder.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_ref() {
        Some(h) => forwarder_status(
            true,
            h.port,
            h.endpoint.url().to_string(),
            h.watchdog_enabled,
            &app,
        ),
        None => forwarder_status(
            false,
            DOH_FORWARDER_DEFAULT_PORT,
            DoHEndpoint::Cloudflare.url().to_string(),
            false,
            &app,
        ),
    }
}

#[cfg(target_os = "windows")]
fn icmp_ping_ms() -> Option<u64> {
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    
    let output = std::process::Command::new("ping")
        .args(["-n", "1", "-w", "3000", "1.1.1.1"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if lower.contains("average") || lower.contains("ortalama") {
            let parts: Vec<&str> = line.split('=').collect();
            if let Some(last) = parts.last() {
                let digits: String = last.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(ms) = digits.parse::<u64>() {
                    return Some(ms);
                }
            }
        }
    }
    None
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub healthy: bool,
    pub latency_ms: u64,
    pub checked_at: String,
    pub target: String,
}

#[tauri::command]
pub async fn get_engine_health(
    targets: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<HealthStatus, String> {
    let mut actual_targets = targets.unwrap_or_default();
    if actual_targets.is_empty() {
        actual_targets.push("discord.com".to_string());
    }

    let mut all_healthy = true;
    for target in &actual_targets {
        let url = if target.starts_with("http") {
            target.to_string()
        } else {
            format!("https://{}", target)
        };

        let result = state.http_client
            .head(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        if !matches!(result, Ok(ref resp) if resp.status().as_u16() < 400) {
            all_healthy = false;
            break;
        }
    }

    #[cfg(target_os = "windows")]
    let latency_ms: u64 = if all_healthy { icmp_ping_ms().unwrap_or(0) } else { 0 };
    #[cfg(not(target_os = "windows"))]
    let latency_ms: u64 = 0;

    let now = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let h = (secs % 86400) / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        format!("{:02}:{:02}:{:02} UTC", h, m, s)
    };

    let display_target = if actual_targets.len() == 1 {
        actual_targets[0].clone()
    } else {
        format!("{} Sites", actual_targets.len())
    };

    Ok(HealthStatus {
        healthy: all_healthy,
        latency_ms,
        checked_at: now,
        target: display_target,
    })
}

#[tauri::command]
pub async fn resolve_via_doh(
    domain: String,
    state: State<'_, AppState>,
) -> Result<Vec<DohResult>, ()> {
    let cloudflare = resolve_doh(&state.http_client, DOH_CLOUDFLARE, &domain).await;
    let google = resolve_doh(&state.http_client, DOH_GOOGLE, &domain).await;
    Ok(vec![cloudflare, google])
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct IpWhoIsConnection {
    pub isp: Option<String>,
    pub org: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct IpWhoIsResponse {
    pub ip: Option<String>,
    pub success: bool,
    pub city: Option<String>,
    pub country: Option<String>,
    pub connection: Option<IpWhoIsConnection>,
}

#[tauri::command]
pub async fn get_geoip_data(state: State<'_, AppState>) -> Result<IpWhoIsResponse, String> {
    let client = &state.http_client;
    let response = client.get("https://ipwho.is/")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    let data = response.json::<IpWhoIsResponse>()
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(data)
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("Güvenlik ihlali: Yalnızca HTTP veya HTTPS şemalarına izin verilir.".into());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_path(url, None::<String>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_network_stats() -> (u64, u64) {
    crate::network::get_total_network_bytes()
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigStatus {
    protocol: String,
    adblock: bool,
    cache: bool,
    socks5_proxy: String,
    forwarder_active: bool,
}

#[tauri::command]
pub async fn sync_dns_settings(
    protocol: String,
    adblock: bool,
    cache: bool,
    socks5_proxy: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DnsConfigStatus, String> {
    let _sync_guard = state.dns_sync.lock().await;
    if protocol != "doh" && protocol != "dot" {
        return Err(format!("Unsupported DNS transport protocol: {}", protocol));
    }
    let settings = crate::dns::forwarder::DnsSettings {
        protocol: protocol.clone(),
        adblock,
        cache,
        socks5_proxy: socks5_proxy.clone(),
    };
    crate::dns::forwarder::update_dns_settings_cache(settings.clone());

    let verified = crate::dns::forwarder::read_dns_settings(&app);
    let forwarder_active = state.forwarder.lock()
        .map_err(|_| "Forwarder lock poisoned.".to_string())?
        .is_some();
    let result = DnsConfigStatus {
        protocol: verified.protocol,
        adblock: verified.adblock,
        cache: verified.cache,
        socks5_proxy: verified.socks5_proxy,
        forwarder_active,
    };
    app.emit("dns_config_synced", result.clone()).map_err(|e| e.to_string())?;
    tracing::info!(
        "DNS settings applied and verified: protocol={}, cache={}, adblock={}, proxy={}",
        result.protocol.to_uppercase(), result.cache, result.adblock,
        if result.socks5_proxy.is_empty() { "direct" } else { "SOCKS5" }
    );
    Ok(result)
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BypassConfigStatus {
    mode: String,
    domain_count: usize,
    engine_restarted: bool,
    engine_running: bool,
    whitelist_domains: Vec<String>,
    blacklist_domains: Vec<String>,
}

#[tauri::command]
pub async fn sync_bypass_config(
    mode: String,
    list: String,
    proxy: String,
    kill_switch: bool,
    whitelist_domains: Vec<String>,
    blacklist_domains: Vec<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BypassConfigStatus, String> {
    let _sync_guard = state.bypass_sync.lock().await;
    if mode != "all" && mode != "whitelist" && mode != "blacklist" {
        return Err(format!("Unsupported bypass mode: {}", mode));
    }
    let whitelist_domains = crate::config::domain::canonicalize_domain_rules(&whitelist_domains)
        .map_err(|error| format!("Invalid whitelist domain: {error}"))?;
    let blacklist_domains = crate::config::domain::canonicalize_domain_rules(&blacklist_domains)
        .map_err(|error| format!("Invalid blacklist domain: {error}"))?;
    let canonical_list = match mode.as_str() {
        "whitelist" => whitelist_domains.join("\n"),
        "blacklist" => blacklist_domains.join("\n"),
        _ => String::new(),
    };
    if list.trim() != canonical_list {
        tracing::warn!(
            "Pattern list received from the interface did not match the canonical domain rules; the verified domain arrays were used."
        );
    }
    crate::engine::manager::update_bypass_config_cache(
        mode.clone(),
        canonical_list.clone(),
        proxy.clone(),
        kill_switch,
    );

    let status = state.engine_manager.current_status();
    let mut engine_restarted = false;
    if let crate::engine::EngineStatus::Running { .. } = status {
        tracing::info!("Bypass config changed while engine is running. Restarting engine silently...");
        let active_preset_id = crate::read_last_preset_id(&app).unwrap_or_else(|| "default".to_string());
        
        let preset_opt = if let Ok(loader) = state.config_loader.lock() {
            preset_opt_guard(loader.find_preset(&active_preset_id))
        } else {
            None
        };

        if let Some(preset) = preset_opt {
            let _ = state.engine_manager.stop(&app);
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            if let Err(e) = state.engine_manager.start(&preset, &app).await {
                return Err(format!("Failed to restart engine: {:?}", e));
            }
            engine_restarted = true;
        }
    }
    let domain_count = match mode.as_str() {
        "whitelist" => whitelist_domains.len(),
        "blacklist" => blacklist_domains.len(),
        _ => 0,
    };
    let result = BypassConfigStatus {
        mode: mode.clone(),
        domain_count,
        engine_restarted,
        engine_running: matches!(state.engine_manager.current_status(), crate::engine::EngineStatus::Running { .. }),
        whitelist_domains,
        blacklist_domains,
    };
    app.emit("bypass_config_synced", result.clone()).map_err(|e| e.to_string())?;
    tracing::info!(
        "Bypass pattern saved and verified: mode={}, domains={}, engine_restarted={}",
        mode, domain_count, engine_restarted
    );
    Ok(result)
}

fn preset_opt_guard<T>(val: Option<T>) -> Option<T> {
    val
}
