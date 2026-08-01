# P03 Test Matrix

Bu doküman, Vane production hardening sürecinde kullanılacak kabul (acceptance) ve regresyon test matrisini ve **P01 Frontend Characterization** safhasında eklenen test durumlarını içerir.

---

## 1. Test Katmanları ve Tanımları

1. **Unit Tests:** Saf fonksiyonlar, parser'lar, serializer'lar ve state reducer'lar (Bağımsız, hızlı).
2. **Contract Tests:** Rust IPC DTO'ları ile TypeScript arayüzlerinin JSON schema uyumluluğu.
3. **Integration Tests:** Bellek içi servis bileşenlerinin (Pattern, Settings, DNS, Engine Manager) eşzamanlı senaryoları.
4. **Privileged Tests:** Yetkili (Elevated Admin / sudo) WinDivert, Firewall ve System DNS canlı testleri.
5. **Packaged Tests:** Paketlenen installer (.exe NSIS, .deb) kurulum, açılış, güncelleme ve kaldırma testleri.
6. **Manual Network Tests:** Gerçek ISP ağ ortamları, adaptör değişimleri, VPN/proxy çakışma ve uyku/uyanma testleri.

---

## 2. Kabul ve Karakterizasyon Test Matrisi

| Test ID | Scenario | Preconditions | Action | Expected Result | Cleanup | Automation Level | Platform | Priority | Current Coverage | Evidence |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **UT-01** | TypeScript Preset Serializer | Valid preset JSON | Parse preset object | Correctly validated Preset type | None | Unit | Cross | P0 | Existing | [presetValidator.test.ts](src/utils/presetValidator.test.ts) |
| **UT-02** | Rust Sanitizer Command Check | Malformed args with Shell Metacharacters | Call `validate_preset_args()` | Returns `EngineError::InvalidPreset` | None | Unit | Cross | P0 | Existing | [sanitizer.rs:L43](src-tauri/src/engine/sanitizer.rs#L43) |
| **UT-03** | Domain Canonicalization | Domain with uppercase, trailing dot, spaces | Call `canonicalize_domain()` | Returns lowercase clean domain string | None | Unit | Cross | P0 | Existing | [domain.rs:L45](src-tauri/src/config/domain.rs#L45) |
| **UT-04** | Atomic Settings Persistence | Primary settings file write error | Trigger atomic save | Recovers from backup file smoothly | Delete temp files | Unit | Cross | P0 | Existing | [settings.rs:L100](src-tauri/src/settings.rs#L100) |
| **CT-01** | IPC Payload Validation Contract | Mismatched DTO JSON schema | Deserialize payload | Rejects unknown/malformed schema gracefully | None | Contract | Cross | P0 | Existing | [ipc.rs:L1](src-tauri/src/ipc.rs#L1) |
| **CT-02** | Engine Status IPC Event | Engine transitions state | Emit status event | Payload matches TypeScript status schema | None | Contract | Cross | P1 | Partial | [engineStore.ts:L45](src/store/engineStore.ts#L45) |
| **P01-A** | Advanced Config Parser (A01-A14) | Raw winws arguments | Call `parseArgsToConfig()` | Correctly maps options & quarantines invalid flags | None | Unit | Cross | P0 | **Added in P01** | [advancedConfig.test.ts](src/test/advancedConfig.test.ts) |
| **P01-B** | Advanced Config Serializer (B01-B12) | AdvancedConfig object | Call `serializeConfigToArgs()` | Produces deterministic winws flag list | None | Unit | Cross | P0 | **Added in P01** | [advancedConfig.test.ts](src/test/advancedConfig.test.ts) |
| **P01-C** | Built-in Preset Round-Trip (C01-C03) | Presets from `builtin.json` | Parse & Reserialize | Characterizes Exact / Semantic / Lossy round-trips | None | Unit | Cross | P0 | **Added in P01** | [advancedConfig.test.ts](src/test/advancedConfig.test.ts) |
| **P01-D** | Store Persistence Write Queue (D01-D06) | Rapid concurrent setItem/getItem calls | Write to mock storage | Operations execute sequentially without deadlock | None | Unit | Cross | P0 | **Added in P01** | [storePersistence.test.ts](src/test/storePersistence.test.ts) |
| **P01-E** | Engine Launch Sequence (E01-E09) | Active Zustand store | Invoke `startEngine()` | Sends expected IPC sequence: sync_bypass -> sync_dns -> start_engine | Reset store | Integration | Cross | P0 | **Added in P01** | [engineLifecycle.test.ts](src/test/engineLifecycle.test.ts) |
| **P01-F** | Pattern Debounce & Revision (F01-F06) | Rapid domain list updates | Trigger `setWhitelistDomains` | Debounces requests by 100ms & drops stale revisions | Reset store | Integration | Cross | P0 | **Added in P01** | [patternDnsSync.test.ts](src/test/patternDnsSync.test.ts) |
| **P01-G** | DNS Debounce & Rollback (G01-G06) | DNS setting change + backend rejection | Trigger `setDnsAdBlock` | Reverts UI state on rejection & clears rollback on success | Reset store | Integration | Cross | P0 | **Added in P01** | [patternDnsSync.test.ts](src/test/patternDnsSync.test.ts) |
| **P01-H** | Engine Stop & Status UI (H01-H06) | Running engine state | Call `stopEngine()` | Updates status to stopped & logs warning line | Reset store | Integration | Cross | P0 | **Added in P01** | [engineLifecycle.test.ts](src/test/engineLifecycle.test.ts) |
| **P01-I** | Preset Import / Export (I01-I08) | Preset JSON / Custom form | Import / Export preset | Validates schema & documents `.json` export behavior | Reset store | Integration | Cross | P0 | **Added in P01** | [presetImportExport.test.ts](src/test/presetImportExport.test.ts) |
| **P01-J** | IPC Error Normalization (J01-J05) | Error objects, strings, null, undefined | Call `normalizeIpcError()` | Normalizes into structured `IpcErrorPayload` | None | Unit | Cross | P0 | **Added in P01** | [presetImportExport.test.ts](src/test/presetImportExport.test.ts) |
| **P01-K** | Store Hydration & Migration (K01-K07) | Persisted JSON payload | Call `migratePersistedEngineState()` | Merges legacy domain strings & preserves valid fields | Reset store | Unit | Cross | P0 | **Added in P01** | [storePersistence.test.ts](src/test/storePersistence.test.ts) |
| **P01-L** | Domain Helpers (L01-L03) | Raw domain string input | Call `normalizePersistedDomains()` | Returns clean array of domain strings | None | Unit | Cross | P0 | **Added in P01** | [patternDnsSync.test.ts](src/test/patternDnsSync.test.ts) |
| **P06-A** | Pattern Transaction Sync | Prepared config & revisioned hostlist | Call `sync_bypass_config` | Writes revisioned hostlist `domains-rev-X-HASH.txt` and updates `AppliedRuntimeConfig` | Reset state | Integration | Cross | P0 | **Added in P06** | [pattern_transaction_tests.rs](src-tauri/src/characterization/pattern_transaction_tests.rs) |
| **P09-A** | Advanced Capabilities IPC & Matrix | `get_advanced_capabilities` command | Call `get_advanced_capabilities()` | Returns platform, supported desync methods, port filtering status, option support states | None | Unit | Cross | P0 | **Added in P09** | [advanced_contract_tests.rs](src-tauri/src/characterization/advanced_contract_tests.rs) |
| **P09-B** | BR-06 Non-443 UDP Port Range Preservation | Arbitrary UDP port range (e.g. `50000-65535`) | Parse & serialize | Non-443 UDP port range survives round-trip without argument loss | None | Unit | Cross | P0 | **Added in P09** | [advancedCapabilities.test.ts](src/test/advancedCapabilities.test.ts) |
| **P09-C** | Cross-Language Test Fixtures Parity | 10 JSON fixtures in `fixtures/advanced/` | Read & validate in Vitest and Rust | All 10 fixtures pass identical validation and canonical output expectations | None | Integration | Cross | P0 | **Added in P09** | [advanced_contract_tests.rs](src-tauri/src/characterization/advanced_contract_tests.rs) |
| **P10-A** | Transactional DNS & Kill Switch Ownership | DNS candidates + Kill Switch | Call `sync_dns_settings` | Monotonic revisioning, latest-wins check, exact rule deletion, partial apply rollback & orphan recovery | Reset state | Integration | Cross | P0 | **Added in P10** | [dns_transaction_tests.rs](src-tauri/src/characterization/dns_transaction_tests.rs) |
| **P11-A** | Dynamic Linux Filter Plan & Rule Ownership | Preset TCP/UDP ports + Linux capabilities | Build & execute `LinuxFilterPlan` | Dynamic NFQUEUE rules, nftables batch, iptables fallback with partial rollback & metadata orphan recovery | Reset state | Integration | Linux | P0 | **Added in P11** | [linux_filter_tests.rs](src-tauri/src/characterization/linux_filter_tests.rs) |
| **P12-A** | Optimizer Safety & Unified Engine Lifecycle | Preset candidates + Original engine state | Execute `run_optimizer_session` | Candidate deduplication, median latency scoring, zero candidate leak & guaranteed atomic restore | Reset state | Integration | Cross | P0 | **Added in P12** | [optimizer_session_tests.rs](src-tauri/src/characterization/optimizer_session_tests.rs) |
| **P13-A** | Binary Integrity & Supply Chain Security | Embedded manifest + Local binaries | Call `verify_current_platform_group()` | Streaming SHA-256 validation, path containment, symlink/reparse-point rejection & TOCTOU protection | None | Unit / Integration | Cross | P0 | **Added in P13** | [binary_integrity_tests.rs](src-tauri/src/characterization/binary_integrity_tests.rs) |
| **P01-BR** | Bug Reproducers (BR01-BR08) | Known race & coercion scenarios | Execute edge flows | Characterizes pending persistence, DoQ coercion, PID-only state | Reset store | Integration | Cross | P0 | **Added in P01** | [bugReproducers.test.ts](src/test/bugReproducers.test.ts) |
| **IT-01** | Pattern Sync Race Protection | Rapidly add/delete 50 domains | Save & restart engine | Final disk state matches memory state | Reset test list | Integration | Cross | P0 | Partial | [engineStore.ts:L110](src/store/engineStore.ts#L110) |
| **IT-02** | DNS Forwarder Lifecycle | Valid DNS provider URL | Start & Stop DNS Forwarder | Local UDP 53 port binds & unbinds cleanly | Unbind port | Integration | Cross | P0 | Missing | [forwarder.rs:L100](src-tauri/src/dns/forwarder.rs#L100) |
| **IT-03** | Engine Planner Command Assembly | Preset + Advanced settings | Generate process args | Flags ordered by phase without injection | None | Integration | Cross | P0 | Missing | [manager.rs:L800](src-tauri/src/engine/manager.rs#L800) |
| **PT-01** | Windows WinDivert Driver Binding | Administrator Privileges | Start engine with `winws.exe` | WinDivert64.sys loads & intercepts packets | Stop winws & unload driver | Privileged | Windows | P0 | Missing | [manager.rs:L140](src-tauri/src/engine/manager.rs#L140) |
| **PT-02** | Windows Firewall Rule Cleanup | Admin Privileges + Active Kill Switch | Force crash process (`taskkill /F`) | Firewall rule automatically removed or rolled back | Remove stale rule | Privileged | Windows | P0 | Missing | [router.rs:L10](src-tauri/src/network/router.rs#L10) |
| **PT-03** | Linux nftables NFQUEUE Hook | Root Privileges | Start engine with `nfqws` | nftables rule diverts port 80/443 to NFQUEUE 10 | Flush nftables chain | Privileged | Linux | P1 | Missing | [router.rs:L30](src-tauri/src/network/router.rs#L30) |
| **PK-01** | NSIS Per-Machine Installation | Fresh Windows 11 system | Run `Vane-Setup.exe` | App installs into `Program Files`, registers uninstaller | Uninstall app | Packaged | Windows | P0 | Existing | [windows-acceptance-build.yml](.github/workflows/windows-acceptance-build.yml) |
| **PK-02** | Auto Updater Signature Check | Installed app v2.1.4 | Trigger check update with invalid signature | Updater rejects unsigned/corrupt binary package | Clean temp files | Packaged | Windows | P0 | Missing | [updater.rs:L10](src-tauri/src/updater.rs#L10) |
| **PK-03** | Uninstaller Resource Cleanup | Installed app with custom firewall rule | Execute Uninstaller | Executables, services & firewall rules completely removed | Verify folder empty | Packaged | Windows | P0 | Missing | [windows/Invoke-VaneAcceptance.ps1](scripts/windows/Invoke-VaneAcceptance.ps1) |
| **MN-01** | ISP DPI Bypass Smoke Test | Active Turkish ISP connection (TT/Superonline) | Select TR 1 Classic Split & start | Blocked target sites load over HTTPS without RST | Stop engine | Manual | Windows | P0 | Manual Only | Physical Test Required |
| **MN-02** | Network Interface Switch | Engine active on Wi-Fi | Switch network cable to Ethernet | WinDivert rebinds without dropping interface | Restore Wi-Fi | Manual | Windows | P1 | Manual Only | Physical Test Required |
| **MN-03** | System Sleep/Resume Cycle | Engine active | Put PC to Sleep, then Resume | Engine recovers process & driver after wake-up | None | Manual | Windows | P1 | Manual Only | Physical Test Required |
| **MN-04** | Dual Stack IPv4/IPv6 Traffic | Active IPv6 network | Enable Kill Switch & Engine | Both IPv4 and IPv6 DNS queries pass through local forwarder | Restore DNS | Manual | Windows | P1 | Manual Only | Physical Test Required |

---

## 3. P03 Engine Launch Planner Test Matrisi

| Test ID | Scenario | Input / Preconditions | Expected Output / Behavior | Component | Coverage Status | Evidence File |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **P03-W01** | Windows Default Preset Plan | Built-in `default` preset, Windows platform | `EngineBinaryKind::Winws`, `HostlistPlan::None`, original args preserved | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-W02** | Whitelist Hostlist Include | Whitelist mode + valid domain list | `HostlistPlan::Include`, `--hostlist=<path>` added as final arg | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-W03** | Blacklist Hostlist Exclude | Blacklist mode + valid domain list | `HostlistPlan::Exclude`, `--hostlist-exclude=<path>` added as final arg | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-W04** | All Mode Hostlist Omission | All mode | `HostlistPlan::None`, no `--hostlist` flag in final args | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-W05** | Empty Whitelist Fail-Closed | Whitelist mode + whitespace domain list | Returns `EngineError::ConfigParseError` fail-closed error | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-W06** | Executable Space Path | Path with spaces | PathBuf retained without shell escaping in data model | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-W09** | Kill Switch Requirement | `kill_switch = true` in bypass input | `KillSwitchRequirement::Required` in plan output | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-L01** | Linux Binary Kind | Linux platform input | `EngineBinaryKind::Nfqws` | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-L02** | Linux Queue Number Arg | Linux platform input | `--qnum=200` prepended as first argument | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-L03** | Linux `--wf-*` Stripping | Linux platform input with `--wf-tcp` | `--wf-*` stripped from final args; recorded in `declared_tcp_spec` | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-L04** | Linux Effective UDP Gap | Linux platform input with `--wf-udp=443` | `declared_udp_spec` = Some("443"), `effective_linux_udp_spec` = None | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-S01** | Deterministic 100 Runs | Identical input ran 100 times | 100 identical `EngineLaunchPlan` outputs | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-S03** | No FS Side-Effects | Temp directory check during plan creation | Temp directory remains 100% empty during planning | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-P01** | Windows Default Parity | Built-in `default` preset | `plan.final_arguments == preset.args` | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |
| **P03-P07** | TR-1 Preset Parity | Built-in `tr-1` preset | `plan.final_arguments == preset.args` | `launch_plan` | **Added in P03** | [launch_plan_tests.rs](src-tauri/src/characterization/launch_plan_tests.rs) |

---

## 4. P04 Runtime Configuration Contract Test Matrisi

| Test ID | Scenario | Input / Preconditions | Expected Output / Behavior | Component | Coverage Status | Evidence File |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **P04-A01** | Candidate All Mode Verification | `RuntimeConfigCandidate` with "all" mode | `VerifiedRuntimeConfig` with `RuntimeBypassMode::All` | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-A03** | Empty Whitelist Candidate | `RuntimeConfigCandidate` whitelist mode + empty domains | `RuntimeConfigError::EmptyWhitelist` fail-closed error | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-B01** | ConfigRevision Increment | `ConfigRevision` = 10 | `checked_next()` returns `ConfigRevision` = 11 | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-F01** | SHA-256 Fingerprint Determinism | Identical candidate inputs | Identical 64-char hex SHA-256 fingerprint string | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-F02** | Revision Fingerprint Independence | Same candidate + diff revision (1 vs 99) | Identical fingerprint output | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-F03** | Domain Case Canonical Fingerprint | "EXAMPLE.COM" vs "example.com." | Identical fingerprint output | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-D01** | PreparedRuntimeConfig Snapshot | Verified config + launch plan | Contains verified snapshot & plan, no process PID | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-E01** | AppliedRuntimeConfig Snapshot | Verified config + process PID (5432) | Contains PID, `AppliedVerification::ProcessStarted` | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-RED** | Telemetry Redaction Check | `VerifiedRuntimeConfig` with sensitive domain | Debug format hides domains, includes domain_count | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |
| **P04-RBR** | Disk Config Source Reproducer | Engine start with disk settings | Documents runtime contract still receiving disk config | `runtime_config` | **Added in P04** | [runtime_config_tests.rs](src-tauri/src/characterization/runtime_config_tests.rs) |

---

## 5. P05 Low-Risk Deterministic Fixes Test Matrisi

| Test ID | Scenario | Input / Preconditions | Expected Output / Behavior | Component | Coverage Status | Evidence File |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **P05-A01** | Export Preset Case-Insensitive Extension | `export_preset` with `.vane` or `.VANE` | Succeeds and writes `.vane` file | `commands.rs` | **Added in P05** | [commands.rs:L285](src-tauri/src/commands.rs#L285) |
| **P05-A02** | Export Preset Rejects Non-.vane | `export_preset` with `.json` path | Returns error `"Preset exports must use the .vane extension."` | `commands.rs` | **Added in P05** | [presetImportExport.test.ts](src/test/presetImportExport.test.ts) |
| **P05-A03** | BR-02 Reproducer Resolved | Export preset in UI dialog | Defaults to `.vane` filter and resolves export cleanly | Frontend UI | **Added in P05** | [bugReproducers.test.ts](src/test/bugReproducers.test.ts) |
| **P05-B01** | Hydration Migration Legacy DoQ | Persisted state with `dnsProtocol: 'doq'` | `migratePersistedEngineState` returns `dnsProtocol: 'doh'` | `persistence` | **Added in P05** | [persistence.ts:L16](src/store/persistence.ts#L16) |
| **P05-B02** | BR-03 Reproducer Resolved | Legacy `doq` choice hydration | Hydrates to `doh` without silent coercion branches in start flow | Frontend Store | **Added in P05** | [bugReproducers.test.ts](src/test/bugReproducers.test.ts) |
| **P05-B03** | Raw IPC DoQ Rejection | Raw IPC candidate with `protocol: 'doq'` | Backend returns `UnsupportedDnsProtocol("doq")` | `runtime_config` | **Added in P05** | [runtime_config.rs:L335](src-tauri/src/engine/runtime_config.rs#L335) |
| **P05-C01** | Documentation Absence of DoQ Claim | README files audit | `README.md` and `README.tr.md` mark DoQ as not supported | Documentation | **Added in P05** | [documentation.test.ts](src/test/documentation.test.ts) |
| **P05-C02** | Documentation Absence of WFP Claim | README files audit | `README.md` and `README.tr.md` specify Windows Firewall (`netsh`) | Documentation | **Added in P05** | [documentation.test.ts](src/test/documentation.test.ts) |



