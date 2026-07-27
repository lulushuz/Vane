# Vane 2.0.8 Comprehensive Audit Report

**Date:** July 27, 2026  
**Target Release:** v2.0.8  
**Repository:** [lulushuz/Vane](https://github.com/lulushuz/Vane)  
**Lead Architect & Senior Security Auditor:** Antigravity AI  

---

## 1. Executive Summary

An end-to-end, multi-dimensional audit of the Vane 2.0.8 codebase was performed. Vane is a desktop DPI bypass application built on React, TypeScript, Zustand, Tauri v2, and Rust, interfacing directly with Windows Kernel Network Drivers (WinDivert / winws), Firewall Rules (netsh/Windows Filtering Platform), and local DNS resolution chains.

The codebase has undergone significant hardening in PRs #2 through #14. High-risk flaws—such as unauthenticated SSRF via health check endpoints, race-condition settings wipes, memory-unsafe handle leaks, unvalidated argument smuggling, and uncontrolled DoH SOCKS5 leaks—have been resolved at the source-code and unit-test levels.

However, **Vane v2.0.8 is NOT ready for a stable public release**. This decision is governed by a **Fail-Closed Release Gate**:
1. **Packaging Blocker:** The GitHub Actions `releases.yml` / `acceptance.yml` workflow failed during artifact signing due to a newline format error in the `TAURI_SIGNING_PRIVATE_KEY` secret.
2. **Missing Runtime Evidence:** Physical Windows acceptance harness testing (real TCP/QUIC traffic bypass verification, Wireshark/pktmon port 53 leak capture, and reboot/autostart persistence) has not yet been executed on the signed binary package.

---

## 2. Scope & Audit Boundaries

### Included Components
- **Frontend Layer:** React components, TypeScript models, Zustand store state & persistence hooks, CustomSelect UI elements, translations.
- **IPC & Security Boundary:** Tauri command handlers (`commands.rs`), IPC contracts (`ipc.rs`), window privilege scoping.
- **Engine Core:** `EngineManager` lifecycle, `ProcessHandle`, `JobObjectGuard`, argument sanitizer (`sanitizer.rs`), Optimizer pipeline.
- **DNS Subsystem:** Local DoH/DoT forwarder (`forwarder.rs`), Hickory DNS 0.26 resolver integration, Smart DNS LRU Cache, SOCKS5 proxying, AdBlock/Malware filter downloader.
- **System Integration:** Windows Firewall Kill Switch (`killswitch.rs`), DNS adapter snapshot/restore system (`adapter.rs`), elevated privilege checking (`checker.rs`).
- **Release Chain:** GitHub Actions workflows (`releases.yml`, `acceptance.yml`, `verify.yml`), updater signature pipeline, bundled binary manifests.

### Audit Methodologies
- Static Code Analysis & AST Inspection
- Threat Modeling (STRIDE / DREAD against 10 actor profiles)
- Control Flow & Concurrency State Machine Tracing
- Architectural Dependency Mapping & SOLID Principle Violation Audit
- Supply Chain & Dependency Vulnerability Scanning (`cargo audit`, `npm audit`)

---

## 3. System Architecture & Component Map

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                   FRONTEND LAYER                                       │
│    React 18 UI ──> Zustand Store (engineStore.ts) ──> Typed IPC Serializer (ipc.rs)     │
└──────────────────────────────────────────┬─────────────────────────────────────────────┘
                                           │ Tauri IPC Commands
                                           ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                  TAURI BACKEND LAYER                                   │
│  ┌───────────────────────┐   ┌────────────────────────┐   ┌─────────────────────────┐  │
│  │ Commands & Scoping    │   │ Engine Lifetime Manager│   │ Local DNS Forwarder     │  │
│  │ (src/commands.rs)     │   │ (src/engine/manager.rs)│   │ (src/dns/forwarder.rs)  │  │
│  └───────────┬───────────┘   └───────────┬────────────┘   └────────────┬────────────┘  │
└──────────────│───────────────────────────│─────────────────────────────│───────────────┘
               │                           │                             │
               ▼                           ▼                             ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              WINDOWS OS & DRIVER LAYER                                 │
│  ┌───────────────────────┐   ┌────────────────────────┐   ┌─────────────────────────┐  │
│  │ WinDivert / winws.exe │   │ Windows Firewall API   │   │ Network Adapter Config  │  │
│  │ (Process & Job Object)│   │ (netsh advfirewall)    │   │ (netsh interface ip)    │  │
│  └───────────────────────┘   └────────────────────────┘   └─────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Trust Boundaries
1. **Renderer ↔ Tauri IPC Boundary:** Unprivileged JavaScript execution context communicating with Rust via Tauri IPC.
2. **User Storage ↔ System State:** Settings persisted in user AppData (`settings.json`) consumed by an elevated process during autostart.
3. **Rust Process ↔ Kernel Driver:** User-mode Rust application passing raw socket rules to the `WinDivert.sys` driver.
4. **Local Resolver ↔ Remote Upstream:** Local DNS listening socket (`127.0.0.1:53`) relaying queries over DoH/DoT/SOCKS5.

---

## 4. Architectural Answers & System Realities

- **Runtime State Owner:** The Rust backend (`EngineManager` + `DnsForwarderHandle`) is the canonical source of truth for runtime execution status. Frontend Zustand state is a reactive reflection.
- **Multiple "Truths":** Prior to v2.0.8, 4 competing truths existed (Zustand store, `settings.json`, Rust cache, Windows system registry). In v2.0.8, `settings.json` maintained strictly by Rust serves as the single offline truth.
- **Unified Engine Transaction:** Pattern and DNS settings are synchronized immediately before `EngineManager::start` is invoked. If DNS forwarder setup fails, the engine startup is rolled back.
- **Rollback Responsibility:** Handled via Rust RAII guards (`KillSwitchGuard`, `JobObjectGuard`, `DnsRestoreSnapshot`).
- **Cleanup Responsibility:** Rust `Drop` implementations and process shutdown hooks ensure Windows Firewall rules and DNS adapter settings are restored even on unexpected crashes.

---

## 5. Security & Stability Assessment Summary

| Metric | Status | Evaluation |
| :--- | :--- | :--- |
| **Static Code Hardening** | 🟢 **PASS** | Critical SSRF, argument smuggling, and handle leaks resolved. |
| **Type Safety & IPC** | 🟢 **PASS** | Strict typed IPC errors and monotonic revision IDs implemented. |
| **Dependency Security** | 🟢 **PASS** | `cargo audit` and `npm audit` report 0 known vulnerabilities. |
| **Build & Compilation** | 🟢 **PASS** | `tsc && vite build` and `cargo check` build clean with 0 warnings. |
| **Updater Signing** | 🔴 **FAIL (Blocker)**| Private key secret formatting error breaks `.sig` generation in CI. |
| **Runtime Acceptance** | 🟡 **PENDING** | Physical Windows driver & packet capture evidence pending execution. |

---

## 6. Release Readiness Conclusion

**Decision: NOT READY (Blocked on Artifact Signing & Physical Evidence)**

Vane 2.0.8 cannot be published as a stable production release until the `TAURI_SIGNING_PRIVATE_KEY` formatting issue is corrected in GitHub Secrets and the resulting signed release package passes physical Windows acceptance testing.
