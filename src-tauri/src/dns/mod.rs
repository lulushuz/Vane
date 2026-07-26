pub mod doh;
pub mod forwarder;
pub mod manager;
pub mod watchdog;

pub const DEFAULT_HEALTH_CHECK_TARGET: &str = "example.com";

pub use doh::{resolve_doh, DohResult, DOH_CLOUDFLARE, DOH_GOOGLE};
pub use forwarder::{
    spawn_doh_forwarder, DoHEndpoint, ForwarderHandle, DOH_FORWARDER_DEFAULT_PORT,
};
pub use manager::{
    apply_dns, builtin_providers, clear_dns_restore_snapshot, get_active_adapters,
    is_using_trusted_dns, recover_stale_dns_snapshot, reset_dns_to_dhcp, restore_dns_snapshot,
    save_dns_restore_snapshot, ApplyDnsResult, DnsProvider, NetworkAdapter,
};
pub use watchdog::spawn_dns_watchdog;
