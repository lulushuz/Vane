use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxPlatformCapabilities {
    pub nftables_available: bool,
    pub nft_atomic_batch: bool,
    pub iptables_available: bool,
    pub ip6tables_available: bool,
    pub nfqueue_available: bool,
    pub comment_match_available: bool,
    pub ipv6_available: bool,
    pub effective_uid: u32,
    pub has_required_privileges: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum LinuxPlatformError {
    #[error("Unsupported platform")]
    UnsupportedPlatform,
    #[error("Capability probe failed: {0}")]
    CapabilityProbeFailed(String),
    #[error("NFQUEUE kernel module/capability is missing")]
    MissingNfqueue,
    #[error("No supported firewall backend found (nftables or iptables required)")]
    MissingFirewallBackend,
    #[error("Insufficient privileges (root or CAP_NET_ADMIN required)")]
    InsufficientPrivileges,
    #[error("Invalid filter intent: {0}")]
    InvalidFilterIntent(String),
    #[error("Invalid queue number: {0}")]
    InvalidQueueNumber(u16),
    #[error("Queue collision detected on queue number {0}")]
    QueueCollision(u16),
    #[error("Plan invariant broken: {0}")]
    PlanInvariant(String),
    #[error("nftables batch execution failed: {0}")]
    NftablesBatchFailed(String),
    #[error("iptables step execution failed: {0}")]
    IptablesStepFailed(String),
    #[error("Partial apply rollback failed: {0}")]
    PartialApplyRollbackFailed(String),
    #[error("Ownership mismatch: expected {expected}, found {found}")]
    OwnershipMismatch { expected: String, found: String },
    #[error("Rule inspection failed: {0}")]
    RuleInspectionFailed(String),
    #[error("Rule removal failed: {0}")]
    RuleRemovalFailed(String),
    #[error("Metadata operation failed: {0}")]
    MetadataFailure(String),
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),
}

