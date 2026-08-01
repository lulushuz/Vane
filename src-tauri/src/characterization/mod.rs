use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
pub struct TempTestDir(pub PathBuf);

#[cfg(test)]
impl TempTestDir {
    pub fn new(prefix: &str) -> Self {
        let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("vane-test-{prefix}-{}-{seq}", std::process::id()));
        let _ = std::fs::create_dir_all(&path);
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
pub mod advanced_contract_tests;
#[cfg(test)]
pub mod binary_integrity_tests;
#[cfg(test)]
pub mod diagnostics_tests;
#[cfg(test)]
pub mod dns_tests;
#[cfg(test)]
pub mod dns_transaction_tests;
#[cfg(test)]
pub mod domain_tests;
#[cfg(test)]
pub mod engine_tests;
#[cfg(test)]
pub mod ipc_tests;
#[cfg(test)]
pub mod kill_switch_tests;
#[cfg(test)]
pub mod launch_plan_tests;
#[cfg(test)]
pub mod lifecycle_tests;
#[cfg(test)]
pub mod linux_filter_tests;
#[cfg(test)]
pub mod loader_tests;
#[cfg(test)]
pub mod optimizer_session_tests;
#[cfg(test)]
pub mod optimizer_tests;
#[cfg(test)]
pub mod pattern_tests;
#[cfg(test)]
pub mod pattern_transaction_tests;
#[cfg(test)]
pub mod preset_pipeline_tests;
#[cfg(test)]
pub mod preset_tests;
#[cfg(test)]
pub mod process_tests;
#[cfg(test)]
pub mod remote_preset_tests;
#[cfg(test)]
pub mod reproducers;
#[cfg(test)]
pub mod runtime_config_tests;
#[cfg(test)]
pub mod settings_tests;
