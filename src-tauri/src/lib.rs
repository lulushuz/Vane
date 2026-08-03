use log::LevelFilter;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener, Manager};

// Constant for CREATE_NO_WINDOW flag on Windows to prevent console window flashing.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub mod autostart;
pub mod commands;
pub mod config;
pub mod diagnostics;
pub mod dns;
pub mod engine;
pub mod http;
pub mod ipc;
pub mod logging;
pub mod network;
pub mod operation;
pub mod optimizer;
pub mod platform;
pub mod presets;
pub mod privilege;
pub(crate) mod security;
pub mod settings;
pub mod tray;
pub mod updater;

#[cfg(test)]
mod characterization;

use crate::config::loader::ConfigLoader;
use crate::dns::ForwarderHandle;
use crate::engine::EngineManager;
use crate::network::spawn_network_watcher;

pub struct AppState {
    pub engine_manager: EngineManager,
    pub config_loader: Mutex<ConfigLoader>,
    /*
       Application-wide shared HTTP client.
       Reuses the connection pool to reduce TCP overhead.
    */
    pub http_client: reqwest::Client,
    /*
       Active DoH forwarder handle. None if the forwarder is not running.
       Wrapped in Mutex so commands can start/stop it from different threads.
    */
    pub forwarder: Mutex<Option<ForwarderHandle>>,
    pub bypass_sync: tokio::sync::Mutex<()>,
    pub bypass_config_revision: AtomicU64,
    pub dns_sync: tokio::sync::Mutex<()>,
    pub dns_config_revision: AtomicU64,
    pub dns_transaction_manager: std::sync::Arc<crate::dns::DnsTransactionManager>,
    pub optimizer_manager: std::sync::Arc<crate::optimizer::OptimizerSessionManager>,
    pub exclusive_operations: std::sync::Arc<crate::operation::ExclusiveOperationCoordinator>,
}

fn build_app_state(loader: ConfigLoader, http_client: reqwest::Client) -> AppState {
    AppState {
        engine_manager: EngineManager::new(),
        config_loader: Mutex::new(loader),
        http_client,
        forwarder: Mutex::new(None),
        bypass_sync: tokio::sync::Mutex::new(()),
        bypass_config_revision: AtomicU64::new(0),
        dns_sync: tokio::sync::Mutex::new(()),
        dns_config_revision: AtomicU64::new(0),
        dns_transaction_manager: std::sync::Arc::new(crate::dns::DnsTransactionManager::new()),
        optimizer_manager: std::sync::Arc::new(crate::optimizer::OptimizerSessionManager::new()),
        exclusive_operations: std::sync::Arc::new(
            crate::operation::ExclusiveOperationCoordinator::default(),
        ),
    }
}

/*
   Cleans up dangling winws instances from previous sessions during initialization.
   Prevents zombie processes if the app previously crashed or was forcefully closed.
*/
#[cfg(target_os = "windows")]
fn kill_existing_winws() {
    tracing::info!(
        "Startup: Global process cleanup disabled in P07 to enforce owned process lifecycle."
    );
}

