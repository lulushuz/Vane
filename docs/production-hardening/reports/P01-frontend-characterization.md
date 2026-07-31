# P01 Frontend Characterization Report

This document records the results and details of the **P01 Frontend Characterization Tests** phase.

---

## 1. Executive Summary

- **Baseline Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238` (`baseline-2.1.4-2026-07-29`)
- **Initial Test Suite:** 5 test files, 24 passed tests
- **Final Test Suite:** 11 test files, 121 passed tests (+97 tests added)
- **Rust Backend Changes:** 0 satır (Backend mocked via `mockIpc.ts`)
- **Production Runtime Changes:** 0 satır (Existing runtime behavior frozen as-is)
- **Status:** ✅ **READY FOR P02**

---

## 2. Test File Inventory

| Test File | Target Domain | Tests | Key Characterized Behavior |
| :--- | :--- | :---: | :--- |
| [src/test/mockIpc.ts](src/test/mockIpc.ts) | IPC Mock Helper | N/A | Intercepts `@tauri-apps/api/core` & `@tauri-apps/api/event` calls, records call sequence & payloads |
| [src/test/advancedConfig.test.ts](src/test/advancedConfig.test.ts) | Groups A, B, C | 29 | AdvancedConfig parser, serializer, passthrough preservation, and built-in preset round-trips |
| [src/test/storePersistence.test.ts](src/test/storePersistence.test.ts) | Groups D, K | 13 | Store write queue ordering, rejection resilience, hydration, and legacy domain list migration |
| [src/test/engineLifecycle.test.ts](src/test/engineLifecycle.test.ts) | Groups E, H, M | 19 | `startEngine` call order (`sync_bypass` → `sync_dns` → `start_engine`), stop handling, PID running state, and logging |
| [src/test/patternDnsSync.test.ts](src/test/patternDnsSync.test.ts) | Groups F, G, L | 15 | Pattern & DNS 100ms debounce, revision gating, DNS rollback on error, and domain helpers |
| [src/test/presetImportExport.test.ts](src/test/presetImportExport.test.ts) | Groups I, J | 13 | Custom preset form validation, `.json` export formatting, deletion, and IPC error normalization |
| [src/test/bugReproducers.test.ts](src/test/bugReproducers.test.ts) | Bug Reproducers | 8 | BR-01 through BR-08 (Persistence timing, `.vane` mismatch, DoQ coercion, Optimistic UI, Start/Stop race) |

---

## 3. Characterized Flow Summaries

1. **Store Persistence:**
   - Sequential write queueing via `enqueueStoreWrite` ensures `setItem` and `removeItem` maintain call ordering.
   - Non-Error rejections do not cause permanent queue deadlock.
   - Session fields (`status`, `logs`, `activeTab`, `dnsSynced`) are strictly omitted from `partialize` serialization.
2. **Pattern Synchronization:**
   - 100ms debounce window collapses rapid domain changes into a single `sync_bypass_config` IPC call.
   - Outdated backend responses are ignored via `bypassSyncRevision` check.
   - Verified backend canonical domain lists replace frontend local domain strings upon IPC resolution.
3. **DNS Synchronization & Rollback:**
   - Changes to DNS options (`dnsProtocol`, `dnsAdBlock`, `dnsCache`) record rollback state in `pendingDnsRollback`.
   - Rejection from `sync_dns_settings` triggers automatic rollback to last verified state and logs error.
   - DoQ selection is silently coerced to DoH in `sync_dns_settings` payload (`protocol: 'doh'`).
4. **Engine Lifecycle & Start Sequence:**
   - Standard engine launch follows deterministic sequence: `sync_bypass_config` → `sync_dns_settings` → `get_doh_forwarder_status` (if kill switch active) → `start_engine_with_dns_guard`.
   - `Running` status variant containing `pid` is set upon backend start response.
5. **Advanced Config Parser & Serializer:**
   - Known desync methods map directly to `desyncMethod`; unknown desync methods map to `desyncMethod = 'custom'` and `customDesyncMethod`.
   - Unsupported flags (`--mss`, `--bind-addr`, `--ipset`, `--socks`) are quarantined in `invalidArgs`.
   - Serializer output ordering is deterministic for identical `AdvancedConfig` objects.
6. **IPC Error Normalization:**
   - Structured Rust error objects (`code`, `message`, `operation`, `retryable`) preserve all metadata.
   - Plain strings and JavaScript Error instances normalize gracefully into `{ code: 'UNKNOWN', message: ... }`.

---

## 4. Bug Reproducers Summary

| Reproducer ID | Title | Target Phase | Documented Behavior |
| :--- | :--- | :---: | :--- |
| **BR-01** | Disk Persistence Timing Race | P06 | `startEngine` resolves before slow background disk write completes |
| **BR-02** | Export File Extension Mismatch | P08 | UI exports `.json` while backend expects signed `.vane` format |
| **BR-03** | DoQ Protocol Coercion | P10 | UI `doq` choice is converted to `doh` in IPC payload |
| **BR-04** | Optimistic Applied State | P04/P05 | Backend `prepared` stage is presented as applied in UI store |
| **BR-05** | PID-only Running State | P14 | Process presence (`pid`) treated as healthy without active traffic probe |
| **BR-06** | Non-443 UDP Argument Loss | P09 | Non-443 UDP port ranges in `--wf-udp=` omitted by parser |
| **BR-07** | Start/Stop In-flight Race | P07 | `stopEngine` called while `startEngine` is in flight can be overridden by late start resolution |
| **BR-08** | Stale DNS Response Override | P10 | Un-gated late DNS responses override newer UI choices |

---

## 5. Verification Results

```text
npm test: PASSED (11 test files, 121 tests passed)
npm run build: PASSED (tsc & vite build successful, 401 modules transformed)
npm audit --audit-level=high: PASSED (0 vulnerabilities found)
```
