# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [2.0.9] - 2026-07-27

### Security
- **Elevated SSRF Boundaries:** Restricted health-check probes to public HTTPS hostnames only, rejecting loopback, link-local, RFC1918 private IPs, credentials, and custom ports.
- **Least-Privilege Process Handles:** Replaced `PROCESS_ALL_ACCESS` Windows process handle requests with minimum required privileges (`PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION`).
- **Updater Keypair Alignment:** Integrated updated Minisign public key into `tauri.conf.json` for release verification.
- **CI Release Hardening:** Enforced strict version-to-tag consistency checks and bundled binary integrity validation prior to packaging; set release workflow to draft mode by default.

### Fixed
- **Typed IPC Contracts:** Centralized and typed all frontend-backend IPC contracts with monotonic revision gating to prevent race conditions or stale async state overwrites.
- **Hickory DNS 0.26 Migration:** Fully migrated DNS resolver subsystem to Hickory 0.26, resolving legacy async DNS protocol deprecations and stabilizing DoH/DoT packet forwarding.
- **Engine Lifecycle Serialization:** Serialized engine start/stop state transitions to eliminate concurrent spawn race conditions and orphaned `winws.exe` processes.
- **Settings & Persistence Reliability:** Enforced array type sanitization for Whitelist and Blacklist domain arrays on rehydration, preventing serialization crashes when loading legacy string formats.

### Added
- **Windows Acceptance Testing Harness:** Added PowerShell-based acceptance test suite (`Invoke-VaneAcceptance.ps1`) for elevated Windows packet capture, WFP firewall validation, and adapter state verification.

---

## [2.0.8] - 2026-07-19

### Security
- Whitelist startup is now fail-closed when persisted settings are corrupt or the verified whitelist is empty.
- Domain rules are canonicalized and validated in Rust; hidden service alias expansion was removed.
- DNS Kill Switch creation checks command results and is rolled back when engine startup fails.
- Unsupported TPWS and IPSet arguments are rejected instead of being passed to the wrong binary.
- Preset-supplied hostlist arguments are rejected so the Pattern screen remains the only authority for DPI scope.
- Health-check targets accept hostnames only, preventing arbitrary URL/network probing through IPC.
- Remote preset and AdBlock downloads now have streaming size/type/validity limits and atomic cache replacement.
- Unused broad shell, process, filesystem, and Store frontend permissions were removed.

### Fixed
- Serialized Pattern and DNS synchronization to prevent overlapping restarts and stale IPC responses.
- Removed competing Rust writes to the Zustand settings file and serialized Store writes.
- Replaced the frontend Store plugin with a Rust-owned, atomically written settings repository, last-known-good backup recovery, schema migration, and stale multi-window merge protection.
- Added a persisted DNS restore snapshot so an interrupted forwarder session restores the exact previous adapter configuration on the next launch.
- Fixed Watchdog being started even when disabled.
- Replaced Watchdog HTTP `HEAD` checks with real DoH/DoT DNS resolution probes.
- Fixed Watchdog recovery resetting every adapter to DHCP instead of restoring the user's previous static/DHCP configuration.
- Fixed DNS TCP forwarding, full-size EDNS responses, negative DoT answers, cache TTL aging, bounded LRU eviction, and cache keys that ignored DNS query options.
- Fixed SOCKS5 leaking resolver lookups or silently falling back to a direct connection; DoH now uses SOCKS5H and incompatible DoT+proxy configurations fail closed.
- Fixed Kill Switch rules blocking Vane's own loopback resolver and added firewall rule verification/rollback.
- Fixed DNS provider selection being persisted before Windows confirmed that the configuration was applied.
- Fixed Advanced numeric/port/cutoff validation, Unicode panic paths, unsafe preset hostlist overrides, unsupported custom payload inputs, and controls that emitted flags absent from the bundled winws.
- Bound TCP Receiver Window to the real `--wssize` flag and replaced invalid built-in `split`/`split2`/OOB strategies with supported bundled-engine modes.
- Fixed custom preset corruption recovery and made preset/domain/cache writes atomic.
- Preserved values containing `=` and safe unknown arguments when editing Advanced presets; invalid numeric arguments are now omitted with an EN/TR warning.
- Added CI verification for frontend builds, Rust tests, and warning-free Clippy on Windows and Linux.

