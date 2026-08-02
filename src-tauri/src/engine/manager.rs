use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

use crate::config::preset::Preset;
use crate::engine::sanitizer::validate_preset_args;
use crate::engine::{error::EngineError, process::ProcessHandle};
#[cfg(target_os = "windows")]
use crate::privilege::checker::is_elevated;

#[cfg(target_os = "windows")]
use crate::engine::job::JobObjectGuard;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const AUTOMATIC_RESTART_ENABLED: bool = false;

// Enum representing engine status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "variant", rename_all = "camelCase")]
pub enum EngineStatus {
    Stopped,
    Starting,
    WaitingForReadiness {
        pid: u32,
    },
    Ready {
        pid: u32,
        generation: u64,
        revision: u64,
        fingerprint: String,
    },
    /// Compatibility-only process-spawned state; it must not be interpreted as ready.
    Running {
        pid: u32,
    },
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },
}

pub trait EngineEventDispatcher: Send + Sync {
    fn emit_log_batch(&self, batch: Vec<String>);
    fn emit_status(&self, status: &EngineStatus);
    fn resolve_path(
        &self,
        path: &str,
        base: tauri::path::BaseDirectory,
    ) -> Result<std::path::PathBuf, tauri::Error>;
    fn clone_app_handle(&self) -> AppHandle;
}

impl EngineEventDispatcher for AppHandle {
    fn emit_log_batch(&self, batch: Vec<String>) {
        let _ = self.emit("log_batch", batch);
    }

    fn emit_status(&self, status: &EngineStatus) {
        let _ = self.emit("engine_status", status);
    }

    fn resolve_path(
        &self,
        path: &str,
        base: tauri::path::BaseDirectory,
    ) -> Result<std::path::PathBuf, tauri::Error> {
        self.path().resolve(path, base)
    }

    fn clone_app_handle(&self) -> AppHandle {
        self.clone()
    }
}

#[derive(Debug)]
pub enum EngineState {
    Idle,
    Starting {
        generation: u64,
        cancel: tokio::sync::oneshot::Sender<()>,
    },
    Running {
        handle: Box<ProcessHandle>,
    },
    Stopping,
    Failed(EngineError),
}

#[derive(Clone)]
pub struct EngineManager {
    status: Arc<Mutex<EngineStatus>>,
    state: Arc<Mutex<EngineState>>,
    generation: Arc<AtomicU64>,
    runtime_config_state: Arc<Mutex<crate::engine::runtime_state::RuntimeConfigState>>,
    pattern_transaction_lock: Arc<tokio::sync::Mutex<()>>,
}

