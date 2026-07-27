# Vane 2.0.8 Feature Wiring Matrix

This document tracks the complete end-to-end wiring status of all user-facing UI controls across the entire system stack:
**UI Control → Component → Store → Serializer → IPC → Rust Command → Validation → Runtime Consumer → Observed System Effect → Persistence → Restore**.

---

## Wiring Status Legend
- **Verified:** Fully wired, validated by Rust, applied to runtime, verified on OS/driver level, persisted, and restored.
- **Code-Wired, Runtime Pending:** Fully implemented in source code and unit tests; pending physical Windows packet/driver capture verification.
- **Partially Wired:** Operates in code but has minor edge cases or missing verification steps.
- **Disabled Intentionally:** Feature control is visibly disabled or hidden in UI because underlying binary support is not yet ready.
- **Broken / Misleading:** UI shows feature active, but backend ignores or misapplies it.

---

## Detailed Control Wiring Matrix

| Feature / UI Control | Store Key | IPC Command | Rust Validator | Runtime Consumer | Persistence Key | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Bypass Mode (All)** | `bypassMode` | `sync_bypass_config` | `sanitizer.rs` | `winws.exe` (No hostlist) | `bypassMode` | **Code-Wired, Runtime Pending** |
| **Bypass Mode (Whitelist)** | `bypassMode` | `sync_bypass_config` | `sanitizer.rs` | `winws.exe` (`--hostlist`) | `bypassMode` | **Code-Wired, Runtime Pending** |
| **Bypass Mode (Blacklist)** | `bypassMode` | `sync_bypass_config` | `sanitizer.rs` | `winws.exe` (`--hostlist-exclude`) | `bypassMode` | **Code-Wired, Runtime Pending** |
| **Whitelist Domain Array** | `whitelistDomains` | `sync_bypass_config` | `sanitizer.rs` | `domains.txt` file | `whitelistDomains` | **Code-Wired, Runtime Pending** |
| **Blacklist Domain Array** | `blacklistDomains` | `sync_bypass_config` | `sanitizer.rs` | `domains.txt` file | `blacklistDomains` | **Code-Wired, Runtime Pending** |
| **DNS Protocol (DoH)** | `dnsProtocol` | `sync_dns_settings` | `forwarder.rs` | `LocalDohForwarder` | `dnsProtocol` | **Code-Wired, Runtime Pending** |
| **DNS Protocol (DoT)** | `dnsProtocol` | `sync_dns_settings` | `forwarder.rs` | `LocalDotForwarder` | `dnsProtocol` | **Code-Wired, Runtime Pending** |
| **DNS Protocol (DoQ)** | `dnsProtocol` | N/A | N/A | N/A | N/A | **Disabled Intentionally** |
| **Smart DNS Cache** | `dnsCache` | `sync_dns_settings` | `forwarder.rs` | `LruDnsCache` | `dnsCache` | **Code-Wired, Runtime Pending** |
| **DNS AdBlock Filter** | `dnsAdBlock` | `sync_dns_settings` | `forwarder.rs` | `AdBlockEngine` | `dnsAdBlock` | **Code-Wired, Runtime Pending** |
| **DNS Leak Kill Switch** | `killSwitch` | `sync_bypass_config` | `killswitch.rs` | Windows Firewall API | `killSwitch` | **Code-Wired, Runtime Pending** |
| **Auto-Recovery Watchdog** | `watchdog` | `sync_bypass_config` | `forwarder.rs` | `WatchdogTask` | `watchdog` | **Code-Wired, Runtime Pending** |
| **SOCKS5 Upstream Proxy** | `proxySocks5` | `sync_bypass_config` | `forwarder.rs` | `reqwest` / DoH Client | `proxySocks5` | **Code-Wired, Runtime Pending** |
| **Preset Selection** | `activePresetId` | `start_engine` | `sanitizer.rs` | `EngineManager` / winws | `activePresetId` | **Code-Wired, Runtime Pending** |
| **TCP Window Size** | `advancedConfig` | `start_engine` | `sanitizer.rs` | winws (`--wssize`) | `advancedConfig` | **Code-Wired, Runtime Pending** |
| **TPWS Proxy Control** | N/A | N/A | N/A | N/A | N/A | **Disabled Intentionally** |
| **IPSet Filter Import** | N/A | N/A | N/A | N/A | N/A | **Disabled Intentionally** |
| **Custom Fake Payload** | N/A | N/A | N/A | N/A | N/A | **Disabled Intentionally** |
