# Vane 2.0.8 Prioritized Remediation Plan

---

## Phase 1: Critical Release Blockers & Packaging Fixes (Immediate)

### Phase 1.1: GitHub Secret Format Correction
- **Target:** Fix `TAURI_SIGNING_PRIVATE_KEY` formatting in GitHub Repository Settings.
- **Action:** Re-enter the base64 private key string without trailing or leading linebreaks.
- **Module Affected:** GitHub Secrets Configuration / `.github/workflows/releases.yml`.
- **Validation:** Re-run `acceptance.yml` workflow and verify `.sig` generation succeeds.
- **Scope Version:** Patch Release (`v2.0.8`).

### Phase 1.2: Release Workflow Checklist Alignment
- **Target:** Ensure GitHub Actions workflow strictly enforces checklist claims.
- **Action:** Add version verification check (`git tag` vs `package.json` vs `Cargo.toml`) in `releases.yml`. Set `releaseDraft: true` by default.
- **Module Affected:** `.github/workflows/releases.yml`.
- **Validation:** Test tag workflow dry run.
- **Scope Version:** Patch Release (`v2.0.8`).

---

## Phase 2: Physical Windows Acceptance Testing (Pre-Release Gate)

### Phase 2.1: Execution of Windows Acceptance Harness
- **Target:** Execute `scripts/windows-acceptance.ps1` on an elevated Windows 11 test machine using the signed v2.0.8 package.
- **Validation Checklist:**
  - [ ] Real winws child command line inspection.
  - [ ] Whitelist positive bypass (Discord/Roblox connectivity check).
  - [ ] Whitelist negative isolation (non-listed domains bypass bypass).
  - [ ] TCP 443 and QUIC/UDP 443 behavior.
  - [ ] Wireshark/pktmon capture verifying 0 plaintext DNS leaks on port 53 while Kill Switch is active.
  - [ ] Reboot/Autostart persistence verification.
- **Scope Version:** Pre-Release Validation (`v2.0.8`).

---

## Phase 3: Post-Release Architectural Refactoring (v2.1.0 Minor Release)

### Phase 3.1: Scoped Window IPC Capabilities
- **Target:** Restrict administrative command execution to the main window context.
- **Action:** Define explicit Tauri v2 capability ACL files (`capabilities/main.json`, `capabilities/settings.json`).
- **Module Affected:** `src-tauri/capabilities/`, `src-tauri/tauri.conf.json`.
- **Scope Version:** Minor Release (`v2.1.0`).

### Phase 3.2: Privileged Broker Service Architecture
- **Target:** Separate unprivileged React UI from elevated Windows driver execution.
- **Action:** Implement a background Windows service that acts as a Privileged Broker.
- **Module Affected:** Architectural Overhaul.
- **Scope Version:** Minor Release (`v2.1.0`).
