use std::sync::{Arc, Mutex};
use std::process::Stdio;
use tauri::{AppHandle, Manager, Emitter};

use crate::config::preset::Preset;
use crate::engine::{error::EngineError, process::ProcessHandle};
use crate::engine::sanitizer::validate_preset_args;
use crate::privilege::checker::is_elevated;

#[cfg(target_os = "windows")]
use crate::engine::job::JobObjectGuard;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// Enum representing engine status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "variant", rename_all = "camelCase")]
pub enum EngineStatus {
    Stopped,
    Starting,
    Running { pid: u32 },
    Error { 
        message: String, 
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String> 
    },
}

pub trait EngineEventDispatcher: Send + Sync {
    fn emit_log_batch(&self, batch: Vec<String>);
    fn emit_status(&self, status: &EngineStatus);
    fn resolve_path(&self, path: &str, base: tauri::path::BaseDirectory) -> Result<std::path::PathBuf, tauri::Error>;
    fn clone_app_handle(&self) -> AppHandle;
}

impl EngineEventDispatcher for AppHandle {
    fn emit_log_batch(&self, batch: Vec<String>) {
        let _ = self.emit("log_batch", batch);
    }

    fn emit_status(&self, status: &EngineStatus) {
        let _ = self.emit("engine_status", status);
    }
    
    fn resolve_path(&self, path: &str, base: tauri::path::BaseDirectory) -> Result<std::path::PathBuf, tauri::Error> {
        self.path().resolve(path, base)
    }

    fn clone_app_handle(&self) -> AppHandle {
        self.clone()
    }
}

#[derive(Debug)]
pub enum EngineState {
    Idle,
    Starting { cancel: tokio::sync::oneshot::Sender<()> },
    Running { handle: ProcessHandle },
    Stopping,
    Failed(EngineError),
}