impl Default for EngineManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineManager {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(EngineStatus::Stopped)),
            state: Arc::new(Mutex::new(EngineState::Idle)),
            generation: Arc::new(AtomicU64::new(0)),
            runtime_config_state: Arc::new(Mutex::new(
                crate::engine::runtime_state::RuntimeConfigState::new(
                    crate::engine::runtime_config::ConfigRevision::new(1),
                ),
            )),
            pattern_transaction_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub(crate) fn runtime_config_state(
        &self,
    ) -> Arc<Mutex<crate::engine::runtime_state::RuntimeConfigState>> {
        self.runtime_config_state.clone()
    }

    pub(crate) fn pattern_transaction_lock(&self) -> Arc<tokio::sync::Mutex<()>> {
        self.pattern_transaction_lock.clone()
    }

    pub(crate) fn desired_config(
        &self,
    ) -> Option<crate::engine::runtime_config::VerifiedRuntimeConfig> {
        self.runtime_config_state.lock().ok()?.desired().cloned()
    }

    pub(crate) fn applied_config(
        &self,
    ) -> Option<crate::engine::runtime_config::AppliedRuntimeConfig> {
        self.runtime_config_state.lock().ok()?.applied().cloned()
    }

    #[allow(dead_code)]
    fn verify_binary_hash(path: &std::path::Path, expected_hex: &str) -> Result<(), EngineError> {
        use sha2::{Digest, Sha256};
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)
            .map_err(|e| EngineError::IoError(format!("Dosya açılamadı: {}", e)))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|e| EngineError::IoError(format!("Dosya okunamadı: {}", e)))?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let result = hasher.finalize();
        let hex_result = format!("{:x}", result);
        if hex_result.eq_ignore_ascii_case(expected_hex) {
            Ok(())
        } else {
            Err(EngineError::IoError(format!(
                "Bütünlük İhlali: SHA-256 imzası uyuşmuyor! Beklenen: {}, Bulunan: {}",
                expected_hex, hex_result
            )))
        }
    }

    // Safely resolves binary path from Resource using binary integrity verifier
    fn resolve_binary_path(
        dispatcher: &impl EngineEventDispatcher,
    ) -> Result<std::path::PathBuf, EngineError> {
        use crate::security::{ArtifactIntegrityVerifier, Sha256ArtifactIntegrityVerifier};

        let resource_root = dispatcher
            .resolve_path("", tauri::path::BaseDirectory::Resource)
            .map_err(|e| EngineError::BinaryNotFound(format!("Tauri Path resolve error: {e}")))?;

        let verifier = Sha256ArtifactIntegrityVerifier::from_embedded()?;
        let verified_group = verifier.verify_current_platform_group(&resource_root)?;

        Ok(verified_group.executable.canonical_path)
    }

    #[allow(dead_code)]
    pub(crate) fn prepare_args(preset_args: &[String]) -> Vec<String> {
        let dummy_preset = Preset {
            id: "dummy".to_string(),
            label: "Dummy".to_string(),
            description: String::new(),
            icon: String::new(),
            args: preset_args.to_vec(),
            is_custom: false,
            priority: 0,
            category: Default::default(),
        };
        let input = crate::engine::launch_plan::EngineLaunchInput {
            preset: &dummy_preset,
            platform: crate::engine::launch_plan::EnginePlatform::current(),
            executable: std::path::PathBuf::from(if cfg!(target_os = "windows") {
                "C:\\dummy\\winws.exe"
            } else {
                "/dummy/nfqws"
            }),
            bypass: crate::engine::launch_plan::LaunchBypassInput {
                mode: crate::engine::launch_plan::LaunchBypassMode::All,
                domain_list: String::new(),
                hostlist_path: None,
                kill_switch: false,
            },
        };
        crate::engine::launch_plan::build_engine_launch_plan(input)
            .map(|plan| plan.final_arguments)
            .unwrap_or_else(|_| preset_args.to_vec())
    }

    pub(crate) async fn start_prepared_config<D: EngineEventDispatcher + Clone + 'static>(
        &self,
        prepared: crate::engine::runtime_config::PreparedRuntimeConfig,
        dispatcher: &D,
    ) -> Result<crate::engine::runtime_config::AppliedRuntimeConfig, EngineError> {
        #[cfg(target_os = "windows")]
        if !is_elevated() {
            return Err(EngineError::InsufficientPrivileges);
        }

        validate_preset_args(&prepared.verified.preset.arguments)?;

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let rx = {
            let mut state_lock = self
                .state
                .lock()
                .map_err(|_| EngineError::IoError("State lock poisoned".into()))?;
            match &*state_lock {
                EngineState::Running { .. }
                | EngineState::Starting { .. }
                | EngineState::Stopping => {
                    return Err(EngineError::AlreadyRunning);
                }
                _ => {}
            }
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            *state_lock = EngineState::Starting {
                generation,
                cancel: tx,
            };
            self.set_status(EngineStatus::Starting, dispatcher);
            rx
        };

        let app_handle = dispatcher.clone_app_handle();
        let preset_clone = prepared.verified.preset.to_preset();
        let state_clone = self.state.clone();
        let status_clone = self.status.clone();
        let dispatcher_clone = dispatcher.clone();

        let handle_res = spawn_and_run_prepared(&prepared, generation, &app_handle, rx).await;

        match handle_res {
            Ok((mut handle, applied)) => {
                let pid = handle.pid();
                self.set_status(EngineStatus::WaitingForReadiness { pid }, &dispatcher_clone);
                tokio::time::sleep(crate::engine::lifecycle::ENGINE_STARTUP_GRACE_PERIOD).await;
                let config_matches = applied.verified.revision == prepared.verified.revision
                    && applied.verified.fingerprint == prepared.verified.fingerprint;
                if !is_process_alive(pid) || !config_matches {
                    let _ = handle.terminate().await;
                    let error = EngineError::SpawnFailed(
                        "Owned process failed readiness or exact-config verification".into(),
                    );
                    let _ = set_state_failed_if_starting(&state_clone, generation, error.clone());
                    self.set_status(
                        EngineStatus::Error {
                            message: error.to_string(),
                            code: Some("ENGINE_READINESS_FAILED".into()),
                        },
                        &dispatcher_clone,
                    );
                    return Err(error);
                }
                if !set_state_running_if_starting(&state_clone, generation, handle) {
                    tracing::info!(generation, pid, "Stale engine start result was discarded.");
                    return Err(EngineError::NotRunning);
                }

                self.set_status(
                    EngineStatus::Ready {
                        pid,
                        generation,
                        revision: prepared.verified.revision.get(),
                        fingerprint: prepared.verified.fingerprint.to_string(),
                    },
                    &dispatcher_clone,
                );
                tracing::info!(
                    "Engine started: preset='{}', pid={}, revision={}",
                    prepared.verified.preset.id,
                    pid,
                    prepared.verified.revision.get()
                );

                let generation_counter = self.generation.clone();
                tokio::spawn(async move {
                    watch_process(
                        pid,
                        app_handle,
                        state_clone,
                        status_clone,
                        preset_clone,
                        generation_counter,
                    )
                    .await;
                });

                Ok(applied)
            }
            Err(e) => {
                if !set_state_failed_if_starting(&state_clone, generation, e.clone()) {
                    tracing::info!(generation, "Stale engine start failure was discarded.");
                    return Err(EngineError::NotRunning);
                }
                self.set_status(
                    EngineStatus::Error {
                        message: e.to_string(),
                        code: None,
                    },
                    &dispatcher_clone,
                );
                Err(e)
            }
        }
    }

    pub async fn start<D: EngineEventDispatcher + Clone + 'static>(
        &self,
        preset: &Preset,
        dispatcher: &D,
    ) -> Result<(), EngineError> {
        let app_handle = dispatcher.clone_app_handle();
        let desired = self.desired_config();
        let prepared = if let Some(d) = desired {
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let (prep, _) =
                crate::engine::pattern_transaction::prepare_runtime_config_for_transaction(
                    d,
                    &app_data_dir,
                )
                .map_err(|e| EngineError::ConfigParseError(e.to_string()))?;
            prep
        } else {
            let bypass_config = read_bypass_config(&app_handle)?;
            bypass_config.validate_for_start()?;
            let candidate = crate::engine::runtime_config::candidate_from_preset_and_sources(
                preset,
                &bypass_config.mode,
                &bypass_config.domain_list,
                bypass_config.kill_switch,
            );
            let revision = crate::engine::runtime_config::ConfigRevision::new(1);
            let verified =
                crate::engine::runtime_config::verify_runtime_config(candidate, revision)?;
            self.runtime_config_state()
                .lock()
                .unwrap()
                .set_desired(verified.clone());
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let (prep, _) =
                crate::engine::pattern_transaction::prepare_runtime_config_for_transaction(
                    verified,
                    &app_data_dir,
                )
                .map_err(|e| EngineError::ConfigParseError(e.to_string()))?;
            prep
        };

        let applied = self.start_prepared_config(prepared, dispatcher).await?;
        let _ = self
            .runtime_config_state()
            .lock()
            .unwrap()
            .commit_applied(applied);
        Ok(())
    }

    pub async fn stop(&self, dispatcher: &impl EngineEventDispatcher) -> Result<(), EngineError> {
        let mut handle = {
            let mut state = self.state.lock().map_err(|_| {
                tracing::error!("State lock poisoned (stop phase).");
                self.set_status(
                    EngineStatus::Error {
                        message: "Internal Error: State poisoned".into(),
                        code: None,
                    },
                    dispatcher,
                );
                EngineError::IoError("State lock poisoned".into())
            })?;

            match std::mem::replace(&mut *state, EngineState::Stopping) {
                EngineState::Idle | EngineState::Failed(_) => {
                    *state = EngineState::Idle;
                    self.set_status(EngineStatus::Stopped, dispatcher);
                    return Ok(());
                }
                EngineState::Stopping => {
                    *state = EngineState::Stopping;
                    return Ok(());
                }
                EngineState::Starting { cancel, .. } => {
                    let _ = cancel.send(());
                    *state = EngineState::Idle;
                    self.set_status(EngineStatus::Stopped, dispatcher);
                    tracing::info!("Engine startup cancelled.");
                    return Ok(());
                }
                EngineState::Running { handle } => handle,
            }
        };

        let termination = handle.terminate().await;
        let mut state = self
            .state
            .lock()
            .map_err(|_| EngineError::IoError("State lock poisoned after kill".into()))?;
        match termination {
            Ok(()) => {
                *state = EngineState::Idle;
                self.set_status(EngineStatus::Stopped, dispatcher);
                tracing::info!("Engine stopped.");
                Ok(())
            }
            Err(error) => {
                *state = EngineState::Failed(error.clone());
                self.set_status(
                    EngineStatus::Error {
                        message: error.to_string(),
                        code: Some(error.code().into()),
                    },
                    dispatcher,
                );
                Err(error)
            }
        }
    }

    pub fn current_status(&self) -> EngineStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(EngineStatus::Stopped)
    }

    pub(crate) fn current_generation(&self) -> crate::engine::owned_process::EngineGeneration {
        crate::engine::owned_process::EngineGeneration::new(self.generation.load(Ordering::SeqCst))
    }

    fn set_status(&self, new_status: EngineStatus, dispatcher: &impl EngineEventDispatcher) {
        if let Ok(mut guard) = self.status.lock() {
            *guard = new_status.clone();
        } else {
            tracing::error!("Status lock poisoned. Status cannot be updated.");
        }
        dispatcher.emit_status(&new_status);
    }
}

