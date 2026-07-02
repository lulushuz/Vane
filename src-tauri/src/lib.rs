use std::sync::Mutex;
use std::time::Duration;
use log::LevelFilter;
use tauri::{AppHandle, Manager, Emitter, Listener};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// Constant for CREATE_NO_WINDOW flag on Windows to prevent console window flashing.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub mod config;
pub mod engine;
pub mod privilege;
pub mod http;
pub mod dns;
pub mod autostart;
pub mod presets;
pub mod updater;
pub mod network;
pub mod tray;
pub mod commands;
pub mod logging;

use crate::config::loader::ConfigLoader;
use crate::engine::EngineManager;
use crate::dns::ForwarderHandle;
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
}





/* 
   Cleans up dangling winws instances from previous sessions during initialization.
   Prevents zombie processes if the app previously crashed or was forcefully closed. 
*/
#[cfg(target_os = "windows")]
fn kill_existing_winws() {
    tracing::info!("Startup cleanup: Searching for existing winws process...");
    let result = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "winws-x86_64-pc-windows-msvc.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match result {
        Ok(out) if out.status.success() => {
            tracing::info!("Zombie winws process terminated.");
        }
        Ok(_) => {
            tracing::debug!("No winws process found to clean up (normal).");
        }
        Err(e) => {
            tracing::warn!("taskkill could not be run: {}", e);
        }
    }
}

#[cfg(target_os = "windows")]
fn cleanup_stale_windivert() {
    tracing::info!("Startup cleanup: Checking for stale WinDivert services...");
    let result = std::process::Command::new("sc")
        .args(["query", "WinDivert"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();

    match result {
        Ok(out) if out.status.success() => {
            let output_str = String::from_utf8_lossy(&out.stdout);
            // If it exists but is not running correctly, wiping it forces a clean reinstall.
            if output_str.contains("STOPPED") || output_str.contains("START_PENDING") || output_str.contains("STOP_PENDING") {
                tracing::info!("Stale WinDivert service found. Disposing...");
                let _ = std::process::Command::new("sc")
                    .args(["delete", "WinDivert"])
                    .creation_flags(0x08000000)
                    .output();
            }
        }
        _ => tracing::debug!("No stale WinDivert service found (normal)."),
    }
}

/// Reads the last active preset ID from the persisted Zustand store file safely with retries.
fn read_last_preset_id(app: &AppHandle) -> Option<String> {
    let app_data = app.path().app_data_dir().ok()?;
    let settings_path = app_data.join("settings.json");
    if !settings_path.exists() {
        return None;
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
        return None;
    }

    let file_json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let zustand_raw = file_json.get("vane-settings")?;
    let zustand_json: serde_json::Value = match zustand_raw {
        serde_json::Value::String(s) => serde_json::from_str(s).ok()?,
        obj => obj.clone(),
    };

    zustand_json
        .get("state")?
        .get("activePresetId")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Automatically resumes DPI bypass after an --autostart launch.
/// Reads the persisted preset and starts the engine silently.
async fn autostart_engine_with_last_preset(app: AppHandle) {
    // Short delay so AppState is guaranteed to be registered via app.manage().
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    let Some(state) = app.try_state::<AppState>() else {
        tracing::warn!("Auto-start: AppState henüz hazır değil, atlanıyor.");
        return;
    };

    let Some(preset_id) = read_last_preset_id(&app) else {
        tracing::info!("Auto-start: Saved preset not found, engine not starting.");
        return;
    };

    let preset = {
        let Ok(loader) = state.config_loader.lock() else { return };
        loader.find_preset(&preset_id)
    };

    // Apply automatic DNS configuration check if not already using a trusted DNS
    let dns_ok = crate::dns::is_using_trusted_dns();
    if !dns_ok {
        let result = crate::dns::apply_dns("1.1.1.1", "1.0.0.1");
        if result.success {
            tracing::info!("DNS Guard: Cloudflare automatically applied during autostart.");
            let _ = app.emit("dns_auto_applied", "Cloudflare DNS (1.1.1.1) automatically applied.");
        } else {
            tracing::warn!("DNS Guard: Cloudflare could not be applied during autostart — {:?}", result.error);
        }
    }

    match preset {
        Some(p) => {
            tracing::info!("Auto-start: '{}' preset'i otomatik devreye alınıyor.", p.label);
            if let Err(e) = state.engine_manager.start(&p, &app).await {
                tracing::error!("Auto-start: Engine could not be started: {}", e);
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
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
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
            #[cfg(target_os = "windows")]
            cleanup_stale_windivert();

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

            let mut loader = ConfigLoader::new();
            if let Ok(app_data) = app.path().app_data_dir() {
                let presets_path = app_data.join("presets");
                let _ = std::fs::create_dir_all(&presets_path);
                loader.load_custom_presets_from(&presets_path);

                // ─── Feature 3: Cache-First Remote Presets ─────────────────
                // Phase 1: Load from disk immediately (synchronous, zero network I/O).
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
                    // TLS init may fail on FIPS-mode Windows or missing root certs.
                    // Fall back to a plain client — functionality degrades gracefully.
                    tracing::warn!("HTTP Client TLS initialization failed, using fallback: {}", e);
                    reqwest::Client::new()
                });

            // Phase 2: Fetch remote presets in background (non-blocking).
            // If offline, the cached presets loaded above remain active.
            if let Ok(app_data) = app.path().app_data_dir() {
                let fetch_client = http_client.clone();
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

            // Global singleton HTTP client for pooled request reusing
            app.manage(AppState {
                engine_manager: EngineManager::new(),
                config_loader: Mutex::new(loader),
                http_client,
                forwarder: Mutex::new(None),
            });

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
                tauri::WindowEvent::Destroyed => {
                    if window.label() == "main" {
                        if let Some(state) = window.try_state::<AppState>() {
                            tracing::info!("Main window is being destroyed, engine is stopping.");
                            let _ = state.engine_manager.stop(window.app_handle());
                        }
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_engine,
            commands::stop_engine,
            commands::get_engine_status,
            commands::list_presets,
            commands::save_custom_preset,
            commands::delete_custom_preset,
            http::check_url_health,
            http::check_dns_block,
            commands::start_auto_optimize,
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
            // Feature 9: Health Check
            commands::get_engine_health,
            commands::export_preset,
            // Utility
            commands::open_url,
            commands::get_geoip_data,
            commands::get_network_stats,
            commands::sync_dns_settings,
            commands::sync_bypass_config,
        ]);

    let app = builder
        .build(tauri::generate_context!())
        .expect("Tauri could not be started");

    app.run(|app_handle: &AppHandle, event: tauri::RunEvent| {
        if let tauri::RunEvent::Exit = event {
            tracing::info!("Tauri application closing (RunEvent::Exit). Stopping engine...");
            if let Some(state) = app_handle.try_state::<AppState>() {
                let _ = state.engine_manager.stop(app_handle);
            }
        }
    });
}