pub struct EngineManager {
    status: Arc<Mutex<EngineStatus>>,
    state: Arc<Mutex<EngineState>>,
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
        }
    }

    fn verify_binary_hash(path: &std::path::Path, expected_hex: &str) -> Result<(), EngineError> {
        use sha2::{Sha256, Digest};
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path).map_err(|e| EngineError::IoError(format!("Dosya açılamadı: {}", e)))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let n = file.read(&mut buffer).map_err(|e| EngineError::IoError(format!("Dosya okunamadı: {}", e)))?;
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

    // Safely resolves binary path from Resource
    fn resolve_binary_path(dispatcher: &impl EngineEventDispatcher) -> Result<std::path::PathBuf, EngineError> {
        #[cfg(target_os = "windows")]
        {
            let path = dispatcher
                .resolve_path("binaries/winws-x86_64-pc-windows-msvc.exe", tauri::path::BaseDirectory::Resource)
                .map_err(|e| EngineError::BinaryNotFound(format!("Tauri Path resolve error: {}", e)))?;
            if !path.exists() {
                return Err(EngineError::BinaryNotFound(format!("winws.exe not found at: {}", path.display())));
            }
            Self::verify_binary_hash(&path, "2DA71E80878DC270AC83F5893ECBB841F9752A57F1DA8FF9325636B4346BC632")?;
            Ok(path)
        }

        #[cfg(target_os = "linux")]
        {
            let path = dispatcher
                .resolve_path("binaries/nfqws-x86_64-unknown-linux-gnu", tauri::path::BaseDirectory::Resource)
                .map_err(|e| EngineError::BinaryNotFound(format!("Tauri Path resolve error: {}", e)))?;
            if !path.exists() {
                return Err(EngineError::BinaryNotFound(format!("nfqws not found at: {}", path.display())));
            }
            Self::verify_binary_hash(&path, "8D3452CE0E0B9D9FED2A3A087B1CAECFD39A910B7A31B304078FCBED3EA0E33C")?;
            // Ensure executable permissions
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&path) {
                let mut perms = metadata.permissions();
                if perms.mode() & 0o111 != 0o111 {
                    perms.set_mode(0o755);
                    if let Err(e) = std::fs::set_permissions(&path, perms) {
                        tracing::warn!("Could not set executable permissions on nfqws: {}", e);
                        return Err(EngineError::IoError(format!("Could not set executable permissions on nfqws: {}", e)));
                    }
                }
            }
            Ok(path)
        }
    }

    fn prepare_args(preset_args: &[String]) -> Vec<String> {
        #[cfg(target_os = "windows")]
        {
            preset_args.to_vec()
        }
        #[cfg(target_os = "linux")]
        {
            let mut linux_args = Vec::new();
            linux_args.push("--qnum=200".to_string());
            for arg in preset_args {
                if arg.starts_with("--wf-") || arg.starts_with("--windivert") || arg.starts_with("tcp.") || arg.starts_with("udp.") || arg.starts_with("icmp.") {
                    continue;
                }
                linux_args.push(arg.clone());
            }
            linux_args
        }
    }

    pub async fn start<D: EngineEventDispatcher + Clone + 'static>(&self, preset: &Preset, dispatcher: &D) -> Result<(), EngineError> {
        #[cfg(target_os = "windows")]
        if !is_elevated() {
            return Err(EngineError::InsufficientPrivileges);
        }

        validate_preset_args(&preset.args)?;

        let rx = {
            let mut state_lock = self.state.lock()
                .map_err(|_| EngineError::IoError("State lock poisoned".into()))?;
            match &*state_lock {
                EngineState::Running { .. } | EngineState::Starting { .. } => {
                    return Err(EngineError::AlreadyRunning);
                }
                _ => {}
            }
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            *state_lock = EngineState::Starting { cancel: tx };
            self.set_status(EngineStatus::Starting, dispatcher);
            rx
        };

        let app_handle = dispatcher.clone_app_handle();
        let preset_clone = preset.clone();
        let state_clone = self.state.clone();
        let status_clone = self.status.clone();
        let dispatcher_clone = dispatcher.clone();

        let handle_res = spawn_and_run(&preset_clone, &app_handle, rx).await;

        match handle_res {
            Ok(handle) => {
                let pid = handle.pid();
                let cancelled = self.current_status() == EngineStatus::Stopped;

                if cancelled {
                    drop(handle); // Kill process safely
                    set_state_idle(&state_clone);
                    self.set_status(EngineStatus::Stopped, &dispatcher_clone);
                    return Err(EngineError::NotRunning);
                }

                set_state_running(&state_clone, handle);

                self.set_status(EngineStatus::Running { pid }, &dispatcher_clone);
                tracing::info!("Engine started: preset='{}', pid={}", preset.id, pid);

                // Spawn process watcher supervisor
                tokio::spawn(async move {
                    watch_process(pid, app_handle, state_clone, status_clone, preset_clone).await;
                });

                Ok(())
            }
            Err(e) => {
                set_state_failed(&state_clone, e.clone());
                self.set_status(EngineStatus::Error { message: e.to_string(), code: None }, &dispatcher_clone);
                Err(e)
            }
        }
    }

    pub fn stop(&self, dispatcher: &impl EngineEventDispatcher) -> Result<(), EngineError> {
        apply_kill_switch(false);

        let mut state_lock = self.state.lock()
            .map_err(|_| {
                tracing::error!("State lock poisoned (stop phase).");
                self.set_status(EngineStatus::Error { message: "Internal Error: State poisoned".into(), code: None }, dispatcher);
                EngineError::IoError("State lock poisoned".into())
            })?;

        match std::mem::replace(&mut *state_lock, EngineState::Stopping) {
            EngineState::Idle => {
                *state_lock = EngineState::Idle;
                Err(EngineError::NotRunning)
            }
            EngineState::Stopping => {
                *state_lock = EngineState::Stopping;
                Ok(())
            }
            EngineState::Starting { cancel } => {
                let _ = cancel.send(());
                *state_lock = EngineState::Idle;
                self.set_status(EngineStatus::Stopped, dispatcher);
                tracing::info!("Engine startup cancelled.");
                Ok(())
            }
            EngineState::Running { mut handle } => {
                *state_lock = EngineState::Stopping;
                drop(state_lock);

                handle.kill_graceful();

                let mut state_lock = self.state.lock()
                    .map_err(|_| EngineError::IoError("State lock poisoned after kill".into()))?;
                *state_lock = EngineState::Idle;
                self.set_status(EngineStatus::Stopped, dispatcher);
                tracing::info!("Engine stopped.");
                Ok(())
            }
            EngineState::Failed(_) => {
                *state_lock = EngineState::Idle;
                self.set_status(EngineStatus::Stopped, dispatcher);
                Ok(())
            }
        }
    }

    pub fn current_status(&self) -> EngineStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(EngineStatus::Stopped)
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