use std::sync::RwLock;

#[derive(Clone, Debug)]
struct BypassConfig {
    mode: String,
    domain_list: String,
    _proxy: String,
    kill_switch: bool,
}

impl BypassConfig {
    fn all_sites() -> Self {
        Self {
            mode: "all".to_string(),
            domain_list: String::new(),
            _proxy: String::new(),
            kill_switch: false,
        }
    }

    fn validate_for_start(&self) -> Result<(), EngineError> {
        if self.mode == "whitelist" && self.domain_list.is_empty() {
            return Err(EngineError::ConfigParseError(
                "Whitelist mode is selected, but the whitelist has no valid domains. DPI bypass was not started."
                    .to_string(),
            ));
        }
        Ok(())
    }
}

static BYPASS_CONFIG_CACHE: RwLock<Option<BypassConfig>> = RwLock::new(None);
static KILL_SWITCH_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn update_bypass_config_cache(mode: String, list: String, proxy: String, kill_switch: bool) {
    KILL_SWITCH_ENABLED.store(kill_switch, Ordering::SeqCst);
    if let Ok(mut guard) = BYPASS_CONFIG_CACHE.write() {
        *guard = Some(BypassConfig {
            mode,
            domain_list: list,
            _proxy: proxy,
            kill_switch,
        });
    }
}

