use std::path::{Path, PathBuf};

use crate::engine::launch_plan::{
    build_engine_launch_plan, EngineLaunchInput, EnginePlatform, HostlistPlan,
};
use crate::engine::runtime_config::{
    ConfigFingerprint, ConfigRevision, PreparedHostlist, PreparedRuntimeConfig, RuntimeBypassMode,
    VerifiedRuntimeConfig,
};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PatternApplyOutcome {
    Superseded {
        revision: ConfigRevision,
    },
    Prepared {
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    Applied {
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
        process_id: u32,
    },
    RolledBack {
        failed_revision: ConfigRevision,
        restored_revision: ConfigRevision,
        restored_fingerprint: ConfigFingerprint,
        process_id: u32,
    },
}

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum PatternApplyError {
    #[error("pattern request was superseded")]
    Superseded,
    #[error("pattern configuration validation failed: {0}")]
    Validation(String),
    #[error("pattern persistence failed: {0}")]
    Persistence(String),
    #[error("pattern hostlist preparation failed: {0}")]
    HostlistPreparation(String),
    #[error("previous engine process could not be stopped: {0}")]
    StopFailed(String),
    #[error("candidate engine process could not be started: {0}")]
    CandidateStartFailed(String),
    #[error("previous engine configuration rollback failed: {0}")]
    RollbackFailed(String),
    #[error("runtime configuration state is inconsistent: {0}")]
    StateInvariant(String),
    #[error("pattern revision overflow")]
    RevisionOverflow,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct PersistedPatternSnapshot {
    pub mode: String,
    pub domain_list: String,
    pub kill_switch: bool,
    pub proxy_socks5: String,
    pub whitelist_domains: Vec<String>,
    pub blacklist_domains: Vec<String>,
    pub active_preset_id: String,
}

pub(crate) fn build_hostlist_filename(
    revision: ConfigRevision,
    fingerprint: &ConfigFingerprint,
) -> String {
    format!(
        "domains-rev-{}-{}.txt",
        revision.get(),
        fingerprint.prefix(8)
    )
}

pub(crate) fn write_revisioned_hostlist(
    app_data_dir: &Path,
    filename: &str,
    content: &str,
) -> Result<PathBuf, PatternApplyError> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(PatternApplyError::HostlistPreparation(
            "Unsafe path traversal characters in hostlist filename".into(),
        ));
    }

    std::fs::create_dir_all(app_data_dir).map_err(|e| {
        PatternApplyError::HostlistPreparation(format!("Could not create data directory: {}", e))
    })?;

    let hostlist_path = app_data_dir.join(filename);
    crate::settings::atomic_replace_bytes(&hostlist_path, content.as_bytes()).map_err(|e| {
        PatternApplyError::HostlistPreparation(format!("Could not write hostlist file: {}", e))
    })?;

    Ok(hostlist_path)
}

pub(crate) fn clean_stale_hostlists(
    app_data_dir: &Path,
    active_filename: Option<&str>,
    previous_filename: Option<&str>,
) -> Result<(), PatternApplyError> {
    if !app_data_dir.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(app_data_dir).map_err(|e| {
        PatternApplyError::HostlistPreparation(format!("Could not read data directory: {}", e))
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with("domains-rev-") && file_name.ends_with(".txt") {
                if Some(file_name) == active_filename || Some(file_name) == previous_filename {
                    continue;
                }
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    Ok(())
}

pub(crate) fn prepare_runtime_config_for_transaction(
    verified: VerifiedRuntimeConfig,
    app_data_dir: &Path,
) -> Result<(PreparedRuntimeConfig, Option<String>), PatternApplyError> {
    let (hostlist_path, filename) = if verified.bypass.mode != RuntimeBypassMode::All {
        let name = build_hostlist_filename(verified.revision, &verified.fingerprint);
        let content = verified.bypass.domains.join("\n");
        let path = write_revisioned_hostlist(app_data_dir, &name, &content)?;
        (Some(path), Some(name))
    } else {
        (None, None)
    };

    let bypass_input = verified.to_launch_bypass_input(hostlist_path);

    let winws_path = PathBuf::from("winws.exe");

    let preset = verified.preset.to_preset();

    let launch_input = EngineLaunchInput {
        preset: &preset,
        platform: EnginePlatform::current(),
        executable: winws_path,
        bypass: bypass_input,
    };

    let launch_plan = build_engine_launch_plan(launch_input).map_err(|e| {
        PatternApplyError::Validation(format!("Launch plan construction failed: {}", e))
    })?;

    let prepared_hostlist = match &launch_plan.hostlist {
        HostlistPlan::Include { path, domain_count } => PreparedHostlist::Planned {
            path: path.clone(),
            domain_count: *domain_count,
        },
        HostlistPlan::Exclude { path, domain_count } => PreparedHostlist::Planned {
            path: path.clone(),
            domain_count: *domain_count,
        },
        HostlistPlan::None => PreparedHostlist::NotRequired,
    };

    let prepared = PreparedRuntimeConfig {
        verified,
        launch_plan,
        hostlist: prepared_hostlist,
    };

    Ok((prepared, filename))
}
