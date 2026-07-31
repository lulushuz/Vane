# P01 Capability Matrix

Bu doküman, Vane uygulamasının tüm özelliklerini ve bileşenlerini gerçek kod üzerinden inceleyerek belgelendirilmiş, doğrulanmış veya eksik durumlarını ortaya koyar.

---

## 1. Yetenek Tanım Sütunları

| Sütun | Açıklama |
| :--- | :--- |
| **Feature** | İnceleyen özelliğin adı |
| **Frontend** | UI'da (React/Zustand) arayüz karşılığı var mı? |
| **Backend** | Rust backend command veya servis karşılığı var mı? |
| **Runtime** | Gerçek işletim sistemi process veya ağ sürücü davranışına dönüşüyor mu? |
| **Windows** | Windows 10/11 destek ve çalışma durumu |
| **Linux** | Linux (nftables/iptables/nfqws) destek durumu |
| **Tests** | Test kapsamı (Unit, Integration vb.) |
| **Documentation** | README / UI açıklamaları ile kod davranışı uyumlu mu? |
| **Confidence** | **Confirmed** (Doğrulandı), **Partial** (Kısmi), **Unverified** (Test Edilmedi), **False Claim** (Dokümanda var ama kodda yok) |
| **Evidence** | İlgili dosya ve fonksiyon kod referansı |
| **Production Status** | **Ready** (Üretime Hazır), **Needs Hardening** (Sertleştirme Gerekiyor), **Experimental** (Deneysel), **Disabled / Missing** (Devre Dışı / Eksik) |

---

## 2. Capability Matrix Table

### 2.1 Engine (Çalıştırma Motoru)

