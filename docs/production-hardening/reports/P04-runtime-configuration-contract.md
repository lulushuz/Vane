# P04 Runtime Configuration Contract Report

This report documents the design, implementation, verification, and parity testing for **P04 — Runtime Configuration Contract, Immutable Snapshot, and Prepared/Applied State Separation**.

---

## 1. Repository State

- **Branch:** `main`
- **Start Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238`
- **End Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238`
- **Baseline Matched:** Yes (`baseline-2.1.4-2026-07-29`)
- **Pre-existing Files Preserved:** Yes (`artifacts/`, `docs/`, `scripts/audit/`, `src/test/`, `src-tauri/fixtures/`, `src-tauri/src/characterization/`, `src-tauri/src/engine/launch_plan.rs`).

---

## 2. Test Execution Results

```text
Rust Before P04:
Tests: 158 passed
Ignored: 0
Failed: 0

Rust After P04:
Tests: 179 passed (+21 new P04 unit, fingerprint, parity, and redaction tests)
Ignored: 0
Failed: 0

Frontend:
Tests: 121 passed (11 test files, 100% preserved)
Failed: 0
```

---

## 3. Created Runtime Contract Models

Module path: `src-tauri/src/engine/runtime_config.rs`

1. **`RuntimeConfigCandidate`:** Raw, unverified candidate payload (`preset_id`, `preset_args`, `RuntimeBypassCandidate`, `RuntimeDnsCandidate`, `RuntimeSecurityCandidate`).
2. **`RuntimeBypassMode`:** Typed authoritative enum (`All`, `Whitelist`, `Blacklist`) with `From` conversion to `LaunchBypassMode`.
3. **`ConfigRevision`:** Monotonic revision counter (`ConfigRevision::new`, `get`, `checked_next` with overflow protection).
4. **`ConfigFingerprint`:** 64-character SHA-256 hex string generated deterministically from canonical config.
5. **`VerifiedRuntimeConfig`:** Immutable verified snapshot carrying `ConfigRevision`, `ConfigFingerprint`, `VerifiedPresetConfig`, `VerifiedBypassConfig`, `VerifiedDnsConfig`, and `VerifiedSecurityConfig`.
6. **`PreparedRuntimeConfig`:** Pre-execution snapshot containing `VerifiedRuntimeConfig`, `EngineLaunchPlan`, and `PreparedHostlist`.
7. **`AppliedRuntimeConfig`:** Post-execution snapshot containing `VerifiedRuntimeConfig`, OS `process_id` (PID), `applied_at` timestamp, and `AppliedVerification::ProcessStarted`.
8. **`RuntimeConfigError`:** Typed error enum with `From<RuntimeConfigError> for EngineError`.
9. **`RuntimeConfigSummary`:** Safe telemetry summary (`revision`, `fingerprint_prefix`, `preset_id`, `bypass_mode`, `domain_count`, `dns_protocol`, `kill_switch`).

---

## 4. Configuration Pipeline Flow

```text
Source Girdisi (settings.json / BYPASS_CONFIG_CACHE)
       │
       ▼
candidate_from_preset_and_sources (RuntimeConfigCandidate)
       │
       ▼
verify_runtime_config (Validation + Domain Canonicalization + SHA-256 Fingerprint)
       │
       ▼
VerifiedRuntimeConfig
       │
       ▼
to_launch_bypass_input -> build_engine_launch_plan
       │
       ▼
PreparedRuntimeConfig (Plan + Hostlist Plan + Verified Snapshot)
       │
       ▼
EngineManager::spawn_and_run (Child process spawn)
       │
       ▼
AppliedRuntimeConfig::process_started (Verified Snapshot + OS PID)
```

---

## 5. Fingerprint Contract & Redaction Specification

- **Algorithm:** SHA-256 hex string (`schema:1` prefix).
- **Included Fields:** `preset_id`, `preset_args`, `bypass_mode`, `canonical_domains` (sorted), `kill_switch`, `dns_enabled`, `dns_protocol`, `dns_provider`, `dns_adblock`, `dns_cache`.
- **Excluded Fields:** `revision`, timestamp, PID, platform path, appdata_dir, secrets/credentials.
- **Domain Order:** Deterministically sorted for canonical hash equality (`a.com, b.com` == `b.com, a.com`).
- **Preset Arg Order:** Original command-line sequence preserved (`--dpi-desync=syndata,fake` != `--dpi-desync=fake,syndata`).
- **Telemetry Redaction:** Custom `Debug` implementation for `VerifiedRuntimeConfig` redacts raw domain lists and displays `domain_count` and `fingerprint_prefix` (first 8 hex chars).

---

## 6. Prepared vs Applied State Separation

- **`PreparedRuntimeConfig`:** Represents a fully validated, planned execution state. Does NOT guarantee process execution.
- **`AppliedRuntimeConfig`:** Represents an active execution state tied to a live OS PID and verified process start.
- **UI Impact:** The Rust backend now maintains strict type-level separation between prepared and applied states. Arayüz (Zustand store) seviyesinde IPC uyumluluğunu bozmamak adına bu ayrımın tam görselleştirilmesi **P07**'ye devredilmiştir.

