use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RuntimeSettings {
    pub active_preset_id: String,
    pub bypass_mode: String,
    pub whitelist_domains: Vec<String>,
    pub blacklist_domains: Vec<String>,
    pub dns_protocol: String,
    pub dns_ad_block: bool,
    pub dns_cache: bool,
    pub proxy_socks5: String,
    pub kill_switch: bool,
    pub watchdog: bool,
    pub dns_forwarder_enabled: bool,
    pub health_check_targets: Vec<String>,
    pub selected_dns_id: String,
    pub dns_custom_primary: String,
    pub dns_custom_secondary: String,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            active_preset_id: "default".into(),
            bypass_mode: "all".into(),
            whitelist_domains: Vec::new(),
            blacklist_domains: Vec::new(),
            dns_protocol: "doh".into(),
            dns_ad_block: false,
            dns_cache: true,
            proxy_socks5: String::new(),
            kill_switch: false,
            watchdog: true,
            dns_forwarder_enabled: false,
            health_check_targets: vec!["example.com".into()],
            selected_dns_id: String::new(),
            dns_custom_primary: String::new(),
            dns_custom_secondary: String::new(),
        }
    }
}

static SETTINGS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static WINDOW_SNAPSHOTS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
const SETTINGS_KEY: &str = "vane-settings";
const MAX_SETTINGS_BYTES: usize = 1024 * 1024;

fn validate_ipc_key(key: &str) -> Result<(), String> {
    if key == SETTINGS_KEY {
        Ok(())
    } else {
        Err("Unsupported settings key.".into())
    }
}

fn validate_ipc_payload(value: &str) -> Result<(), String> {
    if value.len() > MAX_SETTINGS_BYTES {
        return Err("Settings payload exceeds the 1 MiB safety limit.".into());
    }
    let payload: Value = serde_json::from_str(value)
        .map_err(|error| format!("Settings payload is not valid JSON: {error}"))?;
    if !payload.get("state").is_some_and(Value::is_object) {
        return Err("Settings payload has no state object.".into());
    }
    Ok(())
}

fn lock() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    SETTINGS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Settings repository lock is poisoned".to_string())
}

fn paths(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok((dir.join("settings.json"), dir.join("settings.json.bak")))
}

fn parse_store(path: &Path) -> Result<Map<String, Value>, String> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let bytes =
        std::fs::read(path).map_err(|e| format!("{} could not be read: {e}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("{} is not valid JSON: {e}", path.display()))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{} does not contain a JSON object", path.display()))
}

fn load_with_recovery(primary: &Path, backup: &Path) -> Result<Map<String, Value>, String> {
    if !primary.exists() && backup.exists() {
        let store = parse_store(backup)?;
        if !store.is_empty() {
            tracing::error!("Settings primary was missing; using the last-known-good backup.");
            return Ok(store);
        }
    }
    match parse_store(primary) {
        Ok(store) => Ok(store),
        Err(primary_error) => match parse_store(backup) {
            Ok(store) if !store.is_empty() => {
                tracing::error!("Settings primary was damaged; using the last-known-good backup: {primary_error}");
                Ok(store)
            }
            Ok(_) => Err(primary_error),
            Err(backup_error) => Err(format!(
                "Settings and backup are unreadable. Primary: {primary_error}; backup: {backup_error}"
            )),
        },
    }
}