### Changed
- TPWS and IPSet controls are visibly unavailable until their required binary/file-import implementations exist.
- Custom payload controls are visibly unavailable until a safe binary payload picker and format validator are implemented.

### Added
- Added accessible custom select menus for **Bypass Pattern** and **DNS Transport Protocol** controls.
- Added clear, localized EN/TR verification logs for Pattern, DNS transport, Smart DNS Cache, AdBlock, DNS provider, and local DNS Forwarder operations.
- Added backend-confirmed status messages that distinguish settings applied to a running service from settings saved for the next start.

### Changed
- Pattern and DNS settings are now synchronized with the backend immediately before the DPI engine starts.
- Common low-level engine and DNS messages are translated into clearer user-facing explanations while retaining their category and severity.
- DoQ is now shown as unavailable instead of silently using the DoT resolver and reporting an incorrect transport protocol.

### Fixed
- Fixed **Whitelist mode** not reliably limiting DPI bypass to the configured domains.
- Fixed **Blacklist mode** not reliably excluding configured domains from DPI bypass.
- Fixed saved Pattern mode and domain lists being lost, overwritten, or applied too late on the next DPI start.
- Fixed a race where starting DPI immediately after changing Pattern settings could launch Zapret with the previous mode.
- Fixed the main and settings windows overwriting each other's Pattern, DNS Cache, AdBlock, protocol, and proxy values.
- Fixed **Smart DNS Cache** appearing to turn itself back on after being disabled.
- Fixed disabling Smart DNS Cache leaving previously cached DNS records in memory.
- Fixed DNS and Pattern settings being reported as successful before persistence or runtime application was verified.
- Fixed DNS Forwarder stop being reported as successful when the operating system DNS settings could not be restored.
- Fixed invalid domains being accepted into Pattern lists.
- Fixed technical backend log messages being difficult to understand and inconsistently localized between Turkish and English.

---

## [2.0.0] - 2026-07-01

### 🚀 Major Release — Repo Hardening & Documentation Overhaul

