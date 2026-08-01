# P03 Engine Launch Planner Report

This document records the design, implementation, test cases, and parity verification for **P03 — Pure Engine Launch Planner & Side-Effect Boundary Separation**.

---

## 1. Executive Summary

- **Baseline Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238` (`baseline-2.1.4-2026-07-29`)
- **Initial Test Suite:** Frontend 121 tests passed, Rust 142 tests passed
- **Final Test Suite:** Frontend 121 tests passed, Rust 158 tests passed (+16 new P03 tests)
- **Side-Effect Purity:** 100% pure planner (`build_engine_launch_plan`) — 0 process spawns, 0 file writes, 0 firewall/DNS modifications, 0 Tauri `AppHandle` usage, 0 network calls.
- **Status:** ✅ **READY FOR P04**

---

## 2. Engine Launch Call-Graph Mapping

Prior to P03, planning, platform argument manipulation, hostlist file generation, kill switch application, and process spawning were tightly coupled inside `EngineManager::start` and `spawn_and_run`.

```text
Tauri IPC Command (start_engine)
       │
       ▼
ConfigLoader preset lookup
       │
       ▼
EngineManager::start
       ├─ validate_preset_args (Pure argument validation)
       ├─ privilege check (Platform check)
       ├─ EngineState::Starting state update & status emission (State mutation)
       └─ spawn_and_run
              ├─ resolve_binary_path (Resource path resolution & SHA-256 integrity check)
              ├─ read_bypass_config (Disk / settings read) -> validate_for_start
              │
              ├─► [P03 EngineLaunchPlanner Boundary] ◄─
              │      └─ build_engine_launch_plan (Pure, typed, deterministic plan construction)
              │
              ├─ Hostlist file write (Side effect: atomic_replace_bytes to domains.txt)
              ├─ DNS Kill Switch check & apply_kill_switch (Side effect: Windows netsh / Linux iptables)
              ├─ Process spawn (Side effect: OS Command execution)
              ├─ Status transition (Side effect: Running state update)
              └─ watch_process (Side effect: background async task supervisor)
```

---

## 3. Created Typed Data Structures & Planner API

All planner models and function definitions are located in `src-tauri/src/engine/launch_plan.rs`:

```rust
pub(crate) enum EnginePlatform { Windows, Linux }
pub(crate) enum EngineBinaryKind { Winws, Nfqws }

pub(crate) struct EngineBinaryPlan {
    pub executable: PathBuf,
    pub working_directory: PathBuf,
    pub kind: EngineBinaryKind,
}

pub(crate) enum LaunchBypassMode { All, Whitelist, Blacklist }

pub(crate) struct LaunchBypassInput {
    pub mode: LaunchBypassMode,
    pub domain_list: String,
    pub hostlist_path: Option<PathBuf>,
    pub kill_switch: bool,
}

pub(crate) struct TrafficFilterPlan {
    pub declared_tcp_spec: Option<String>,
    pub declared_udp_spec: Option<String>,
    pub effective_linux_tcp_spec: Option<String>,
    pub effective_linux_udp_spec: Option<String>,
}

pub(crate) enum HostlistPlan {
    None,
    Include { path: PathBuf, domain_count: usize },
    Exclude { path: PathBuf, domain_count: usize },
}

pub(crate) enum KillSwitchRequirement { Disabled, Required }

pub(crate) struct LinuxFirewallBehavior {
    pub tcp_ports: Vec<u16>,
    pub udp_ports: Vec<u16>,
    pub uses_nftables_fallback: bool,
    pub performs_global_process_cleanup: bool,
}

pub(crate) enum PlatformLaunchPlan {
    Windows { arguments: Vec<String> },
    Linux {
        arguments: Vec<String>,
        queue_number: u16,
        current_firewall_behavior: LinuxFirewallBehavior,
    },
}

pub(crate) struct EngineLaunchPlan {
    pub preset_id: String,
    pub binary: EngineBinaryPlan,
    pub hostlist: HostlistPlan,
    pub kill_switch: KillSwitchRequirement,
    pub platform: EnginePlatform,
    pub traffic_filter: TrafficFilterPlan,
    pub platform_launch: PlatformLaunchPlan,
    pub final_arguments: Vec<String>,
}

