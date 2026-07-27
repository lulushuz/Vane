# Vane Architecture & SOLID Audit Review

---

## SOLID Violations & Refactoring Analysis

### 1. Single Responsibility Principle (SRP)
- **Violation:** `src-tauri/src/commands.rs` (1,141 lines) acts as a monolithic authority hub. It handles IPC deserialization, preset loading, DNS forwarder management, settings serialization, and window management simultaneously.
- **Violation:** `src/store/engineStore.ts` (636 lines) mixes React UI state, Zustand persist configuration, Tauri IPC calls, log translation regexes, and domain list parsing.
- **Remediation Plan:** Split `commands.rs` into domain-specific modules (`ipc/preset_commands.rs`, `ipc/dns_commands.rs`, `ipc/engine_commands.rs`). Move log translation out of `engineStore.ts` into a dedicated UI presenter module.

### 2. Open/Closed Principle (OCP)
- **Violation:** Adding a new DNS protocol or desync method requires modifying large `match` blocks across `forwarder.rs`, `manager.rs`, and `sanitizer.rs`.
- **Remediation Plan:** Implement a `DnsResolver` trait and a `DesyncStrategy` strategy pattern in Rust, allowing new protocols/strategies to be registered without editing core execution loops.

### 3. Liskov Substitution Principle (LSP)
- **Status:** Satisfied. The `EngineEventDispatcher` trait in `manager.rs` allows `AppHandle` or mock test dispatchers to be substituted transparently.

### 4. Interface Segregation Principle (ISP)
- **Violation:** All Webview windows receive access to every registered Tauri command via global IPC bindings, exposing administrative engine controls to auxiliary windows.
- **Remediation Plan:** Scope IPC capabilities per window identifier using Tauri v2 capability ACL files (`capabilities/main.json`, `capabilities/settings.json`).

### 5. Dependency Inversion Principle (DIP)
- **Violation:** High-level engine lifecycle code in `manager.rs` directly invokes platform-specific Windows `CommandExt` process creation APIs.
- **Remediation Plan:** Abstract process management behind a `ProcessRunner` trait, isolating Windows-specific Job Object creation into a dedicated platform adapter.

---

## Target Architecture Blueprint (v2.1.0 Roadmap)

```
┌─────────────────────────────────────────────────────────────────┐
│                    UNPRIVILEGED REACT UI                        │
│             (Runs in standard User context)                     │
└────────────────────────────────┬────────────────────────────────┘
                                 │ Authenticated IPC
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    PRIVILEGED BROKER SERVICE                    │
│             (Runs as low-privilege Windows Service)             │
│                                                                 │
│  ┌───────────────────────┐           ┌───────────────────────┐  │
│  │ EngineRuntime Machine │           │ Transactional DNS     │  │
│  └───────────┬───────────┘           └───────────┬───────────┘  │
└──────────────│───────────────────────────────────│──────────────┘
               ▼                                   ▼
┌─────────────────────────────────────────────────────────────────┐
│                      PLATFORM ADAPTERS                          │
│  ┌───────────────────────┐           ┌───────────────────────┐  │
│  │ WinDivert / JobObject │           │ Windows Firewall API  │  │
│  └───────────────────────┘           └───────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```