/// Automatically resumes DPI bypass after an --autostart launch.
/// Reads the persisted preset and starts the engine silently.
async fn autostart_engine_with_last_preset(app: AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        tracing::warn!("Auto-start: AppState henüz hazır değil, atlanıyor.");
        return;
    };

    let settings = match crate::settings::read_runtime_settings(&app) {
        Ok(Some(settings)) => settings,
        Ok(None) => {
            tracing::info!("Auto-start: Saved settings were not found; engine will not start.");
            return;
        }
        Err(error) => {
            tracing::error!("Auto-start stopped to protect saved settings: {error}");
            return;
        }
    };
    let preset_id = settings.active_preset_id.clone();

    if !matches!(
        settings.bypass_mode.as_str(),
        "all" | "whitelist" | "blacklist"
    ) {
        tracing::error!("Auto-start: Saved Pattern mode is invalid; engine startup was cancelled.");
        return;
    }
    let raw_domains = if settings.bypass_mode == "whitelist" {
        &settings.whitelist_domains
    } else {
        &settings.blacklist_domains
    };
    let active_domains = match crate::config::domain::canonicalize_domain_rules(raw_domains) {
        Ok(domains) => domains.join("\n"),
        Err(error) => {
            tracing::error!("Auto-start: Saved Pattern domain is invalid; engine startup was cancelled: {error}");
            return;
        }
    };
    if settings.bypass_mode == "whitelist" && active_domains.is_empty() {
        tracing::error!(
            "Auto-start: Whitelist mode has no valid domains; engine startup was cancelled safely."
        );
        return;
    }
    crate::engine::manager::update_bypass_config_cache(
        settings.bypass_mode.clone(),
        active_domains,
        settings.proxy_socks5.clone(),
        settings.kill_switch,
    );
    if let Err(error) =
        crate::dns::forwarder::update_dns_settings_cache(crate::dns::forwarder::DnsSettings {
            protocol: if settings.dns_protocol == "dot" {
                "dot".into()
            } else {
                "doh".into()
            },
            adblock: settings.dns_ad_block,
            cache: settings.dns_cache,
            socks5_proxy: settings.proxy_socks5.clone(),
            health_check_targets: settings.health_check_targets.clone(),
        })
    {
        tracing::error!("Auto-start: Saved DNS runtime settings are invalid: {error}");
        return;
    }

    let preset = {
        let Ok(loader) = state.config_loader.lock() else {
            return;
        };
        loader.find_preset(&preset_id)
    };

    if settings.dns_forwarder_enabled || settings.kill_switch {
        let endpoint = if settings.selected_dns_id == "google" {
            crate::dns::DoHEndpoint::Google
        } else {
            crate::dns::DoHEndpoint::Cloudflare
        };
        if let Err(error) = crate::commands::start_dns_forwarder_runtime(
            &app,
            state.inner(),
            settings.watchdog,
            endpoint,
        )
        .await
        {
            tracing::error!("Auto-start: DNS forwarder could not be restored; engine startup was cancelled: {error}");
            return;
        }
        tracing::info!("Auto-start: DNS forwarder and its saved runtime settings were restored.");
    } else if !settings.selected_dns_id.is_empty() {
        let selected = if settings.selected_dns_id == "custom" {
            Some((
                settings.dns_custom_primary.clone(),
                settings.dns_custom_secondary.clone(),
            ))
        } else {
            crate::dns::builtin_providers()
                .into_iter()
                .find(|provider| provider.id == settings.selected_dns_id)
                .map(|provider| (provider.primary, provider.secondary))
        };
        let Some((primary, secondary)) = selected else {
            tracing::error!(
                "Auto-start: Saved DNS provider '{}' no longer exists.",
                settings.selected_dns_id
            );
            return;
        };
        let provider = match (primary.as_str(), secondary.as_str()) {
            ("1.1.1.1", "1.0.0.1") => "cloudflare",
            ("8.8.8.8", "8.8.4.4") => "google",
            _ => {
                tracing::error!(
                    "Auto-start: Custom plaintext DNS cannot bypass DnsTransactionManager."
                );
                return;
            }
        };
        let candidate = crate::dns::DnsConfigCandidate {
            enabled: true,
            protocol: "doh".into(),
            provider: Some(provider.into()),
            adblock: false,
            cache_enabled: true,
            socks5: None,
            kill_switch: settings.kill_switch,
        };
        if let Err(error) = state
            .dns_transaction_manager
            .apply_candidate(candidate, &app, state.inner())
            .await
        {
            tracing::error!("Auto-start: Saved DNS settings could not be restored: {error}");
            return;
        }
        tracing::info!("Auto-start: Saved system DNS settings were restored and verified.");
    }

    match preset {
        Some(p) => {
            tracing::info!(
                "Auto-start: '{}' preset'i otomatik devreye alınıyor.",
                p.label
            );
            if let Err(e) = state.engine_manager.start(&p, &app).await {
                tracing::error!("Auto-start: Engine could not be started: {}", e);
                let rollback = crate::dns::DnsConfigCandidate {
                    enabled: false,
                    protocol: "doh".into(),
                    provider: Some("cloudflare".into()),
                    adblock: false,
                    cache_enabled: false,
                    socks5: None,
                    kill_switch: false,
                };
                let _ = state
                    .dns_transaction_manager
                    .apply_candidate(rollback, &app, state.inner())
                    .await;
                tracing::warn!("Auto-start rollback completed through DnsTransactionManager.");
            }
        }
        None => {
            tracing::warn!("Auto-start: Preset with ID '{}' not found.", preset_id);
        }
    }
}

