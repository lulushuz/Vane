<p align="center">
  <img src="src-tauri/icons/icon.png" width="160" alt="Vane Logo">
</p>

<h1 align="center">Vane DPI</h1>

<p align="center">
  <strong>Advanced DPI Bypass and Encrypted DNS Control Center (Powered by Zapret)</strong>
</p>

<p align="center">
  <a href="README.tr.md"><img src="https://img.shields.io/badge/Türkçe-TR-red?style=flat-square&labelColor=black" alt="Türkçe"></a>
  <a href="README.ru.md"><img src="https://img.shields.io/badge/Русский-RU-blue?style=flat-square&labelColor=black" alt="Русский"></a>
  <a href="README.zh.md"><img src="https://img.shields.io/badge/简体中文-ZH-yellow?style=flat-square&labelColor=black" alt="简体中文"></a>
  <a href="README.fa.md"><img src="https://img.shields.io/badge/فارسی-FA-green?style=flat-square&labelColor=black" alt="فارسی"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/lulushuz/Vane/releases.yml?style=flat-square&label=build" alt="Build Status">
  <img src="https://img.shields.io/github/license/lulushuz/Vane?style=flat-square&color=blue" alt="License">
  <img src="https://img.shields.io/github/v/release/lulushuz/Vane?style=flat-square&color=blue&label=release" alt="Release Version">
  <a href="https://discord.com/users/852103749228036136"><img src="https://img.shields.io/badge/Discord-lulushu-5865F2?style=flat-square&logo=discord&logoColor=white" alt="Discord"></a>
</p>

<p align="center">
  <img src="docs/assets/preview.png" width="580" alt="Vane Desktop Interface Preview">
</p>

---

## Table of Contents

