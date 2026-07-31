use crate::dns::firewall_plan::{
    FirewallExecutor, FirewallRuleSpec, FirewallStep, KillSwitchOwnership, SystemFirewallExecutor,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const KILL_SWITCH_METADATA_FILE: &str = "dns-kill-switch.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedKillSwitchMetadata {
    pub schema_version: u8,
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
        schema_version: 1,
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

pub fn recover_orphan_kill_switch_rules(app: &AppHandle, active_installation_id: &str) -> bool {
    let executor = SystemFirewallExecutor;
    let mut recovered_any = false;

    // 1. Recover from dns-kill-switch.json
    if let Ok(path) = metadata_file_path(app) {
        if path.exists() {
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(metadata) = serde_json::from_slice::<PersistedKillSwitchMetadata>(&bytes)
                {
                    if metadata.installation_id == active_installation_id {
                        tracing::info!(
                            "Found orphan Kill Switch rules from previous instance: {:?}",
                            metadata.rule_names
                        );
                        for rule_name in &metadata.rule_names {
                            let step = FirewallStep::RemoveRule(FirewallRuleSpec {
                                name: rule_name.clone(),
                                direction: "out".into(),
                                action: "block".into(),
                                protocol: "UDP".into(),
                                port: 53,
                                remote_ip: None,
                                comment: None,
                            });
                            let _ = executor.execute(&step);
                        }
                        recovered_any = true;
                    } else {
                        tracing::warn!(
                            "Kill Switch metadata belongs to foreign installation ID; skipping cleanup."
                        );
                    }
                }
            }
            let _ = clear_kill_switch_metadata(app);
        }
    }

    // 2. Safe Legacy Rule Migration (remove pre-P10 rules if present)
    let legacy_rules = vec!["Vane-KillSwitch-UDP", "Vane-KillSwitch-TCP"];
    for rule in legacy_rules {
        let step = FirewallStep::RemoveRule(FirewallRuleSpec {
            name: rule.to_string(),
            direction: "out".into(),
            action: "block".into(),
            protocol: "UDP".into(),
            port: 53,
            remote_ip: None,
            comment: None,
        });
        let _ = executor.execute(&step);
    }

    recovered_any
}
