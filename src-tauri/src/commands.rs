use crate::config::preset::Preset;
use crate::dns::{
    apply_dns, builtin_providers, get_active_adapters, is_using_trusted_dns, reset_dns_to_dhcp,
    resolve_doh, spawn_doh_forwarder, ApplyDnsResult, DnsProvider, DoHEndpoint, DohResult,
    NetworkAdapter, DEFAULT_HEALTH_CHECK_TARGET, DOH_CLOUDFLARE, DOH_FORWARDER_DEFAULT_PORT,
    DOH_GOOGLE,
};
use crate::engine::{EngineError, EngineStatus};
use crate::ipc::IpcError;
use crate::privilege::checker::is_elevated;
use crate::AppState;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

// ─── Macro: Map Mutex lock error to EngineError ────────────────────────────
macro_rules! lock_or_err {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|_| EngineError::IoError("Config lock poisoned".into()))
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
pub async fn stop_engine(app: AppHandle, state: State<'_, AppState>) -> Result<(), EngineError> {
    state.engine_manager.stop(&app).await
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
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| EngineError::IoError(e.to_string()))?;
    let custom_dir = app_data.join("presets");
    std::fs::create_dir_all(&custom_dir).map_err(|e| EngineError::IoError(e.to_string()))?;

    lock_or_err!(state.config_loader)?.save_custom_preset(preset, &custom_dir)
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

    lock_or_err!(state.config_loader)?.delete_custom_preset(&preset_id, &custom_dir)
}

use crate::engine::optimizer::{OptimizePayload, Optimizer};