- [1. What is Vane?](#1-what-is-vane)
  - [What Vane Automates](#what-vane-automates)
- [2. Core System Features](#2-core-system-features)
  - [2.1. DNS Guard: Local Encrypted DNS Resolver](#21-dns-guard-local-encrypted-dns-resolver)
  - [2.2. AdBlock DNS Filtering](#22-adblock-dns-filtering)
  - [2.3. Dual-Stack DNS Kill Switch](#23-dual-stack-dns-kill-switch)
  - [2.4. Auto-Recovery Watchdog](#24-auto-recovery-watchdog)
  - [2.5. Preset Optimizer](#25-preset-optimizer)
  - [2.6. SOCKS5 Upstream Proxy](#26-socks5-upstream-proxy)
  - [2.7. Remote Preset Synchronization](#27-remote-preset-synchronization)
  - [2.8. Tagged Logging Console](#28-tagged-logging-console)
  - [2.9. Platform-Aware Auto-Start](#29-platform-aware-auto-start)
  - [2.10. Network Change Detection](#210-network-change-detection)
- [3. Security and System Architecture](#3-security-and-system-architecture)
  - [3.1. Supply-Chain and TOCTOU Binary Verification](#31-supply-chain-and-toctou-binary-verification)
  - [3.2. Privilege Isolation and Sandbox Integrity](#32-privilege-isolation-and-sandbox-integrity)
  - [3.3. IPC and WebView Hardening](#33-ipc-and-webview-hardening)
  - [3.4. Privacy-First Diagnostics](#34-privacy-first-diagnostics)
- [4. Built-in Presets and Quick Bypass Guide](#4-built-in-presets-and-quick-bypass-guide)
- [5. Linux and Windows Firewall Architecture](#5-linux-and-windows-firewall-architecture)
  - [5.1. Windows: WinDivert Pipeline](#51-windows-windivert-pipeline)
  - [5.2. Linux: Primary nftables Backend](#52-linux-primary-nftables-backend)
  - [5.3. Linux: iptables Fallback Lifecycle](#53-linux-iptables-fallback-lifecycle)
  - [5.4. Orphaned Filter Recovery](#54-orphaned-filter-recovery)
- [6. Installation and Building from Source](#6-installation-and-building-from-source)
  - [6.1. System Requirements](#61-system-requirements)
  - [6.2. Installation Packages](#62-installation-packages)
  - [6.3. Compilation Steps](#63-compilation-steps)
  - [6.4. Verification Gates](#64-verification-gates)
- [7. Troubleshooting, Limitations, License and Credits](#7-troubleshooting-limitations-license-and-credits)
  - [7.1. Troubleshooting](#71-troubleshooting)
  - [7.2. Technical Limitations](#72-technical-limitations)
  - [7.3. Credits and License](#73-credits-and-license)
  - [7.4. Community and Support](#74-community-and-support)

---

## 1. What is Vane?

Vane is an open-source graphical desktop application that controls the [zapret](https://github.com/bol-van/zapret) packet inspection evasion engine. It wraps the low-level `winws` (Windows) and `nfqws` (Linux) binaries into a lightweight desktop interface built with Tauri v2, Rust, React 19, and Zustand.

**Vane is not a VPN.** It never routes your internet traffic through a remote server or commercial proxy cluster. Instead, it runs on your local workstation, capturing and modifying outgoing connection handshakes at the packet level. These micro-modifications confuse Deep Packet Inspection (DPI) equipment deployed by internet service providers, allowing access to restricted domains while preserving your full native bandwidth and ping.

### What Vane Automates

| Task | Manual Zapret Setup | With Vane |
|------|---------------------|-----------|
| **Binary Integrity** | Unchecked local files | Minisign signature + SHA-256 + pre-spawn TOCTOU verification |
| **Engine Execution** | Long terminal commands with 20+ raw flags | One-click activation with visual state management |
| **DNS Encryption** | Manual OS stub configuration | Built-in DoH / DoT encrypted DNS with transactional rollback |
| **DNS Leak Protection** | External tools or unmanaged leaks | Dual-Stack (IPv4 + IPv6) Kill Switch |
| **Firewall Setup** | Manual driver installation or iptables commands | Automated nftables / iptables / WinDivert rulesets |
| **Crash and Filter Cleanup** | Lingering rules and broken networking | Windows Job Objects + Linux Orphan Recovery (`linux-engine-filter.json`) |
| **Preset Management** | Editing raw shell scripts | Visual editor, `.vane` bundle import/export, signed remote sync |
| **Network Diagnostics** | Manual curl, ping, and traceroute | One-click HTTP connectivity tests and redacted debug bundles |

---

## 2. Core System Features

### 2.1. DNS Guard: Local Encrypted DNS Resolver

DNS Guard runs an embedded local resolver listening on `127.0.0.127:5353`. It catches standard plaintext UDP/53 DNS queries generated by your operating system and web browsers, redirecting them through encrypted transport channels.

| Provider | Supported Protocols | Canonical Endpoint | Status |
|----------|---------------------|--------------------|--------|
| **Cloudflare** | DoH / DoT | `https://cloudflare-dns.com/dns-query` | ✅ Supported |
| **Google** | DoH / DoT | `https://dns.google/dns-query` | ✅ Supported |
| **AdGuard** | DoH | `https://dns.adguard.com/dns-query` | ⏳ Planned |
| **Custom** | DoH / DoT | User-specified URL or IP | ⏳ Planned |

**Protocol and Operational Capabilities:**

| Feature | Status |
|---------|--------|
| DNS-over-HTTPS (DoH) | ✅ Supported |
| DNS-over-TLS (DoT) | ✅ Supported |
| DNS-over-QUIC | ❌ Not supported (Use DoH or DoT) |
| In-memory DNS cache | ✅ Supported (with TTL compliance) |
| Local domain fallback (`.local`, `.lan`) | ✅ Supported |
| Concurrency bounding | 100 parallel asynchronous workers |
| SOCKS5 proxy support for DoH | ✅ Supported |
| Transactional rollback on failure | ✅ Supported (`DnsTransactionManager`) |

> [!NOTE]
> Canonical preset files use the `.vane` extension. Legacy `.json` files remain fully supported for import compatibility.

### 2.2. AdBlock DNS Filtering

DNS Guard bundles the [StevenBlack hosts list](https://github.com/StevenBlack/hosts), cataloging over 100,000 domains known for tracking, telemetries, and advertising. When any local program queries a blacklisted domain, DNS Guard replies immediately with `0.0.0.0` (NXDOMAIN equivalent) without dispatching any upstream network request.

### 2.3. Dual-Stack DNS Kill Switch

Standard DNS encryption leaves your connection vulnerable if an application queries an alternate public DNS server directly over plaintext UDP port 53. Vane provides a Dual-Stack Kill Switch that isolates port 53 traffic:

- **Windows:** Injects Windows Firewall rules using `netsh advfirewall` covering all IPv4 destinations (`0.0.0.0-127.0.0.0,127.0.0.2-255.255.255.255`) and IPv6 destinations (`::-::0,::2-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff`), preserving local loopback traffic (`127.0.0.1` and `::1`).
- **Linux:** Deploys strict `iptables` and `ip6tables` filtering rules in the `OUTPUT` chain via an elevated runner, blocking plaintext outbound DNS to non-loopback destinations.

### 2.4. Auto-Recovery Watchdog

The Watchdog subsystem runs in the background to monitor connection vitality against target domains (such as `discord.com`). If an engine crash occurs or the current bypass parameters stop working on your network, the watchdog initiates an automated recovery routine, restarting the daemon or prompting the preset optimizer.

### 2.5. Preset Optimizer

The Preset Optimizer performs automated testing across your available preset library. It evaluates each candidate configuration against real-world HTTP targets, tracks response latencies, and switches your active session to the configuration with the highest success rate.

### 2.6. SOCKS5 Upstream Proxy

For networks where DNS-over-HTTPS endpoints face IP-level censorship, Vane routes outgoing DoH queries through an authenticated SOCKS5 proxy. This keeps your DNS resolution path hidden from local network monitors.

### 2.7. Remote Preset Synchronization

You can synchronize verified bypass presets from a remote repository or Gist:
- Presets load instantly from local cache during startup without network lag.
- Background tasks verify updates using cryptographic Minisign signatures.
- Built-in default presets cannot be overwritten by remote definitions.

### 2.8. Tagged Logging Console

Vane captures daemon output in real time, sorting standard output and error streams into categorized tags:
- `[MOTOR]`: Low-level zapret engine and packet events.
- `[DNS]`: DNS Guard resolution and cache activity.
- `[ADBLOCK]`: Domain blocks and telemetry sinkholes.
- `[GÜVENLİK]`: Privilege assertions, integrity checks, and sanitization alerts.
- `[SİSTEM]`: Autostart registration and interface switch events.
- `[HATA]`: Unrecoverable errors and critical warnings.

### 2.9. Platform-Aware Auto-Start

Vane adapts its startup integration to the underlying host platform:
- **Windows:** Registers a Windows Task Scheduler job running with highest available privileges (`/rl highest`), allowing WinDivert driver initialization without repeated UAC popups.
- **Linux:** Employs a rootless `systemd --user` unit (`vane.service`), launching Vane within your standard desktop session without requiring global administrative login.

### 2.10. Network Change Detection

Vane listens to operating system network notifications (`WM_DEVICECHANGE` on Windows and netlink route sockets on Linux). When your machine switches from Wi-Fi to Ethernet or joins a new VPN, Vane refreshes internal adapters and reapplies packet filters without dropping active sessions.

---

## 3. Security and System Architecture

```
+--------------------------------------------------------------------------+
|                             Vane Architecture                            |
+--------------------------------------------------------------------------+
|  [ Web UI: React 19 + Zustand ]                                          |
|         |                                                                |
|  [ Tauri IPC Bridge ] -- (Strict CSP, Path & SSRF Allowlisting)          |
|         |                                                                |
|  [ Rust Core: EngineManager, DnsTransactionManager, DiagnosticsStore ]   |
|         |                                                                |
|   +-----+--------------------------------+                               |
|   | (Windows)                            | (Linux)                       |
|   v                                      v                               |
| [ WinDivert64.sys Driver ]         [ nftables / iptables Engine ]        |
| [ winws.exe Daemon ]               [ nfqws Daemon (Isolated setcap) ]    |
| [ Windows Job Object (Auto-Kill) ] [ Orphan Filter Recovery Tracker ]    |
+--------------------------------------------------------------------------+
```

### 3.1. Supply-Chain and TOCTOU Binary Verification

Vane embeds cryptographic SHA-256 digests into its compiled manifest (`ContentArtifactManifest`). Before spawning `winws.exe`, `WinDivert64.sys`, or `nfqws`, the `Sha256ArtifactIntegrityVerifier` reads the target file from disk, confirms file size, and computes its SHA-256 checksum.

This validation executes immediately before process execution, eliminating Time-of-Check to Time-of-Use (TOCTOU) file replacement attacks. Remote updates and preset downloads also require valid Minisign digital signatures.

### 3.2. Privilege Isolation and Sandbox Integrity

- **Linux Capability Restriction:** Root privileges (`pkexec setcap cap_net_admin,cap_net_raw+ep`) apply strictly to the isolated `nfqws` binary. Vane never assigns capabilities to the primary Tauri binary (`current_exe`). Setting capabilities on WebKitGTK executables activates `AT_SECURE=1`, which disables `LD_PRELOAD` and crashes the bubblewrap (`bwrap`) WebKit rendering sandbox. Vane stays completely rootless and isolates privilege grants to the packet daemon.
- **Parametric Command Execution:** Shell invocation (`sh -c`) is eliminated. All administrative actions execute via positional vectors passed directly to `pkexec`.
- **Windows FFI Containment:** Unsafe Rust operations are isolated to three RAII modules: Windows Job Object management (`job.rs`), process token privilege queries, and service controls. The Linux backend contains zero unsafe code blocks outside standard libc bindings.

### 3.3. IPC and WebView Hardening

The frontend interface communicates with the Rust backend through a locked-down IPC layer:
- **Minimal Capability Scope:** The `default.json` capability configuration excludes dangerous APIs such as `shell:allow-execute`, `fs:allow-read`, and `fs:allow-write`. All filesystem operations route through backend handlers with strict parameter checks.
- **Path Traversal Defenses:** Export paths must pass `resolve_safe_export_path`, restricting file writes to authorized user directories (Downloads, Documents, Desktop, AppData) and enforcing valid `.vane` or `.json` extensions.
- **SSRF Protection:** The `get_engine_health` command enforces a maximum of 16 targets (`MAX_HEALTH_TARGETS`) and confirms that every target IP satisfies `is_public_ip`, blocking private IP ranges (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), loopback, and carrier-grade NAT space.
- **Strict Hostlist Validation:** Presets containing `--hostlist` arguments must reference only `.txt` files located inside Vane application data directories.

### 3.4. Privacy-First Diagnostics

Vane includes a built-in diagnostic bundle generator for troubleshooting. Before any diagnostic data is written to disk, the `redaction.rs` engine strips:
- Public IPv4 and IPv6 addresses.
- Usernames, home directory paths, and absolute system locations.
- SOCKS5 credentials, API tokens, and authorization headers.

Diagnostic bundles write to disk via temporary files bearing process identifiers and atomic sequence counters (`.tmp-<pid>-<seq>`), opened with `create_new(true)` to prevent symlink hijack attacks.

---

## 4. Built-in Presets and Quick Bypass Guide

Vane includes pre-configured presets tuned for common internet censorship patterns:

| Preset ID | Preset Name | Target Protocols | Primary Evasion Strategy | Recommended Use Case |
|-----------|-------------|------------------|--------------------------|----------------------|
| `tr-1` | **TR Standard** | TCP 80, 443 | `fake,multidisorder` + `autottl` + `badseq` | General web browsing and news portals |
| `tr-2` | **TR Aggressive** | TCP 80, 443 | `fake,multidisorder` + `md5sig` + fixed TTL | ISPs with stateful TCP reassembly |
| `tr-3` | **TR Fragment** | TCP 80, 443 | `multisplit` + `fakedsplit` | Strict TLS inspection gateways |
| `tr-4` | **TR Desync-HTTPS**| TCP 443 | Custom SNI record split | HTTPS-only web censorship |
| `tr-5` | **TR QUIC** | UDP 443 | `fakeknown` + `udplen` | YouTube 4K streaming and QUIC traffic |
| `discord-voip` | **Discord and VoIP**| UDP 50000-65535 | UDP fake packet injection | Discord voice calls and gaming RTC |

> [!TIP]
> **Technical Reference:** For in-depth packet flow diagrams, TCP segmentation internals, fooling modes (`badsum`, `badseq`, `md5sig`), and complete parameter definitions, consult the [DPI and Zapret Internals Guide](docs/DPI_AND_ZAPRET_INTERNALS.md).

---

## 5. Linux and Windows Firewall Architecture

```
+--------------------------------------------------------------------------+
|                        Linux Packet Processing Loop                      |
+--------------------------------------------------------------------------+
| Outgoing Packet                                                          |
|       |                                                                  |
|       v                                                                  |
| [ nftables / iptables OUTPUT ]                                           |
|   |-- (oifname "lo" return) -------------> Skip local loopback           |
|   |-- (mark 0x40000000 return) ----------> Skip packets sent by nfqws    |
|   +-- (tcp dport 80,443 / udp 50000-65535) -> NFQUEUE num 200 (bypass)   |
|                                                     |                    |
|                                                     v                    |
|                                            [ nfqws daemon ]              |
|                                            Apply desync & reinject       |
+--------------------------------------------------------------------------+
```

### 5.1. Windows: WinDivert Pipeline

On Windows, Vane controls the `winws.exe` executable, which interacts with the network stack using the signed `WinDivert64.sys` kernel driver. Process lifecycles are bound to a Windows Job Object configured with `KILL_ON_JOB_CLOSE`. When Vane exits or terminates unexpectedly, the Windows kernel automatically halts the daemon and releases network filters.

### 5.2. Linux: Primary nftables Backend

On Linux distributions running kernel 5.x or newer, Vane defaults to `nftables`:
- **Atomic Transaction Rules:** Rules are applied using transactional nftables commands.
- **Loopback Exemption:** The rule `oifname "lo" return` prevents disruption to local DNS queries (`127.0.0.127:5353`) and loopback traffic.
- **PortRange Preservation:** Preserves multi-port specifications such as `50000-65535` without truncating port ranges to singular port values.

```nftables
table inet vane_mangle {
    chain output {
        type filter hook output priority mangle; policy accept;
        oifname "lo" return
        mark 0x40000000 return
        tcp dport { 80, 443 } queue num 200 bypass
        udp dport { 443, 50000-65535 } queue num 200 bypass
    }
}
```

### 5.3. Linux: iptables Fallback Lifecycle

On legacy Linux environments where `nftables` is unavailable, Vane initializes an `iptables` ruleset:
1. **Chain Creation:** Creates dedicated chains (`VANE_OUTPUT` and `VANE_INPUT`).
2. **Loop Avoidance:** Adds an early rule checking `mark 0x40000000/0x40000000 -j ACCEPT` to prevent reinjection loops from `nfqws`.
3. **Queue Redirection:** Redirects target ports to `NFQUEUE --queue-num 200 --queue-bypass`.
4. **Clean Teardown:** During shutdown, unlinks chains from `OUTPUT`, flushes rules, and deletes custom chains.

### 5.4. Orphaned Filter Recovery

If Vane or the system experiences a power loss while firewall rules are active, Vane recovers state on the next boot:
- Active firewall parameters are recorded in `linux-engine-filter.json`.
- On startup, Vane reads the file, validates chain names against `^[a-zA-Z0-9_-]{1,32}$`, and removes leftover rules before starting new tasks.

---

## 6. Installation and Building from Source

### 6.1. System Requirements

| Platform | Minimum OS Version | Dependencies | Privileges |
|----------|--------------------|--------------|------------|
| **Windows** | Windows 10 (1809+) or Windows 11 | None (WinDivert driver embedded) | Administrator (UAC elevation) |
| **Linux** | Ubuntu 22.04+, Debian 12+, Fedora 38+, Arch | `libwebkit2gtk-4.1`, `nftables` or `iptables` | Standard user (`pkexec` for nfqws) |

### 6.2. Installation Packages

Pre-compiled production binaries are available on the [GitHub Releases](https://github.com/lulushuz/Vane/releases) page:
- **Windows:** Download and execute `Vane_0.2.0_x64-setup.exe` or the `.msi` package.
- **Linux:** Download `vane_0.2.0_amd64.deb` or the portable `.AppImage` build.

```bash
# Debian / Ubuntu installation
sudo dpkg -i vane_0.2.0_amd64.deb

# AppImage execution
chmod +x Vane_0.2.0_amd64.AppImage
./Vane_0.2.0_amd64.AppImage
```

### 6.3. Compilation Steps

To build Vane from source:

```bash
# 1. Clone repository
git clone https://github.com/lulushuz/Vane.git
cd Vane

# 2. Install JavaScript dependencies
npm install

# 3. Launch development instance with hot-reload
npm run tauri dev

# 4. Compile optimized production release
npm run tauri build
```

### 6.4. Verification Gates

Before submitting code changes, verify your build passes all continuous integration test suites:

```bash
# Execute Rust test suite (372 unit and characterization tests)
cd src-tauri
cargo test --all-targets

# Execute Rust linter
cargo clippy --all-targets -- -D warnings

# Execute frontend unit and contract tests
cd ..
npm test -- --run

# Execute TypeScript type verification
npx tsc --noEmit
```

---

## 7. Troubleshooting, Limitations, License and Credits

### 7.1. Troubleshooting

| Symptom | Probable Cause | Corrective Action |
|---------|----------------|-------------------|
| **Engine fails to start on Windows** | Driver conflict with third-party tools | Exit GoodbyeDPI or other WinDivert services |
| **DNS resolution failure** | Upstream DoH blocked by ISP | Enable SOCKS5 upstream proxy in DNS settings |
| **Linux sandbox error on startup** | Capabilities applied to main binary | Verify setcap is applied only to `nfqws` |
| **Discord voice connection stuck** | UDP port ranges blocked | Select the `discord-voip` preset |
| **High latency during web browsing** | Window size (`wssize`) set too low | Increase `--wssize` value or use standard presets |
| **Rules remain after reboot on Linux** | Previous session crashed uncleanly | Vane removes stale rules automatically on launch |

### 7.2. Technical Limitations

Packet-level evasion operates within physical protocol boundaries:
- **IP Routing Blocks:** If an ISP drops all IP packets routed to a destination address, packet modification cannot restore connectivity. Use a proxy or VPN for IP-level blacklists.
- **Stateful Proxy Reassembly:** ISPs deploying terminating TCP proxies absorb manipulated segments and reconstruct standard streams before forwarding.
- **Root Certificate Inspection:** Evasion tools cannot bypass corporate SSL/TLS inspection proxies where custom CA certificates are installed in your workstation trust store.

### 7.3. Credits and License

- **[zapret](https://github.com/bol-van/zapret)** by bol-van: Packet manipulation core (`winws` and `nfqws`).
- **[WinDivert](https://reqrypt.org/windivert.html)** by basil00: Windows user-mode packet capture driver.
- **[Tauri](https://tauri.app)**: Cross-platform desktop application framework.
- **[StevenBlack/hosts](https://github.com/StevenBlack/hosts)**: Curated AdBlock hosts list.
- **[Minisign](https://jedisct1.github.io/minisign/)**: Cryptographic signature validation.

This project is licensed under the **GNU General Public License v3.0** (`GPL-3.0`). See the [LICENSE](LICENSE) file for complete terms.

### 7.4. Community and Support

- **Bug Reports and Discussions:** [GitHub Issues](https://github.com/lulushuz/Vane/issues)
- **Discord Contact:** [lulushu (Discord ID: 852103749228036136)](https://discord.com/users/852103749228036136)
- **Security Contacts:** Refer to [SECURITY.md](SECURITY.md) for vulnerability disclosure procedures.
