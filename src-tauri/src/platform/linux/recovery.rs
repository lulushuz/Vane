use crate::platform::linux::ownership::LinuxRuleOwnership;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const LINUX_FILTER_METADATA_FILE: &str = "linux-engine-filter.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedLinuxFilterMetadata {
    pub schema_version: u8,
    pub installation_id: String,
    pub instance_id: String,
    pub generation: u64,
    pub config_revision: u64,
    pub config_fingerprint: String,
    pub backend: String,
    pub queue_number: u16,
    pub table_name: String,
    pub chain_name: String,
    pub created_at: String,
}

fn filter_metadata_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(LINUX_FILTER_METADATA_FILE))
        .map_err(|e| format!("AppData dir resolve error: {e}"))
}

pub fn save_linux_filter_metadata(
    app: &AppHandle,
    ownership: &LinuxRuleOwnership,
    backend: &str,
) -> Result<(), String> {
    let path = filter_metadata_file_path(app)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let payload = PersistedLinuxFilterMetadata {
        schema_version: 1,
        installation_id: ownership.installation_id.clone(),
        instance_id: ownership.instance_id.clone(),
        generation: ownership.generation,
        config_revision: ownership.config_revision,
        config_fingerprint: ownership.config_fingerprint.clone(),
        backend: backend.to_string(),
        queue_number: ownership.queue_number,
        table_name: ownership.table_name.clone(),
        chain_name: ownership.chain_name.clone(),
        created_at: format!("{now}"),
    };

    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|e| format!("Linux filter metadata serialization error: {e}"))?;

    crate::settings::atomic_replace_bytes(&path, &bytes)
        .map_err(|e| format!("Linux filter metadata persistence error: {e}"))
}

pub fn clear_linux_filter_metadata(app: &AppHandle) -> Result<(), String> {
    let path = filter_metadata_file_path(app)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Linux filter metadata clear error: {e}")),
    }
}

pub fn recover_orphan_linux_filter_rules(app: &AppHandle, active_installation_id: &str) -> bool {
    let mut recovered = false;

    if let Ok(path) = filter_metadata_file_path(app) {
        if path.exists() {
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(meta) = serde_json::from_slice::<PersistedLinuxFilterMetadata>(&bytes) {
                    if meta.installation_id == active_installation_id {
                        tracing::info!(
                            "Recovering orphan Linux filter table {} chain {}",
                            meta.table_name,
                            meta.chain_name
                        );

                        #[cfg(target_os = "linux")]
                        {
                            use std::process::Command;
                            if meta.backend == "nftables" {
                                let batch = format!("delete table ip {}\n", meta.table_name);
                                let _ = Command::new("nft")
                                    .arg("-f")
                                    .arg("-")
                                    .stdin(std::process::Stdio::piped())
                                    .spawn()
                                    .and_then(|mut child| {
                                        if let Some(stdin) = child.stdin.as_mut() {
                                            use std::io::Write;
                                            let _ = stdin.write_all(batch.as_bytes());
                                        }
                                        child.wait()
                                    });
                            } else {
                                let _ = Command::new("iptables")
                                    .args(["-t", &meta.table_name, "-F", &meta.chain_name])
                                    .output();
                            }
                        }
                        recovered = true;
                    }
                }
            }
            let _ = clear_linux_filter_metadata(app);
        }
    }

    recovered
}
