# Vane 2.0.8 Detailed Audit Findings Log

---

## Finding Summary Matrix

| ID | Title | Severity | Confidence | Category | File & Line Location | Status | Release Blocker |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **VANE-FINDING-01** | Updater Signing Key Format Exception | **Critical** | Confirmed | Release Chain | `.github/workflows/releases.yml:55` | Open | **YES** |
| **VANE-FINDING-02** | Elevated WebView Renderer Attack Surface | **High** | Confirmed | Architecture / Security | `src-tauri/tauri.conf.json:1-70` | Architectural Debt | No (Patch Scope) |
| **VANE-FINDING-03** | Lack of Privileged UI-Broker Separation | **High** | Confirmed | Privilege Escalation | `src-tauri/src/lib.rs:100-250` | Architectural Debt | No (Minor Scope) |
| **VANE-FINDING-04** | Process Cleanup via Generic Process Name | **Medium** | Confirmed | System Integrity | `src-tauri/src/engine/manager.rs:850` | Resolved in v2.0.8 | No |
| **VANE-FINDING-05** | Unchecked User Export Absolute Path Write | **Medium** | Confirmed | Path Traversal | `src-tauri/src/commands.rs:710` | Mitigated | No |
| **VANE-FINDING-06** | Regex-Based English Translation of Logs | **Low** | Confirmed | Observability / UX | `src/store/engineStore.ts:395` | Open | No |
| **VANE-FINDING-07** | Release Checklist vs Workflow Discrepancy | **Low** | Confirmed | Release Quality | `RELEASE_CHECKLIST.md:120` | Open | No |

---

## Detailed Findings Breakdown

### VANE-FINDING-01: Updater Signing Key Format Exception
- **Severity:** Critical
- **Confidence:** Confirmed
- **Category:** Release Chain / CI-CD Pipeline
- **Affected Component:** `.github/workflows/releases.yml` & GitHub Secrets
- **File Location:** `.github/workflows/releases.yml` (Line 55)
- **Current Behavior:** The GitHub Actions workflow fails during the build step when Tauri attempts to sign the installer artifact using `TAURI_SIGNING_PRIVATE_KEY`.
- **Root Cause:** A trailing or leading newline character (`\n`) in the GitHub Repository Secret breaks the Minisign key parser inside `tauri-action`.
- **Impact:** Production artifacts (.msi, .exe) cannot be automatically signed, producing invalid `.sig` updater manifests.
- **Recommended Minimum Fix:** Re-enter the secret in GitHub Settings after trimming all whitespace and linebreaks.
- **Release Blocker:** **YES**

---

### VANE-FINDING-02: Elevated WebView Renderer Attack Surface
- **Severity:** High
- **Confidence:** Confirmed
- **Category:** Privilege Scoping / Renderer Compromise
- **Affected Component:** Whole Application Process Lifecycle
- **File Location:** `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`
- **Current Behavior:** The entire Tauri application, including the Chromium Webview2 frontend, runs elevated with full Administrator privileges on Windows.
- **Root Cause:** Windows network driver interaction (`WinDivert.sys`) requires elevation, forcing the wrapper process to elevate the entire app stack.
- **Impact:** An XSS or remote content injection in the renderer immediately grants system-level privileges to an attacker.
- **Recommended Target Architecture:** Separate the application into an unprivileged React UI process and a low-privilege background Windows Service (Privileged Broker).
- **Release Blocker:** No (Deferred to v2.1.0 Minor Release)

---

### VANE-FINDING-03: Lack of Privileged UI-Broker Separation
- **Severity:** High
- **Confidence:** Confirmed
- **Category:** Architecture & SOLID Principles
- **Affected Component:** Tauri Commands (`commands.rs`)
- **File Location:** `src-tauri/src/commands.rs` (Lines 1-1141)
- **Current Behavior:** Any open Webview window (main or settings) can invoke any registered Tauri command without origin verification.
- **Root Cause:** All IPC handlers are registered globally on `AppHandle` rather than scoped per window capability.
- **Impact:** If a secondary window is compromised, it can invoke raw DNS or firewall modification commands.
- **Recommended Target Architecture:** Implement caller-window validation and scoped capabilities per window.
- **Release Blocker:** No (Patch mitigations active)

---

### VANE-FINDING-04: Process Cleanup via Generic Process Name
- **Severity:** Medium
- **Confidence:** Confirmed
- **Category:** System Integrity / Process Ownership
- **Affected Component:** `EngineManager` Stop Logic
- **File Location:** `src-tauri/src/engine/manager.rs`
- **Current Behavior:** Legacy process termination targeted generic binary names (`winws.exe`), potentially killing unrelated instances.
- **Root Cause:** Lack of strict Windows Job Object tracking.
- **Mitigation in v2.0.8:** Process handles are now assigned exclusively to RAII-managed `JobObjectGuard` instances scoped by PID.
- **Release Blocker:** No (Resolved)

---

### VANE-FINDING-05: Unchecked User Export Absolute Path Write
- **Severity:** Medium
- **Confidence:** Confirmed
- **Category:** Arbitrary File Write / Path Traversal
- **Affected Component:** Custom Preset Export Handler
- **File Location:** `src-tauri/src/commands.rs` (Line 710)
- **Current Behavior:** Preset export writes to user-provided file paths.
- **Root Cause:** Direct file path acceptance without canonical path validation.
- **Mitigation in v2.0.8:** Extension is restricted to `.vane` and constrained to user-selected dialog paths.
- **Release Blocker:** No (Mitigated)

---

### VANE-FINDING-06: Regex-Based English Translation of Logs
- **Severity:** Low
- **Confidence:** Confirmed
- **Category:** Observability / Code Quality
- **Affected Component:** Log Translator Helper
- **File Location:** `src/store/engineStore.ts` (Line 395)
- **Current Behavior:** Certain raw backend logs are translated using regex matching on strings instead of structured event codes.
- **Impact:** Minor UX localization inconsistency if backend log strings format slightly changes.
- **Recommended Fix:** Migrate to a strongly-typed `EventCode` IPC payload.
- **Release Blocker:** No

---

### VANE-FINDING-07: Release Checklist vs Workflow Discrepancy
- **Severity:** Low
- **Confidence:** Confirmed
- **Category:** Release Assurance
- **Affected Component:** `RELEASE_CHECKLIST.md` vs `.github/workflows/releases.yml`
- **File Location:** `RELEASE_CHECKLIST.md` (Line 120)
- **Current Behavior:** Documentation lists tag-to-version validation and Zapret hash verification as active CI steps, but `releases.yml` lacks these explicit check steps.
- **Impact:** Discrepancy between release checklist claims and actual CI execution gates.
- **Recommended Fix:** Add explicit version verification step to `releases.yml`.
- **Release Blocker:** No
