use crate::dns::firewall_plan::{
    rebuild_owned_kill_switch_plan, remove_kill_switch_plan, FirewallPlatform, KillSwitchOwnership,
    SystemFirewallExecutor,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const KILL_SWITCH_METADATA_FILE: &str = "dns-kill-switch.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedKillSwitchMetadata {
    pub schema_version: u8,
    pub platform: FirewallPlatform,
    pub installation_id: String,
    pub instance_id: String,
    pub dns_revision: u64,
    pub dns_fingerprint: String,
    pub rule_names: Vec<String>,
    pub created_at: String,
}

fn metadata_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join(KILL_SWITCH_METADATA_FILE))
        .map_err(|error| format!("AppData dir could not be resolved: {error}"))
}

pub fn save_kill_switch_metadata(
    app: &AppHandle,
    ownership: &KillSwitchOwnership,
) -> Result<(), String> {
    let path = metadata_file_path(app)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let payload = PersistedKillSwitchMetadata {
        schema_version: 2,
        platform: ownership.platform,
        installation_id: ownership.installation_id.clone(),
        instance_id: ownership.instance_id.clone(),
        dns_revision: ownership.revision.get(),
        dns_fingerprint: ownership.fingerprint.as_str().to_string(),
        rule_names: ownership.rule_ids.clone(),
        created_at: format!("{now}"),
    };

    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|e| format!("KillSwitch metadata serialization failed: {e}"))?;

    crate::settings::atomic_replace_bytes(&path, &bytes)
        .map_err(|e| format!("KillSwitch metadata persistence failed: {e}"))
}

pub fn clear_kill_switch_metadata(app: &AppHandle) -> Result<(), String> {
    let path = metadata_file_path(app)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("KillSwitch metadata could not be cleared: {e}")),
    }
}

pub fn get_or_create_installation_id(app: &AppHandle) -> String {
    let file_path = match app.path().app_data_dir() {
        Ok(dir) => dir.join("installation_id.txt"),
        Err(_) => return "vane-default-inst-id".to_string(),
    };
    if file_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            let id = content.trim().to_string();
            if !id.is_empty() {
                return id;
            }
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let new_id = format!("vane-inst-{:x}-{:x}", nanos, std::process::id());
    let _ = crate::settings::atomic_replace_bytes(&file_path, new_id.as_bytes());
    new_id
}

pub fn recover_orphan_kill_switch_rules(
    app: &AppHandle,
    active_installation_id: &str,
) -> Result<bool, String> {
    let path = metadata_file_path(app)?;
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("KillSwitch metadata could not be read: {error}")),
    };
    let metadata: PersistedKillSwitchMetadata =
        serde_json::from_slice(&bytes).map_err(|error| {
            format!("KillSwitch metadata is malformed or has an unsupported schema: {error}")
        })?;
    if metadata.schema_version != 2 {
        return Err(format!(
            "Unsupported KillSwitch metadata schema {}; platform cannot be safely inferred.",
            metadata.schema_version
        ));
    }
    if metadata.installation_id != active_installation_id {
        return Err(
            "KillSwitch metadata belongs to a foreign installation; cleanup was not attempted."
                .into(),
        );
    }
    let ownership = KillSwitchOwnership {
        installation_id: metadata.installation_id,
        instance_id: metadata.instance_id,
        revision: crate::dns::runtime_config::DnsConfigRevision(metadata.dns_revision),
        fingerprint: crate::dns::runtime_config::DnsConfigFingerprint(metadata.dns_fingerprint),
        platform: metadata.platform,
        rule_ids: metadata.rule_names,
    };
    let plan = rebuild_owned_kill_switch_plan(&ownership);
    let executor = SystemFirewallExecutor::new(ownership.platform);
    remove_kill_switch_plan(&executor, &plan)
        .map_err(|error| format!("Orphan KillSwitch cleanup failed: {error}"))?;
    clear_kill_switch_metadata(app)?;
    Ok(true)
}