pub fn invalidate_bypass_config_cache() {
    if let Ok(mut guard) = BYPASS_CONFIG_CACHE.write() {
        *guard = None;
    }
}

pub(crate) fn kill_switch_enabled() -> bool {
    KILL_SWITCH_ENABLED.load(Ordering::SeqCst)
}

fn read_bypass_config(app: &AppHandle) -> Result<BypassConfig, EngineError> {
    let Some(settings) =
        crate::settings::read_runtime_settings(app).map_err(EngineError::ConfigParseError)?
    else {
        return Ok(BypassConfig::all_sites());
    };
    if !matches!(
        settings.bypass_mode.as_str(),
        "all" | "whitelist" | "blacklist"
    ) {
        return Err(EngineError::ConfigParseError(format!(
            "Unsupported persisted bypass mode: {}",
            settings.bypass_mode
        )));
    }
    let raw_domains = if settings.bypass_mode == "whitelist" {
        settings.whitelist_domains
    } else {
        settings.blacklist_domains
    };
    let domains =
        crate::config::domain::canonicalize_domain_rules(&raw_domains).map_err(|error| {
            EngineError::ConfigParseError(format!("Persisted bypass domain is invalid: {error}"))
        })?;
    let config = BypassConfig {
        mode: settings.bypass_mode,
        domain_list: domains.join("\n"),
        _proxy: settings.proxy_socks5,
        kill_switch: settings.kill_switch,
    };
    Ok(config)
}

