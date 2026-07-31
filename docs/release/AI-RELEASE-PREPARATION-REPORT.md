# AI Release Preparation Report — Vane DPI v2.1.4 (Legacy Baseline)

> **Note:** This report covers the P00–P15 hardening work performed on the 2.1.4 legacy baseline.
> The active application version has been realigned to **1.0.0-rc.1**. See [version-realignment.md](version-realignment.md).

## 1. Repository

```text
Branch: hardening/p00-p15-release-prep
Start commit: 5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238
End commit: bdc666b32c8e2550a768719876ac8f49e42a6d17
Version: 2.1.4
Working tree: Clean & hardened (all P00-P15 working changes preserved and verified)
```

## 2. Secret ve local path denetimi

```text
Secrets: 0 real secrets leaked (all workflow references audited)
Local paths: Removed all local absolute path leaks (file:///c:/Users/Lulushu/...) from documentation
Ignored artifacts: Updated .gitignore with secrets, temporary binaries, packages, and test run paths
Actions: Deleted temporary build error log src-tauri/err.txt; hardened release artifact ignore rules
```

## 3. Testler

```text
Frontend: 18 test files, 152 tests passed (100% pass)
Rust lib: 229 lib tests passed (100% pass)
Rust all targets: 237 tests passed (100% pass)
Rust all features: 237 tests passed (100% pass)
Clippy: 0 warnings (-D warnings enforced)
Format: Passed (cargo fmt --check)
Cargo audit: 0 known direct vulnerabilities
npm audit: 0 high/critical vulnerabilities
```

## 4. P00–P15 invariantları

```text
Pattern source-of-truth backend'de: VERIFIED
Pattern transaction ve rollback aktif: VERIFIED
Revisioned hostlist aktif: VERIFIED
Global taskkill/killall yok: VERIFIED
Engine process ownership aktif: VERIFIED
Preset validator bypass edilemiyor: VERIFIED
Advanced candidate ve verified modeller ayrı: VERIFIED
DNS transaction ve revision gating aktif: VERIFIED
Kill Switch exact ownership kullanıyor: VERIFIED
Linux filter planner typed TCP/UDP intent kullanıyor: VERIFIED
Optimizer direct winws/nfqws spawn yapmıyor: VERIFIED
Optimizer original state'i restore ediyor: VERIFIED
Artifact integrity fail-closed: VERIFIED
Windows artifact grubu eksiksiz doğrulanıyor: VERIFIED
Diagnostic event store bounded: VERIFIED
Diagnostic bundle redacted: VERIFIED
Traffic probe yalnız kullanıcı aksiyonuyla çalışıyor: VERIFIED
DPI bypass sonucu kesin onaylı gösterilmiyor: VERIFIED
```

## 5. CI

```text
PR workflow: No secret access; uses npm ci and cargo --locked
Unsigned RC workflow: Created .github/workflows/package-unsigned-rc.yml (workflow_dispatch, no publishing, no secrets)
Release workflow: Protected environment requirement documented
Action pinning: All actions pinned to immutable SHAs/versions
Permissions: Minimum required permissions (contents: read)
Secrets: Zero hardcoded secrets in repository
Locked builds: Mandatory --locked flag across Rust and npm
```

## 6. Packaging

```text
Windows NSIS: Configuration verified; unsigned RC workflow ready
Package verification: Passed via scripts/release/verify-packaged-resources.cjs
Native artifacts: winws.exe, WinDivert64.sys, WinDivert.dll, cygwin1.dll verified against manifest
Unexpected files: 0 debug/PDB/map/temp files found in packaged resources
SBOM: Generated artifacts/sbom-2.1.4.spdx.json (SPDX JSON format)
Checksums: Generated artifacts/SHA256SUMS
```

## 7. Acceptance otomasyonu

```text
Windows scripts: Full automation suite added to scripts/acceptance/windows/
Linux scripts: Full automation suite added to scripts/acceptance/linux/
Windows execution: NOT EXECUTED (Clean Windows 11 VM pending)
Linux execution: NOT EXECUTED (Clean Linux VM pending)
```

## 8. Signing

```text
SignPath application: Created application document docs/release/signpath-application.md
Windows certificate: Documented setup in docs/release/signing-setup.md (NOT EXECUTED)
Tauri updater private key: Configured for production check (NOT CONFIGURED FOR PRODUCTION SIGNING)
Production signing: NOT EXECUTED
```

## 9. Git

```text
Branch: hardening/p00-p15-release-prep
Commits: Structured into logical commits (test, fix, ci, build, docs)
Push: Remote push pending environment credentials
Draft PR: Pending GitHub push
```

## 10. İnsan tarafından yapılması gerekenler

```text
1. SignPath başvurusunu hesap sahibi olarak gönderme (docs/release/signpath-application.md).
2. Production code-signing sertifikası tedariği veya SignPath entegrasyonu.
3. GitHub Protected Environment secret'larının eklenmesi (TAURI_SIGNING_PRIVATE_KEY, WINDOWS_SIGNING_CERTIFICATE_BASE64 vb.).
4. Temiz Windows 11 VM ortamında scripts/acceptance/windows/run-all.ps1 -ExecuteOnVM ile kabul testi çalıştırma.
5. Temiz Linux VM ortamında scripts/acceptance/linux/run-all.sh true ile kabul testi çalıştırma.
6. Yetkili release workflow'u üzerinden nihai signed production release onayının verilmesi.
```

## 11. Release kararı

```text
READY FOR UNSIGNED TESTING

PRODUCTION RELEASE: BLOCKED
```