pub(crate) fn build_engine_launch_plan(
    input: EngineLaunchInput<'_>,
) -> Result<EngineLaunchPlan, EngineError>;
```

---

## 4. Separated Responsibilities

### Planning Responsibilities (Entrusted to `build_engine_launch_plan`):
1. Preset argument validation via existing `validate_preset_args`.
2. Binary platform selection (`Winws` vs `Nfqws`) and working directory derivation.
3. Hostlist include/exclude argument formatting (`--hostlist=` / `--hostlist-exclude=`).
4. Linux `--qnum=200` prepending and `--wf-*` argument stripping.
5. Declared vs effective traffic filter specs extraction (`declared_tcp_spec`, `declared_udp_spec`, `effective_linux_tcp_spec`).
6. Kill switch requirement tagging (`KillSwitchRequirement::Required`).
7. Deterministic final argument sequence generation.

### Side-Effect Responsibilities (Preserved in Runtime Layer):
1. Disk settings / bypass config reading (`read_bypass_config`).
2. Hostlist file disk writing (`domains.txt` atomic write).
3. System firewall / DNS Kill Switch application (`apply_kill_switch`).
4. OS Child process execution (`std::process::Command` / `tokio::process::Command`).
5. Engine lifecycle status updates (`EngineStatus::Starting`, `Running`, `Error`).
6. Process supervisor watching (`watch_process`).

---

## 5. Parity Test Results

| Test Scenario | Legacy Output vs Planner Output | Parity Status | Evidence |
| :--- | :--- | :---: | :--- |
| **Windows Default Preset** | Exact match | **MATCHED** | `p01_windows_default_preset_parity` |
| **Windows Whitelist Mode** | Exact match | **MATCHED** | `w02_whitelist_hostlist_include_windows` |
| **Windows Blacklist Mode** | Exact match | **MATCHED** | `w03_blacklist_hostlist_exclude_windows` |
| **Linux Default Preset** | Exact match (`--qnum=200` prepended, `--wf-*` stripped) | **MATCHED** | `l02_linux_includes_qnum_200_as_first_argument` |
| **Linux UDP Preset** | Exact match (declared UDP spec captured, effective Linux UDP none) | **MATCHED** | `l04_documents_current_linux_udp_filter_not_being_applied` |
| **TR-1 Preset** | Exact match | **MATCHED** | `p07_tr_1_preset_parity` |
| **TR-2 Preset** | Exact match | **MATCHED** | `build_engine_launch_plan` parity |
| **Custom Preset** | Exact match | **MATCHED** | `l03_documents_linux_wf_stripping_behavior` |

---

## 6. Preserved Known Gaps (Intentionally Unfixed in P03)

1. **Pattern Cache vs Disk State Race:** Planner accepts `LaunchBypassInput` from caller; cache authority is deferred to **P06**.
2. **Linux `--wf-*` Stripping & TCP 80/443 Hardcoding:** Planner accurately reflects current Linux behavior (stripping `--wf-*` and hardcoding TCP 80,443 NFQUEUE rules); Linux platform isolation is deferred to **P11**.
3. **Linux Effective UDP Filter Gap:** Planner captures `declared_udp_spec` while explicitly documenting `effective_linux_udp_spec = None`.
4. **Global Process Cleanup:** Planner captures `performs_global_process_cleanup = true` on Linux; cleanup isolation is deferred to **P11**.
5. **Optimizer Direct Execution:** Optimizer bypass of `EngineManager` is preserved; safety isolation is deferred to **P12**.

---

## 7. Verification Summary

```text
cargo fmt --check: PASSED (0 formatting diffs)
cargo test --lib: PASSED (158 passed, 0 failed, 0 ignored, 0.21s)
cargo clippy --lib -- -D warnings: PASSED (0 warnings)
npm test: PASSED (11 test files, 121 tests passed)
npm run build: PASSED (tsc & vite build successful)
```
