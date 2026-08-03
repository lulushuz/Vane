#[cfg(target_os = "linux")]
use crate::platform::linux::command::SystemLinuxCommandRunner;
#[cfg(any(test, target_os = "linux"))]
use crate::platform::linux::command::{LinuxCommandRunner, LinuxCommandSpec};
use crate::platform::linux::ownership::LinuxRuleOwnership;
use crate::platform::linux::LinuxPlatformError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const LINUX_FILTER_METADATA_FILE: &str = "linux-engine-filter.json";
#[cfg(any(test, target_os = "linux"))]
const STDERR_SUMMARY_LIMIT: usize = 512;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxFilterRecoveryOutcome {
    NoMetadata,
    Recovered,
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

fn clear_metadata_path(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Linux filter metadata clear error: {e}")),
    }
}

pub fn clear_linux_filter_metadata(app: &AppHandle) -> Result<(), String> {
    clear_metadata_path(&filter_metadata_file_path(app)?)
}

#[cfg(any(test, target_os = "linux"))]
fn recovery_command(
    meta: &PersistedLinuxFilterMetadata,
) -> Result<LinuxCommandSpec, LinuxPlatformError> {
    match meta.backend.as_str() {
        "nftables" => Ok(LinuxCommandSpec {
            program: "nft".into(),
            args: vec!["-f".into(), "-".into()],
            stdin: Some(format!("delete table ip {}\n", meta.table_name).into_bytes()),
        }),
        "iptables" => Ok(LinuxCommandSpec {
            program: "iptables".into(),
            args: vec![
                "-t".into(),
                meta.table_name.clone(),
                "-F".into(),
                meta.chain_name.clone(),
            ],
            stdin: None,
        }),
        backend => Err(LinuxPlatformError::RecoveryFailed(format!(
            "Unsupported persisted firewall backend: {backend}"
        ))),
    }
}

#[cfg(any(test, target_os = "linux"))]
fn summarize(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(STDERR_SUMMARY_LIMIT)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(any(test, target_os = "linux"))]
fn recover_orphan_at_path(
    path: &Path,
    active_installation_id: &str,
    runner: &dyn LinuxCommandRunner,
) -> Result<LinuxFilterRecoveryOutcome, LinuxPlatformError> {
    if !path.exists() {
        return Ok(LinuxFilterRecoveryOutcome::NoMetadata);
    }
    let bytes = std::fs::read(path).map_err(|error| {
        LinuxPlatformError::MetadataFailure(format!("Orphan metadata read failed: {error}"))
    })?;
    let meta: PersistedLinuxFilterMetadata = serde_json::from_slice(&bytes).map_err(|error| {
        LinuxPlatformError::MetadataFailure(format!("Orphan metadata is malformed: {error}"))
    })?;
    if meta.installation_id != active_installation_id {
        return Err(LinuxPlatformError::OwnershipMismatch {
            expected: active_installation_id.to_string(),
            found: meta.installation_id,
        });
    }

    let command = recovery_command(&meta)?;
    let context = format!(
        "backend={} program={} args={:?}",
        meta.backend, command.program, command.args
    );
    let output = runner.run(&command).map_err(|error| {
        LinuxPlatformError::RecoveryFailed(format!("{context} io_error={error}"))
    })?;
    match output.exit_code {
        Some(0) => {}
        Some(code) => {
            return Err(LinuxPlatformError::RecoveryFailed(format!(
                "{context} exit_code={code} stderr={:?}",
                summarize(&output.stderr)
            )))
        }
        None => {
            return Err(LinuxPlatformError::RecoveryFailed(format!(
                "{context} exit_code=signal stderr={:?}",
                summarize(&output.stderr)
            )))
        }
    }

    clear_metadata_path(path).map_err(|error| {
        tracing::error!(
            "Orphan Linux firewall cleanup succeeded, but metadata clear failed: {error}"
        );
        LinuxPlatformError::MetadataFailure(error)
    })?;
    Ok(LinuxFilterRecoveryOutcome::Recovered)
}

pub fn recover_orphan_linux_filter_rules(
    app: &AppHandle,
    active_installation_id: &str,
) -> Result<LinuxFilterRecoveryOutcome, LinuxPlatformError> {
    let path = filter_metadata_file_path(app).map_err(LinuxPlatformError::MetadataFailure)?;
    #[cfg(target_os = "linux")]
    {
        recover_orphan_at_path(&path, active_installation_id, &SystemLinuxCommandRunner)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (path, active_installation_id);
        Ok(LinuxFilterRecoveryOutcome::NoMetadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::linux::command::test_support::FakeLinuxCommandRunner;
    use crate::platform::linux::command::LinuxCommandOutput;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "vane-linux-recovery-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn metadata() -> PersistedLinuxFilterMetadata {
        PersistedLinuxFilterMetadata {
            schema_version: 1,
            installation_id: "owned-installation".into(),
            instance_id: "instance".into(),
            generation: 1,
            config_revision: 2,
            config_fingerprint: "fingerprint".into(),
            backend: "nftables".into(),
            queue_number: 4242,
            table_name: "vane_tbl_owned".into(),
            chain_name: "vane_chain_owned".into(),
            created_at: "0".into(),
        }
    }

    fn write_metadata(path: &Path) {
        std::fs::write(path, serde_json::to_vec_pretty(&metadata()).unwrap()).unwrap();
    }

    fn output(
        code: i32,
        stderr: &str,
    ) -> Result<LinuxCommandOutput, crate::platform::linux::LinuxCommandRunError> {
        Ok(LinuxCommandOutput {
            exit_code: Some(code),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        })
    }

    #[test]
    fn orphan_recovery_command_failure_preserves_metadata() {
        let directory = TestDirectory::new();
        let path = directory.0.join(LINUX_FILTER_METADATA_FILE);
        write_metadata(&path);
        let runner = FakeLinuxCommandRunner::new(vec![output(1, "denied")]);
        assert!(recover_orphan_at_path(&path, "owned-installation", &runner).is_err());
        assert!(path.exists());
    }

    #[test]
    fn orphan_recovery_success_clears_metadata() {
        let directory = TestDirectory::new();
        let path = directory.0.join(LINUX_FILTER_METADATA_FILE);
        write_metadata(&path);
        let runner = FakeLinuxCommandRunner::success(1);
        assert_eq!(
            recover_orphan_at_path(&path, "owned-installation", &runner).unwrap(),
            LinuxFilterRecoveryOutcome::Recovered
        );
        assert!(!path.exists());
    }

    #[test]
    fn orphan_recovery_malformed_metadata_is_preserved() {
        let directory = TestDirectory::new();
        let path = directory.0.join(LINUX_FILTER_METADATA_FILE);
        std::fs::write(&path, b"{malformed").unwrap();
        let runner = FakeLinuxCommandRunner::success(0);
        assert!(matches!(
            recover_orphan_at_path(&path, "owned-installation", &runner),
            Err(LinuxPlatformError::MetadataFailure(_))
        ));
        assert!(path.exists());
    }

    #[test]
    fn orphan_recovery_ownership_mismatch_runs_no_command() {
        let directory = TestDirectory::new();
        let path = directory.0.join(LINUX_FILTER_METADATA_FILE);
        write_metadata(&path);
        let runner = FakeLinuxCommandRunner::success(0);
        assert!(matches!(
            recover_orphan_at_path(&path, "foreign-installation", &runner),
            Err(LinuxPlatformError::OwnershipMismatch { .. })
        ));
        assert!(runner.commands().is_empty());
        assert!(path.exists());
    }
}
