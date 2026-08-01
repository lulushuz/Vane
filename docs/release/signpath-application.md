# SignPath Foundation Application Document for Vane DPI

## Project Identity
- **Project Name:** Vane
- **Repository URL:** https://github.com/lulushuz/Vane
- **License:** MIT License
- **Target Platform:** Windows 11 x64 (Secondary: Linux x64)

## Project Overview
Vane is an open-source Desktop Network Bypass & Privacy Utility for Windows and Linux. It allows users to bypass Deep Packet Inspection (DPI) censorship and manage encrypted DNS (DoH/DoT) settings with precise, local process ownership.

## Technical Execution & Privilege Rationale
- **Administrator Elevation:** Required solely for WinDivert driver initialization (kernel packet filtering) and local DNS forwarder socket binding.
- **WinDivert Usage:** Enables user-space packet modification for desynchronization strategies without modifying permanent system system32 binaries.
- **Process Ownership:** Vane manages only its dedicated sub-processes (`winws.exe`, `WinDivert64.sys`) tagged with unique installation UUIDs. Global process kills (`taskkill /F /IM`) and blanket firewall flushes are strictly prohibited.

## Security & Tedarik Zinciri Güvenliği
- **Build System:** Sealed GitHub Actions CI workflows (`.github/workflows/releases.yml`).
- **Dependency Policy:** Locked builds via `Cargo.lock` and `package-lock.json`. Zero high/critical vulnerabilities enforced by `cargo audit` and `npm audit`.
- **Artifact Integrity:** Fail-closed SHA-256 binary integrity verifier embedded in the Rust backend.
- **Signing Policy:** Private keys are never exported or made accessible to maintainers; signing is conducted strictly via isolated CI workflows using trusted SignPath / GitHub secrets.

## Maintainer Contact Placeholder
- **Maintainer:** [INSERT MAINTAINER NAME]
- **Email:** [INSERT MAINTAINER EMAIL]
