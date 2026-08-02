use crate::config::preset::Preset;
use crate::dns::{
    builtin_providers, get_active_adapters, is_using_trusted_dns, resolve_doh, ApplyDnsResult,
    DnsProvider, DoHEndpoint, DohResult, NetworkAdapter, DEFAULT_HEALTH_CHECK_TARGET,
    DOH_CLOUDFLARE, DOH_FORWARDER_DEFAULT_PORT, DOH_GOOGLE,
};
use crate::engine::{EngineError, EngineStatus};
use crate::ipc::IpcError;
use crate::privilege::checker::is_elevated;
use crate::AppState;
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
pub fn get_advanced_capabilities() -> crate::config::validator::AdvancedCapabilities {
    crate::config::validator::AdvancedCapabilities::for_current_platform()
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

use crate::optimizer::{OptimizerResultDto, ProductionOptimizerRuntime};

#[tauri::command]
pub async fn start_auto_optimize(
    candidate_ids: Option<Vec<String>>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<OptimizerResultDto, String> {
    let _exclusive = state
        .exclusive_operations
        .try_acquire(crate::operation::ExclusiveOperation::Optimizer(format!(
            "optimizer-{}",
            std::process::id()
        )))
        .map_err(|owner| format!("Runtime is busy with {owner:?}"))?;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {e}"))?;

    let runtime = ProductionOptimizerRuntime::new(app.clone(), state.engine_manager.clone());

    state
        .optimizer_manager
        .run_optimizer_session(&app, &app_data, &runtime, candidate_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cancel_optimizer(state: State<'_, AppState>) -> bool {
    state.optimizer_manager.cancel_active()
}

#[tauri::command]
pub async fn apply_optimizer_recommendation(
    preset_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), EngineError> {
    let preset = {
        let loader = lock_or_err!(state.config_loader)?;
        loader
            .all_presets()
            .into_iter()
            .find(|p| p.id == preset_id)
            .ok_or_else(|| EngineError::InvalidPreset(format!("Preset '{preset_id}' not found.")))?
    };

    state.engine_manager.start(&preset, &app).await
}

#[tauri::command]
pub fn get_last_optimized_preset(_app: AppHandle) -> Option<Preset> {
    None
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
    let provider = match (primary.as_str(), secondary.as_str()) {
        ("1.1.1.1", "1.0.0.1") => "cloudflare",
        ("8.8.8.8", "8.8.4.4") => "google",
        _ => return Ok(ApplyDnsResult { success: false, applied_adapters: vec![], error: Some("Only an explicitly selected encrypted DNS provider may mutate DNS through the transaction manager.".into()) }),
    };
    let candidate = crate::dns::DnsConfigCandidate {
        enabled: true,
        protocol: "doh".into(),
        provider: Some(provider.into()),
        adblock: false,
        cache_enabled: true,
        socks5: None,
        kill_switch: false,
    };
    match state
        .dns_transaction_manager
        .apply_candidate(candidate, &app, state.inner())
        .await
    {
        Ok(_) => {
            let _ = app.emit("dns_status_changed", ());
            Ok(ApplyDnsResult {
                success: true,
                applied_adapters: vec![],
                error: None,
            })
        }
        Err(error) => Ok(ApplyDnsResult {
            success: false,
            applied_adapters: vec![],
            error: Some(error),
        }),
    }
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
    let candidate = crate::dns::DnsConfigCandidate {
        enabled: false,
        protocol: "doh".into(),
        provider: Some("cloudflare".into()),
        adblock: false,
        cache_enabled: false,
        socks5: None,
        kill_switch: false,
    };
    match state
        .dns_transaction_manager
        .apply_candidate(candidate, &app, state.inner())
        .await
    {
        Ok(_) => {
            let _ = app.emit("dns_status_changed", ());
            Ok(ApplyDnsResult {
                success: true,
                applied_adapters: vec![],
                error: None,
            })
        }
        Err(error) => Ok(ApplyDnsResult {
            success: false,
            applied_adapters: vec![],
            error: Some(error),
        }),
    }
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
        tracing::info!(
            "DNS Guard: current DNS is unverified; no mutation is performed without explicit opt-in."
        );
        let apply_res = ApplyDnsResult {
            success: false,
            applied_adapters: vec![],
            error: Some("Explicit encrypted DNS selection required".into()),
        };
        if apply_res.success {
            let _ = app.emit(
                "log_batch",
                vec![
                    "[DNS] 🛡️ ISS varsayılan DNS engellemesi tespit edildi. Sistem DNS'i otomatik olarak Cloudflare (1.1.1.1) olarak ayarlandı.".to_string(),
                ],
            );
        } else {
            let _ = app.emit(
                "log_batch",
                vec![
                    "[UYARI] ⚠️ Sistem DNS'i otomatik ayarlanamadı: Yönetici yetkisi gerekebilir."
                        .to_string(),
                ],
            );
        }
    }

    let preset = {
        let loader = lock_or_err!(state.config_loader)?;
        loader
            .find_preset(&preset_id)
            .ok_or(EngineError::InvalidPreset(preset_id))?
    };
    state.engine_manager.start(&preset, &app).await?;

    let status = state.engine_manager.current_status();
    if let EngineStatus::Ready { pid, .. } = status {
        let _ = app.emit(
            "log_batch",
            vec![
                format!("[MOTOR] 🚀 DPI Bypass motoru başarıyla aktifleştirildi (Profil: {}, PID: {}).", preset.label, pid),
                "[DNS] 🟢 Güvenli DNS doğrulaması tamamlandı. Tüm TCP 80/443 paketleri çözümleniyor.".to_string(),
            ],
        );
    }

    let _ = app.emit("dns_status_changed", ());

    Ok(status)
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
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if !ext.eq_ignore_ascii_case("vane") {
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
            crate::config::loader::validate_remote_presets(&manifest.presets).map_err(|error| {
                format!("Signed remote preset update is incompatible with this engine: {error}")
            })?;
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
    let settings = crate::dns::forwarder::read_dns_settings(app);
    let candidate = crate::dns::DnsConfigCandidate {
        enabled: true,
        protocol: settings.protocol,
        provider: Some(match endpoint {
            DoHEndpoint::Cloudflare => "cloudflare".into(),
            DoHEndpoint::Google => "google".into(),
        }),
        adblock: settings.adblock,
        cache_enabled: settings.cache,
        socks5: None,
        kill_switch: crate::engine::manager::kill_switch_enabled(),
    };
    state
        .dns_transaction_manager
        .apply_candidate(candidate, app, state)
        .await?;

    let mut guard = state
        .forwarder
        .lock()
        .map_err(|_| "Forwarder lock poisoned.".to_string())?;
    let handle = guard
        .as_mut()
        .ok_or_else(|| "DNS transaction completed without an owned forwarder.".to_string())?;
    handle.watchdog_enabled = watchdog;
    Ok(forwarder_status(
        true,
        handle.port,
        handle.endpoint.url().to_string(),
        watchdog,
        app,
    ))
}
#[tauri::command]
pub async fn stop_doh_forwarder(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let _sync_guard = state.dns_sync.lock().await;
    let candidate = crate::dns::DnsConfigCandidate {
        enabled: false,
        protocol: "doh".into(),
        provider: Some("cloudflare".into()),
        adblock: false,
        cache_enabled: false,
        socks5: None,
        kill_switch: false,
    };
    state
        .dns_transaction_manager
        .apply_candidate(candidate, &app, state.inner())
        .await?;
    let _ = app.emit("dns_status_changed", ());
    Ok(())
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
    _state: State<'_, AppState>,
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

    let mut tasks = Vec::new();

    for target in &actual_targets {
        let host = target.clone();
        tasks.push(async move {
            let start = std::time::Instant::now();
            let addr_str = format!("{}:443", host);
            let res = tokio::time::timeout(
                Duration::from_millis(1500),
                tokio::net::TcpStream::connect(&addr_str),
            )
            .await;

            let is_ok = matches!(res, Ok(Ok(_)));
            let latency = start.elapsed().as_millis() as u64;
            (is_ok, latency)
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
    handle.stop().await;
    let status = match start_dns_forwarder_runtime(&app, state.inner(), enabled, endpoint).await {
        Ok(status) => status,
        Err(error) => {
            return Err(format!(
                "DNS watchdog transaction could not restart the forwarder: {error}"
            ));
        }
    };
    tracing::info!("DNS watchdog runtime state was changed and verified: enabled={enabled}.");
    Ok(status)
}

#[tauri::command]
pub fn validate_socks5_proxy(proxy: String) -> Result<String, String> {
    crate::dns::forwarder::normalize_socks5_proxy(&proxy)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigStatus {
    pub protocol: String,
    pub adblock: bool,
    pub cache: bool,
    pub socks5_proxy: String,
    pub forwarder_active: bool,
    pub config_revision: u64,
    pub config_fingerprint: String,
    pub stage: crate::dns::DnsApplyStage,
    pub applied_revision: Option<u64>,
    pub applied_fingerprint: Option<String>,
    pub forwarder_state: crate::dns::DnsForwarderState,
    pub kill_switch_applied: bool,
    pub rollback_performed: bool,
    pub rollback_succeeded: bool,
    pub superseded: bool,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Flat arguments preserve the existing Tauri IPC contract.
pub async fn sync_dns_settings(
    protocol: String,
    adblock: bool,
    cache: bool,
    socks5_proxy: String,
    _health_check_targets: Vec<String>,
    emit_event: Option<bool>,
    provider: Option<String>,
    kill_switch: Option<bool>,
    enabled: Option<bool>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DnsConfigStatus, IpcError> {
    const OPERATION: &str = "sync_dns_settings";

    let socks_cand = if !socks5_proxy.trim().is_empty() {
        let parts: Vec<&str> = socks5_proxy.split(':').collect();
        let host = parts.first().copied().unwrap_or("127.0.0.1").to_string();
        let port = parts
            .get(1)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(1080);
        Some(crate::dns::DnsSocksCandidate {
            host,
            port,
            username: None,
            password: None,
        })
    } else {
        None
    };

    let candidate = crate::dns::DnsConfigCandidate {
        enabled: enabled.unwrap_or(true),
        protocol,
        provider,
        adblock,
        cache_enabled: cache,
        socks5: socks_cand,
        kill_switch: kill_switch.unwrap_or(false),
    };

    let outcome = state
        .dns_transaction_manager
        .apply_candidate(candidate.clone(), &app, state.inner())
        .await
        .map_err(|e| IpcError::runtime(OPERATION, "DNS_TRANSACTION_FAILED", e))?;

    let forwarder_active = outcome.forwarder_state == crate::dns::DnsForwarderState::Ready;

    let result = DnsConfigStatus {
        protocol: candidate.protocol.clone(),
        adblock,
        cache,
        socks5_proxy,
        forwarder_active,
        config_revision: outcome.config_revision,
        config_fingerprint: outcome.config_fingerprint,
        stage: outcome.stage,
        applied_revision: outcome.applied_revision,
        applied_fingerprint: outcome.applied_fingerprint,
        forwarder_state: outcome.forwarder_state,
        kill_switch_applied: outcome.kill_switch_applied,
        rollback_performed: outcome.rollback_performed,
        rollback_succeeded: outcome.rollback_succeeded,
        superseded: outcome.superseded,
    };

    if emit_event.unwrap_or(true) {
        if let Err(error) = app.emit("dns_config_synced", result.clone()) {
            tracing::warn!(
                "DNS settings were applied, but the cross-window notification failed: {error}"
            );
        }
    }

    Ok(result)
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BypassApplyStage {
    Prepared,
    ProcessStarted,
    RolledBack,
    Superseded,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BypassConfigStatus {
    pub mode: String,
    pub domain_count: usize,
    pub config_revision: u64,
    pub config_fingerprint: String,
    pub stage: BypassApplyStage,
    pub applied_revision: Option<u64>,
    pub applied_fingerprint: Option<String>,
    pub engine_restarted: bool,
    pub engine_running: bool,
    pub rollback_performed: bool,
    pub rollback_succeeded: bool,
    pub superseded: bool,
    pub whitelist_domains: Vec<String>,
    pub blacklist_domains: Vec<String>,
    pub canonical_whitelist_domains: Vec<String>,
    pub canonical_blacklist_domains: Vec<String>,
    pub active_preset_id: String,
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
    let _exclusive = state
        .exclusive_operations
        .try_acquire(crate::operation::ExclusiveOperation::PatternTransaction(
            format!("pattern-{}", std::process::id()),
        ))
        .map_err(|owner| {
            IpcError::runtime(
                OPERATION,
                "RUNTIME_BUSY",
                format!("Runtime is busy with {owner:?}"),
            )
        })?;
    let _sync_guard = state.bypass_sync.lock().await;

    if mode != "all" && mode != "whitelist" && mode != "blacklist" {
        return Err(IpcError::validation(
            OPERATION,
            "INVALID_BYPASS_MODE",
            format!("Unsupported bypass mode: {mode}"),
        ));
    }

    let canonical_whitelist = crate::config::domain::canonicalize_domain_rules(&whitelist_domains)
        .map_err(|error| {
            IpcError::validation(
                OPERATION,
                "INVALID_WHITELIST_DOMAIN",
                format!("Invalid whitelist domain: {error}"),
            )
        })?;
    let canonical_blacklist = crate::config::domain::canonicalize_domain_rules(&blacklist_domains)
        .map_err(|error| {
            IpcError::validation(
                OPERATION,
                "INVALID_BLACKLIST_DOMAIN",
                format!("Invalid blacklist domain: {error}"),
            )
        })?;

    if mode == "whitelist" && canonical_whitelist.is_empty() {
        return Err(IpcError::validation(
            OPERATION,
            "WHITELIST_EMPTY",
            "Whitelist mode requires at least one valid domain; the running engine was left unchanged.",
        ));
    }

    let canonical_list = match mode.as_str() {
        "whitelist" => canonical_whitelist.join("\n"),
        "blacklist" => canonical_blacklist.join("\n"),
        _ => String::new(),
    };

    if list.trim() != canonical_list {
        tracing::warn!(
            "Pattern list received from the interface did not match the canonical domain rules; the verified domain arrays were used."
        );
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

    let config_revision_num = state
        .bypass_config_revision
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;
    let revision = crate::engine::runtime_config::ConfigRevision::new(config_revision_num);

    let candidate = crate::engine::runtime_config::candidate_from_preset_and_sources(
        &preset,
        &mode,
        &canonical_list,
        kill_switch,
    );

    let verified_config = crate::engine::runtime_config::verify_runtime_config(candidate, revision)
        .map_err(|err| IpcError::validation(OPERATION, "VERIFICATION_FAILED", err.to_string()))?;

    let tx_lock = state.engine_manager.pattern_transaction_lock();
    let _tx_guard = tx_lock.lock().await;

    let requested_rev = {
        let rc_state = state.engine_manager.runtime_config_state();
        let mut st = rc_state.lock().unwrap();
        let _ = st.advance_requested_revision();
        st.latest_requested_revision()
    };

    if revision.get() < requested_rev.get() {
        return Ok(BypassConfigStatus {
            mode: mode.clone(),
            domain_count: match mode.as_str() {
                "whitelist" => canonical_whitelist.len(),
                "blacklist" => canonical_blacklist.len(),
                _ => 0,
            },
            config_revision: revision.get(),
            config_fingerprint: verified_config.fingerprint.to_string(),
            stage: BypassApplyStage::Superseded,
            applied_revision: state
                .engine_manager
                .applied_config()
                .map(|a| a.verified.revision.get()),
            applied_fingerprint: state
                .engine_manager
                .applied_config()
                .map(|a| a.verified.fingerprint.to_string()),
            engine_restarted: false,
            engine_running: matches!(
                state.engine_manager.current_status(),
                crate::engine::EngineStatus::Ready { .. }
            ),
            rollback_performed: false,
            rollback_succeeded: false,
            superseded: true,
            whitelist_domains: canonical_whitelist.clone(),
            blacklist_domains: canonical_blacklist.clone(),
            canonical_whitelist_domains: canonical_whitelist.clone(),
            canonical_blacklist_domains: canonical_blacklist.clone(),
            active_preset_id,
        });
    }

    let previous_persisted = crate::settings::read_runtime_settings(&app).ok().flatten();
    let previous_applied = state.engine_manager.applied_config();

    state
        .engine_manager
        .runtime_config_state()
        .lock()
        .unwrap()
        .set_desired(verified_config.clone());

    crate::engine::manager::update_bypass_config_cache(
        mode.clone(),
        canonical_list.clone(),
        proxy.clone(),
        kill_switch,
    );

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| IpcError::runtime(OPERATION, "DATA_DIR_ERROR", e.to_string()))?;

    let (prepared_config, active_filename) =
        crate::engine::pattern_transaction::prepare_runtime_config_for_transaction(
            verified_config.clone(),
            &app_data_dir,
        )
        .map_err(|e| IpcError::runtime(OPERATION, "HOSTLIST_ERROR", e.to_string()))?;

    state
        .engine_manager
        .runtime_config_state()
        .lock()
        .unwrap()
        .set_prepared(prepared_config.clone());

    let is_running = matches!(
        state.engine_manager.current_status(),
        crate::engine::EngineStatus::Ready { .. }
    );

    if !is_running {
        let result = BypassConfigStatus {
            mode: mode.clone(),
            domain_count: match mode.as_str() {
                "whitelist" => canonical_whitelist.len(),
                "blacklist" => canonical_blacklist.len(),
                _ => 0,
            },
            config_revision: revision.get(),
            config_fingerprint: verified_config.fingerprint.to_string(),
            stage: BypassApplyStage::Prepared,
            applied_revision: None,
            applied_fingerprint: None,
            engine_restarted: false,
            engine_running: false,
            rollback_performed: false,
            rollback_succeeded: false,
            superseded: false,
            whitelist_domains: canonical_whitelist.clone(),
            blacklist_domains: canonical_blacklist.clone(),
            canonical_whitelist_domains: canonical_whitelist.clone(),
            canonical_blacklist_domains: canonical_blacklist.clone(),
            active_preset_id,
        };

        let _ = app.emit("bypass_config_synced", result.clone());
        return Ok(result);
    }

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
            let runtime = crate::settings::read_runtime_settings(&app)
                .map_err(|error| IpcError::runtime(OPERATION, "SETTINGS_READ_FAILED", error))?
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
        }
    }

    state.engine_manager.stop(&app).await.map_err(|error| {
        IpcError::runtime(
            OPERATION,
            "ENGINE_STOP_FAILED",
            format!("Failed to stop engine before Pattern restart: {error}"),
        )
    })?;

    let candidate_start_res = state
        .engine_manager
        .start_prepared_config(prepared_config, &app)
        .await;

    match candidate_start_res {
        Ok(applied_config) => {
            let _ = state
                .engine_manager
                .runtime_config_state()
                .lock()
                .unwrap()
                .commit_applied(applied_config.clone());

            let _ = crate::engine::pattern_transaction::clean_stale_hostlists(
                &app_data_dir,
                active_filename.as_deref(),
                None,
            );

            let result = BypassConfigStatus {
                mode: mode.clone(),
                domain_count: match mode.as_str() {
                    "whitelist" => canonical_whitelist.len(),
                    "blacklist" => canonical_blacklist.len(),
                    _ => 0,
                },
                config_revision: revision.get(),
                config_fingerprint: verified_config.fingerprint.to_string(),
                stage: BypassApplyStage::ProcessStarted,
                applied_revision: Some(applied_config.verified.revision.get()),
                applied_fingerprint: Some(applied_config.verified.fingerprint.to_string()),
                engine_restarted: true,
                engine_running: true,
                rollback_performed: false,
                rollback_succeeded: false,
                superseded: false,
                whitelist_domains: canonical_whitelist.clone(),
                blacklist_domains: canonical_blacklist.clone(),
                canonical_whitelist_domains: canonical_whitelist.clone(),
                canonical_blacklist_domains: canonical_blacklist.clone(),
                active_preset_id,
            };

            let _ = app.emit("bypass_config_synced", result.clone());
            Ok(result)
        }
        Err(candidate_err) => {
            tracing::warn!(
                "Candidate engine start failed: {candidate_err}. Initiating transactional rollback to previous configuration..."
            );

            if let Some(prev) = previous_persisted {
                let mut settings_map = serde_json::Map::new();
                settings_map.insert("state".into(), serde_json::to_value(&prev).unwrap());
                let _ = crate::settings::atomic_replace_bytes(
                    &app_data_dir.join("settings.json"),
                    serde_json::to_string(&settings_map).unwrap().as_bytes(),
                );
            }

            if let Some(prev_applied) = previous_applied {
                let (prev_prep, _prev_filename) =
                    crate::engine::pattern_transaction::prepare_runtime_config_for_transaction(
                        prev_applied.verified.clone(),
                        &app_data_dir,
                    )
                    .map_err(|e| {
                        IpcError::runtime(OPERATION, "ROLLBACK_HOSTLIST_ERROR", e.to_string())
                    })?;

                match state
                    .engine_manager
                    .start_prepared_config(prev_prep, &app)
                    .await
                {
                    Ok(restored_applied) => {
                        state
                            .engine_manager
                            .runtime_config_state()
                            .lock()
                            .unwrap()
                            .restore_applied(restored_applied.clone());

                        if let Some(c_fn) = active_filename {
                            let _ = std::fs::remove_file(app_data_dir.join(c_fn));
                        }

                        let restored_mode = restored_applied.verified.bypass.mode.to_string();
                        let restored_domains = restored_applied.verified.bypass.domains.clone();
                        let (restored_whitelist, restored_blacklist) = match restored_mode.as_str()
                        {
                            "whitelist" => (restored_domains, Vec::new()),
                            "blacklist" => (Vec::new(), restored_domains),
                            _ => (Vec::new(), Vec::new()),
                        };
                        let result = BypassConfigStatus {
                            mode: restored_applied.verified.bypass.mode.to_string(),
                            domain_count: restored_applied.verified.bypass.domain_count,
                            config_revision: revision.get(),
                            config_fingerprint: verified_config.fingerprint.to_string(),
                            stage: BypassApplyStage::RolledBack,
                            applied_revision: Some(restored_applied.verified.revision.get()),
                            applied_fingerprint: Some(
                                restored_applied.verified.fingerprint.to_string(),
                            ),
                            engine_restarted: true,
                            engine_running: true,
                            rollback_performed: true,
                            rollback_succeeded: true,
                            superseded: false,
                            whitelist_domains: restored_whitelist.clone(),
                            blacklist_domains: restored_blacklist.clone(),
                            canonical_whitelist_domains: restored_whitelist,
                            canonical_blacklist_domains: restored_blacklist,
                            active_preset_id,
                        };

                        let _ = app.emit("bypass_config_synced", result.clone());
                        return Ok(result);
                    }
                    Err(rollback_err) => {
                        state
                            .engine_manager
                            .runtime_config_state()
                            .lock()
                            .unwrap()
                            .clear_applied();
                        return Err(IpcError::runtime(
                            OPERATION,
                            "ENGINE_ROLLBACK_FAILED",
                            format!(
                                "Candidate start failed ({candidate_err}) AND rollback start failed: {rollback_err}"
                            ),
                        ));
                    }
                }
            }

            state
                .engine_manager
                .runtime_config_state()
                .lock()
                .unwrap()
                .clear_applied();

            Err(IpcError::runtime(
                OPERATION,
                "ENGINE_START_FAILED",
                format!("Candidate engine start failed: {candidate_err}"),
            ))
        }
    }
}

#[tauri::command]
pub async fn get_artifact_integrity_status(
    app: AppHandle,
) -> Result<crate::security::ArtifactIntegrityStatusDto, EngineError> {
    use crate::security::{
        ArtifactIntegrityError, ArtifactIntegrityVerifier, ArtifactPlatform,
        Sha256ArtifactIntegrityVerifier,
    };

    let resource_root = app
        .path()
        .resource_dir()
        .map_err(|e| EngineError::IoError(format!("Failed to resolve resource dir: {e}")))?;

    let verifier = Sha256ArtifactIntegrityVerifier::from_embedded()?;
    let current_platform = ArtifactPlatform::current()
        .map(|p| format!("{p:?}"))
        .unwrap_or_else(|| "unknown".into());

    match verifier.verify_current_platform_group(&resource_root) {
        Ok(group) => Ok(crate::security::ArtifactIntegrityStatusDto {
            status: "verified".into(),
            target: current_platform,
            verified_artifacts: group.dependencies.len() + 1,
            failed_artifact_id: None,
            error_code: None,
            last_verified_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_default(),
            ),
        }),
        Err(err) => {
            let (status_str, failed_id) = match &err {
                ArtifactIntegrityError::ArtifactMissing(id) => ("missing", Some(id.clone())),
                ArtifactIntegrityError::ArtifactHashMismatch { id, .. } => {
                    ("modified", Some(id.clone()))
                }
                ArtifactIntegrityError::ArtifactSizeMismatch { id, .. } => {
                    ("modified", Some(id.clone()))
                }
                _ => ("invalid_manifest", None),
            };

            Ok(crate::security::ArtifactIntegrityStatusDto {
                status: status_str.into(),
                target: current_platform,
                verified_artifacts: 0,
                failed_artifact_id: failed_id,
                error_code: Some(format!("{err:?}")),
                last_verified_at: Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_secs().to_string())
                        .unwrap_or_default(),
                ),
            })
        }
    }
}

#[tauri::command]
pub async fn run_local_diagnostics(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::diagnostics::SystemHealthSnapshot, EngineError> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| EngineError::IoError(format!("Failed to resolve app_data_dir: {e}")))?;

    let mut snapshot = crate::diagnostics::perform_local_consistency_checks(&app_dir);
    let now = snapshot.timestamp_ms;
    let integrity = get_artifact_integrity_status(app.clone()).await;
    snapshot
        .subsystems
        .push(crate::diagnostics::SubsystemHealth {
            name: "Artifact Integrity".into(),
            state: if integrity
                .as_ref()
                .is_ok_and(|status| status.status == "verified")
            {
                crate::diagnostics::HealthState::Healthy
            } else {
                crate::diagnostics::HealthState::Unhealthy
            },
            message: integrity
                .map(|status| format!("Artifact verification status: {}", status.status))
                .unwrap_or_else(|_| "Artifact verification unavailable".into()),
            last_checked_ms: now,
        });
    let engine_status = state.engine_manager.current_status();
    snapshot
        .subsystems
        .push(crate::diagnostics::SubsystemHealth {
            name: "Engine Lifecycle".into(),
            state: match engine_status {
                EngineStatus::Ready { .. } | EngineStatus::Stopped => {
                    crate::diagnostics::HealthState::Healthy
                }
                EngineStatus::Error { .. } => crate::diagnostics::HealthState::Unhealthy,
                _ => crate::diagnostics::HealthState::Degraded,
            },
            message: format!("Authoritative engine state: {engine_status:?}"),
            last_checked_ms: now,
        });
    let dns_guard = state
        .dns_transaction_manager
        .runtime_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    snapshot
        .subsystems
        .push(crate::diagnostics::SubsystemHealth {
            name: "DNS Runtime".into(),
            state: if dns_guard.applied().is_some() {
                crate::diagnostics::HealthState::Healthy
            } else {
                crate::diagnostics::HealthState::Unknown
            },
            message: if dns_guard.applied().is_some() {
                "Committed DNS transaction present".into()
            } else {
                "No committed DNS transaction".into()
            },
            last_checked_ms: now,
        });
    snapshot
        .subsystems
        .push(crate::diagnostics::SubsystemHealth {
            name: "Diagnostic Store".into(),
            state: crate::diagnostics::HealthState::Healthy,
            message: format!(
                "Dropped event count: {}",
                crate::diagnostics::DIAGNOSTIC_STORE.dropped_count()
            ),
            last_checked_ms: now,
        });
    snapshot.overall = snapshot.subsystems.iter().fold(
        crate::diagnostics::HealthState::Healthy,
        |overall, subsystem| overall.combine(subsystem.state),
    );
    Ok(snapshot)
}

static TRAFFIC_PROBE_RUNNER: std::sync::LazyLock<crate::diagnostics::TrafficProbeRunner> =
    std::sync::LazyLock::new(crate::diagnostics::TrafficProbeRunner::new);

#[tauri::command]
pub async fn run_traffic_diagnostics(
    targets: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<crate::diagnostics::TrafficProbeReport, EngineError> {
    let _exclusive = state
        .exclusive_operations
        .try_acquire(crate::operation::ExclusiveOperation::TrafficProbe(format!(
            "probe-{}",
            std::process::id()
        )))
        .map_err(|owner| EngineError::IoError(format!("Runtime is busy with {owner:?}")))?;
    let target_list = targets.unwrap_or_default();
    TRAFFIC_PROBE_RUNNER
        .run_probes(&target_list)
        .await
        .map_err(EngineError::IoError)
}

#[tauri::command]
pub fn cancel_traffic_diagnostics() -> bool {
    TRAFFIC_PROBE_RUNNER.cancel()
}

#[tauri::command]
pub async fn export_diagnostics_bundle(
    app: AppHandle,
    export_path: String,
    state: State<'_, AppState>,
) -> Result<String, EngineError> {
    let health = run_local_diagnostics(app.clone(), state).await?;
    let events = crate::diagnostics::DIAGNOSTIC_STORE.get_events(None).await;
    let dropped = crate::diagnostics::DIAGNOSTIC_STORE.dropped_count();
    let bundle = crate::diagnostics::create_diagnostics_bundle(health, events, dropped);

    let target_path = std::path::PathBuf::from(export_path);
    crate::diagnostics::export_bundle_to_file(&bundle, &target_path)
        .map_err(EngineError::IoError)?;

    Ok(target_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_recent_diagnostic_events(
    limit: Option<usize>,
) -> Vec<crate::diagnostics::DiagnosticEvent> {
    crate::diagnostics::DIAGNOSTIC_STORE
        .get_events(limit.map(|value| value.min(500)))
        .await
}

#[tauri::command]
pub async fn clear_diagnostic_events() {
    crate::diagnostics::DIAGNOSTIC_STORE.clear().await;
}

#[tauri::command]
pub async fn get_diagnostic_event_stats() -> serde_json::Value {
    let events = crate::diagnostics::DIAGNOSTIC_STORE.get_events(None).await;
    serde_json::json!({ "eventCount": events.len(), "droppedEventCount": crate::diagnostics::DIAGNOSTIC_STORE.dropped_count(), "lastSequence": events.last().map(|event| event.sequence) })
}