### Added
- **Community Health Files**: `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `CHANGELOG.md` — full community infrastructure for open-source contributors.
- **GitHub Issue & PR Templates**: Bug report, feature request, and pull request templates under `.github/`.
- **Binary Integrity Verification**: SHA-256 hash check for `winws.exe` (Windows) and `nfqws` (Linux) at engine startup — prevents binary substitution attacks.
- **Security Contact Information**: `alp@archey.com.tr` and Discord (`852103749228036136`) as official vulnerability reporting channels in `SECURITY.md`.
- **Comprehensive README (EN + TR)**: Full technical documentation in both English and Turkish — covers DPI theory, all zapret desync strategies, every Advanced tab parameter, fooling modes, payload customization, Linux firewall setup, security architecture table, and troubleshooting guide.
- **Remote Preset Sync**: Fetch and cryptographically verify (Minisign) preset definitions from a remote CDN endpoint.
- **TPWS Proxy Mode**: Transparent SOCKS5 proxy via `tpws` as an alternative to raw packet diversion.
- **IPSet File Support**: Target desync rules to specific IP ranges via `--ipset` file path.
- **Advanced Fooling Flags Panel**: Multi-select checkboxes for `badseq`, `badsum`, `md5sig`, `datanoack`, `hopbyhop` (IPv6), `destopt` (IPv6).
- **Custom Payload Fields**: Per-protocol (HTTP/TLS/QUIC) custom fake packet payload strings and file paths.
- **Per-Protocol Desync Overrides**: Independent desync method selectors for HTTP, HTTPS/TLS, and QUIC connections.
- **Second Stage Desync (`--dpi-desync2`)**: Fallback strategy when the primary desync fails.
- **Desync Cutoff (`--dpi-desync-cutoff`)**: Limit desync to the first N data or SYN packets per session.
- **TCP Receiver Window Override (`--tcp-window-size`)**: Force smaller server-sent segments to evade certificate-based inspection.
- **Bind Interface (`--bind-addr`)**: Attach the engine to a specific network interface IP.
- **TLS Split Type (`sni` / `snh`)**: Split TLS ClientHello at the SNI or SNH boundary for precise evasion.
- **Windows Job Object Process Isolation**: `KILL_ON_JOB_CLOSE` flag ensures child processes are terminated on app exit or crash.
- **Network Speed & DNS Traffic Meters**: Real-time upload/download speed and DNS queries-per-second metrics in the Log view.

### Changed
- Removed all `.claude/` AI assistant configuration files from the repository (cleanup).
- Import/Export preset buttons relocated to the Advanced tab header section.
- Kill Switch and Watchdog settings unified under the Advanced tab.
- Default installer mode changed to `perMachine` for system-level binary protection.
- DNS socket read buffer increased from 512 bytes to 4096 bytes (EDNS0 compatibility).

### Security
- Fixed IPC `open_url` command to only accept `http://` and `https://` URI schemes (prevents arbitrary protocol execution).
- Fixed shell command injection in the Linux `pkexec` root wrapper via single-quote argument escaping.
- Enforced 5,000 entry cap on the DNS cache to prevent unbounded memory growth.
- All user-supplied arguments validated against a strict whitelist before being passed to the engine process (`sanitizer.rs`).

### Fixed
- Resolved all NPM advisory CVEs via `npm audit fix`.
- Fixed UI layout and toggle component alignment across DNS, Pattern, and Advanced views.

---

## [1.1.4] - 2026-06-15

### Added
- Expose all advanced Zapret desync parameters in the **Advanced Settings** tab (including HTTP/HTTPS/QUIC protocol-specific methods, desync2 secondary strategy, desync cutoff limits, bind address, and TCP receiver window sizing).
- Multiple checkbox checklists for TCP desync evasion/fooling flags (`badseq`, `badsum`, `md5sig`, `datanoack`, `hopbyhop`, `destopt`).
- Custom payloads & SNI configurations (TLS custom fake SNI domain, custom payload strings/file paths for HTTP/TLS/QUIC fake injections).
- SOCKS5 Transparent Proxy mode using TPWS, and custom IPSet domain ranges list file parsing.
- Cryptographic startup integrity checks: Vane now verifies the SHA-256 hash of `winws.exe` (Windows) and `nfqws` (Linux) before launch to prevent DLL/Binary substitution.

### Changed
- Relocated **Import** and **Export** profile buttons to the top-right header section of the Advanced tab next to the preset dropdown.
- Moved **DNS Leak Protection (Kill Switch)** and **Auto-Recovery Watchdog** settings into the Advanced tab, simplifying the settings sidebar navigation.
- Changed default installation mode to `perMachine` in the installer configuration to protect engine binaries in system protected directories (`Program Files`).
- Expanded DNS socket reading buffers from `512` bytes to `4096` bytes to handle larger EDNS0 DNS packets.

### Fixed
- Fixed critical security bypass in IPC `open_url` command by restricting URIs to safe HTTP/HTTPS schemes.
- Fixed shell command injection breakout on Linux root executor by properly quoting and escaping arguments.
- Fixed unbounded memory growth in DNS Cache by enforcing a 5,000 active entries limit.
- Upgraded local Vite dev dependencies to completely resolve advisory CVEs.
- Fixed UI layout alignments and corrected toggle components styling across DNS, Pattern, and Advanced view containers.
