pub mod doh;
pub mod firewall_plan;
pub mod forwarder;
pub mod forwarder_lifecycle;
pub mod kill_switch;
pub mod manager;
pub mod runtime_config;
pub mod transaction;
pub mod watchdog;

pub const DEFAULT_HEALTH_CHECK_TARGET: &str = "example.com";

pub use doh::{resolve_doh, DohResult, DOH_CLOUDFLARE, DOH_GOOGLE};
pub use firewall_plan::{build_kill_switch_plan, KillSwitchOwnership, KillSwitchPlan};
pub use forwarder::{
    spawn_doh_forwarder, DoHEndpoint, ForwarderHandle, DOH_FORWARDER_DEFAULT_PORT,
};
pub use forwarder_lifecycle::{DnsForwarderIdentity, DnsForwarderState};
pub use kill_switch::{
    clear_kill_switch_metadata, get_or_create_installation_id, recover_orphan_kill_switch_rules,
    save_kill_switch_metadata, PersistedKillSwitchMetadata,
};
pub use manager::{
    apply_dns, builtin_providers, clear_dns_restore_snapshot, get_active_adapters,
    is_using_trusted_dns, recover_stale_dns_snapshot, reset_dns_to_dhcp, restore_dns_snapshot,
    save_dns_restore_snapshot, ApplyDnsResult, DnsProvider, NetworkAdapter,
};
pub use runtime_config::{
    verify_dns_config, DnsConfigCandidate, DnsConfigFingerprint, DnsConfigRevision, DnsProtocol,
    DnsSocksCandidate, DnsValidationError, VerifiedDnsConfig, VerifiedDnsSocks,
};
pub use transaction::{
    AppliedDnsConfig, DnsApplyStage, DnsRuntimeState, DnsTransactionManager, DnsTransactionOutcome,
    PreparedDnsConfig,
};
pub use watchdog::spawn_dns_watchdog;
