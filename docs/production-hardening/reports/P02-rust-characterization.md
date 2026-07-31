# P02 Rust Backend Characterization Report

This document records the findings, test additions, and characterization results for **P02 — Rust Backend Characterization Tests**.

---

## 1. Executive Summary

- **Baseline Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238` (`baseline-2.1.4-2026-07-29`)
- **Initial Rust Test Suite:** 6 modules, 38 passed unit tests
- **Final Rust Test Suite:** 16 modules, 142 passed tests (+104 tests added)
- **Frontend Regression Suite:** 11 test files, 121 tests passed (100% preserved)
- **Runtime Code Modifications:** 0 satır (yalnızca `settings.rs` içindeki 4 private fonksiyona `pub(crate)` erişimi verildi)
- **Status:** ✅ **READY FOR P03**

---

## 2. Test Inventory & Coverage Breakdown

| Test Group | Module Path | Test Count | Component Covered | Key Characterization |
| :--- | :--- | :---: | :--- | :--- |
| **Group A** | `characterization::domain_tests` | 13 | Domain Canonicalization | URL/port rejection, IDN/punycode, trailing dots, proptest idempotency |
| **Group B** | `characterization::sanitizer_tests` | 15 | Preset Argument Sanitizer | Allowed exact & prefix flags, individual shell char rejection, port lists, proptest |
| **Group C** | `characterization::preset_tests` | 8 | Built-in Preset Corpus | ID uniqueness, format verification, sanitizer pass audit, `https-sni-ghost` audit |
| **Group D** | `characterization::loader_tests` | 10 | ConfigLoader | Custom preset loading, corrupt JSON recovery, backup `.json.bak`, quarantine |
| **Group E** | `characterization::settings_tests` | 9 | Settings Persistence | Atomic replace, backup creation, damaged file recovery, runtime mapping |
| **Group F** | `characterization::pattern_tests` | 7 | Pattern Cache & State | Cache update/invalidation, fail-closed empty whitelist, RBR-01 reproducer |
| **Group G & H** | `characterization::engine_tests` | 7 | Engine Arg Prep & Lifecycle | Windows pass-through, Linux `--wf-` stripping (RBR-04), status enum JSON |
| **Group I** | `characterization::process_tests` | 2 | Process Ownership & Cleanup | Owned PID handle termination, RBR-06 global process cleanup reproducer |
| **Group J** | `characterization::dns_tests` | 4 | DNS Parser & DoH | Cloudflare/Google endpoints, wire-format encode/decode, DoQ absence contract |
| **Group K** | `characterization::kill_switch_tests` | 2 | Kill Switch & Firewall | Firewall rule names (`Vane-KillSwitch-*`), RBR-10 ownership tag absence |
| **Group L** | `characterization::ipc_tests` | 3 | IPC Contract Serialization | `camelCase` DTO rename, optional field omission, JSON fixtures matching |
| **Group M** | `characterization::remote_preset_tests` | 4 | Remote Preset Security | Minisign public key base64 parse, signature missing cache cleanup |
| **Group N** | `characterization::binary_integrity_tests` | 3 | Binary Integrity | winws & nfqws SHA-256 hash constant validation, RBR-12 reproducer |
| **Group O** | `characterization::optimizer_tests` | 5 | Optimizer Characterization | Preset priority sorting, static target IPs (RBR-09), EngineManager bypass (RBR-08) |
| **Reproducers** | `characterization::reproducers` | 12 | Bug Reproducer Index | RBR-01 through RBR-12 source-backed reproducer suite |

---

## 3. Characterized Bug Reproducers Index

| Reproducer ID | Risk ID | Target Phase | Documented Runtime Behavior | Expected Production Behavior |
| :--- | :--- | :---: | :--- | :--- |
| **RBR-01** | R-01 | **P06** | Engine start reads `settings.json` from disk instead of verified runtime cache | Verified runtime config in memory cache must be authoritative |
| **RBR-02** | R-08 / R-13 | **P08** | Sanitizer accepts out-of-order phase desync strategies (e.g. `fake,syndata`) | Phase-order validator must enforce Phase 0 → Phase 1 → Phase 2 sequence |
| **RBR-03** | R-08 | **P08** | Built-in `https-sni-ghost` preset passes sanitizer despite out-of-order strategy | Built-in presets must conform to strict phase ordering rules |
| **RBR-04** | R-11 / R-23 | **P11** | Linux launcher silently drops all `--wf-*`, `--windivert`, and `tcp.` flags | Linux netfilter/nftables rules must be generated explicitly |
| **RBR-05** | R-24 | **P11** | Linux firewall script hardcodes TCP port 80/443 and drops UDP QUIC traffic | Rules should support configurable ports and UDP QUIC redirection |
| **RBR-06** | R-12 | **P07 / P11** | Startup cleanup invokes `taskkill /IM winws...` or `killall nfqws...` globally | Cleanup must target only owned process PIDs or Job Object bounds |
| **RBR-07** | R-17 | **P07 / P14** | Process presence (`pid`) is treated as healthy `Running` status | Active traffic health probes should complement PID presence |
| **RBR-08** | R-16 | **P12** | Optimizer spawns `winws`/`nfqws` directly via `std::process::Command` | Optimizer must request process execution through `EngineManager` |
| **RBR-09** | R-15 | **P12** | Optimizer uses hardcoded static IP overrides for YouTube, Discord, X | Dynamic DNS or user-configurable target endpoints should be used |
| **RBR-10** | R-25 | **P10** | System firewall rules lack installation UUID or instance metadata tags | Firewall rules should include Vane installation UUID tags for clean orphan cleanup |
| **RBR-11** | R-21 | **P10 / P14** | AdBlock returns `0.0.0.0` or empty address list instead of NXDOMAIN wire packet | Blocked queries should return RFC-compliant NXDOMAIN responses |
| **RBR-12** | R-19 | **P13 / P15** | Code constant expected SHA-256 hash must match bundled binary checksums | CI/CD release workflow should verify binary checksums before packaging |

---

## 4. Verification Results

```text
cargo test --lib: PASSED (142 passed, 0 failed, 0 ignored, 0.39s)
cargo clippy --lib -- -D warnings: PASSED (0 warnings)
npm test: PASSED (11 test files, 121 tests passed)
npm run build: PASSED (tsc & vite build successful)
```