pub fn probe_linux_capabilities() -> Result<LinuxPlatformCapabilities, LinuxPlatformError> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let uid_output = Command::new("id").arg("-u").output();
        let effective_uid = match uid_output {
            Ok(out) => {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                s.parse::<u32>().unwrap_or(1000)
            }
            _ => 1000,
        };

        let nftables_available = Command::new("nft")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || std::path::Path::new("/usr/sbin/nft").exists()
            || std::path::Path::new("/sbin/nft").exists();

        let iptables_available = Command::new("iptables")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || std::path::Path::new("/usr/sbin/iptables").exists()
            || std::path::Path::new("/sbin/iptables").exists();

        let ip6tables_available = Command::new("ip6tables")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || std::path::Path::new("/usr/sbin/ip6tables").exists()
            || std::path::Path::new("/sbin/ip6tables").exists();

        let has_required_privileges = effective_uid == 0
            || Command::new("iptables")
                .args(["-t", "mangle", "-L"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            || Command::new("nft")
                .args(["list", "tables"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

        Ok(LinuxPlatformCapabilities {
            nftables_available,
            nft_atomic_batch: nftables_available,
            iptables_available,
            ip6tables_available,
            nfqueue_available: true,
            comment_match_available: true,
            ipv6_available: ip6tables_available,
            effective_uid,
            has_required_privileges,
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(LinuxPlatformCapabilities {
            nftables_available: true,
            nft_atomic_batch: true,
            iptables_available: true,
            ip6tables_available: true,
            nfqueue_available: true,
            comment_match_available: true,
            ipv6_available: true,
            effective_uid: 0,
            has_required_privileges: true,
        })
    }
}

#[cfg(target_os = "linux")]
pub const LINUX_STAGED_NFQWS_PATH: &str = "/var/lib/vane/nfqws-x86_64-unknown-linux-gnu";

#[cfg(target_os = "linux")]
pub fn is_readonly_or_fuse_bundle_path(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains("/.mount_")
        || path_str.starts_with("/tmp/")
        || path_str.starts_with("/run/")
        || std::env::var_os("APPIMAGE").is_some()
}

#[cfg(target_os = "linux")]
pub fn has_nfqws_file_capabilities(nfqws_path: &std::path::Path) -> bool {
    if unsafe { libc::geteuid() == 0 } {
        return true;
    }
    for getcap_bin in ["/usr/sbin/getcap", "/sbin/getcap", "getcap"] {
        if let Ok(output) = std::process::Command::new(getcap_bin)
            .arg(nfqws_path)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
                if stdout.contains("cap_net_admin") && stdout.contains("cap_net_raw") {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(target_os = "linux")]
pub fn ensure_linux_capabilities(app: &tauri::AppHandle) -> Result<(), LinuxPlatformError> {
    use std::path::Path;
    use std::process::Command;

    let (nfqws_path, expected_sha256) =
        crate::engine::EngineManager::resolve_verified_resource_binary(app).map_err(|e| {
            LinuxPlatformError::CapabilityProbeFailed(format!("Failed to resolve nfqws path: {e}"))
        })?;

    let staged_path = Path::new(LINUX_STAGED_NFQWS_PATH);

    if !is_readonly_or_fuse_bundle_path(&nfqws_path) && has_nfqws_file_capabilities(&nfqws_path) {
        return Ok(());
    }

    if staged_path.is_file()
        && crate::engine::EngineManager::verify_binary_hash(staged_path, &expected_sha256).is_ok()
        && has_nfqws_file_capabilities(staged_path)
    {
        return Ok(());
    }

    let setcap_bin = ["/usr/sbin/setcap", "/sbin/setcap", "setcap"]
        .into_iter()
        .find(|candidate| Path::new(candidate).exists())
        .unwrap_or("setcap");

    let temp_stage = std::env::temp_dir().join(format!("vane-nfqws-stage-{}", std::process::id()));
    let _ = std::fs::copy(&nfqws_path, &temp_stage);

    // Assign capabilities only to the isolated nfqws helper binary.
    // If the bundle path is on a writable root filesystem (.deb/.rpm), set capabilities in place;
    // if it is on a read-only/FUSE mount (AppImage or immutable distro), install the SHA-256
    // verified binary into /var/lib/vane and assign file capabilities there.
    let script = if is_readonly_or_fuse_bundle_path(&nfqws_path) {
        "install -d -m 0755 /var/lib/vane && install -o 0 -g 0 -m 0755 \"$2\" \"$4\" && \"$3\" cap_net_admin,cap_net_raw+eip \"$4\""
    } else {
        "(chmod 0755 \"$1\" && \"$3\" cap_net_admin,cap_net_raw+eip \"$1\") || (install -d -m 0755 /var/lib/vane && install -o 0 -g 0 -m 0755 \"$2\" \"$4\" && \"$3\" cap_net_admin,cap_net_raw+eip \"$4\")"
    };

    let status_res = Command::new("pkexec")
        .arg("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg("vane-setcap")
        .arg(&nfqws_path)
        .arg(&temp_stage)
        .arg(setcap_bin)
        .arg(LINUX_STAGED_NFQWS_PATH)
        .status();

    let _ = std::fs::remove_file(&temp_stage);

    let status = status_res.map_err(|e| {
        LinuxPlatformError::CapabilityProbeFailed(format!("pkexec execution failed: {e}"))
    })?;

    let in_place_ready =
        !is_readonly_or_fuse_bundle_path(&nfqws_path) && has_nfqws_file_capabilities(&nfqws_path);
    let staged_ready = staged_path.is_file()
        && crate::engine::EngineManager::verify_binary_hash(staged_path, &expected_sha256).is_ok()
        && has_nfqws_file_capabilities(staged_path);

    if status.success() && (in_place_ready || staged_ready) {
        Ok(())
    } else {
        Err(LinuxPlatformError::InsufficientPrivileges)
    }
}

#[cfg(not(target_os = "linux"))]
pub fn ensure_linux_capabilities(_app: &tauri::AppHandle) -> Result<(), LinuxPlatformError> {
    Ok(())
}