---

## 7. EngineManager Integration

- **File:** `src-tauri/src/engine/manager.rs`
- **Function:** `spawn_and_run`
- **Changes:** Integrated `candidate_from_preset_and_sources`, `verify_runtime_config`, `PreparedRuntimeConfig`, and `AppliedRuntimeConfig::process_started` into the startup pipeline.
- **Runtime Behavior Changed:** **No** (Disk reading, atomic `domains.txt` file writing, kill switch execution, and process spawn sequence are 100% preserved).

---

## 8. Parity Verification Results

| Scenario | Legacy Execution vs P04 Runtime Contract | Status | Evidence Test |
| :--- | :--- | :---: | :--- |
| **Windows All Mode** | Exact match | **MATCHED** | `a01_valid_all_mode_candidate` |
| **Windows Whitelist Mode** | Exact match | **MATCHED** | `a02_valid_whitelist_candidate` |
| **Windows Blacklist Mode** | Exact match | **MATCHED** | `make_test_candidate("blacklist", ...)` |
| **Linux Mode** | Exact match | **MATCHED** | `to_launch_bypass_input` |
| **TR-1 Preset** | Exact match | **MATCHED** | `candidate_from_preset_and_sources` |
| **TR-2 Preset** | Exact match | **MATCHED** | `candidate_from_preset_and_sources` |
| **Custom Preset** | Exact match | **MATCHED** | `f06_preset_arg_order_difference_changes_fingerprint` |
| **Invalid Preset** | Exact match (Sanitizer error returned) | **MATCHED** | `verify_runtime_config` |
| **Cache/Disk Reproducer** | Preserved (Reads disk config as before) | **MATCHED** | `documents_runtime_contract_still_receiving_persisted_disk_config` |

---

## 9. Preserved Known Gaps (Intentionally Unfixed in P04)

1. **Pattern Cache vs Disk State Race:** P04 creates the runtime contract but intentionally maintains legacy disk reading (`read_bypass_config`); deferred to **P06**.
2. **Hostlist Revision Paths:** `PreparedHostlist` uses standard `domains.txt` path without revision suffixing; deferred to **P06**.
3. **Rollback & Transactional Recovery:** Failure to spawn does not roll back settings; deferred to **P06**.
4. **Running vs Healthy Separation:** `AppliedVerification::ProcessStarted` checks process start/PID, not network health probe; deferred to **P07** & **P14**.
5. **Process Ownership & Global Cleanup:** Legacy process management preserved; deferred to **P07**.

---

## 10. Verification Commands

```text
Command: cargo fmt --check
Result: PASSED (0 formatting diffs)
Warnings: 0

Command: cargo test --lib
Result: PASSED (179 passed; 0 failed; 0 ignored; 0.27s)
Warnings: 0

Command: cargo clippy --lib -- -D warnings
Result: PASSED (0 warnings)
Warnings: 0

Command: npm test
Result: PASSED (11 test files, 121 tests passed)
Warnings: None

Command: npm run build
Result: PASSED (tsc && vite build successful)
Warnings: None
```

---

## 11. Git Working Tree

```text
Modified:
  src-tauri/src/engine/manager.rs
  src-tauri/src/engine/mod.rs
  src-tauri/src/characterization/mod.rs
  docs/production-hardening/02-risk-register.md
  docs/production-hardening/03-test-matrix.md
  docs/production-hardening/05-known-gaps.md

Added:
  src-tauri/src/engine/runtime_config.rs
  src-tauri/src/characterization/runtime_config_tests.rs
  docs/production-hardening/reports/P04-runtime-configuration-contract.md

Pre-existing Files:
  All preserved
```

---

## 12. New Identified Risks

```text
None
```

---

## 13. Transition Decision

```text
READY FOR P05
```

---

## 14. Explicit Non-Actions Confirmation

We explicitly confirm that the following non-actions were adhered to during P04:
- ✅ Pattern cache/disk source-of-truth bug was NOT fixed.
- ✅ Pattern transactions were NOT added.
- ✅ Revision-based hostlist file paths were NOT added.
- ✅ Pattern rollback was NOT added.
- ✅ Process ownership was NOT changed.
- ✅ Global process cleanup was NOT removed.
- ✅ Running vs Healthy separation was NOT implemented.
- ✅ Readiness probes were NOT added.
- ✅ Preset phase validation was NOT added.
- ✅ Built-in presets were NOT modified.
- ✅ Linux firewall behavior was NOT changed.
- ✅ Linux UDP support was NOT added.
- ✅ DNS or Kill Switch behavior was NOT changed.
- ✅ DoQ implementation was NOT added or removed.
- ✅ Optimizer was NOT modified.
- ✅ IPC breaking changes were NOT made.
- ✅ Frontend production behavior was NOT modified.
- ✅ Version bump was NOT performed.
- ✅ Release or tag was NOT created.
- ✅ User's P00/P01/P02/P03 files were NOT deleted or reset.
- ✅ P05 (Low-Risk Deterministic Fixes) phase was NOT auto-started; execution stopped cleanly.
