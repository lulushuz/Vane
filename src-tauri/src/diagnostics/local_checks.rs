use crate::diagnostics::health::{HealthState, SubsystemHealth, SystemHealthSnapshot};
use std::path::Path;
use std::time::SystemTime;

pub fn perform_local_consistency_checks(app_data_dir: &Path) -> SystemHealthSnapshot {
    let now_ms = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut subsystems = Vec::new();

    // Storage writability is proven with an actual create/write/sync/remove cycle.
    let storage_probe = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(app_data_dir)?;
        let path = app_data_dir.join(format!(".vane-health-{}", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        use std::io::Write;
        file.write_all(b"vane-storage-probe")?;
        file.sync_all()?;
        drop(file);
        std::fs::remove_file(path)
    })();
    let storage_health = if storage_probe.is_ok() {
        SubsystemHealth {
            name: "App Data Directory".into(),
            state: HealthState::Healthy,
            message: "App data directory exists and writable".into(),
            last_checked_ms: now_ms,
        }
    } else {
        SubsystemHealth {
            name: "App Data Directory".into(),
            state: HealthState::Degraded,
            message: "App data directory not created yet".into(),
            last_checked_ms: now_ms,
        }
    };
    subsystems.push(storage_health);

    // Combine overall health
    let mut overall = HealthState::Healthy;
    for sub in &subsystems {
        overall = overall.combine(sub.state);
    }

    SystemHealthSnapshot {
        overall,
        subsystems,
        timestamp_ms: now_ms,
    }
}