fn read_bypass_config(app: &AppHandle) -> (String, String, String, bool) {
    let default_res = ("all".to_string(), "".to_string(), "".to_string(), false);
    let Ok(app_data) = app.path().app_data_dir() else { return default_res; };
    let settings_path = app_data.join("settings.json");
    if !settings_path.exists() {
        return default_res;
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
        return default_res;
    }

    let Ok(file_json) = serde_json::from_str::<serde_json::Value>(&content) else { return default_res; };
    let Some(zustand_raw) = file_json.get("vane-settings") else { return default_res; };
    let Ok(zustand_json) = (match zustand_raw {
        serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(&s),
        obj => Ok(obj.clone()),
    }) else { return default_res; };

    let state = zustand_json.get("state");
    let mode = state
        .and_then(|s| s.get("bypassMode"))
        .and_then(|m| m.as_str())
        .unwrap_or("all")
        .to_string();
    let list = state
        .and_then(|s| s.get("domainList"))
        .and_then(|l| l.as_str())
        .unwrap_or("")
        .to_string();
    let proxy = state
        .and_then(|s| s.get("proxySocks5"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    let kill_switch = state
        .and_then(|s| s.get("killSwitch"))
        .and_then(|k| k.as_bool())
        .unwrap_or(false);

    (mode, list, proxy, kill_switch)
}

fn apply_kill_switch(enabled: bool) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("netsh")
            .args(&["advfirewall", "firewall", "delete", "rule", "name=VaneDNSKillSwitch"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        if enabled {
            tracing::info!("DNS Kill Switch aktif ediliyor...");
            let _ = std::process::Command::new("netsh")
                .args(&[
                    "advfirewall", "firewall", "add", "rule",
                    "name=VaneDNSKillSwitch", "dir=out", "action=block",
                    "protocol=UDP", "remoteport=53"
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .status();

            let _ = std::process::Command::new("netsh")
                .args(&[
                    "advfirewall", "firewall", "add", "rule",
                    "name=VaneDNSKillSwitch", "dir=out", "action=block",
                    "protocol=TCP", "remoteport=53"
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .status();
        }
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("iptables")
            .args(&["-D", "OUTPUT", "-p", "udp", "--dport", "53", "-j", "DROP"])
            .status();
        let _ = std::process::Command::new("iptables")
            .args(&["-D", "OUTPUT", "-p", "tcp", "--dport", "53", "-j", "DROP"])
            .status();

        if enabled {
            tracing::info!("DNS Kill Switch aktif ediliyor (iptables)...");
            let _ = std::process::Command::new("iptables")
                .args(&["-A", "OUTPUT", "-p", "udp", "--dport", "53", "-j", "DROP"])
                .status();
            let _ = std::process::Command::new("iptables")
                .args(&["-A", "OUTPUT", "-p", "tcp", "--dport", "53", "-j", "DROP"])
                .status();
        }
    }
}

async fn spawn_and_run(
    preset: &Preset,
    app: &AppHandle,
    _cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<ProcessHandle, EngineError> {
    let winws_path = EngineManager::resolve_binary_path(app)?;
    let working_dir = winws_path.parent()
        .ok_or_else(|| EngineError::BinaryNotFound(
            format!("Binary path'in parent klasörü alınamadı: {:?}", winws_path)
        ))?;

    let mut prepared_args = EngineManager::prepare_args(&preset.args);

    let (bypass_mode, domain_list, _proxy_socks5, kill_switch) = read_bypass_config(app);
    
    // Apply DNS Kill Switch
    apply_kill_switch(kill_switch);

    if bypass_mode == "whitelist" || bypass_mode == "blacklist" {
        if let Ok(app_data) = app.path().app_data_dir() {
            let _ = std::fs::create_dir_all(&app_data);
            let domains_file_path = app_data.join("domains.txt");
            if let Err(e) = std::fs::write(&domains_file_path, &domain_list) {
                tracing::error!("Could not write domains.txt: {}", e);
            } else {
                let file_path_str = domains_file_path.to_string_lossy().to_string();
                if bypass_mode == "whitelist" {
                    prepared_args.push(format!("--hostlist={}", file_path_str));
                } else {
                    prepared_args.push(format!("--hostlist-exclude={}", file_path_str));
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    let mut cancel_rx = _cancel_rx;

    #[cfg(target_os = "linux")]
    {
        let mut escaped_args = Vec::new();
        for arg in &prepared_args {
            let escaped = arg.replace('\'', "'\\''");
            escaped_args.push(format!("'{}'", escaped));
        }
        let args_str = escaped_args.join(" ");
        let binary_path_escaped = winws_path.to_string_lossy().replace('\'', "'\\''");
        let binary_path_str = format!("'{}'", binary_path_escaped);
        
        let script = format!(
            "clean_up() {{ \
                 if [ -n \"$ENGINE_PID\" ]; then kill \"$ENGINE_PID\" 2>/dev/null; fi; \
                 if command -v nft >/dev/null 2>&1; then \
                     nft delete table ip vane_mangle 2>/dev/null; \
                 else \
                     iptables -t mangle -D OUTPUT -p tcp -m multiport --dports 80,443 -j NFQUEUE --queue-num 200 2>/dev/null; \
                 fi; \
             }}; \
             trap clean_up EXIT INT TERM HUP; \
             killall nfqws-x86_64-unknown-linux-gnu 2>/dev/null; \
             if command -v nft >/dev/null 2>&1; then \
                 nft delete table ip vane_mangle 2>/dev/null; \
                 nft add table ip vane_mangle || exit 1; \
                 nft add chain ip vane_mangle output '{{ type filter hook output priority mangle; policy accept; }}' || exit 1; \
                 nft add rule ip vane_mangle output tcp dport '{{ 80, 443 }}' queue num 200 || exit 1; \
             else \
                 iptables -t mangle -D OUTPUT -p tcp -m multiport --dports 80,443 -j NFQUEUE --queue-num 200 2>/dev/null; \
                 iptables -t mangle -I OUTPUT -p tcp -m multiport --dports 80,443 -j NFQUEUE --queue-num 200 || exit 1; \
             fi; \
             {} {} & ENGINE_PID=$!; \
             echo \"READY:$ENGINE_PID\"; \
             cat > /dev/null",
            binary_path_str, args_str
        );

        let can_run_directly = {
            let uid_output = std::process::Command::new("id")
                .arg("-u")
                .output();
            let is_root = match uid_output {
                Ok(out) => {
                    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    s == "0"
                }
                _ => false,
            };
            is_root || std::process::Command::new("iptables")
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

        root_cmd.arg("-c")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = tokio::process::Command::from(root_cmd).spawn().map_err(|e| {
            EngineError::SpawnFailed(format!("Linux Root Wrapper could not be started: {}", e))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| EngineError::IoError("Stdout alınamadı".into()))?;
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

        let stdout = child.stdout.take().ok_or_else(|| EngineError::IoError("stdout pipe oluşturulamadı".into()))?;
        let stderr = child.stderr.take().ok_or_else(|| EngineError::IoError("stderr pipe oluşturulamadı".into()))?;

        crate::engine::logger::spawn_log_reader(stdout, app.clone(), None);
        crate::engine::logger::spawn_log_reader(stderr, app.clone(), Some("HATA: "));

        let handle = ProcessHandle::new(child, pid, None);
        Ok(handle)
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = std::process::Command::new(&winws_path);
        command.args(&prepared_args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
        
        let mut child = tokio::process::Command::from(command).spawn().map_err(|e| {
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
                return Err(EngineError::IoError(
                    format!("Kernel-level process guard (Job Object) could not be created: {}", e)
                ));
            }
        };

        let stdout = child.stdout.take().ok_or_else(|| EngineError::IoError("stdout pipe oluşturulamadı".into()))?;
        let stderr = child.stderr.take().ok_or_else(|| EngineError::IoError("stderr pipe oluşturulamadı".into()))?;

        crate::engine::logger::spawn_log_reader(stdout, app.clone(), None);
        crate::engine::logger::spawn_log_reader(stderr, app.clone(), Some("HATA: "));

        let handle = ProcessHandle::new(child, pid, job_guard);
        Ok(handle)
    }
}

fn watch_process(
    pid: u32,
    app: AppHandle,
    state: Arc<Mutex<EngineState>>,
    status: Arc<Mutex<EngineStatus>>,
    preset: Preset,
) -> futures::future::BoxFuture<'static, ()> {
    use futures::FutureExt;
    async move {
        let mut attempt = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            let active = is_state_running_with_pid(&state, pid);

            if !active {
                break;
            }

            if !is_process_alive(pid) {
                tracing::warn!("Engine process (PID {}) died unexpectedly.", pid);

                let backoff_secs = match attempt {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    3 => 8,
                    4 => 16,
                    _ => {
                        tracing::error!("Engine restart limit reached. Transitioning to Failed.");
                        set_state_failed(&state, EngineError::IoError("Engine crashed repeatedly".into()));
                        set_status_error(&status, &app, "Süreç çöktü ve yeniden başlatılamadı.".into(), Some("CRASH_RESTART_FAILED".into()));
                        break;
                    }
                };

                tracing::info!("Attempting engine restart in {}s (attempt {}/5)...", backoff_secs, attempt + 1);
                tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;

                let still_running = is_state_running_with_pid(&state, pid);

                if still_running {
                    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                    set_state_starting(&state, tx);
                    set_status_starting(&status, &app);

                    match spawn_and_run(&preset, &app, rx).await {
                        Ok(new_handle) => {
                            let new_pid = new_handle.pid();
                            
                            let cancelled = is_status_stopped(&status);
                            if cancelled {
                                drop(new_handle);
                                set_state_idle(&state);
                                set_status_stopped(&status, &app);
                                break;
                            }

                            set_state_running(&state, new_handle);
                            set_status_running(&status, &app, new_pid);

                            tracing::info!("Engine successfully restarted, new PID: {}", new_pid);
                            let state_clone = state.clone();
                            let status_clone = status.clone();
                            let app_clone = app.clone();
                            let preset_clone = preset.clone();
                            tokio::spawn(async move {
                                watch_process(new_pid, app_clone, state_clone, status_clone, preset_clone).await;
                            });
                            break;
                        }
                        Err(e) => {
                            tracing::error!("Engine restart attempt failed: {}", e);
                            attempt += 1;
                        }
                    }
                } else {
                    break;
                }
            }
        }
    }.boxed()
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

fn set_state_starting(state: &Mutex<EngineState>, cancel_tx: tokio::sync::oneshot::Sender<()>) {
    if let Ok(mut sl) = state.lock() {
        *sl = EngineState::Starting { cancel: cancel_tx };
    }
}

fn set_state_running(state: &Mutex<EngineState>, handle: ProcessHandle) {
    if let Ok(mut sl) = state.lock() {
        *sl = EngineState::Running { handle };
    }
}

fn set_state_failed(state: &Mutex<EngineState>, err: EngineError) {
    if let Ok(mut sl) = state.lock() {
        *sl = EngineState::Failed(err);
    }
}

fn set_state_idle(state: &Mutex<EngineState>) {
    if let Ok(mut sl) = state.lock() {
        *sl = EngineState::Idle;
    }
}

fn set_status_error(status: &Mutex<EngineStatus>, app: &AppHandle, msg: String, code: Option<String>) {
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

fn is_status_stopped(status: &Mutex<EngineStatus>) -> bool {
    if let Ok(st) = status.lock() {
        *st == EngineStatus::Stopped
    } else {
        false
    }
}

fn set_status_stopped(status: &Mutex<EngineStatus>, app: &AppHandle) {
    if let Ok(mut st) = status.lock() {
        *st = EngineStatus::Stopped;
        let _ = app.emit("engine_status", &*st);
    }
}

#[cfg(target_os = "windows")]
fn is_process_alive(pid: u32) -> bool {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::System::Threading::GetExitCodeProcess;
    use windows::Win32::Foundation::{CloseHandle, FALSE};

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