#[cfg(target_os = "windows")]
fn replace_file(temp: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|e| e.to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn replace_file(temp: &Path, target: &Path) -> Result<(), String> {
    std::fs::rename(temp, target).map_err(|e| e.to_string())
}

fn atomic_write(primary: &Path, backup: &Path, store: &Map<String, Value>) -> Result<(), String> {
    use std::io::Write;

    let bytes = serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?;
    let temp = primary.with_extension("json.tmp");

    if primary.exists() && parse_store(primary).is_ok() {
        std::fs::copy(primary, backup)
            .map_err(|e| format!("Settings backup could not be written: {e}"))?;
        std::fs::OpenOptions::new()
            .read(true)
            .open(backup)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    replace_file(&temp, primary)
}

pub fn atomic_replace_bytes(target: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let temp = target.with_extension("tmp");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    replace_file(&temp, target)
}

pub fn get_value(app: &AppHandle, key: &str) -> Result<Option<String>, String> {
    let _guard = lock()?;
    let (primary, backup) = paths(app)?;
    let store = load_with_recovery(&primary, &backup)?;
    match store.get(key) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(value) => Ok(Some(value.to_string())),
        None => Ok(None),
    }
}

pub fn set_value(app: &AppHandle, key: &str, value: String) -> Result<(), String> {
    let _guard = lock()?;
    let (primary, backup) = paths(app)?;
    let mut store = if primary.exists() || backup.exists() {
        load_with_recovery(&primary, &backup)?
    } else {
        Map::new()
    };
    if store.get(key).and_then(Value::as_str) == Some(value.as_str()) {
        return Ok(());
    }
    store.insert(key.to_string(), Value::String(value));
    atomic_write(&primary, &backup, &store)
}

fn merge_window_state(
    app: &AppHandle,
    key: &str,
    incoming: String,
    window_label: &str,
) -> Result<(), String> {
    let _guard = lock()?;
    let (primary, backup) = paths(app)?;
    let mut store = if primary.exists() || backup.exists() {
        load_with_recovery(&primary, &backup)?
    } else {
        Map::new()
    };

    let mut snapshots = WINDOW_SNAPSHOTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Window settings snapshot lock is poisoned".to_string())?;
    let previous = snapshots.get(window_label).cloned();
    let merged = match (store.get(key).and_then(Value::as_str), previous.as_deref()) {
        (Some(current), Some(previous)) => merge_zustand_payload(current, previous, &incoming)?,
        _ => incoming.clone(),
    };
    if store.get(key).and_then(Value::as_str) == Some(merged.as_str()) {
        snapshots.insert(window_label.to_string(), incoming);
        return Ok(());
    }
    drop(snapshots);
    store.insert(key.to_string(), Value::String(merged));
    atomic_write(&primary, &backup, &store)?;
    WINDOW_SNAPSHOTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Window settings snapshot lock is poisoned".to_string())?
        .insert(window_label.to_string(), incoming);
    Ok(())
}

fn merge_zustand_payload(current: &str, previous: &str, incoming: &str) -> Result<String, String> {
    let mut current: Value = serde_json::from_str(current)
        .map_err(|e| format!("Current settings payload is invalid: {e}"))?;
    let previous: Value = serde_json::from_str(previous)
        .map_err(|e| format!("Previous window settings payload is invalid: {e}"))?;
    let incoming: Value = serde_json::from_str(incoming)
        .map_err(|e| format!("Incoming settings payload is invalid: {e}"))?;

    let current_state = current
        .get_mut("state")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "Current settings payload has no state object.".to_string())?;
    let previous_state = previous
        .get("state")
        .and_then(Value::as_object)
        .ok_or_else(|| "Previous window settings payload has no state object.".to_string())?;
    let incoming_state = incoming
        .get("state")
        .and_then(Value::as_object)
        .ok_or_else(|| "Incoming settings payload has no state object.".to_string())?;

    for (field, value) in incoming_state {
        if previous_state.get(field) != Some(value) {
            current_state.insert(field.clone(), value.clone());
        }
    }
    if let Some(version) = incoming.get("version").cloned() {
        current["version"] = version;
    }
    serde_json::to_string(&current).map_err(|e| e.to_string())
}

pub fn remove_value(app: &AppHandle, key: &str) -> Result<(), String> {
    let _guard = lock()?;
    let (primary, backup) = paths(app)?;
    let mut store = load_with_recovery(&primary, &backup)?;
    store.remove(key);
    atomic_write(&primary, &backup, &store)
}

pub fn read_zustand_state(app: &AppHandle) -> Result<Option<Value>, String> {
    let Some(raw) = get_value(app, "vane-settings")? else {
        return Ok(None);
    };
    let root: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Saved application state is invalid: {e}"))?;
    Ok(root.get("state").cloned())
}

pub fn read_runtime_settings(app: &AppHandle) -> Result<Option<RuntimeSettings>, String> {
    let Some(state) = read_zustand_state(app)? else {
        return Ok(None);
    };
    serde_json::from_value(state)
        .map(Some)
        .map_err(|e| format!("Saved runtime settings have an invalid schema: {e}"))
}

#[tauri::command]
pub fn settings_get(
    app: AppHandle,
    window: tauri::WebviewWindow,
    key: String,
) -> Result<Option<String>, String> {
    validate_ipc_key(&key)?;
    let value = get_value(&app, &key)?;
    if let Some(snapshot) = value.as_ref() {
        WINDOW_SNAPSHOTS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| "Window settings snapshot lock is poisoned".to_string())?
            .insert(window.label().to_string(), snapshot.clone());
    }
    Ok(value)
}

#[tauri::command]
pub fn settings_set(
    app: AppHandle,
    window: tauri::WebviewWindow,
    key: String,
    value: String,
) -> Result<(), String> {
    validate_ipc_key(&key)?;
    validate_ipc_payload(&value)?;
    merge_window_state(&app, &key, value, window.label())
}

#[tauri::command]
pub fn settings_remove(app: AppHandle, key: String) -> Result<(), String> {
    validate_ipc_key(&key)?;
    remove_value(&app, &key)
}

