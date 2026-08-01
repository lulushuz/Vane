# Native Artifact Provenance & Supply-Chain Security

This document records the upstream provenance, expected binary integrity metrics, and license terms for all third-party native artifacts bundled within Vane DPI.

---

## 1. Bundled Native Executables & Libraries Inventory

| Artifact ID | Relative Path | Size (Bytes) | SHA-256 Digest | Upstream Project | Version / Commit | License |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`windows-winws`** | `binaries/winws-x86_64-pc-windows-msvc.exe` | 223,232 | `2da71e80878dc270ac83f5893ecbb841f9752a57f1da8ff9325636b4346bc632` | [bol-van/zapret](https://github.com/bol-van/zapret) | `v1.6.2` | GPL-3.0-or-later |
| **`windows-windivert-sys`** | `binaries/WinDivert64.sys` | 94,144 | `8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2` | [basil00/Divert](https://github.com/basil00/Divert) | `v2.2.0` | LGPL-3.0-or-later |
| **`windows-windivert-dll`** | `binaries/WinDivert.dll` | 47,616 | `c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2` | [basil00/Divert](https://github.com/basil00/Divert) | `v2.2.0` | LGPL-3.0-or-later |
| **`windows-cygwin-dll`** | `binaries/cygwin1.dll` | 2,954,293 | `103104a52e5293ce418944725df19e2bf81ad9269b9a120d71d39028e821499b` | [Cygwin Project](https://cygwin.com) | `v3.4.6` | GPL-3.0-or-later |
| **`linux-nfqws`** | `binaries/nfqws-x86_64-unknown-linux-gnu` | 125,760 | `8d3452ce0e0b9d9fed2a3a087b1caecfd39a910b7a31b304078fcbed3ea0e33c` | [bol-van/zapret](https://github.com/bol-van/zapret) | `v1.6.2` | GPL-3.0-or-later |

---

## 2. Bundled Content Artifacts Inventory

| Artifact ID | Relative Path | Size (Bytes) | SHA-256 Digest | Purpose | License |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`builtin-presets`** | `presets/builtin.json` | 5,332 | `6716028522e795b1245d982598f296b2014cd9ecfdcedab4196fc057d697b551` | Default curated bypass strategies | MIT / Apache-2.0 |
| **`remote-presets-template`** | `presets/remote_template.json` | 4,414 | `0f93fe72f68416a7cf0d2d4a0a7eb3dd5d2eb75e46233b42bdbc3bf6202b9187` | Remote preset signature template | MIT / Apache-2.0 |

---

## 3. Public Key Taxonomy & Role Separation

| Public Key Identifier | Source / Location | Purpose & Consumer | Fail-Closed Policy |
| :--- | :--- | :--- | :--- |
| `TauriUpdaterRelease` | `tauri.conf.json -> plugins.updater.pubkey` | Verifies Tauri release bundle signatures before installation | Unsigned updates rejected |
| `RemotePresetSignature` | `src-tauri/src/presets/mod.rs -> MINISIGN_PUBLIC_KEY` | Verifies Minisign signatures of remote strategy JSON payloads | Invalid signatures rejected |
