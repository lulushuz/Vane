# Vane 2.0.8 Threat Model & Security Architecture

**Framework:** STRIDE / DREAD Model  
**Target Release:** v2.0.8  

---

## Threat Actor Profiles & Risk Assessment

### 1. Compromised Renderer (Webview XSS)
- **Asset:** System-level command execution, local file access, WinDivert driver manipulation.
- **Entry Point:** Injected malicious script in Webview context via unescaped log string or remote HTTP payload.
- **Trust Boundary:** Frontend Webview ↔ Tauri Rust Core IPC.
- **Existing Mitigations:** Strict Content Security Policy (CSP), TypeScript IPC payload serialization, allowlist sanitization of winws arguments in Rust backend.
- **Residual Risk:** Because the entire process runs elevated, a remote code execution in Webview inherits Administrator rights.

### 2. Malicious Local User Process (Same Machine Hostile Application)
- **Asset:** User settings (`settings.json`), DNS snapshots, WinDivert kernel handle.
- **Entry Point:** Modifying AppData configuration files or injecting IPC messages into open Webview socket.
- **Trust Boundary:** Local Non-Admin Process ↔ Elevated Vane Process.
- **Existing Mitigations:** Rust backend validates JSON schema on startup and recovers from `.bak` backup if primary file is corrupted.
- **Residual Risk:** Non-admin user can edit `settings.json`, which will be parsed by Vane on elevated launch. Mitigated via strict backend argument allowlisting (`sanitizer.rs`).

### 3. Malicious Imported Preset (`.vane` File)
- **Asset:** WinWS command line execution, WinDivert rule injection.
- **Entry Point:** User imports a crafted `.vane` preset file containing malicious flags.
- **Trust Boundary:** External File ↔ Config Loader Parser.
- **Existing Mitigations:** `sanitizer.rs` enforces strict argument allowlist; `--hostlist` and `--hostlist-exclude` overrides in preset files are explicitly stripped.
- **Residual Risk:** Low. Sanitizer rejects unknown or dangerous flags.

### 4. Malicious Remote Blocklist Server / DNS Upstream
- **Asset:** RAM exhaustion, DNS hijacking, AdBlock list poisoning.
- **Entry Point:** Upstream AdBlock URL returning oversized or malformed text files.
- **Trust Boundary:** External HTTPS Endpoint ↔ Local DNS Forwarder Engine.
- **Existing Mitigations:** Streaming size limits (10 MB max), strict MIME-type validation, atomic cache replacement, timeout enforcement.
- **Residual Risk:** Minimal. Upstream errors fail-safe to local cache.

### 5. Network Attacker (Man-in-the-Middle)
- **Asset:** Plaintext DNS queries, SNI inspection.
- **Entry Point:** Passive eavesdropping on local network.
- **Trust Boundary:** Local Machine ↔ Remote DoH/DoT Resolver.
- **Existing Mitigations:** Enforced TLS 1.3 for DoH/DoT; SOCKS5 proxying routed via SOCKS5H (remote hostname resolution) to eliminate local leaks.
- **Residual Risk:** None identified.

### 6. Compromised Build Pipeline (CI/CD Action)
- **Asset:** Production releases, updater binary signatures.
- **Entry Point:** Malicious PR or compromised GitHub Action dependency.
- **Trust Boundary:** GitHub Actions Workflow ↔ Production Release Assets.
- **Existing Mitigations:** Workflow actions pinned to full 40-character commit SHAs; `permissions: read-all` default scoping; protected release environment.
- **Residual Risk:** Low.
