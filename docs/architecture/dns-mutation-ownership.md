# DNS Mutation Ownership

The authoritative transaction boundary is `DnsTransactionManager::apply_candidate`. Engine startup does not select a provider or mutate system DNS. A provider change requires an explicit user-selected candidate.

| Entry point | Owner | Transaction lock | Snapshot | Apply / verification | Rollback / recovery | Status |
|---|---|---|---|---|---|---|
| `apply_dns_settings` | `DnsTransactionManager` adapter | manager lock | adapter snapshot | verified candidate and local readiness | exact prior applied config | authoritative |
| `reset_dns_settings` | `DnsTransactionManager` adapter | manager lock | current applied config | disabled candidate | exact prior applied config | authoritative |
| `sync_dns_settings` | `DnsTransactionManager` | manager lock | current applied config | verified candidate | transaction rollback | authoritative |
| `start_engine_with_dns_guard` | EngineManager (read-only DNS guard) | none | none | no mutation; emits explicit-selection warning | not applicable | authoritative |
| `start_doh_forwarder` / `stop_doh_forwarder` | legacy forwarder adapter | `dns_sync` | adapter list | loopback DNS plus forwarder readiness | adapter snapshot | BLOCKED: must be folded fully into `DnsTransactionManager` |
| watchdog failure recovery | legacy recovery adapter | forwarder lifecycle | persisted adapter list | no new provider selection | adapter snapshot | BLOCKED: must invoke transaction recovery |
| platform `apply_dns` / `restore_dns_snapshot` | low-level adapter used by the transaction implementation | transaction caller owns lock | exact adapter list | Windows `netsh` / Linux `nmcli` readback | exact adapter list | internal-only target state |
| Kill Switch firewall plan | `DnsTransactionManager` | manager lock | ownership metadata | owned plan/readiness | ownership-scoped reverse plan | authoritative |

## Invariants

- No automatic Cloudflare fallback is permitted during engine startup.
- EngineManager never owns DNS or DNS firewall mutation.
- Kill Switch rules are valid only when committed ownership metadata matches installation, instance, revision, and fingerprint.
- Low-level platform adapters are not public mutation entry points; callers must hold the transaction manager boundary.
- Until the two legacy adapter rows above are removed or redirected, DNS single-owner proof remains **BLOCKED**.