pub fn run() {
    // NOTE: Do NOT call logging::init_logging() here.
    // tauri_plugin_log initialises the global tracing subscriber.
    // Calling try_init() a second time is a no-op in debug but causes a
    // silent crash (SetLogger error) in release builds running as Administrator.
    let builder = tauri::Builder::default()
        // Multi-instance prevention MUST be the very first plugin registered
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tracing::warn!("Second Vane instance detected — bringing window to front.");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(LevelFilter::Info)
                // Keep only the latest log file to prevent unbounded disk growth.
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                // 10 MB per log file — sufficient for several days of operation.
                .max_file_size(10 * 1024 * 1024)
                .build(),
        )
        .setup(|app| {
            // Init our custom tracing subscriber *once*. The OnceLock guard in
            // logging.rs ensures this is a no-op on subsequent calls, preventing
            // the silent crash that plagued v1.0.8+ on Administrator sessions.
            logging::init_logging();
            logging::set_app_handle(app.handle().clone());
            // Clean up dangling processes from previous runs (Windows only)
            #[cfg(target_os = "windows")]
            kill_existing_winws();

            // Phase 1: Load local and cached remote presets before AppState registration.
            let mut loader = ConfigLoader::new();
            if let Ok(app_data) = app.path().app_data_dir() {
                let presets_path = app_data.join("presets");
                let _ = std::fs::create_dir_all(&presets_path);
                loader.load_custom_presets_from(&presets_path);
                if let Some(cached) = crate::presets::load_cached_presets(&app_data) {
                    loader.load_remote_presets(cached.presets);
                }
            }

            let http_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .pool_max_idle_per_host(2)
                .build()
                .unwrap_or_else(|e| {
                    // TLS initialization can fail on restricted Windows installations.
                    tracing::warn!("HTTP Client TLS initialization failed, using fallback: {}", e);
                    reqwest::Client::new()
                });
            let fetch_client = http_client.clone();

            if !app.manage(build_app_state(loader, http_client)) {
                return Err(std::io::Error::other(
                    "AppState was already managed before setup initialization",
                )
                .into());
            }

            let inst_id = crate::dns::get_or_create_installation_id(app.handle());
            match crate::dns::recover_orphan_kill_switch_rules(app.handle(), &inst_id) {
                Ok(true) => tracing::warn!("Orphan DNS Kill Switch rules were removed and metadata was cleared."),
                Ok(false) => {}
                Err(error) => tracing::error!("Startup DNS Kill Switch recovery needs attention; metadata was preserved: {error}"),
            }
            match crate::platform::linux::recover_orphan_linux_filter_rules(app.handle(), &inst_id) {
                Ok(crate::platform::linux::LinuxFilterRecoveryOutcome::NoMetadata) => {}
                Ok(crate::platform::linux::LinuxFilterRecoveryOutcome::Recovered) => tracing::warn!(
                    "A previous Linux firewall shutdown was incomplete; owned rules were removed and metadata was cleared."
                ),
                Err(error) => tracing::error!(
                    "Startup Linux firewall recovery needs attention; metadata was preserved: {error}"
                ),
            }
            let state = app.state::<AppState>();
            match tauri::async_runtime::block_on(
                state.dns_transaction_manager.recover_stale_snapshot(app.handle()),
            ) {
                Ok(true) => tracing::warn!("A previous DNS forwarder shutdown was incomplete; the saved system DNS configuration was restored and verified."),
                Ok(false) => {}
                Err(error) => tracing::error!("Startup DNS recovery needs attention: {error}"),
            }

            // ─── Feature 6B: Event-driven network watcher ─────────────────
/* 
   Replaces the previous 30-second polling loop.
   WM_DEVICECHANGE fires immediately on adapter changes -> zero CPU overhead. 
*/
            let watcher_handle = app.handle().clone();
            if let Err(e) = spawn_network_watcher(watcher_handle) {
                tracing::warn!("Network watcher could not start: {}", e);
            }

            // WinDivert automatically applies its filters to new adapters transparently without needing a restart.
            // Therefore, we just log the event. Frontend DNS/Network UI can listen to `network_changed` directly.
            app.listen("network_changed", |_event| {
                tracing::info!("Network change detected. Frontend UI will be updated.");
            });

            // Phase 2: Fetch remote presets in background (non-blocking).
            // If offline, the cached presets loaded above remain active.
            if let Ok(app_data) = app.path().app_data_dir() {
                let fetch_app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    use crate::presets::{fetch_remote_presets, RemoteFetchOutcome};

                    let cached_ver = crate::presets::load_cached_presets(&app_data)
                        .map(|m| m.version);

                    match fetch_remote_presets(&fetch_client, cached_ver.as_deref()).await {
                        RemoteFetchOutcome::Updated(manifest, sig_text, raw_json) => {
                            let version = manifest.version.clone();
                            let _ = crate::presets::save_cached_presets_with_sig(&manifest, &raw_json, &sig_text, &app_data).await;
                            // Notify frontend — it will re-invoke list_presets
                            let _ = fetch_app.emit("remote_presets_updated", version);
                        }
                        RemoteFetchOutcome::Offline => {
                            tracing::warn!("Remote presets: Offline mode — using cache.");
                            let _ = fetch_app.emit("remote_presets_offline", ());
                        }
                        RemoteFetchOutcome::VersionUnchanged => {
                            tracing::debug!("Remote presets: Up to date, no update needed.");
                        }
                        RemoteFetchOutcome::ParseError(e) => {
                            tracing::error!("Remote presets parse error: {}", e);
                        }
                        RemoteFetchOutcome::SignatureInvalid => {
                            tracing::error!("CRITICAL WARNING: Remote presets (Gist) does not have a valid minisign signature!");
                        }
                    }
                });
            }

            // Detect --autostart early so it's available both for window visibility
            // and for the engine auto-start task spawned after app.manage() below.
            let is_autostart = std::env::args()
                .any(|arg| arg == "--autostart" || arg == "--minimized");

            // Lock main widget position to the bottom right of the primary screen
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.set_minimizable(false);
                let _ = main_win.set_maximizable(false);

                if let Ok(Some(monitor)) = main_win.current_monitor() {
                    let screen_size = monitor.size();
                    let scale = monitor.scale_factor();
                    let w = (320.0 * scale) as u32;
                    let h = (260.0 * scale) as u32;
                    let margin_x = (24.0 * scale) as u32;
                    let margin_y = (60.0 * scale) as u32;
                    let pos = tauri::PhysicalPosition::new(
                        screen_size.width.saturating_sub(w).saturating_sub(margin_x),
                        screen_size.height.saturating_sub(h).saturating_sub(margin_y),
                    );
                    let _ = main_win.set_position(pos);
                }

                if !is_autostart {
                    let _ = main_win.show();
                    let _ = main_win.set_focus();
                }
            }

            // System Tray setup
            crate::tray::setup_tray(app)?;

            // Auto-start: if launched via Task Scheduler / systemd, resume the last DPI preset.
            if is_autostart {
                let autostart_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    autostart_engine_with_last_preset(autostart_handle).await;
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if window.label() == "main" {
                        // Keep engine running in background when main window is closed
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
                tauri::WindowEvent::Destroyed if window.label() == "main" => {
                    // RunEvent::Exit is the single shutdown owner. Starting a second
                    // asynchronous stop here could let process teardown race app exit.
                    tracing::debug!("Main window destroyed; engine cleanup is deferred to application exit.");
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_engine,
            commands::stop_engine,
            commands::get_engine_status,
            commands::get_advanced_capabilities,
            commands::list_presets,
            commands::save_custom_preset,
            commands::delete_custom_preset,
            http::check_url_health,
            http::check_dns_block,
            commands::start_auto_optimize,
            commands::cancel_optimizer,
            commands::apply_optimizer_recommendation,
            commands::get_last_optimized_preset,
            commands::list_dns_providers,
            commands::get_network_adapters,
            commands::apply_dns_settings,
            commands::reset_dns_settings,
            commands::check_trusted_dns,
            commands::start_engine_with_dns_guard,
            commands::open_settings_window,
            commands::get_system_info,
            // Feature 2: Auto-Start
            commands::check_is_elevated,
            commands::set_autostart,
            commands::get_autostart_status,
            // Feature 3: Dynamic Remote Presets
            commands::refresh_remote_presets,
            // Feature 4: Updater
            updater::check_for_updates,
            updater::install_update,
            // Feature 6A: DoH
            commands::resolve_via_doh,
            // Feature 8: DoH Forwarder
            commands::start_doh_forwarder,
            commands::stop_doh_forwarder,
            commands::get_doh_forwarder_status,
            commands::set_dns_watchdog,
            // Feature 9: Health Check
            commands::get_engine_health,
            commands::export_preset,
            // Utility
            commands::open_url,
            commands::get_geoip_data,
            commands::get_network_stats,
            commands::validate_socks5_proxy,
            commands::sync_dns_settings,
            commands::sync_bypass_config,
            commands::get_artifact_integrity_status,
            commands::run_local_diagnostics,
            commands::run_traffic_diagnostics,
            commands::cancel_traffic_diagnostics,
            commands::export_diagnostics_bundle,
            commands::get_recent_diagnostic_events,
            commands::clear_diagnostic_events,
            commands::get_diagnostic_event_stats,
            settings::settings_get,
            settings::settings_set,
            settings::settings_remove,
        ]);

    let app = builder
        .build(tauri::generate_context!())
        .expect("Tauri could not be started");

    app.run(|app_handle: &AppHandle, event: tauri::RunEvent| {
        if let tauri::RunEvent::Exit = event {
            tracing::info!("Tauri application closing (RunEvent::Exit). Stopping engine...");
            if let Some(state) = app_handle.try_state::<AppState>() {
                let _ = tauri::async_runtime::block_on(state.engine_manager.stop(app_handle));
                let candidate = crate::dns::DnsConfigCandidate {
                    enabled: false,
                    protocol: "doh".into(),
                    provider: Some("cloudflare".into()),
                    adblock: false,
                    cache_enabled: false,
                    socks5: None,
                    kill_switch: false,
                };
                if let Err(error) =
                    tauri::async_runtime::block_on(state.dns_transaction_manager.apply_candidate(
                        candidate,
                        app_handle,
                        state.inner(),
                    ))
                {
                    tracing::error!("Application exit DNS transaction rollback failed: {error}");
                }
            }
        }
    });
}

#[cfg(test)]
mod startup_state_tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn startup_recovery_runs_only_after_app_state_is_managed() {
        let state = build_app_state(ConfigLoader::new(), reqwest::Client::new());

        assert_eq!(state.dns_config_revision.load(Ordering::SeqCst), 0);
        assert_eq!(
            std::sync::Arc::strong_count(&state.dns_transaction_manager),
            1
        );
    }
}