#[tauri::command]
pub async fn start_auto_optimize(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Preset, EngineError> {
    let optimizer = Optimizer::new(app.clone());

    let _ = app.emit(
        "optimize_progress",
        OptimizePayload {
            step: "Starting...".into(),
            preset_name: "Preparation".into(),
            progress_pct: 0,
        },
    );

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
pub async fn apply_dns_settings(
    primary: String,
    secondary: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ApplyDnsResult, String> {
    let _sync_guard = state.dns_sync.lock().await;
    if state
        .forwarder
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(true)
    {
        return Ok(ApplyDnsResult {
            success: false,
            applied_adapters: Vec::new(),
            error: Some(
                "Stop the local DNS forwarder before changing the system DNS provider.".into(),
            ),
        });
    }
    let res = apply_dns(&primary, &secondary);
    if res.success {
        let _ = app.emit("dns_status_changed", ());
    }
    Ok(res)
}

#[tauri::command]
pub async fn reset_dns_settings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ApplyDnsResult, String> {
    let _sync_guard = state.dns_sync.lock().await;
    if state
        .forwarder
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(true)
    {
        return Ok(ApplyDnsResult {
            success: false,
            applied_adapters: Vec::new(),
            error: Some(
                "Stop the local DNS forwarder before resetting the system DNS configuration."
                    .into(),
            ),
        });
    }
    let res = reset_dns_to_dhcp();
    if res.success {
        let _ = app.emit("dns_status_changed", ());
    }
    Ok(res)
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
    let forwarder_active = state
        .forwarder
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    let dns_ok = forwarder_active || is_using_trusted_dns();
    if !dns_ok {
        tracing::warn!("DNS Guard: The current system DNS is not a built-in trusted provider; Vane preserved the user's explicit DNS choice.");
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
    let device_model = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Windows Desktop".into());

    serde_json::json!({
        "os": os,
        "device_model": device_model,
    })
}

#[tauri::command]
pub fn export_preset(file_path: String, content: String) -> Result<(), String> {
    let path = std::path::Path::new(&file_path);
    if path.extension().and_then(|value| value.to_str()) != Some("vane") {
        return Err("Preset exports must use the .vane extension.".into());
    }
    if content.len() > 1024 * 1024 {
        return Err("Preset export exceeds the 1 MiB safety limit.".into());
    }
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| format!("Preset export is not valid JSON: {error}"))?;
    if !parsed.is_object() {
        return Err("Preset export must contain a JSON object.".into());
    }
    crate::settings::atomic_replace_bytes(path, content.as_bytes())
        .map_err(|error| format!("Could not export preset: {error}"))
}

#[tauri::command]
pub fn check_is_elevated() -> bool {
    is_elevated()
}

#[tauri::command]
pub fn set_autostart(enabled: bool, _app: AppHandle) -> Result<(), String> {
    if !is_elevated() {
        return Err(
            "Administrator privileges are required for task scheduler registration.".into(),
        );
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

    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let cached_version = crate::presets::load_cached_presets(&app_data).map(|m| m.version);

    match fetch_remote_presets(&state.http_client, cached_version.as_deref()).await {
        RemoteFetchOutcome::Updated(manifest, sig_text, raw_json) => {
            let version = manifest.version.clone();
            crate::config::loader::validate_remote_presets(&manifest.presets)
                .map_err(|error| format!("Signed remote preset update is incompatible with this engine: {error}"))?;
            crate::presets::save_cached_presets_with_sig(
                &manifest, &raw_json, &sig_text, &app_data,
            )
            .await
            .map_err(|error| format!("Verified remote presets could not be persisted: {error}"))?;

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
        RemoteFetchOutcome::SignatureInvalid => {
            Err("CRITICAL: Security Signature invalid! (CVE-5 Protection)".into())
        }
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
    let _sync_guard = state.dns_sync.lock().await;
    let endpoint = match crate::settings::read_runtime_settings(&app)? {
        Some(settings) if settings.selected_dns_id == "google" => DoHEndpoint::Google,
        _ => DoHEndpoint::Cloudflare,
    };
    start_dns_forwarder_runtime(&app, state.inner(), watchdog, endpoint).await
}

pub(crate) async fn start_dns_forwarder_runtime(
    app: &AppHandle,
    state: &AppState,
    watchdog: bool,
    endpoint: DoHEndpoint,
) -> Result<ForwarderStatus, String> {
    {
        let guard = state
            .forwarder
            .lock()
            .map_err(|_| "Forwarder lock poisoned.".to_string())?;
        if guard.is_some() {
            return Err("DoH Forwarder is already running.".into());
        }
    }

    let mut handle = spawn_doh_forwarder(
        app.clone(),
        state.http_client.clone(),
        DOH_FORWARDER_DEFAULT_PORT,
        endpoint,
    )
    .await?;

    if let Err(error) = crate::dns::save_dns_restore_snapshot(app, &handle.previous_dns) {
        handle.stop().await;
        return Err(format!(
            "DNS forwarder was not activated because a safe restore point could not be saved: {error}"
        ));
    }

    let dns_applied = apply_dns("127.0.0.1", "127.0.0.1");
    if !dns_applied.success {
        let previous_dns = handle.previous_dns.clone();
        let _ = handle.stop().await;
        let _ = crate::dns::restore_dns_snapshot(&previous_dns);
        let _ = crate::dns::clear_dns_restore_snapshot(app);
        return Err(format!(
            "Sistem DNS'i 127.0.0.1 olarak ayarlanamadı: {:?}",
            dns_applied.error
        ));
    }

    let shutdown_clone = Arc::clone(&handle.shutdown);
    let client_clone = state.http_client.clone();
    let watchdog_endpoint = handle.endpoint;
    let watchdog_settings = crate::dns::forwarder::read_dns_settings(app);
    let watchdog_protocol = watchdog_settings.protocol;
    let health_check_target = watchdog_settings
        .health_check_targets
        .first()
        .cloned()
        .unwrap_or_else(|| DEFAULT_HEALTH_CHECK_TARGET.into());
    let app_clone = app.clone();

    if watchdog {
        crate::dns::spawn_dns_watchdog(
            client_clone,
            watchdog_endpoint,
            watchdog_protocol,
            health_check_target,
            shutdown_clone,
            app_clone,
        );
        handle.watchdog_enabled = true;
        tracing::info!("DNS watchdog was enabled and its health-check task was started.");
    } else {
        tracing::info!("DNS watchdog was disabled; no health-check task was started.");
    }

    let status = {
        let mut guard = state
            .forwarder
            .lock()
            .map_err(|_| "Forwarder lock poisoned.".to_string())?;

        if guard.is_some() {
            tauri::async_runtime::spawn(async move {
                handle.stop().await;
            });
            return Err("DoH Forwarder is already running.".into());
        }

        let port = handle.port;
        let endpoint = handle.endpoint.url().to_string();
        *guard = Some(handle);

        forwarder_status(true, port, endpoint, watchdog, app)
    };

    tracing::info!(
        "DNS forwarder is running and verified: protocol={}, cache={}, adblock={}, port={}",
        status.protocol.to_uppercase(),
        status.cache,
        status.adblock,
        status.port
    );
    Ok(status)
}

#[tauri::command]
pub async fn stop_doh_forwarder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _sync_guard = state.dns_sync.lock().await;
    let handle = {
        let mut guard = state
            .forwarder
            .lock()
            .map_err(|_| "Forwarder lock poisoned.".to_string())?;
        guard.take()
    };

    if let Some(h) = handle {
        let previous_dns = h.previous_dns.clone();
        h.stop().await;
        let reset = crate::dns::restore_dns_snapshot(&previous_dns);
        if !reset.success {
            return Err(format!(
                "Forwarder stopped but automatic DNS restore failed: {:?}",
                reset.error
            ));
        }
        crate::dns::clear_dns_restore_snapshot(&app)?;
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
    let raw_targets = targets.unwrap_or_default();
    let mut cleaned_targets = Vec::new();

    for t in &raw_targets {
        let clean = t
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_lowercase();
        if !clean.is_empty() && !clean.starts_with("*.") {
            cleaned_targets.push(clean);
        }
    }

    if cleaned_targets.is_empty() {
        cleaned_targets.push(DEFAULT_HEALTH_CHECK_TARGET.to_string());
    }

    let actual_targets = crate::config::domain::canonicalize_domain_rules(&cleaned_targets)
        .unwrap_or_else(|_| vec![DEFAULT_HEALTH_CHECK_TARGET.to_string()]);

    let client = &state.http_client;
    let mut tasks = Vec::new();

    for target in &actual_targets {
        let url = format!("https://{}", target);
        let client_clone = client.clone();
        tasks.push(async move {
            let start = std::time::Instant::now();
            let res = client_clone
                .get(&url)
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                )
                .timeout(Duration::from_millis(2500))
                .send()
                .await;
            (res.is_ok(), start.elapsed().as_millis() as u64)
        });
    }

    let results = futures::future::join_all(tasks).await;

    let mut healthy_count = 0;
    let mut min_latency: u64 = 0;

    for (ok, latency) in results {
        if ok {
            healthy_count += 1;
            if min_latency == 0 || latency < min_latency {
                min_latency = latency;
            }
        }
    }

    let is_healthy = healthy_count > 0;
    let latency_ms = if is_healthy { min_latency } else { 0 };

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
        healthy: is_healthy,
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
    let response = client
        .get("https://ipwho.is/")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let data = response
        .json::<IpWhoIsResponse>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(data)
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "The URL is invalid.".to_string())?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err("Security policy: only absolute HTTPS URLs are allowed.".into());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(parsed.as_str(), None::<String>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_network_stats() -> (u64, u64) {
    crate::network::get_total_network_bytes()
}

#[tauri::command]
pub async fn set_dns_watchdog(
    enabled: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ForwarderStatus, String> {
    let _sync_guard = state.dns_sync.lock().await;
    let existing = {
        let mut guard = state
            .forwarder
            .lock()
            .map_err(|_| "Forwarder lock poisoned.".to_string())?;
        guard.take()
    };
    let Some(handle) = existing else {
        tracing::info!("DNS watchdog setting saved; forwarder is not running.");
        return Ok(forwarder_status(
            false,
            DOH_FORWARDER_DEFAULT_PORT,
            DoHEndpoint::Cloudflare.url().to_string(),
            false,
            &app,
        ));
    };
    let endpoint = handle.endpoint;
    let previous_dns = handle.previous_dns.clone();
    handle.stop().await;
    let status = match start_dns_forwarder_runtime(&app, state.inner(), enabled, endpoint).await {
        Ok(status) => status,
        Err(error) => {
            let restored = crate::dns::restore_dns_snapshot(&previous_dns);
            if restored.success {
                let _ = crate::dns::clear_dns_restore_snapshot(&app);
            }
            return Err(format!(
                "DNS watchdog change could not restart the forwarder; previous DNS restore success={}: {error}",
                restored.success
            ));
        }
    };
    if let Ok(mut guard) = state.forwarder.lock() {
        if let Some(handle) = guard.as_mut() {
            handle.previous_dns = previous_dns;
        }
    }
    tracing::info!("DNS watchdog runtime state was changed and verified: enabled={enabled}.");
    Ok(status)
}

#[tauri::command]
pub fn validate_socks5_proxy(proxy: String) -> Result<String, String> {
    crate::dns::forwarder::normalize_socks5_proxy(&proxy)
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigStatus {
    protocol: String,
    adblock: bool,
    cache: bool,
    socks5_proxy: String,
    forwarder_active: bool,
    config_revision: u64,
    stage: DnsApplyStage,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DnsApplyStage {
    Persisted,
    Applied,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Flat arguments preserve the existing Tauri IPC contract.
pub async fn sync_dns_settings(
    protocol: String,
    adblock: bool,
    cache: bool,
    socks5_proxy: String,
    health_check_targets: Vec<String>,
    emit_event: Option<bool>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DnsConfigStatus, IpcError> {
    const OPERATION: &str = "sync_dns_settings";
    let _sync_guard = state.dns_sync.lock().await;
    if protocol != "doh" && protocol != "dot" {
        return Err(IpcError::validation(
            OPERATION,
            "INVALID_DNS_PROTOCOL",
            format!("Unsupported DNS transport protocol: {protocol}"),
        ));
    }
    let health_check_targets = crate::config::domain::canonicalize_domain_rules(
        &health_check_targets,
    )
    .map_err(|error| {
        IpcError::validation(
            OPERATION,
            "INVALID_HEALTH_CHECK_TARGET",
            format!("Invalid DNS health-check target: {error}"),
        )
    })?;
    if health_check_targets.is_empty() {
        return Err(IpcError::validation(
            OPERATION,
            "HEALTH_CHECK_TARGETS_EMPTY",
            "At least one DNS health-check target is required.",
        ));
    }
    let settings = crate::dns::forwarder::DnsSettings {
        protocol: protocol.clone(),
        adblock,
        cache,
        socks5_proxy: socks5_proxy.clone(),
        health_check_targets,
    };
    let settings_changed = crate::dns::forwarder::read_dns_settings(&app) != settings;
    crate::dns::forwarder::update_dns_settings_cache(settings.clone())
        .map_err(|error| IpcError::runtime(OPERATION, "DNS_SETTINGS_PERSIST_FAILED", error))?;
    if adblock {
        crate::dns::forwarder::initialize_adblock(&app);
    }
    let running = if settings_changed {
        let mut guard = state.forwarder.lock().map_err(|_| {
            IpcError::runtime(
                OPERATION,
                "INTERNAL_STATE_ERROR",
                "Forwarder lock poisoned.",
            )
        })?;
        guard.take()
    } else {
        None
    };
    if let Some(handle) = running {
        let endpoint = handle.endpoint;
        let watchdog_enabled = handle.watchdog_enabled;
        let previous_dns = handle.previous_dns.clone();
        handle.stop().await;
        if let Err(error) =
            start_dns_forwarder_runtime(&app, state.inner(), watchdog_enabled, endpoint).await
        {
            let restored = crate::dns::restore_dns_snapshot(&previous_dns);
            if restored.success {
                let _ = crate::dns::clear_dns_restore_snapshot(&app);
            }
            return Err(IpcError::runtime(
                OPERATION,
                "DNS_FORWARDER_RESTART_FAILED",
                format!(
                    "DNS runtime restart failed; previous DNS restore success={}: {error}",
                    restored.success
                ),
            ));
        }
        if let Ok(mut guard) = state.forwarder.lock() {
            if let Some(handle) = guard.as_mut() {
                handle.previous_dns = previous_dns;
            }
        }
        tracing::info!("Running DNS forwarder was restarted to verify the changed settings.");
    } else if !settings_changed {
        tracing::info!("DNS settings were already active; no forwarder restart was needed.");
    }

    let verified = crate::dns::forwarder::read_dns_settings(&app);
    let forwarder_active = state
        .forwarder
        .lock()
        .map_err(|_| {
            IpcError::runtime(
                OPERATION,
                "INTERNAL_STATE_ERROR",
                "Forwarder lock poisoned.",
            )
        })?
        .is_some();
    let config_revision = state
        .dns_config_revision
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;
    let result = DnsConfigStatus {
        protocol: verified.protocol,
        adblock: verified.adblock,
        cache: verified.cache,
        socks5_proxy: verified.socks5_proxy,
        forwarder_active,
        config_revision,
        stage: if forwarder_active {
            DnsApplyStage::Applied
        } else {
            DnsApplyStage::Persisted
        },
    };
    if emit_event.unwrap_or(true) {
        if let Err(error) = app.emit("dns_config_synced", result.clone()) {
            tracing::warn!(
                "DNS settings were applied, but the cross-window notification failed: {error}"
            );
        }
    }
    tracing::info!(
        "DNS settings accepted: revision={}, stage={}, protocol={}, cache={}, adblock={}, proxy={}, health_target={}",
        config_revision,
        if forwarder_active { "applied" } else { "persisted" },
        result.protocol.to_uppercase(), result.cache, result.adblock,
        if result.socks5_proxy.is_empty() { "direct" } else { "SOCKS5H" },
        verified.health_check_targets.first().map(String::as_str).unwrap_or(DEFAULT_HEALTH_CHECK_TARGET)
    );
    Ok(result)
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BypassApplyStage {
    Prepared,
    ProcessStarted,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BypassConfigStatus {
    mode: String,
    domain_count: usize,
    config_revision: u64,
    stage: BypassApplyStage,
    engine_restarted: bool,
    engine_running: bool,
    whitelist_domains: Vec<String>,
    blacklist_domains: Vec<String>,
    active_preset_id: String,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Flat arguments preserve the existing Tauri IPC contract.
pub async fn sync_bypass_config(
    mode: String,
    list: String,
    proxy: String,
    kill_switch: bool,
    whitelist_domains: Vec<String>,
    blacklist_domains: Vec<String>,
    active_preset_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BypassConfigStatus, IpcError> {
    const OPERATION: &str = "sync_bypass_config";
    let _sync_guard = state.bypass_sync.lock().await;
    if mode != "all" && mode != "whitelist" && mode != "blacklist" {
        return Err(IpcError::validation(
            OPERATION,
            "INVALID_BYPASS_MODE",
            format!("Unsupported bypass mode: {mode}"),
        ));
    }
    let whitelist_domains = crate::config::domain::canonicalize_domain_rules(&whitelist_domains)
        .map_err(|error| {
            IpcError::validation(
                OPERATION,
                "INVALID_WHITELIST_DOMAIN",
                format!("Invalid whitelist domain: {error}"),
            )
        })?;
    let blacklist_domains = crate::config::domain::canonicalize_domain_rules(&blacklist_domains)
        .map_err(|error| {
            IpcError::validation(
                OPERATION,
                "INVALID_BLACKLIST_DOMAIN",
                format!("Invalid blacklist domain: {error}"),
            )
        })?;
    if mode == "whitelist" && whitelist_domains.is_empty() {
        return Err(IpcError::validation(
            OPERATION,
            "WHITELIST_EMPTY",
            "Whitelist mode requires at least one valid domain; the running engine was left unchanged.",
        ));
    }
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
        tracing::info!(
            "Bypass config changed while engine is running. Restarting engine silently..."
        );
        if kill_switch {
            let forwarder_active = state
                .forwarder
                .lock()
                .map_err(|_| {
                    IpcError::runtime(
                        OPERATION,
                        "INTERNAL_STATE_ERROR",
                        "Forwarder lock poisoned.",
                    )
                })?
                .is_some();
            if !forwarder_active {
                let _dns_guard = state.dns_sync.lock().await;
                let forwarder_active = state
                    .forwarder
                    .lock()
                    .map_err(|_| {
                        IpcError::runtime(
                            OPERATION,
                            "INTERNAL_STATE_ERROR",
                            "Forwarder lock poisoned.",
                        )
                    })?
                    .is_some();
                if forwarder_active {
                    tracing::info!("DNS forwarder became active while the Pattern update was waiting; reusing it.");
                } else {
                    let runtime = crate::settings::read_runtime_settings(&app)
                        .map_err(|error| {
                            IpcError::runtime(OPERATION, "SETTINGS_READ_FAILED", error)
                        })?
                        .unwrap_or_default();
                    let endpoint = if runtime.selected_dns_id == "google" {
                        DoHEndpoint::Google
                    } else {
                        DoHEndpoint::Cloudflare
                    };
                    start_dns_forwarder_runtime(&app, state.inner(), runtime.watchdog, endpoint)
                        .await
                        .map_err(|error| {
                            IpcError::runtime(OPERATION, "DNS_FORWARDER_START_FAILED", error)
                        })?;
                    tracing::info!(
                        "DNS forwarder was started before applying Kill Switch to the running engine."
                    );
                }
            }
        }
        let preset = state
            .config_loader
            .lock()
            .map_err(|_| {
                IpcError::runtime(
                    OPERATION,
                    "INTERNAL_STATE_ERROR",
                    "Preset loader lock is poisoned.",
                )
            })?
            .find_preset(&active_preset_id)
            .ok_or_else(|| {
                IpcError::validation(
                    OPERATION,
                    "PRESET_NOT_FOUND",
                    format!(
                        "Active preset '{}' was not found; Pattern restart was cancelled.",
                        active_preset_id
                    ),
                )
            })?;
        state.engine_manager.stop(&app).await.map_err(|error| {
            IpcError::runtime(
                OPERATION,
                "ENGINE_STOP_FAILED",
                format!("Failed to stop engine before Pattern restart: {error}"),
            )
        })?;
        if let Err(e) = state.engine_manager.start(&preset, &app).await {
            return Err(IpcError::runtime(
                OPERATION,
                "ENGINE_RESTART_FAILED",
                format!("Failed to restart engine: {e}"),
            ));
        }
        engine_restarted = true;
    }
    let domain_count = match mode.as_str() {
        "whitelist" => whitelist_domains.len(),
        "blacklist" => blacklist_domains.len(),
        _ => 0,
    };
    let config_revision = state
        .bypass_config_revision
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;
    let result = BypassConfigStatus {
        mode: mode.clone(),
        domain_count,
        config_revision,
        stage: if engine_restarted {
            BypassApplyStage::ProcessStarted
        } else {
            BypassApplyStage::Prepared
        },
        engine_restarted,
        engine_running: matches!(
            state.engine_manager.current_status(),
            crate::engine::EngineStatus::Running { .. }
        ),
        whitelist_domains,
        blacklist_domains,
        active_preset_id,
    };
    if let Err(error) = app.emit("bypass_config_synced", result.clone()) {
        tracing::warn!(
            "Bypass settings were applied, but the cross-window notification failed: {error}"
        );
    }
    tracing::info!(
        "Bypass pattern accepted: revision={}, mode={}, domains={}, stage={}",
        config_revision,
        mode,
        domain_count,
        if engine_restarted { "process_started" } else { "prepared" }
    );
    Ok(result)
}
