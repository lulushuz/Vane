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
            .unwrap_or(false);

        let iptables_available = Command::new("iptables")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let ip6tables_available = Command::new("ip6tables")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

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
