<p align="center">
  <img src="src-tauri/icons/icon.png" width="160" alt="Vane Logo">
</p>

<h1 align="center">Vane DPI</h1>

<p align="center">
  <strong>Advanced DPI Bypass & Encrypted DNS Control Center — Powered by Zapret</strong>
</p>

<p align="center">
  <a href="README.tr.md"><img src="https://img.shields.io/badge/lang-tr-blue.svg" alt="tr"></a>
  <img src="https://img.shields.io/github/actions/workflow/status/lulushuz/Vane/releases.yml?style=flat-square&label=build" alt="Build Status">
  <img src="https://img.shields.io/github/license/lulushuz/Vane?style=flat-square&color=blue" alt="License">
  <img src="https://img.shields.io/github/v/release/lulushuz/Vane?style=flat-square" alt="Version">
  <img src="https://img.shields.io/discord/luppux?style=flat-square&logo=discord&color=5865F2" alt="Discord">
</p>

---

## Table of Contents

- [1. What is Vane?](#1-what-is-vane)
- [2. What is Zapret?](#2-what-is-zapret)
- [3. How Deep Packet Inspection (DPI) Works](#3-how-deep-packet-inspection-dpi-works)
  - [3.1. Passive DPI vs Active DPI](#31-passive-dpi-vs-active-dpi)
  - [3.2. SNI and Hostname Extraction](#32-sni-and-hostname-extraction)
  - [3.3. Block Injection — RST and HTTP Redirects](#33-block-injection--rst-and-http-redirects)
  - [3.4. DNS Poisoning and Hijacking](#34-dns-poisoning-and-hijacking)
  - [3.5. Deep Fingerprinting and Behavioral Analysis](#35-deep-fingerprinting-and-behavioral-analysis)
- [4. Zapret Architecture — nfqws / winws Core](#4-zapret-architecture--nfqws--winws-core)
  - [4.1. WinDivert (Windows)](#41-windivert-windows)
  - [4.2. NFQUEUE (Linux)](#42-nfqueue-linux)
  - [4.3. Packet Processing Pipeline](#43-packet-processing-pipeline)
- [5. DPI Desync Strategies](#5-dpi-desync-strategies)
  - [5.1. Desync Phase System — Combo Order](#51-desync-phase-system--combo-order)
  - [5.2. TCP Segmentation Methods](#52-tcp-segmentation-methods)
  - [5.3. fakedsplit and fakeddisorder — altorder Table](#53-fakedsplit-and-fakeddisorder--altorder-table)
  - [5.4. Split Position Markers](#54-split-position-markers)
  - [5.5. Fake Packet Injection](#55-fake-packet-injection)
  - [5.6. Fooling Modes](#56-fooling-modes)
  - [5.7. Fake Payload Customization](#57-fake-payload-customization)
  - [5.8. Sequence Number Overlap (seqovl)](#58-sequence-number-overlap-seqovl)
  - [5.9. IP ID Assignment Schemes](#59-ip-id-assignment-schemes)
  - [5.10. SYNDATA Mode](#510-syndata-mode)
  - [5.11. Original Packet Modding](#511-original-packet-modding)
  - [5.12. Duplicate Packet Injection](#512-duplicate-packet-injection)
  - [5.13. Server-Side Window Manipulation (wssize)](#513-server-side-window-manipulation-wssize)
  - [5.14. UDP / QUIC Desync](#514-udp--quic-desync)
  - [5.15. IP Fragmentation](#515-ip-fragmentation)
- [6. tpws — Transparent TCP Proxy Mode](#6-tpws--transparent-tcp-proxy-mode)
- [7. Desync Parameter Reference Table](#7-desync-parameter-reference-table)
- [7. Fragmented Handshake and Kyber Support](#7-fragmented-handshake-and-kyber-support)
- [8. Connection Tracking (Conntrack)](#8-connection-tracking-conntrack)
- [9. IP Cache Management](#9-ip-cache-management)
- [10. Vane System Features](#10-vane-system-features)
  - [10.1. DNS Guard — Local DoH / DoT / DoQ Forwarder](#101-dns-guard--local-doh--dot--doq-forwarder)
  - [10.2. AdBlock DNS Filtering](#102-adblock-dns-filtering)
  - [10.3. Kill Switch — DNS Leak Protection](#103-kill-switch--dns-leak-protection)
  - [10.4. Auto-Recovery Watchdog](#104-auto-recovery-watchdog)
  - [10.5. Preset Optimizer](#105-preset-optimizer)
  - [10.6. SOCKS5 Upstream Proxy](#106-socks5-upstream-proxy)
  - [10.7. Remote Preset Sync](#107-remote-preset-sync)
  - [10.8. Log Console with Tagged Output](#108-log-console-with-tagged-output)
  - [10.9. Auto-Start and Tray Integration](#109-auto-start-and-tray-integration)
  - [10.10. Network Change Detection](#1010-network-change-detection)
- [11. Built-in Presets](#11-built-in-presets)
- [12. Advanced Configuration — Full Parameter Table](#12-advanced-configuration--full-parameter-table)
- [13. Practical Bypass Strategies](#13-practical-bypass-strategies)
- [14. Firewall Setup — Linux (Iptables / Nftables)](#14-firewall-setup--linux-iptables--nftables)
- [15. Security Architecture](#15-security-architecture)
- [16. Installation](#16-installation)
- [17. Building from Source](#17-building-from-source)
- [18. Troubleshooting](#18-troubleshooting)
- [19. Limitations and When Evasion Fails](#19-limitations-and-when-evasion-fails)
- [20. Credits and License](#20-credits-and-license)
- [21. Community](#21-community)

---

## 1. What is Vane?

Vane is a secure, graphical desktop application that acts as a control center for the [zapret](https://github.com/bol-van/zapret) DPI bypass engine. It wraps the low-level `winws` (Windows) and `nfqws` (Linux) daemons in a modern Tauri v2 + Rust + React/TypeScript interface.

**Vane is NOT a VPN.** It does not route your traffic through a remote server. Instead, it manipulates outgoing and incoming packets at the kernel level to confuse ISP Deep Packet Inspection systems. All traffic remains on your own connection — only the packet structure is modified.

### What Vane Automates

| Task | Without Vane | With Vane |
|------|-------------|-----------|
| Engine startup | Manual CLI with 20+ flags | One click |
| DNS encryption | Manual DoH/DoT configuration | Built-in DNS Guard |
| Firewall rules | Manual iptables/WFP setup | Automatic |
| DNS leak protection | External tools required | Integrated Kill Switch |
| Preset management | Text files | Visual editor + remote sync |
| Binary integrity | Not verified | SHA-256 verified at startup |
| Process cleanup on crash | Manual | Windows Job Object / SIGTERM |

---

## 2. What is Zapret?

[Zapret](https://github.com/bol-van/zapret) is an open-source DPI bypass library and daemon authored by bol-van. It works by capturing outgoing (and sometimes incoming) TCP/UDP packets, applying configurable manipulations, and re-injecting them into the network stack before they are forwarded by the router or ISP's DPI equipment.

### Core Daemons

| Daemon | Platform | Mechanism | Notes |
|--------|----------|-----------|-------|
| `winws` | Windows | WinDivert kernel driver | Requires Administrator privileges |
| `nfqws` | Linux | NFQUEUE netfilter target | Requires iptables/nftables rules |
| `tpws` | Linux | SOCKS5 transparent proxy | No kernel modules required |

### Supported Protocols

| Protocol | Port | Transport | Bypass Methods |
|----------|------|-----------|----------------|
| HTTP | 80 | TCP | Split, disorder, fake |
| HTTPS (TLS 1.2/1.3) | 443 | TCP | Split, fake, seqovl, wssize |
| QUIC (HTTP/3) | 443 | UDP | Fake, udplen, fakeknown |
| VoIP / Discord RTP | 50000-65535 | UDP | Fake, udplen |
| DoH (DNS-over-HTTPS) | 443 | TCP | Managed by DNS Guard |

---

## 3. How Deep Packet Inspection (DPI) Works

To design effective evasion strategies, it is critical to understand the architecture of the inspection systems deployed by ISPs.

### 3.1. Passive DPI vs Active DPI

| Property | Passive DPI | Active DPI |
|----------|-------------|------------|
| Placement | Mirror port / optical tap | Inline (in the data path) |
| Can drop packets | ❌ No | ✅ Yes |
| Can delay packets | ❌ No | ✅ Yes |
| Block method | RST injection / HTTP redirect | Drop, TCP reset, proxy intercept |
| Evasion difficulty | Low to Medium | High |

**Passive DPI** receives a copy of traffic. It races to inject a spoofed TCP RST or HTTP 302 before the server's real response reaches the client. If the fake packet wins the race, the connection is torn down.

**Active DPI** sits directly in the transmission path. It acts as a transparent proxy or a stateful packet filter. It can reconstruct TCP streams, apply regex matching on reassembled payloads, and block connections before they complete.

### 3.2. SNI and Hostname Extraction

```
Client → [TCP SYN] → [TCP SYN-ACK] → [TLS ClientHello]
                                             ↑
                              DPI reads SNI extension here
                              "server_name: blocked.example.com"
```

| Connection Type | Header Inspected | Location in Packet |
|----------------|------------------|--------------------|
| HTTP/1.1 | `Host:` header | Plain text, TCP payload |
| HTTPS (TLS) | `server_name` SNI extension | ClientHello, plain text before encryption |
| HTTP/2 over TLS | SNI in ClientHello | Same as HTTPS |
| QUIC (HTTP/3) | QUIC CRYPTO SNI | First QUIC CRYPTO frame |

### 3.3. Block Injection — RST and HTTP Redirects

When a forbidden domain is identified by the DPI sensor:

1. The sensor generates a **spoofed TCP RST** packet with the server's source IP and port (passive DPI).
2. For HTTP connections, it may inject a **spoofed HTTP 302 redirect** pointing to an ISP warning page.
3. The race condition: if the injected RST/redirect arrives at the client **before** the real server response, the OS terminates the socket.
4. Active DPIs drop the real packet entirely and return their own RST — no race condition needed.

### 3.4. DNS Poisoning and Hijacking

Before a TCP connection can be established, DNS resolution must succeed. ISPs intercept DNS in two common ways:

| Method | Mechanism | Result |
|--------|-----------|--------|
| DNS Hijacking | Intercept UDP/53, return fake IP | Client connects to block portal |
| DNS Poisoning | Inject wrong DNS response in race | Same as hijacking |
| DNS Blocking | Drop DNS request entirely | Connection times out |
| Transparent DNS Proxy | Force all DNS through ISP resolver | ISP resolver returns censored answers |

**Vane's DNS Guard** solves all of these by routing DNS through encrypted DoH/DoT/DoQ tunnels.

### 3.5. Deep Fingerprinting and Behavioral Analysis

Advanced DPI systems supplement SNI reading with behavioral fingerprinting:

| Technique | Description | Countermeasure |
|-----------|-------------|----------------|
| JA3/JA3S fingerprint | Hash of TLS ClientHello parameters | Modify TLS extension ordering |
| TCP fingerprinting | OS identification via TCP option ordering | `--dpi-desync-ttl` + split |
| Flow length analysis | Detect VPN/proxy patterns by packet size | `--udplen`, payload padding |
| Timing correlation | Match encrypted flows with known patterns | Chaos injection via disorder mode |

---

## 4. Zapret Architecture — nfqws / winws Core

### 4.1. WinDivert (Windows)

On Windows, `winws` uses the [WinDivert](https://reqrypt.org/windivert.html) kernel driver. The workflow is:

```
Application → TCP Stack → WinDivert (kernel driver captures packet)
                                   ↓
                         winws.exe (user-space processing)
                                   ↓
                         WinDivert re-inject → Router → Internet
```

WinDivert installs as a Windows kernel driver (`WinDivert64.sys`) with a filter expression that specifies which packets to intercept (e.g., `tcp.DstPort == 443 and outbound`).

### 4.2. NFQUEUE (Linux)

On Linux, `nfqws` uses the kernel's NFQUEUE netfilter target:

```
Application → TCP Stack → iptables NFQUEUE rule → NFQUEUE (kernel)
                                                         ↓
                                             nfqws (user-space processing)
                                                         ↓
                                         NF_ACCEPT / NF_DROP → Router → Internet
```

Traffic is redirected to a user-space queue. nfqws receives each packet, decides to accept (with modifications) or drop it.

### 4.3. Packet Processing Pipeline

```
Incoming Packet
      │
      ├─ Is it TCP? ──→ Parse TCP flags, sequence numbers
      │                        │
      │                        ├─ Is it SYN? ──→ SYNDATA mode
      │                        │
      │                        ├─ Is it application data? ──→ Parse protocol
      │                                │
      │                                ├─ HTTP → Find Host: header
      │                                ├─ TLS  → Find ClientHello + SNI
      │                                └─ QUIC → Find CRYPTO SNI
      │
      ├─ Is it UDP? ──→ QUIC / VoIP handler
      │
      └─ Apply desync strategy → Re-inject modified packets
```

---

## 5. DPI Desync Strategies

### 5.1. Desync Phase System — Combo Order

`--dpi-desync` accepts **up to 3 comma-separated modes**. Each mode belongs to a specific phase and must be written in **ascending phase order**.

| Phase | When It Runs | Available Modes |
|-------|--------------|-----------------|
| **Phase 0** | During TCP connection establishment (SYN/SYN-ACK) | `synack`, `syndata`, `--wssize` |
| **Phase 1** | Before original data — fakes sent first | `fake`, `fakeknown`, `rst`, `rstack`, `hopbyhop`, `destopt`, `ipfrag1` |
| **Phase 2** | Original data sent in a modified way | `multisplit`, `multidisorder`, `fakedsplit`, `fakeddisorder`, `hostfakesplit`, `ipfrag2`, `udplen`, `tamper` |

**Rules:**
- You can combine **one Phase 1 mode + one Phase 2 mode**: `fake,multidisorder`
- You cannot mix two modes from the same phase: ❌ `fake,fakeknown`
- Phase 0 modes (`synack`, `syndata`) can precede Phase 1 + Phase 2 combos: `syndata,fake,multisplit`

**Example combos:**
```
fake,multidisorder          → Phase 1 + Phase 2 (most common)
fake,multisplit             → Phase 1 + Phase 2
syndata,fake,fakedsplit     → Phase 0 + Phase 1 + Phase 2
```

> ⚠️ If you write modes in the wrong order (e.g., `multidisorder,fake`), zapret will reject the configuration.

---

### 5.2. TCP Segmentation Methods

TCP desync works by exploiting the resource constraints of DPI hardware. When a TCP stream is split into unusual fragments or reordered, the DPI's reassembly buffer may fail to process the SNI before the connection is established.

| Method | Phase | Description | Compatibility |
|--------|-------|-------------|---------------|
| `split` | 2 | Split TCP payload at one position | Very High |
| `split2` | 2 | Split at two positions | High |
| `disorder` | 2 | Split and send segments in reverse order | High |
| `disorder2` | 2 | Disorder at two positions | Medium-High |
| `fakedsplit` | 2 | Sequential one-position split with fake mix | High |
| `fakeddisorder` | 2 | Reverse one-position split with fake mix | Medium-High |
| `multisplit` | 2 | Split at N positions (list) | High |
| `multidisorder` | 2 | Multi-position disorder | High |
| `hostfakesplit` | 2 | Split around hostname field with fake host | Medium |
| `fake` | 1 | Inject fake TLS/HTTP/QUIC packet before real | High |
| `fakeknown` | 1 | Inject known-protocol fake (QUIC Initial, WireGuard…) | High |
| `rst` | 1 | Inject fake RST before real packet | Medium |
| `rstack` | 1 | Inject fake RST+ACK before real packet | Medium |
| `syndata` | 0 | Insert data payload inside SYN packet | Medium |
| `synack` | 0 | Perform TCP split handshake manipulation | Low |
| `ipfrag1` | 1 | Add IPv6 fragment header to fakes (IPv6 only) | Medium |
| `ipfrag2` | 2 | IP-level fragmentation of original packets | Medium |

**How disorder works:**

```
Original:    [Seg1: bytes 1-10][Seg2: bytes 11-20][Seg3: bytes 21-30]
             Contains: "example.com" (blocked SNI)

Disordered:  [Seg2][Seg3][Seg1]
Server:      Buffers Seg2, Seg3, then receives Seg1 → reassembles correctly
DPI:         Cannot reassemble → ignores SNI
```

---

### 5.3. fakedsplit and fakeddisorder — altorder Table

`fakedsplit` and `fakeddisorder` surround real TCP segments with fake decoy packets. The **exact sequence** in which fakes and real segments are sent is controlled by the `altorder=N` modifier:

```
--dpi-desync-fakedsplit-mod=altorder=N
```

#### fakedsplit — Multi-packet (with defined split position)

| altorder | Packet sequence sent |
|----------|----------------------|
| `0` (default) | `fake[1st]` → `real[1st]` → `fake[1st]` → `fake[2nd]` → `real[2nd]` → `fake[2nd]` |
| `1` | `real[1st]` → `fake[1st]` → `fake[2nd]` → `real[2nd]` → `fake[2nd]` |
| `2` | `real[1st]` → `fake[2nd]` → `real[2nd]` → `fake[2nd]` |
| `3` | `real[1st]` → `fake[2nd]` → `real[2nd]` |

#### fakeddisorder — Multi-packet (reverse order + fakes)

| altorder | Packet sequence sent |
|----------|----------------------|
| `0` (default) | `fake[2nd]` → `real[2nd]` → `fake[2nd]` → `fake[1st]` → `real[1st]` → `fake[1st]` |
| `1` | `real[2nd]` → `fake[2nd]` → `fake[1st]` → `real[1st]` → `fake[1st]` |
| `2` | `real[2nd]` → `fake[1st]` → `real[1st]` → `fake[1st]` |
| `3` | `real[2nd]` → `fake[1st]` → `real[1st]` |

#### Single-packet mode (no split pos defined)

| altorder | Sequence |
|----------|----------|
| `0` | `fake` → `real` → `fake` |
| `8` | `real` → `fake` |
| `16` | `real` only (fakes disabled) |

Combined values are additive — `altorder=N` for multi-packet + `0`, `8`, or `16` for single-packet part. Most ISPs are defeated by `altorder=0` (default) or `altorder=1`.

> 💡 **Custom preset example:** `--dpi-desync=fake,fakedsplit --dpi-desync-fakedsplit-mod=altorder=1 --dpi-desync-fooling=badseq`

---

### 5.4. Split Position Markers

Split positions define where in the packet the TCP payload is divided.

| Marker | Resolves To | Applicable Protocol |
|--------|-------------|---------------------|
| `method` | Start of HTTP method (GET, POST…) | HTTP |
| `host` | Start of hostname field | HTTP, TLS |
| `endhost` | One byte after the end of hostname | HTTP, TLS |
| `sld` | Start of second-level domain | HTTP, TLS |
| `endsld` | One byte after end of SLD | HTTP, TLS |
| `midsld` | Middle byte of second-level domain | HTTP, TLS |
| `sniext` | Start of SNI extension data | TLS only |
| `0`, `1`, `N` | Absolute byte offset from payload start | Any |
| `method+N` | Method marker + N offset | HTTP |
| `host-N` | Host marker − N offset | HTTP, TLS |

**Example:**
```
--dpi-desync-split-pos=method+2,midsld
```
This splits at `method+2` for HTTP requests and at `midsld` for TLS connections.

### 5.3. Fake Packet Injection

Fake packet injection sends a decoy packet crafted to trigger the DPI's block logic. Once the DPI processes the fake payload, it assumes the session has been handled and stops tracking it. The real packet is then sent normally.

```
Timeline:
  T=0ms  → Client sends fake packet (contains forbidden SNI)
             DPI: "Oh, this is blocked.example.com, activating block rule"
  T=1ms  → Client sends real segmented packets (no complete SNI visible)
             DPI: "Already handled this session, ignoring"
  T=2ms  → Server assembles real packets, connection succeeds
```

**Critical requirement**: The fake packet must not reach the real server (otherwise the server terminates the connection). This is handled by **fooling modes**.

### 5.6. Fooling Modes

Fooling modes are applied to fake packets to prevent them from being accepted by the destination server.

| Mode | Mechanism | Reliability | Notes |
|------|-----------|-------------|-------|
| `ttl` | Set TTL low enough to expire before destination | High | Requires hop counting; doesn't work through TTL-rewriting routers |
| `autottl` | Measure server's incoming TTL, auto-calculate hop distance | Very High | Best default choice; requires server TTL observation |
| `badsum` | Inject incorrect TCP checksum | High | **Fails through most home routers** — see note below |
| `badseq` | Use sequence number outside server's window | High | Default offset: -10000; use `0x80000000` for 100% outside window |
| `md5sig` | Add RFC 2385 MD5 option to TCP header | High | Most non-Linux servers reject MD5; may cause MTU overflow on low split positions |
| `datanoack` | Send fake packet without ACK flag | Medium | May conflict with NAT/masquerade; works correctly with nftables |
| `ts` | Spoofed TCP timestamp causing PAWS rejection | Medium | Requires `net.ipv4.tcp_timestamps=1` on client; enable on Windows with `netsh interface tcp set global timestamps=enabled` |
| `hopbyhop` | Add empty IPv6 Hop-by-Hop extension header | Medium | **IPv6 only fooling** — some ISPs drop these packets |
| `hopbyhop2` | Add two Hop-by-Hop headers (RFC violation) | Medium | All OS discard RFC-violating double hop-by-hop |
| `destopt` | Add IPv6 Destination Options extension header | Medium | **IPv6 only** |

> ⚠️ **`badsum` and home routers**: Most Linux-based home routers (default `sysctl net.netfilter.nf_conntrack_checksum=1`) verify TCP/UDP checksums via conntrack and mark packets with wrong checksum as `INVALID`. iptables FORWARD chain rules that drop `INVALID` packets will silently discard your `badsum` fakes before they even reach the WAN interface. To fix this on your router: `sysctl -w net.netfilter.nf_conntrack_checksum=0`. Note: OpenWRT sets this to `0` by default, so it works there. On hardware-level adapters (e.g., Mediatek MT7621), `badsum` may be enforced at the NIC level and cannot be bypassed in software.

> ⚠️ **`hopbyhop`/`destopt` as fooling vs as desync modes**: These exist in **two different contexts**. As a **fooling mode** (`--dpi-desync-fooling=hopbyhop`), they add an extension header to fake packets only. As a **desync mode** (`--dpi-desync=hopbyhop`), they add the extension header to **all** desynced original packets. These are different use cases.

**AutoTTL Explained:**

```
Server sends packet with TTL=119 (started at 128, 9 hops away)
autottl calculates: 128 - 119 = 9 hops
Fake packet gets TTL = 9 - delta (expires before destination)
```

### 5.5. Fake Payload Customization

| Option | Effect |
|--------|--------|
| `rnd` | Randomize TLS Random field and Session ID on every request |
| `rndsni` | Replace SNI extension with random SLD + random TLD |
| `dupsid` | Copy Session ID from the original ClientHello into the fake packet |
| `sni=domain` | Replace SNI in fake packet with `domain` (auto-adjusts length headers) |
| `padencap` | Extend fake padding extension to match the real packet's size |
| `oob` | Send out-of-band data (TCP Urgent flag) in the fake packet |

### 5.6. Sequence Number Overlap (seqovl)

The `seqovl` technique creates intentional overlapping TCP sequence ranges to confuse stateful DPI engines.

```
Fake segment:   [seq=100, len=10] → payload: garbage_data
Real segment:   [seq=105, len=10] → payload: real_sni_data

DPI sees: garbage_data (first received, registered as canonical)
Server:   May prioritize second-received or later data depending on OS
```

> ⚠️ Windows-hosted servers generally do not preserve overlaps the same way Linux/BSD servers do. `seqovl` reliability varies against Windows endpoints.

### 5.7. IP ID Assignment Schemes

| Mode | Description | Use Case |
|------|-------------|----------|
| `seq` | Increment IP ID for each injected packet | Default |
| `seqgroup` | Match IP ID of fake segment to its original | Stateful DPI evasion |
| `rnd` | Random IP ID on each packet | Anti-fingerprinting |
| `zero` | Force IP ID to 0 | Linux/BSD hosts only |

### 5.8. SYNDATA Mode

Normally, TCP SYN packets carry no payload. SYNDATA inserts data inside the SYN packet.

```
Standard SYN:  [SYN flag][no data]
SYNDATA SYN:   [SYN flag][16 null bytes or custom payload]
```

- The destination OS discards the SYN payload unless **TCP Fast Open (TFO)** is negotiated.
- The DPI, however, attempts to parse the SYN payload, causing its session state machine to desynchronize from the real handshake.

### 5.9. Original Packet Modding

You can modify TTL and IP ID fields of **real** data packets (not just fakes):

| Parameter | Function |
|-----------|----------|
| `--orig-ttl=N` | Set real packet TTL to N |
| `--orig-autottl` | Auto-calculate TTL for real packet (same logic as fake autottl) |

This forces the DPI to compute incorrect hop distances when comparing the real and fake flows.

### 5.10. Duplicate Packet Injection

`--dup=N` sends N duplicate copies of original packets before the real packet.

| Parameter | Effect |
|-----------|--------|
| `--dup=1` | Send 1 duplicate before real packet |
| `--dup-ttl=N` | Apply TTL N to duplicates |
| `--dup-autottl` | Auto-calculate TTL for duplicates |
| `--dup-badseq` | Apply bad sequence number to duplicates |

**Purpose**: Force the DPI to process contradictory copies of the same packet, causing it to drop session tracking.

### 5.13. Server-Side Window Manipulation (wssize)

Some DPIs inspect not only the ClientHello but also the **server's TLS ServerHello and Certificate** to identify forbidden domains. `wssize` defeats this by advertising a tiny TCP receive window to the server, forcing it to fragment its response into small pieces the DPI cannot reassemble.

```
Client → [SYN, Window=65535] → Server
Client ← [SYN-ACK] ← Server
Client → [ACK, Window=1] → Server  (wssize active: advertise window=1)
Server: "Receive window is 1 byte, I must fragment my response"
DPI:    "Cannot read complete ServerHello Certificate — abandoning inspection"
```

| Parameter | Description |
|-----------|-------------|
| `--wssize=1:6` | Recommend: window=1, scale=6 (effective window = 1×2⁶ = 64 bytes) |
| `--wssize=0:0` | Maximum restriction (1 byte, no scaling) |

**wssize-cutoff — when does wssize stop?**

`wssize` must stop after the TLS ClientHello is sent, otherwise it cripples the entire download speed. Conntrack handles this automatically:

| Trigger | Behavior |
|---------|----------|
| HTTP request detected | wssize stops immediately (auto forced cutoff) |
| TLS ClientHello detected | wssize stops immediately (auto forced cutoff) |
| `--wssize-cutoff=d3` | Stop after 3rd data packet (manual for non-HTTP/TLS protocols) |
| `--wssize-forced-cutoff=0` | Disable auto stop — wssize runs indefinitely (use with care) |

> ⚠️ `wssize` is **not applied in desync profiles that use a hostlist filter**, because it must activate at connection initiation before the hostname is known. It works correctly with auto-hostlist profiles since those use the IP cache.

### 5.14. UDP / QUIC Desync

QUIC (HTTP/3) uses UDP and carries the connection's SNI in the first QUIC CRYPTO frame. UDP cannot be fragmented at the transport layer — only at the IP layer.

| Method | Description | Protocol |
|--------|-------------|----------|
| `fake` | Inject fake UDP packet with arbitrary payload before real | Any UDP |
| `fakeknown` | Inject protocol-aware fake (QUIC Initial, WireGuard handshake, Discord IP Discovery, STUN…) | Protocol-specific |
| `udplen` | Increase UDP payload length by N bytes (`--dpi-desync-udplen-increment=N`) | Any UDP |
| `ipfrag2` | IP-level fragmentation of original UDP packets | Any UDP |
| `hopbyhop` | Add IPv6 Hop-by-Hop extension to UDP packets | IPv6 UDP only |
| `destopt` | Add IPv6 Destination Options to UDP packets | IPv6 UDP only |

`udplen` fills the extra bytes with zeroes by default. A custom fill pattern can be set with `--dpi-desync-udplen-pattern`. This bypasses DPIs that track outgoing UDP payload sizes.

---

### 5.15. IP Fragmentation

IP fragmentation splits packets at the **network (IP) layer** rather than the TCP layer. This is a lower-level form of evasion that some DPIs cannot handle.

| Mode | Description | Phase |
|------|-------------|-------|
| `ipfrag1` | Add IPv6 Fragment extension header to **fake** packets | 1 (with Phase 2) |
| `ipfrag2` | Fragment **original** packets at the IP level | 2 |

**TCP:** `--dpi-desync-ipfrag-pos-tcp=N` — fragment position from transport header (must be multiple of 8, default 32)

**UDP:** `--dpi-desync-ipfrag-pos-udp=N` — fragment position from transport header (must be multiple of 8, default 8)

Combinations like `hopbyhop,ipfrag2` produce: `IPv6 → Hop-by-Hop → Fragment → TCP/UDP` — the DPI sees the Hop-by-Hop header first and may not walk the chain to the transport header.

> ⚠️ `ipfrag1+ipfrag2` cannot be combined. `ipfrag2` requires that the kernel allows sending pre-fragmented packets (may need `echo 1 > /proc/sys/net/ipv4/ip_no_pmtu_disc`).

---

## 6. tpws — Transparent TCP Proxy Mode

`tpws` is an alternative to `nfqws`/`winws`. Instead of operating at the packet level via kernel hooks, it runs as a **SOCKS5 transparent proxy** in user space. Vane exposes this via the **TPWS Proxy Mode** toggle in the Advanced tab.

### nfqws vs tpws

| Property | nfqws / winws | tpws |
|----------|--------------|------|
| Level | Packet (kernel + user space) | Stream (TCP proxy) |
| Kernel module required | Yes (WinDivert / NFQUEUE) | No |
| Can modify IP headers | Yes (TTL, IP ID, IP flags) | No |
| Can do IP fragmentation | Yes | No |
| Can send fakes with bad TTL | Yes | No |
| TCP segmentation | Yes | Yes (`--split-pos`) |
| TLS record splitting | No | Yes (`--tlsrec`) |
| MSS manipulation | Yes (`--mss`) | Yes (`--mss`) |
| Privilege required | Root / Administrator | No root needed |
| Performance | Higher (kernel path) | Slightly lower |

### When to use tpws

- When you **cannot load a kernel driver** (e.g., locked-down Linux environment).
- When **simple TCP splitting** at the stream level is sufficient for your ISP.
- When you want to **avoid Administrator/root requirements** on the host.
- As a **fallback** when nfqws/winws fails due to driver conflicts.

### tpws Specific Features

**TLSREC — TLS Record Splitting:**

tpws can split at the **TLS record layer** (inside the TLS record boundary) rather than the TCP segment layer. This is invisible to the TCP layer and can fool DPIs that reassemble TCP but do not reassemble TLS records.

```
--tlsrec=sni
```
Splits TLS records at the SNI boundary. The DPI receives two partial TLS records and cannot extract the SNI.

**MSS — Maximum Segment Size:**

```
--mss=256
```
Advertises a small MSS during the TCP handshake, forcing the kernel to send small TCP segments. Unlike packet-level splitting, this works at the OS TCP stack level.

**Split Positions in tpws:**

tpws supports the same split position markers as nfqws: `method`, `host`, `endhost`, `midsld`, `sniext`, and absolute offsets.

### tpws Traffic Redirection (Linux)

With iptables (redirect to tpws listening on port 1080):
```bash
iptables -A OUTPUT -p tcp -m multiport --dports 80,443 \
  -j REDIRECT --to-ports 1080
```

With nftables:
```nftables
table ip tpws_redir {
    chain output {
        type nat hook output priority -100;
        tcp dport { 80, 443 } redirect to :1080
    }
}
```

---

## 7. Desync Parameter Reference Table

Full reference for all zapret parameters exposed in Vane's Advanced tab:

| Parameter | Values | Default | Description |
|-----------|--------|---------|-------------|
| `--dpi-desync` | `split`,`split2`,`disorder`,`disorder2`,`fake`,`multisplit`,`multidisorder`... | — | Primary desync method(s), comma-separated |
| `--dpi-desync2` | Same as above | — | Secondary desync method for established connections |
| `--dpi-desync-split-pos` | Marker or integer list | `2` | Split position(s) |
| `--dpi-desync-split-http-req` | `none`,`method`,`host` | `none` | Specific HTTP request split position |
| `--dpi-desync-split-pos-http-req` | Integer | — | Byte offset for HTTP request split |
| `--dpi-desync-split-tls` | `none`,`sni`,`snh` | `none` | Specific TLS split position |
| `--dpi-desync-split-pos-tls` | Integer | — | Byte offset for TLS split |
| `--dpi-desync-fooling` | `badsum`,`badseq`,`md5sig`,`ts`,`datanoack`,`hopbyhop`,`destopt` | — | Fake packet fooling mode(s) |
| `--dpi-desync-autottl` | `[-]N:N-N` | — | Auto-calculate TTL bounds |
| `--dpi-desync-ttl` | Integer | — | Fixed TTL for fake packets |
| `--dpi-desync-ttl-ext` | Integer | — | Additional TTL offset |
| `--dpi-desync-repeats` | Integer | `1` | Number of fake packets per real packet |
| `--dpi-desync-any-protocol` | Flag | Off | Apply desync to all TCP connections (not just HTTP/TLS) |
| `--dpi-desync-cutoff` | `d1`-`d9`, `s1`-`sN` | — | Apply desync only to first N data/SYN packets |
| `--dpi-desync-fake-tls-sni` | domain | — | Custom SNI in fake TLS ClientHello |
| `--dpi-desync-fake-http` | string or file path | — | Custom HTTP payload for fake packets |
| `--dpi-desync-fake-tls` | string or file path | — | Custom TLS ClientHello payload |
| `--dpi-desync-fake-quic` | string or file path | — | Custom QUIC payload |
| `--dpi-desync-http` | same as `--dpi-desync` | — | Override method for HTTP connections |
| `--dpi-desync-https` | same as `--dpi-desync` | — | Override method for HTTPS/TLS connections |
| `--dpi-desync-quic` | same as `--dpi-desync` | — | Override method for QUIC/UDP connections |
| `--mss` | Integer | — | TCP Maximum Segment Size override |
| `--tcp-window-size` | Integer | — | TCP Window Size for sent packets |
| `--wssize` | `N:N` | — | Window scale factor advertised to server |
| `--wf-tcp` | port list | — | TCP ports to intercept |
| `--wf-udp` | port list | — | UDP ports to intercept |
| `--ipset` | file path | — | IP allowlist/blocklist file |
| `--bind-addr` | IP address | — | Bind to specific network interface |
| `--ipcache-lifetime` | Seconds | 7200 | IP cache entry TTL |
| `--dup` | Integer | — | Number of duplicate packets per original |

---

## 7. Fragmented Handshake and Kyber Support

Modern browsers (Chrome 124+, Firefox 126+) use **ML-KEM (Kyber)** for post-quantum key encapsulation. This increases the TLS ClientHello size dramatically:

| ClientHello Type | Typical Size | Fits in One Packet? |
|-----------------|-------------|---------------------|
| Classic TLS 1.3 | ~300 bytes | ✅ Yes (MTU ~1500 bytes) |
| TLS 1.3 + Kyber768 | ~1500-2000 bytes | ❌ No, split across 2+ packets |

**How Vane handles this:**

1. Captures the first TCP segment of the handshake.
2. Detects multi-packet ClientHello by checking `handshake_length > (packet_size - headers)`.
3. Buffers all fragments until the full ClientHello is received.
4. Applies the configured desync strategy across the reassembled message.
5. Re-injects modified segments in the correct order.

This ensures Kyber-extended ClientHello SNI is properly hidden even when the SNI spans packet boundaries.

---

## 8. Connection Tracking (Conntrack)

Vane's internal conntrack module tracks live TCP and UDP sessions to enable multi-packet operations like wssize and fragmented handshake support.

| Feature | Details |
|---------|---------|
| TCP state tracking | SYN → ESTABLISHED → FIN / RST |
| UDP flow tracking | Source IP/port + Destination IP/port keyed |
| Inactive timeout | Configurable; default: 60s (UDP), 120s (TCP established) |
| Max table size | Bounded to prevent memory exhaustion |
| Diagnostic dump | Send `SIGUSR1` to daemon process to print conntrack table |

---

## 9. IP Cache Management

The IP cache stores previously computed hop distances for destination IPs, enabling instant autottl calibration from the very first packet of a new session.

| Property | Value |
|----------|-------|
| Cache key | Destination IP + network interface |
| Cache value | Observed TTL, computed hop distance, hostname |
| Default lifetime | 7200 seconds (2 hours) |
| Override | `--ipcache-lifetime=N` |
| Eviction policy | LRU; oldest entries evicted when capacity is exceeded |

---

## 10. Vane System Features

### 10.1. DNS Guard — Local DoH / DoT / DoQ Forwarder

DNS Guard runs a local resolver on `127.0.0.127:5353`. It intercepts standard UDP/53 DNS queries and forwards them over encrypted channels.

| Provider | Protocol | Endpoint |
|----------|----------|----------|
| Cloudflare | DoH | `https://cloudflare-dns.com/dns-query` |
| Google | DoH | `https://dns.google/dns-query` |
| AdGuard | DoH | `https://dns.adguard.com/dns-query` |
| NextDNS | DoH | Custom via configuration |
| Custom | DoH | User-defined URL |

**Feature Summary:**

| Feature | Status |
|---------|--------|
| DNS-over-HTTPS | ✅ |
| DNS-over-TLS | ✅ |
| DNS-over-QUIC | ✅ |
| In-memory DNS cache | ✅ |
| Cache TTL respect | ✅ |
| Local domain fallback (`.local`, `.lan`) | ✅ |
| Concurrency limit (100 parallel requests) | ✅ |
| SOCKS5 proxy for DoH queries | ✅ |

### 10.2. AdBlock DNS Filtering

DNS Guard integrates the [StevenBlack hosts list](https://github.com/StevenBlack/hosts) (~100,000+ domains) for DNS-level blocking.

| Category | Blocked |
|----------|---------|
| Advertising networks | ✅ |
| Telemetry and analytics | ✅ |
| Malware / phishing domains | ✅ |
| Social media trackers | Optional |

When a blocked domain is queried, DNS Guard returns `0.0.0.0` (NXDOMAIN-equivalent) instead of forwarding the query.

### 10.3. Kill Switch — DNS Leak Protection

The Kill Switch blocks outbound UDP/TCP port 53 traffic using native OS APIs:

| OS | Mechanism |
|----|-----------|
| Windows | Windows Filtering Platform (WFP) callout driver |
| Linux | iptables OUTPUT chain rule |

When enabled, all DNS queries outside the encrypted DNS Guard tunnel are silently dropped. This prevents ISP-level DNS interception regardless of application configuration.

### 10.4. Auto-Recovery Watchdog

The Watchdog continuously monitors connectivity to configurable target domains (default: `discord.com`).

| Trigger Condition | Action |
|-------------------|--------|
| HTTP HEAD request fails | Run preset optimizer |
| Optimizer finds better preset | Switch to optimal preset |
| ICMP ping timeout | Log warning + retry |
| Engine process crash | Restart engine with same preset |

### 10.5. Preset Optimizer

The Optimizer tests all available presets against live connectivity targets to find the most effective configuration for the current network.

**Process:**
1. Stop current engine session.
2. Iterate through all presets in priority order.
3. For each preset, start the engine and run an HTTP HEAD probe to the test target.
4. Select the first preset that returns HTTP < 400.
5. Switch to the winning preset and resume normal operation.

### 10.6. SOCKS5 Upstream Proxy

Allows tunneling DNS Guard's outgoing DoH queries through a SOCKS5 proxy:

```
DNS Query → DNS Guard → SOCKS5 Proxy → Internet → DoH Server
```

This conceals the encrypted DNS lookup path from the ISP, useful in environments where DoH endpoints are also blocked.

### 10.7. Remote Preset Sync

Vane can fetch preset definitions from a remote JSON endpoint (GitHub Gist or CDN). The remote presets:

- Are loaded at startup if a cached copy exists (zero network I/O).
- Are refreshed in the background after startup (non-blocking).
- Are verified with Minisign cryptographic signatures to prevent tampering.
- Cannot overwrite built-in presets (ID collision protection).

### 10.8. Log Console with Tagged Output

Vane parses all process stdout/stderr and classifies lines into tagged categories:

| Tag | Color | Source |
|-----|-------|--------|
| `[MOTOR]` | Purple | Zapret engine process |
| `[DNS]` | Green | DNS Guard forwarder |
| `[ADBLOCK]` | Red | DNS filtering events |
| `[GÜVENLİK]` | Yellow | Privilege checks, sanitization |
| `[SİSTEM]` | Blue | Autostart, network changes |
| `[HATA]` | Red | Process errors, critical failures |
| `[UYARI]` | Amber | Non-critical warnings |

### 10.9. Auto-Start and Tray Integration

When auto-start is enabled, Vane registers a Windows Task Scheduler entry (`--autostart` flag). On startup:

1. Reads the last active preset ID from the persisted settings file.
2. Checks if the system DNS is trusted; applies Cloudflare DNS if not.
3. Silently starts the engine with the saved preset.
4. Hides the main window; shows the system tray icon.

### 10.10. Network Change Detection

Vane listens for `WM_DEVICECHANGE` (Windows) or netlink socket events (Linux) to detect network adapter changes.

- When a new adapter is connected, a `network_changed` event is emitted.
- The frontend UI refreshes DNS adapter status and network statistics.
- WinDivert automatically applies its capture filters to new adapters — no engine restart required.

---

## 11. Built-in Presets

| ID | Label | Strategy | Target |
|----|-------|----------|--------|
| `tr-1` | TR Standard | `fake,multidisorder` + `autottl` + `badseq` | Turkey ISPs |
| `tr-2` | TR Aggressive | `fake,multidisorder` + `md5sig` + fixed TTL | Strict Turkey ISPs |
| `tr-3` | TR Fragment | `multisplit` + `fakedsplit` | Fragment-based bypass |
| `tr-4` | TR Desync-HTTPS | HTTPS-specific desync with custom split | HTTPS-only inspection |
| `tr-5` | TR QUIC | UDP 443 + `fakeknown` | YouTube QUIC streams |
| `discord-voip` | Discord & VoIP Fix | UDP 50000-65535 fake injection | Voice chat stability |

---

## 12. Advanced Configuration — Full Parameter Table

The following parameters are exposed in Vane's Advanced Settings tab:

### DPI Desync

| Setting | Parameter | Values | Description |
|---------|-----------|--------|-------------|
| Desync Method | `--dpi-desync` | `split`, `disorder`, `fake`, `multisplit`, `multidisorder`... | Primary bypass method |
| Secondary Method | `--dpi-desync2` | Same as above | Applied after first method |
| Split Position | `--dpi-desync-split-pos` | Marker or integer list | Where to cut the TCP payload |
| HTTP Split Target | `--dpi-desync-split-http-req` | `none`, `method`, `host` | HTTP-specific split anchor |
| TLS Split Target | `--dpi-desync-split-tls` | `none`, `sni`, `snh` | TLS-specific split anchor |
| Fooling Mode | `--dpi-desync-fooling` | `badsum`, `badseq`, `md5sig`, `ts`, `datanoack`... | Prevent fake reaching server |
| Auto TTL | `--dpi-desync-autottl` | `[-]N:N-N` | Dynamically calibrate TTL |
| Fixed TTL | `--dpi-desync-ttl` | Integer | Fixed fake TTL value |
| Extended TTL | `--dpi-desync-ttl-ext` | Integer | Offset added to TTL |
| Fake Repeats | `--dpi-desync-repeats` | Integer | Number of fake packets |
| Any Protocol | `--dpi-desync-any-protocol` | Flag | Apply to all TCP, not just HTTP/TLS |
| Cutoff | `--dpi-desync-cutoff` | `d1`-`d9`, `s1`-`sN` | Limit desync to first N packets |

### Per-Protocol Override

| Setting | Parameter | Description |
|---------|-----------|-------------|
| HTTP Method | `--dpi-desync-http` | Override desync for HTTP traffic only |
| HTTPS Method | `--dpi-desync-https` | Override desync for HTTPS/TLS traffic only |
| QUIC Method | `--dpi-desync-quic` | Override desync for QUIC/UDP traffic only |

### Fake Payload

| Setting | Parameter | Description |
|---------|-----------|-------------|
| Custom TLS SNI | `--dpi-desync-fake-tls-sni` | Domain name for fake TLS ClientHello SNI |
| Fake HTTP Payload | `--dpi-desync-fake-http` | String or path to file for HTTP fakes |
| Fake TLS Payload | `--dpi-desync-fake-tls` | String or path to file for TLS fakes |
| Fake QUIC Payload | `--dpi-desync-fake-quic` | String or path to file for QUIC fakes |

### Packet & Traffic

| Setting | Parameter | Description |
|---------|-----------|-------------|
| MSS Override | `--mss` | TCP Maximum Segment Size |
| TCP Window Size | `--tcp-window-size` | Override TCP window in sent packets |
| Server Window Scale | `--wssize` | Restrict window advertised to server |

### Protocol & Ports

| Setting | Parameter | Example | Description |
|---------|-----------|---------|-------------|
| TCP Ports | `--wf-tcp` | `80,443` | TCP ports to capture |
| UDP Ports | `--wf-udp` | `443` | UDP ports to capture |
| QUIC UDP | `--wf-udp=443` | Flag | Enable QUIC bypass |

### System

| Setting | Parameter | Description |
|---------|-----------|-------------|
| IP List | `--ipset` | Path to file with target IP ranges |
| Bind Interface | `--bind-addr` | Bind engine to specific network interface IP |
| TPWS Mode | `--tpws` | Use SOCKS5 transparent proxy mode instead of nfqws |

---

## 13. Practical Bypass Strategies

### Strategy 1: Basic Split (Safe, Maximum Compatibility)

Best for passive DPI and home routers. No fake packets.

```
--wf-tcp=80,443 --dpi-desync=split --dpi-desync-split-pos=2
```

### Strategy 2: AutoTTL Fake + Disorder (Default — Turkey)

```
--wf-tcp=80,443 --wf-udp=443 --dpi-desync=fake,multidisorder
--dpi-desync-autottl=-1:3-20 --dpi-desync-fooling=badseq
--dpi-desync-any-protocol --dpi-desync-cutoff=d3
```

**Mechanism:**
1. Measures server TTL to calibrate autottl.
2. Sends fake packet with calibrated TTL and bad sequence number.
3. Sends remaining real segments in disorder order.
4. Applied only to first 3 data packets per session (`cutoff=d3`).

### Strategy 3: MD5 Signature + Split (Heavy ISP Evasion)

```
--wf-tcp=80,443 --wf-udp=443 --dpi-desync=fake,multidisorder
--dpi-desync-fooling=md5sig --dpi-desync-autottl
--dpi-desync-split-pos=4 --dpi-desync-any-protocol
```

### Strategy 4: QUIC + VoIP Bypass

```
--wf-udp=443,50000-65535 --dpi-desync=fake --dpi-desync-repeats=2
--dpi-desync-fooling=badseq --dpi-desync-any-protocol
```

### Strategy 5: wssize (Server-Side Certificate Hiding)

```
--wf-tcp=443 --wssize=1:6 --dpi-desync=split --dpi-desync-split-pos=2
```

### Strategy 6: Fragment Method

```
--wf-tcp=80,443 --dpi-desync=multisplit,fakedsplit
--dpi-desync-split-pos=2,4 --dpi-desync-fooling=badseq
```

---

## 14. Firewall Setup — Linux (Iptables / Nftables)

### Iptables

```bash
# Outgoing TCP (HTTP + HTTPS)
iptables -A OUTPUT -p tcp -m multiport --dports 80,443 \
  -j NFQUEUE --queue-num 200 --queue-bypass

# Incoming TCP (required for autottl to observe server TTL)
iptables -A INPUT -p tcp -m multiport --sports 80,443 \
  -j NFQUEUE --queue-num 200 --queue-bypass

# Outgoing QUIC (HTTP/3)
iptables -A OUTPUT -p udp --dport 443 \
  -j NFQUEUE --queue-num 200 --queue-bypass
```

### Nftables

```nftables
table ip vane_mangle {
    chain output {
        type filter hook output priority mangle; policy accept;
        tcp dport { 80, 443 } queue num 200
        udp dport 443 queue num 200
    }
    chain input {
        type filter hook input priority mangle; policy accept;
        tcp sport { 80, 443 } queue num 200
    }
}
```

> Vane on Linux handles these rules automatically when the engine is started.

### Cleanup (on engine stop)

```bash
nft delete table ip vane_mangle
# or
iptables -D OUTPUT -p tcp -m multiport --dports 80,443 -j NFQUEUE --queue-num 200
```

---

## 15. Security Architecture

Vane implements multiple layers of security controls:

| Control | Mechanism | Location |
|---------|-----------|----------|
| IPC whitelist | All Tauri commands validate URL schemes and preset IDs | `commands.rs` |
| Argument sanitization | Strict whitelist — only known zapret parameters accepted | `sanitizer.rs` |
| Shell injection prevention | Single-quote escaping of all arguments in Linux root wrapper | `manager.rs` |
| Binary integrity | SHA-256 verification of `winws.exe` / `nfqws` before execution | `manager.rs` |
| Process isolation | Windows Job Object with `KILL_ON_JOB_CLOSE` | `job.rs` |
| Capabilities restriction | No `fs:write`, `fs:read`, or `shell:execute` in WebView | `capabilities/default.json` |
| Content Security Policy | `script-src 'self'` — no external scripts | `tauri.conf.json` |
| Updater signature | Minisign signature verification before install | `updater.rs` |
| Preset ID validation | Alphanumeric + `-` + `_` only, max length enforced | `loader.rs` |
| DNS leak prevention | Kill Switch blocks outbound UDP/TCP 53 | `dns/mod.rs` |

---

## 16. Installation

### Windows

1. Download the latest `.msi` installer from [Releases](https://github.com/lulushuz/Vane/releases).
2. Run the installer as Administrator.
3. Launch Vane from the Start Menu.

> Vane requires Administrator privileges to load the WinDivert kernel driver.

### Linux

1. Download the `.deb` (Debian/Ubuntu) or `.AppImage` from [Releases](https://github.com/lulushuz/Vane/releases).
2. Install with `sudo dpkg -i vane_*.deb` or `chmod +x Vane_*.AppImage && ./Vane_*.AppImage`.
3. Launch Vane; it will request `pkexec` (PolicyKit) root elevation on engine start.

---

## 17. Building from Source

### Requirements

| Tool | Version |
|------|---------|
| Node.js | LTS (20+) |
| npm | 10+ |
| Rust | Stable (2021 edition) |
| Tauri CLI | v2 |

### Build Steps

```bash
# Clone the repository
git clone https://github.com/lulushuz/Vane.git
cd Vane

# Install frontend dependencies
npm install

# Development mode (hot reload)
npm run tauri dev

# Production build
npm run tauri build
```

### Backend Tests

```bash
cd src-tauri
cargo test
cargo clippy
```

---

## 18. Troubleshooting

| Problem | Possible Cause | Solution |
|---------|---------------|----------|
| Engine fails to start | WinDivert driver conflict | Close other DPI tools (GoodbyeDPI, etc.) |
| DNS leak detected | Kill Switch disabled | Enable Kill Switch in DNS tab |
| `[HATA]` in logs | Not running as Administrator | Restart Vane as Administrator |
| High latency after enabling | wssize active on all connections | Disable wssize or limit to specific ports |
| QUIC streams still blocked | QUIC bypass not enabled | Enable UDP 443 in Advanced tab |
| Engine starts then immediately stops | Binary hash mismatch | Re-download Vane installer |
| Remote presets not loading | Firewall blocking GitHub CDN | Check network or disable preset sync |

---

## 19. Limitations and When Evasion Fails

Packet-level manipulation is not a universal solution. The following scenarios cannot be addressed by Vane:

| Scenario | Reason | Alternative |
|----------|--------|-------------|
| IP-level block | Target IP is blocked at routing layer | VPN or proxy |
| Transparent TCP proxy | ISP fully terminates and rebuilds TCP — Vane's modified packets are absorbed | VPN |
| Active probing | ISP sends probes to verify connection legitimacy | VPN with probe resistance |
| TLS 1.2 fingerprinting | Static cipher suite orders are detected | Update client OS / TLS library |
| MITM certificate inspection | ISP installs own CA in OS trust store | Remove untrusted CAs from OS |
| Full blocking (no connection at all) | TCP SYN is dropped at routing level | VPN |

---

## 20. Credits and License

- **[zapret](https://github.com/bol-van/zapret)** by bol-van — The underlying DPI bypass engine (nfqws / winws).
- **[WinDivert](https://reqrypt.org/windivert.html)** by basil00 — Windows kernel packet capture driver.
- **[Tauri](https://tauri.app)** — Secure desktop application framework.
- **[Minisign](https://jedisct1.github.io/minisign/)** — Cryptographic signature verification.
- **[StevenBlack/hosts](https://github.com/StevenBlack/hosts)** — AdBlock hosts list.

Licensed under the **GPL-3.0 License** — see [LICENSE](LICENSE) for details.

---

## 21. Community

| Channel | Link |
|---------|------|
| 🐛 Bug Reports | [GitHub Issues](https://github.com/lulushuz/Vane/issues) |
| 💡 Feature Requests | [GitHub Issues](https://github.com/lulushuz/Vane/issues) |
| 💬 Discord (Personal) | [luppux](https://discord.com/users/852103749228036136) |
| 💬 Discord (Community) | [discord.gg/luppux](https://discord.gg/luppux) |
| 📧 Security Reports | alp@archey.com.tr |
| 📋 Contributing | [CONTRIBUTING.md](CONTRIBUTING.md) |
| 🔒 Security Policy | [SECURITY.md](SECURITY.md) |
| 📝 Changelog | [CHANGELOG.md](CHANGELOG.md) |