| Feature | Frontend | Backend | Runtime | Windows | Linux | Tests | Doc | Confidence | Evidence | Production Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Engine start** | Yes | Yes | Yes | Supported | Partial | Yes | Yes | Confirmed | [manager.rs:L140](src-tauri/src/engine/manager.rs#L140) | Needs Hardening |
| **Engine stop** | Yes | Yes | Yes | Supported | Partial | Yes | Yes | Confirmed | [manager.rs:L250](src-tauri/src/engine/manager.rs#L250) | Needs Hardening |
| **Engine restart** | Yes | Yes | Yes | Supported | Partial | Yes | Yes | Confirmed | [manager.rs:L310](src-tauri/src/engine/manager.rs#L310) | Needs Hardening |
| **Process ownership** | No | Yes | Partial | Supported | Missing | Yes | No | Partial | [process.rs:L20](src-tauri/src/engine/process.rs#L20), [job.rs:L10](src-tauri/src/engine/job.rs#L10) | Needs Hardening |
| **Process crash recovery** | No | Yes | Yes | Supported | Missing | Yes | Yes | Confirmed | [manager.rs:L420](src-tauri/src/engine/manager.rs#L420) | Needs Hardening |
| **Binary SHA verification** | No | No | No | Missing | Missing | No | Yes | False Claim | Backend hashes not checked on launch | Needs Hardening |
| **WinDivert cleanup** | No | Yes | Yes | Supported | N/A | Partial | Yes | Confirmed | [manager.rs:L380](src-tauri/src/engine/manager.rs#L380) | Needs Hardening |
| **Linux NFQUEUE setup** | No | Yes | Partial | N/A | Experimental | No | Yes | Partial | [router.rs:L10](src-tauri/src/network/router.rs#L10) | Experimental |
| **Engine status** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [engineStore.ts:L45](src/store/engineStore.ts#L45) | Ready |
| **Health status** | Yes | Yes | Partial | Supported | Supported | Yes | Yes | Partial | Separated from Process Alive state | Needs Hardening |
| **Concurrent start/stop protection** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [manager.rs:L70](src-tauri/src/engine/manager.rs#L70) (Atomic Generation) | Ready |

---

### 2.2 Pattern (Site/Domain Listesi)

| Feature | Frontend | Backend | Runtime | Windows | Linux | Tests | Doc | Confidence | Evidence | Production Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **All sites mode** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [domain.rs:L20](src-tauri/src/config/domain.rs#L20) | Ready |
| **Whitelist mode** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [manager.rs:L820](src-tauri/src/engine/manager.rs#L820) | Ready |
| **Blacklist mode** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [manager.rs:L835](src-tauri/src/engine/manager.rs#L835) | Ready |
| **Domain canonicalization** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [domain.rs:L45](src-tauri/src/config/domain.rs#L45) | Ready |
| **Empty whitelist fail-closed** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [manager.rs:L860](src-tauri/src/engine/manager.rs#L860) | Ready |
| **Hostlist file generation** | No | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [manager.rs:L890](src-tauri/src/engine/manager.rs#L890) | Needs Hardening |
| **Hostlist exclude** | No | Partial | No | Missing | Missing | No | No | False Claim | Args stripped by sanitizer | Disabled |
| **Runtime cache** | Yes | Yes | Yes | Supported | Supported | Partial | Yes | Partial | [engineStore.ts:L110](src/store/engineStore.ts#L110) | Needs Hardening |
| **Disk persistence** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [settings.rs:L100](src-tauri/src/settings.rs#L100) | Needs Hardening |
| **Config revision** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [revisionGate.ts:L10](src/store/revisionGate.ts#L10) | Ready |
| **Engine restart on change** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [engineStore.ts:L210](src/store/engineStore.ts#L210) | Ready |
| **Rollback on corrupt config** | No | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [settings.rs:L230](src-tauri/src/settings.rs#L230) | Ready |

---

### 2.3 Preset (Ön Tanımlı Modlar)

| Feature | Frontend | Backend | Runtime | Windows | Linux | Tests | Doc | Confidence | Evidence | Production Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Built-in preset** | Yes | Yes | Yes | Supported | Partial | Yes | Yes | Confirmed | [builtin.json](presets/builtin.json) | Needs Hardening |
| **Custom preset** | Yes | Yes | Yes | Supported | Partial | Yes | Yes | Confirmed | [CustomPresetView.tsx:L15](src/views/CustomPresetView.tsx#L15) | Needs Hardening |
| **Imported preset** | Yes | Yes | Yes | Supported | Partial | Yes | Yes | Confirmed | [presetValidator.ts:L20](src/utils/presetValidator.ts#L20) | Needs Hardening |
| **Exported preset** | Yes | Yes | N/A | Supported | Supported | Yes | Yes | Partial | `.json` vs `.vane` mismatch | Needs Hardening |
| **Remote preset fetch** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [remote.rs:L40](src-tauri/src/presets/remote.rs#L40) | Needs Hardening |
| **Signature verification** | No | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [remote.rs:L110](src-tauri/src/presets/remote.rs#L110) (Minisign) | Needs Hardening |
| **Argument allowlist** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [sanitizer.rs:L6](src-tauri/src/engine/sanitizer.rs#L6) | Ready |
| **Semantic validation** | Partial | Partial | No | Missing | Missing | Partial | Yes | Partial | Range limits partial | Needs Hardening |
| **Phase order validation** | No | No | No | Missing | Missing | No | Yes | False Claim | Phase ordering not checked | Needs Hardening |
| **Platform compatibility** | Yes | Partial | Partial | Supported | Experimental | Yes | Yes | Confirmed | `--wf-*` stripped on Linux | Experimental |
| **Preset smoke tests** | Yes | Yes | No | Supported | Supported | Yes | Yes | Confirmed | [presetValidator.test.ts:L1](src/utils/presetValidator.test.ts#L1) | Ready |

---

### 2.4 Advanced (Gelişmiş Ayarlar)

| Feature | Frontend | Backend | Runtime | Windows | Linux | Tests | Doc | Confidence | Evidence | Production Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **TCP ports** | Yes | Yes | Yes | Supported | Partial | Yes | Yes | Confirmed | `--filter-tcp` arg building | Ready |
| **UDP ports** | Yes | Yes | Yes | Supported | Partial | Yes | Yes | Confirmed | `--filter-udp` arg building | Ready |
| **DPI desync method** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync=` arg building | Ready |
| **Split position** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-split-pos=` | Ready |
| **Repeats** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-repeats=` | Ready |
| **Fooling** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-fooling=` | Ready |
| **AutoTTL** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-autottl` | Ready |
| **Fixed TTL** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-ttl=` | Ready |
| **Any protocol** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-any-protocol` | Ready |
| **Cutoff** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-cutoff=` | Ready |
| **HTTP split** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-split-http-req` | Ready |
| **TLS split** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-split-tls` | Ready |
| **Wssize** | Yes | Yes | No | Missing | Missing | No | Yes | False Claim | Struct field unused in cmd building | Disabled |
| **MSS** | Yes | Yes | No | Missing | Missing | No | Yes | False Claim | Struct field unused in cmd building | Disabled |
| **Custom payload** | Yes | Yes | No | Missing | Missing | No | Yes | False Claim | Struct field unused in cmd building | Disabled |
| **Fake TLS SNI** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `--dpi-desync-fake-tls=` | Needs Hardening |
| **IPSet** | No | No | No | Missing | Missing | No | No | Disabled | Sanitizer explicitly rejects `--ipset` | Disabled |
| **TPWS** | No | No | No | Missing | Missing | No | No | Disabled | Sanitizer explicitly rejects `tpws` | Disabled |
| **Bind interface** | Yes | No | No | Missing | Missing | No | Yes | False Claim | UI toggle only | Disabled |
| **Proxy** | Yes | Partial | No | Missing | Missing | No | Yes | False Claim | Proxy only in DNS reqwest | Needs Hardening |

---

### 2.5 DNS (DNS & AdBlock Katmanı)

| Feature | Frontend | Backend | Runtime | Windows | Linux | Tests | Doc | Confidence | Evidence | Production Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **DoH (DNS over HTTPS)** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [doh.rs:L10](src-tauri/src/dns/doh.rs#L10) | Needs Hardening |
| **DoT (DNS over TLS)** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [forwarder.rs:L150](src-tauri/src/dns/forwarder.rs#L150) | Needs Hardening |
| **DoQ (DNS over QUIC)** | Yes | Yes | No | Missing | Missing | No | Yes | False Claim | Silent fallback to DoH in DNS resolver | Needs Hardening |
| **Cloudflare Provider** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Default built-in DoH endpoint | Ready |
| **Google Provider** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Built-in DoH endpoint | Ready |
| **AdGuard Provider** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Built-in DoH endpoint | Ready |
| **NextDNS Provider** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Built-in DoH endpoint | Ready |
| **Custom provider** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | User IP / URL input | Ready |
| **DNS cache** | No | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Memory TTL cache in forwarder | Ready |
| **AdBlock filter** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [forwarder.rs:L310](src-tauri/src/dns/forwarder.rs#L310) | Needs Hardening |
| **SOCKS5 proxy** | Yes | Yes | Partial | Supported | Supported | Yes | Yes | Partial | Only proxies DNS query traffic | Needs Hardening |
| **Local DNS listen address** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | `127.0.0.1:53` local UDP listener | Needs Hardening |
| **System DNS application** | Yes | Yes | Yes | Supported | Experimental | Yes | Yes | Confirmed | Windows netsh / WMI adapter DNS set | Needs Hardening |
| **Watchdog & Recovery** | No | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [watchdog.rs:L20](src-tauri/src/dns/watchdog.rs#L20) | Needs Hardening |

---

### 2.6 Kill Switch

| Feature | Frontend | Backend | Runtime | Windows | Linux | Tests | Doc | Confidence | Evidence | Production Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Windows implementation** | Yes | Yes | Yes | Supported | N/A | Yes | Yes | Confirmed | WFP / Windows Firewall rule block | Needs Hardening |
| **Linux implementation** | Yes | Yes | Partial | N/A | Experimental | No | Yes | Unverified | nftables drop rule | Experimental |
| **TCP 53 filtering** | Yes | Yes | Yes | Supported | Experimental | Yes | Yes | Confirmed | Block outbound TCP 53 except local listener | Needs Hardening |
| **UDP 53 filtering** | Yes | Yes | Yes | Supported | Experimental | Yes | Yes | Confirmed | Block outbound UDP 53 except local listener | Needs Hardening |
| **IPv4 support** | Yes | Yes | Yes | Supported | Experimental | Yes | Yes | Confirmed | IPv4 filter rules | Ready |
| **IPv6 support** | Yes | Yes | Partial | Supported | Experimental | Partial | Yes | Partial | IPv6 binding/rule parity incomplete | Needs Hardening |
| **Loopback exemption** | Yes | Yes | Yes | Supported | Experimental | Yes | Yes | Confirmed | Allow `127.0.0.1` traffic | Ready |
| **Firewall rule ownership** | No | No | No | Missing | Missing | No | No | False Claim | Rules not tagged with Vane UUID | Needs Hardening |
| **Crash cleanup** | No | Yes | Partial | Supported | Missing | Partial | Yes | Partial | System exit handler | Needs Hardening |
| **Uninstall cleanup** | N/A | N/A | N/A | Missing | Missing | No | No | False Claim | NSIS uninstaller omits rule cleanup | Needs Hardening |

---

### 2.7 Optimizer (Otomatik Test & İyileştirme)

| Feature | Frontend | Backend | Runtime | Windows | Linux | Tests | Doc | Confidence | Evidence | Production Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Stops active engine** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | [optimizer.rs:L45](src-tauri/src/engine/optimizer.rs#L45) | Needs Hardening |
| **Uses EngineManager** | No | No | No | Missing | Missing | No | Yes | False Claim | Spawns direct `std::process::Command` | Needs Hardening |
| **Uses sanitizer** | No | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Sanitizes preset args | Ready |
| **Uses binary hash verification** | No | No | No | Missing | Missing | No | Yes | False Claim | Direct process launch without checksum | Needs Hardening |
| **Uses Pattern settings** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Reads target domains | Ready |
| **Uses DNS settings** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Resolves targets via DNS | Needs Hardening |
| **Baseline measurement** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Latency & HTTP status check | Ready |
| **Preset scoring** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Sorts presets by success rate | Ready |
| **Cancellation support** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Atomic cancellation token | Ready |
| **Rollback state** | Yes | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Restores previous preset on cancel | Ready |
| **Dynamic target resolution** | Partial | Partial | No | Missing | Missing | No | Yes | Partial | Static IP list used for latency check | Needs Hardening |

---

### 2.8 Distribution & Packaging (Dağıtım ve Güncelleme)

| Feature | Frontend | Backend | Runtime | Windows | Linux | Tests | Doc | Confidence | Evidence | Production Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **NSIS Installer** | N/A | Yes | Yes | Supported | N/A | Yes | Yes | Confirmed | [tauri.conf.json#L53](src-tauri/tauri.conf.json#L53) | Ready |
| **Per-machine installation** | N/A | Yes | Yes | Supported | N/A | Yes | Yes | Confirmed | Require Admin install mode | Ready |
| **Code signing** | N/A | Yes | Yes | Supported | N/A | Partial | Yes | Confirmed | Authenticode in CI workflow | Needs Hardening |
| **Auto updater** | Yes | Yes | Yes | Supported | Experimental | Yes | Yes | Confirmed | [updater.rs:L10](src-tauri/src/updater.rs#L10) | Needs Hardening |
| **Signed update verification** | No | Yes | Yes | Supported | Supported | Yes | Yes | Confirmed | Minisign signature check | Ready |
| **Version consistency** | N/A | N/A | N/A | Supported | Supported | Yes | Yes | Confirmed | Checked in CI release workflow | Ready |
| **Release artifact smoke test** | N/A | N/A | N/A | Supported | N/A | Yes | Yes | Confirmed | `windows-acceptance-build.yml` | Ready |
| **Linux package (.deb/.appimage)** | N/A | Partial | Partial | N/A | Experimental | No | Yes | Unverified | Default tauri bundle target | Experimental |
| **Software Bill of Materials (SBOM)** | N/A | No | No | Missing | Missing | No | Yes | False Claim | SBOM generation pipeline missing | Needs Hardening |