#[cfg(test)]
mod tests {
    use super::{
        atomic_write, load_with_recovery, merge_zustand_payload, parse_store, validate_ipc_key,
        validate_ipc_payload, RuntimeSettings, SETTINGS_KEY,
    };
    use serde_json::{json, Map, Value};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "vane-settings-test-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("test directory must be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn stale_window_only_updates_fields_it_changed() {
        let current = r#"{"state":{"language":"tr","dnsCache":false,"watchdog":true},"version":1}"#;
        let previous = r#"{"state":{"language":"en","dnsCache":true,"watchdog":true},"version":1}"#;
        let incoming =
            r#"{"state":{"language":"en","dnsCache":true,"watchdog":false},"version":1}"#;
        let merged: serde_json::Value = serde_json::from_str(
            &merge_zustand_payload(current, previous, incoming).expect("merge must succeed"),
        )
        .expect("valid JSON");
        assert_eq!(merged["state"]["language"], "tr");
        assert_eq!(merged["state"]["dnsCache"], false);
        assert_eq!(merged["state"]["watchdog"], false);
    }

    #[test]
    fn settings_ipc_rejects_unknown_keys_and_malformed_payloads() {
        assert!(validate_ipc_key(SETTINGS_KEY).is_ok());
        assert!(validate_ipc_key("arbitrary-key").is_err());
        assert!(validate_ipc_payload(r#"{"state":{"dnsCache":false},"version":1}"#).is_ok());
        assert!(validate_ipc_payload(r#"{"version":1}"#).is_err());
        assert!(validate_ipc_payload("not-json").is_err());
    }

    #[test]
    fn damaged_primary_recovers_the_last_known_good_backup() {
        let directory = TestDirectory::new();
        let primary = directory.path().join("settings.json");
        let backup = directory.path().join("settings.json.bak");
        std::fs::write(&primary, b"{damaged").expect("damaged primary must be written");
        std::fs::write(
            &backup,
            serde_json::to_vec(&json!({ SETTINGS_KEY: "saved-state" }))
                .expect("backup must serialize"),
        )
        .expect("backup must be written");

        let recovered = load_with_recovery(&primary, &backup).expect("backup must recover");
        assert_eq!(recovered.get(SETTINGS_KEY), Some(&json!("saved-state")));
    }

    #[test]
    fn atomic_write_keeps_the_previous_valid_primary_as_backup() {
        let directory = TestDirectory::new();
        let primary = directory.path().join("settings.json");
        let backup = directory.path().join("settings.json.bak");

        let mut first = Map::new();
        first.insert(SETTINGS_KEY.to_string(), Value::String("revision-1".into()));
        atomic_write(&primary, &backup, &first).expect("initial write must succeed");

        let mut second = Map::new();
        second.insert(SETTINGS_KEY.to_string(), Value::String("revision-2".into()));
        atomic_write(&primary, &backup, &second).expect("replacement must succeed");

        assert_eq!(
            parse_store(&primary)
                .expect("primary must parse")
                .get(SETTINGS_KEY),
            Some(&Value::String("revision-2".into()))
        );
        assert_eq!(
            parse_store(&backup)
                .expect("backup must parse")
                .get(SETTINGS_KEY),
            Some(&Value::String("revision-1".into()))
        );
    }

    #[test]
    fn runtime_settings_preserve_security_sensitive_values() {
        let settings: RuntimeSettings = serde_json::from_value(json!({
            "activePresetId": "general-alt4",
            "bypassMode": "whitelist",
            "whitelistDomains": ["example.com"],
            "blacklistDomains": ["blocked.example"],
            "dnsProtocol": "dot",
            "dnsAdBlock": true,
            "dnsCache": false,
            "proxySocks5": "127.0.0.1:1080",
            "killSwitch": true,
            "watchdog": false,
            "dnsForwarderEnabled": true,
            "healthCheckTargets": ["example.com"],
            "selectedDnsId": "custom",
            "dnsCustomPrimary": "1.1.1.1",
            "dnsCustomSecondary": "1.0.0.1"
        }))
        .expect("runtime settings must deserialize");

        assert_eq!(settings.active_preset_id, "general-alt4");
        assert_eq!(settings.bypass_mode, "whitelist");
        assert_eq!(settings.whitelist_domains, ["example.com"]);
        assert!(!settings.dns_cache);
        assert!(settings.dns_ad_block);
        assert!(settings.kill_switch);
        assert!(!settings.watchdog);
        assert!(settings.dns_forwarder_enabled);
        assert_eq!(settings.proxy_socks5, "127.0.0.1:1080");
    }
}