#[cfg(test)]
fn parse_bypass_config(content: &str) -> Result<BypassConfig, EngineError> {
    let file_json = serde_json::from_str::<serde_json::Value>(content).map_err(|error| {
        EngineError::ConfigParseError(format!("Settings JSON is invalid: {error}"))
    })?;
    let zustand_raw = file_json.get("vane-settings").ok_or_else(|| {
        EngineError::ConfigParseError("The persisted Vane settings entry is missing.".to_string())
    })?;
    let zustand_json = match zustand_raw {
        serde_json::Value::String(value) => serde_json::from_str::<serde_json::Value>(value)
            .map_err(|error| {
                EngineError::ConfigParseError(format!(
                    "Persisted Vane settings are invalid: {error}"
                ))
            })?,
        object => object.clone(),
    };
    let state = zustand_json.get("state").ok_or_else(|| {
        EngineError::ConfigParseError("The persisted Vane settings state is missing.".to_string())
    })?;
    let mode = state
        .get("bypassMode")
        .and_then(|value| value.as_str())
        .unwrap_or("all")
        .to_string();
    if !matches!(mode.as_str(), "all" | "whitelist" | "blacklist") {
        return Err(EngineError::ConfigParseError(format!(
            "Unsupported persisted bypass mode: {mode}"
        )));
    }

    let array_key = if mode == "whitelist" {
        "whitelistDomains"
    } else {
        "blacklistDomains"
    };
    let raw_domains: Vec<String> = state
        .get(array_key)
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let domains =
        crate::config::domain::canonicalize_domain_rules(&raw_domains).map_err(|error| {
            EngineError::ConfigParseError(format!("Persisted bypass domain is invalid: {error}"))
        })?;

    Ok(BypassConfig {
        mode,
        domain_list: domains.join("\n"),
        _proxy: state
            .get("proxySocks5")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
        kill_switch: state
            .get("killSwitch")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod bypass_config_tests {
    use super::parse_bypass_config;

    #[test]
    fn corrupt_settings_are_rejected_instead_of_falling_back_to_all() {
        assert!(parse_bypass_config("{not-json").is_err());
    }

    #[test]
    fn whitelist_is_built_from_verified_array_not_legacy_domain_list() {
        let settings = serde_json::json!({
            "vane-settings": serde_json::json!({
                "state": {
                    "bypassMode": "whitelist",
                    "domainList": "attacker.example",
                    "whitelistDomains": [" Roblox.COM. "],
                    "blacklistDomains": [],
                    "proxySocks5": "",
                    "killSwitch": false
                }
            }).to_string()
        });
        let config = parse_bypass_config(&settings.to_string()).expect("valid settings");
        assert_eq!(config.domain_list, "roblox.com");
        assert!(config.validate_for_start().is_ok());
    }

    #[test]
    fn empty_whitelist_is_fail_closed() {
        let settings = serde_json::json!({
            "vane-settings": serde_json::json!({
                "state": {
                    "bypassMode": "whitelist",
                    "whitelistDomains": [],
                    "blacklistDomains": []
                }
            }).to_string()
        });
        let config = parse_bypass_config(&settings.to_string()).expect("valid settings");
        assert!(config.validate_for_start().is_err());
    }
}

async fn spawn_and_run_prepared(
    prepared: &crate::engine::runtime_config::PreparedRuntimeConfig,
    _generation: u64,
    app: &AppHandle,
    _cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<
    (
        ProcessHandle,
        crate::engine::runtime_config::AppliedRuntimeConfig,
    ),
    EngineError,
> {
    let winws_path = EngineManager::resolve_binary_path(app)?;
    #[cfg(target_os = "windows")]
    let working_dir = winws_path.parent().ok_or_else(|| {
        EngineError::BinaryNotFound(format!(
            "Binary path'in parent klasörü alınamadı: {:?}",
            winws_path
        ))
    })?;

    let prepared_args = prepared.launch_plan.final_arguments.clone();
    let kill_switch = prepared.verified.bypass.kill_switch;
    tracing::info!(
        "Prepared runtime config spawning: revision={}, fingerprint={}",
        prepared.verified.revision.get(),
        prepared.verified.fingerprint.prefix(8)
    );

    if kill_switch {
        let forwarder_active = app
            .try_state::<crate::AppState>()
            .and_then(|state| state.forwarder.lock().ok().map(|guard| guard.is_some()))
            .unwrap_or(false);
        if !forwarder_active {
            return Err(EngineError::ConfigParseError(
                "DNS Kill Switch requires the encrypted local DNS forwarder to be running.".into(),
            ));
        }
    }

    #[cfg(target_os = "linux")]
    let mut cancel_rx = _cancel_rx;

    #[cfg(target_os = "linux")]
    {
        let installation_id = crate::dns::get_or_create_installation_id(app);
        let instance_id = format!(
            "engine-{}-{}",
            std::process::id(),
            prepared.verified.fingerprint.prefix(8)
        );
        let intent = crate::platform::linux::LinuxFilterIntent::from_specs(
            prepared
                .launch_plan
                .traffic_filter
                .effective_linux_tcp_spec
                .as_deref(),
            prepared
                .launch_plan
                .traffic_filter
                .effective_linux_udp_spec
                .as_deref(),
            crate::platform::linux::LinuxHostlistMode::from(prepared.verified.bypass.mode),
        );
        let filter_guard = crate::platform::linux::LinuxFilterGuard::apply(
            app,
            intent,
            &installation_id,
            &instance_id,
            _generation,
            prepared.verified.revision.get(),
            &prepared.verified.fingerprint.to_string(),
        )
        .map_err(|error| EngineError::AuthorizationFailed(error.to_string()))?;
        if !filter_guard.verify_owned() {
            return Err(EngineError::AuthorizationFailed(
                "Linux filter ownership verification failed".into(),
            ));
        }
        let mut args = prepared_args
            .iter()
            .filter(|arg| !arg.starts_with("--qnum="))
            .cloned()
            .collect::<Vec<_>>();
        args.insert(0, format!("--qnum={}", filter_guard.queue_number()));
        let mut command = tokio::process::Command::new(&winws_path);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|error| EngineError::SpawnFailed(error.to_string()))?;
        let pid = child
            .id()
            .ok_or_else(|| EngineError::SpawnFailed("Linux engine PID unavailable".into()))?;
        tokio::select! {
            _ = tokio::time::sleep(crate::engine::lifecycle::ENGINE_STARTUP_GRACE_PERIOD) => {}
            _ = &mut cancel_rx => {
                let _ = child.start_kill();
                return Err(EngineError::NotRunning);
            }
        }
        if child
            .try_wait()
            .map_err(|error| EngineError::IoError(error.to_string()))?
            .is_some()
        {
            return Err(EngineError::SpawnFailed(
                "Linux engine exited before readiness".into(),
            ));
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::IoError("stdout pipe unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| EngineError::IoError("stderr pipe unavailable".into()))?;
        crate::engine::logger::spawn_log_reader(stdout, app.clone(), None);
        crate::engine::logger::spawn_log_reader(stderr, app.clone(), Some("ERROR: "));
        let handle = ProcessHandle::new(child, pid, None, Some(filter_guard));
        let applied = crate::engine::runtime_config::AppliedRuntimeConfig::process_started(
            prepared.verified.clone(),
            pid,
        );
        return Ok((handle, applied));
    }

    #[cfg(all(target_os = "linux", any()))]
    {
        let mut escaped_args = Vec::new();
        for arg in &prepared_args {
            let escaped = arg.replace('\'', "'\\''");
            escaped_args.push(format!("'{}'", escaped));
        }
        let args_str = escaped_args.join(" ");
        let binary_path_escaped = winws_path.to_string_lossy().replace('\'', "'\\''");
        let binary_path_str = format!("'{}'", binary_path_escaped);

        let script = format!("{} {}", binary_path_str, args_str);

        let can_run_directly = {
            let uid_output = std::process::Command::new("id").arg("-u").output();
            let is_root = match uid_output {
                Ok(out) => {
                    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    s == "0"
                }
                _ => false,
            };
            is_root
                || std::process::Command::new("iptables")
                    .args(["-t", "mangle", "-L"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                || std::process::Command::new("nft")
                    .args(["list", "tables"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
        };

        let mut root_cmd = if can_run_directly {
            std::process::Command::new("sh")
        } else {
            let mut cmd = std::process::Command::new("pkexec");
            cmd.arg("sh");
            cmd
        };

        root_cmd
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = tokio::process::Command::from(root_cmd)
            .spawn()
            .map_err(|e| {
                EngineError::SpawnFailed(format!("Linux Root Wrapper could not be started: {}", e))
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::IoError("Stdout alınamadı".into()))?;
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();
        use tokio::io::AsyncBufReadExt;

        tokio::select! {
            res = reader.read_line(&mut line) => {
                match res {
                    Ok(n) if n > 0 && line.trim().starts_with("READY") => {
                        tracing::info!("Linux Root Wrapper aktif: {}", line.trim());
                    }
                    _ => {
                        let _ = child.start_kill();
                        return Err(EngineError::AuthorizationFailed("Authorization denied or script error.".into()));
                    }
                }
            }
            _ = &mut cancel_rx => {
                tracing::info!("Spawn cancelled during PolicyKit wait.");
                let _ = child.start_kill();
                return Err(EngineError::NotRunning);
            }
        }

        child.stdout = Some(reader.into_inner());
        let pid = child.id().unwrap_or(0);
        tracing::info!("Engine process spawned successfully, PID: {}", pid);

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::IoError("stdout pipe oluşturulamadı".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| EngineError::IoError("stderr pipe oluşturulamadı".into()))?;

        crate::engine::logger::spawn_log_reader(stdout, app.clone(), None);
        crate::engine::logger::spawn_log_reader(stderr, app.clone(), Some("HATA: "));

        let handle = ProcessHandle::new(child, pid, None);
        let applied = crate::engine::runtime_config::AppliedRuntimeConfig::process_started(
            prepared.verified.clone(),
            pid,
        );
        Ok((handle, applied))
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = std::process::Command::new(&winws_path);
        command
            .args(&prepared_args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);

        let mut child = tokio::process::Command::from(command)
            .spawn()
            .map_err(|e| {
                tracing::error!("Process could not be started: {}", e);
                EngineError::SpawnFailed(e.to_string())
            })?;

        let pid = child.id().unwrap_or(0);
        tracing::info!("Engine process spawned successfully, PID: {}", pid);

        let job_guard = match JobObjectGuard::new().and_then(|j| j.assign(pid).map(|_| j)) {
            Ok(j) => {
                tracing::info!("winws PID {} Job Object'a atandı.", pid);
                Some(j)
            }
            Err(e) => {
                tracing::error!("Job Object could not be assigned: {}", e);
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Err(EngineError::IoError(format!(
                    "Kernel-level process guard (Job Object) could not be created: {}",
                    e
                )));
            }
        };

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::IoError("stdout pipe oluşturulamadı".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| EngineError::IoError("stderr pipe oluşturulamadı".into()))?;

        crate::engine::logger::spawn_log_reader(stdout, app.clone(), None);
        crate::engine::logger::spawn_log_reader(stderr, app.clone(), Some("HATA: "));

        let handle = ProcessHandle::new(child, pid, job_guard);
        let applied = crate::engine::runtime_config::AppliedRuntimeConfig::process_started(
            prepared.verified.clone(),
            pid,
        );
        Ok((handle, applied))
    }
}

fn watch_process(
    mut pid: u32,
    app: AppHandle,
    state: Arc<Mutex<EngineState>>,
    status: Arc<Mutex<EngineStatus>>,
    preset: Preset,
    generation_counter: Arc<AtomicU64>,
) -> futures::future::BoxFuture<'static, ()> {
    use futures::FutureExt;
    async move {
        'monitor: loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            let active = is_state_running_with_pid(&state, pid);

            if !active {
                break;
            }

            if !is_process_alive(pid) {
                tracing::warn!("Engine process (PID {}) died unexpectedly.", pid);

                if !AUTOMATIC_RESTART_ENABLED {
                    let error = EngineError::IoError(
                        "Owned engine process exited unexpectedly; explicit restart required".into(),
                    );
                    if let Ok(mut guard) = state.lock() {
                        if matches!(&*guard, EngineState::Running { handle } if handle.pid() == pid) {
                            *guard = EngineState::Failed(error);
                        }
                    }
                    set_status_error(
                        &status,
                        &app,
                        "Engine process exited unexpectedly. Automatic restart is disabled.".into(),
                        Some("ENGINE_CRASH_OBSERVED".into()),
                    );
                    break;
                }

                let generation = generation_counter.fetch_add(1, Ordering::SeqCst) + 1;
                let (initial_cancel, initial_rx) = tokio::sync::oneshot::channel::<()>();
                if !set_state_starting_if_running_pid(&state, pid, generation, initial_cancel) {
                    break;
                }
                set_status_starting(&status, &app);

                let mut initial_rx = Some(initial_rx);
                let mut last_error = None;

                for (index, backoff_secs) in [1_u64, 2, 4, 8, 16].into_iter().enumerate() {
                    let attempt = index + 1;
                    tracing::info!(
                        "Attempting engine restart in {}s (attempt {}/5)...",
                        backoff_secs,
                        attempt
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;

                    let rx = if let Some(rx) = initial_rx.take() {
                        if !is_state_starting_generation(&state, generation) {
                            break 'monitor;
                        }
                        rx
                    } else {
                        let (cancel, rx) = tokio::sync::oneshot::channel::<()>();
                        if !replace_start_cancel_if_starting(&state, generation, cancel) {
                            break 'monitor;
                        }
                        rx
                    };

                    let app_data_dir = app
                        .path()
                        .app_data_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."));

                    let prepared_res = (|| -> Result<crate::engine::runtime_config::PreparedRuntimeConfig, EngineError> {
                        let bypass_config = read_bypass_config(&app)?;
                        bypass_config.validate_for_start()?;
                        let candidate = crate::engine::runtime_config::candidate_from_preset_and_sources(
                            &preset,
                            &bypass_config.mode,
                            &bypass_config.domain_list,
                            bypass_config.kill_switch,
                        );
                        let revision = crate::engine::runtime_config::ConfigRevision::new(1);
                        let verified = crate::engine::runtime_config::verify_runtime_config(candidate, revision)?;
                        let (prep, _) = crate::engine::pattern_transaction::prepare_runtime_config_for_transaction(verified, &app_data_dir)
                            .map_err(|e| EngineError::ConfigParseError(e.to_string()))?;
                        Ok(prep)
                    })();

                    let spawn_res = match prepared_res {
                        Ok(prep) => spawn_and_run_prepared(&prep, generation, &app, rx).await,
                        Err(e) => Err(e),
                    };

                    match spawn_res {
                        Ok((new_handle, _applied)) => {
                            let new_pid = new_handle.pid();
                            if !set_state_running_if_starting(&state, generation, new_handle) {
                                tracing::info!(
                                    generation,
                                    new_pid,
                                    "Stale automatic restart result was discarded."
                                );
                                break 'monitor;
                            }
                            set_status_running(&status, &app, new_pid);
                            tracing::info!("Engine successfully restarted, new PID: {}", new_pid);
                            pid = new_pid;
                            continue 'monitor;
                        }
                        Err(error) => {
                            tracing::error!(
                                "Engine restart attempt {}/5 failed: {}",
                                attempt,
                                error
                            );
                            last_error = Some(error);
                        }
                    }
                }

                let detail = last_error
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "unknown restart error".into());
                let error = EngineError::IoError(format!(
                    "Engine crashed and automatic recovery failed after 5 attempts: {detail}"
                ));
                if set_state_failed_if_starting(&state, generation, error) {
                    tracing::error!("Engine restart limit reached. Transitioning to Failed.");
                    set_status_error(
                        &status,
                        &app,
                        "Süreç çöktü ve yeniden başlatılamadı.".into(),
                        Some("CRASH_RESTART_FAILED".into()),
                    );
                }
                break;
            }
        }
    }
    .boxed()
}

// Non-async helper functions to perform Mutex operations and drop guards immediately
fn is_state_running_with_pid(state: &Mutex<EngineState>, pid: u32) -> bool {
    if let Ok(sl) = state.lock() {
        if let EngineState::Running { handle } = &*sl {
            return handle.pid() == pid;
        }
    }
    false
}

fn set_state_starting_if_running_pid(
    state: &Mutex<EngineState>,
    pid: u32,
    generation: u64,
    cancel: tokio::sync::oneshot::Sender<()>,
) -> bool {
    let Ok(mut state) = state.lock() else {
        return false;
    };
    if !matches!(&*state, EngineState::Running { handle } if handle.pid() == pid) {
        return false;
    }
    *state = EngineState::Starting { generation, cancel };
    true
}

fn set_state_running_if_starting(
    state: &Mutex<EngineState>,
    generation: u64,
    handle: ProcessHandle,
) -> bool {
    let Ok(mut state) = state.lock() else {
        return false;
    };
    if !matches!(&*state, EngineState::Starting { generation: active, .. } if *active == generation)
    {
        return false;
    }
    *state = EngineState::Running {
        handle: Box::new(handle),
    };
    true
}

fn is_state_starting_generation(state: &Mutex<EngineState>, generation: u64) -> bool {
    state
        .lock()
        .map(|state| {
            matches!(
                &*state,
                EngineState::Starting {
                    generation: active,
                    ..
                } if *active == generation
            )
        })
        .unwrap_or(false)
}

fn replace_start_cancel_if_starting(
    state: &Mutex<EngineState>,
    generation: u64,
    cancel: tokio::sync::oneshot::Sender<()>,
) -> bool {
    let Ok(mut state) = state.lock() else {
        return false;
    };
    if !matches!(&*state, EngineState::Starting { generation: active, .. } if *active == generation)
    {
        return false;
    }
    *state = EngineState::Starting { generation, cancel };
    true
}

fn set_state_failed_if_starting(
    state: &Mutex<EngineState>,
    generation: u64,
    error: EngineError,
) -> bool {
    let Ok(mut state) = state.lock() else {
        return false;
    };
    if !matches!(&*state, EngineState::Starting { generation: active, .. } if *active == generation)
    {
        return false;
    }
    *state = EngineState::Failed(error);
    true
}

fn set_status_error(
    status: &Mutex<EngineStatus>,
    app: &AppHandle,
    msg: String,
    code: Option<String>,
) {
    if let Ok(mut st) = status.lock() {
        *st = EngineStatus::Error { message: msg, code };
        let _ = app.emit("engine_status", &*st);
    }
}

fn set_status_starting(status: &Mutex<EngineStatus>, app: &AppHandle) {
    if let Ok(mut st) = status.lock() {
        *st = EngineStatus::Starting;
        let _ = app.emit("engine_status", &*st);
    }
}

fn set_status_running(status: &Mutex<EngineStatus>, app: &AppHandle, pid: u32) {
    if let Ok(mut st) = status.lock() {
        *st = EngineStatus::Running { pid };
        let _ = app.emit("engine_status", &*st);
    }
}

#[cfg(target_os = "windows")]
fn is_process_alive(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, FALSE};
    use windows::Win32::System::Threading::GetExitCodeProcess;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid) {
            Ok(h) => h,
            Err(_) => return false,
        };
        let mut exit_code = 0;
        let ok = GetExitCodeProcess(handle, &mut exit_code).is_ok();
        let _ = CloseHandle(handle);
        ok && exit_code == 259
    }
}

#[cfg(not(target_os = "windows"))]
fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[test]
    fn stale_start_failure_cannot_replace_newer_start() {
        let (cancel, _receiver) = tokio::sync::oneshot::channel();
        let state = Mutex::new(EngineState::Starting {
            generation: 2,
            cancel,
        });

        assert!(!set_state_failed_if_starting(
            &state,
            1,
            EngineError::SpawnFailed("stale".into()),
        ));
        assert!(matches!(
            &*state.lock().expect("state lock"),
            EngineState::Starting { generation: 2, .. }
        ));
    }

    #[test]
    fn current_start_failure_transitions_to_failed() {
        let (cancel, _receiver) = tokio::sync::oneshot::channel();
        let state = Mutex::new(EngineState::Starting {
            generation: 3,
            cancel,
        });

        assert!(set_state_failed_if_starting(
            &state,
            3,
            EngineError::SpawnFailed("current".into()),
        ));
        assert!(matches!(
            &*state.lock().expect("state lock"),
            EngineState::Failed(EngineError::SpawnFailed(message)) if message == "current"
        ));
    }

    #[test]
    fn recovery_attempt_refreshes_cancel_channel_for_current_generation() {
        let (initial_cancel, mut initial_receiver) = tokio::sync::oneshot::channel();
        let state = Mutex::new(EngineState::Starting {
            generation: 4,
            cancel: initial_cancel,
        });
        let (next_cancel, _next_receiver) = tokio::sync::oneshot::channel();

        assert!(replace_start_cancel_if_starting(&state, 4, next_cancel));
        assert_eq!(
            initial_receiver.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Closed)
        );
        assert!(is_state_starting_generation(&state, 4));
    }

    #[test]
    fn stale_recovery_attempt_cannot_replace_cancel_channel() {
        let (initial_cancel, _initial_receiver) = tokio::sync::oneshot::channel();
        let state = Mutex::new(EngineState::Starting {
            generation: 5,
            cancel: initial_cancel,
        });
        let (stale_cancel, _stale_receiver) = tokio::sync::oneshot::channel();

        assert!(!replace_start_cancel_if_starting(&state, 4, stale_cancel));
        assert!(is_state_starting_generation(&state, 5));
    }
}
