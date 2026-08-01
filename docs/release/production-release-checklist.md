# Production Release Checklist — Vane DPI v1.0.0-rc.1

## Pre-Release Gate Verification
- [x] All 237 Rust unit and characterization tests passing (`cargo test --lib --locked`)
- [x] All 152 Vitest frontend tests passing (`npm test`)
- [x] Production web bundle compiled without errors (`npm run build`)
- [x] Zero high/critical npm vulnerabilities (`npm audit --audit-level=high`)
- [x] Zero direct Rust crate vulnerabilities (`cargo audit`)
- [x] Artifact integrity manifest verified against embedded signatures
- [x] Version parity verified across `package.json`, `Cargo.toml`, `tauri.conf.json`, `native-artifacts.json` (`node scripts/release/check-version.cjs`)
- [x] Machine-readable SBOM generated (`node scripts/release/generate-sbom.cjs`)
- [x] Release readiness manifest updated (`artifacts/release-readiness.json`)

## Code Signing & Key Security
- [ ] Windows Authenticode Code Signing Certificate (`.pfx`) applied (Pending live release pipeline)
- [ ] Tauri Updater Minisign Signature generated (Pending live release pipeline)
- [ ] Timestamp server signature verified
- [ ] Zero private keys stored in repository or workflow files

## Acceptance Testing
- [ ] Clean Windows 11 VM acceptance test executed ([windows-acceptance.md](docs/release/windows-acceptance.md))
- [ ] Clean Linux VM acceptance test executed ([linux-acceptance.md](docs/release/linux-acceptance.md))

## Release Decision
- **Current Status:** `BLOCKED` (Unsigned Release Candidate — Requires live Code Signing Certificate & privileged VM acceptance)
