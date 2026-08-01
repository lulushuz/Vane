# P00 Baseline Referansı

Bu doküman, Vane projesinin production hardening sürecine başlandığı tarihteki kesin repository durumunu, sürüm değerlerini, build ve test komutlarını, artifact envanterini ve kritik dosyaların SHA-256 hash manifestosunu kaydetmektedir.

---

## 1. Repository Bilgileri

| Metrik / Alan | Değer |
| :--- | :--- |
| **Repository Adı** | `lulushuz/Vane` |
| **Ana Branch** | `main` |
| **Commit SHA** | `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238` |
| **Commit Tarihi** | `2026-07-29 04:50:12 +0300` |
| **Son Commit Mesajı** | `feat(preset): include both TR 1 Classic Split and TR 2 MultiSplit Focus presets (release 2.1.4)` |
| **Repository Visibility** | Public |
| **Lisans** | GNU General Public License v3.0 (`GPL-3.0`) |
| **Uygulama Sürümü (`package.json`)** | `2.1.4` |
| **Uygulama Sürümü (`Cargo.toml`)** | `2.1.4` |
| **Uygulama Sürümü (`tauri.conf.json`)** | `2.1.4` |
| **Sürüm Tutarlılığı Status** | ✅ **Tam Tutarlı** (Tüm manifests 2.1.4) |
| **Önerilen Baseline Referans Adı** | `baseline-2.1.4-2026-07-29` |
| **Tag Oluşturuldu mu?** | ❌ **Hayır** (`v*` release workflow tetiklenmesini engellemek için tag atılmadı) |
| **Rust Edition** | `2021` |
| **Tauri Sürüm Ailesi** | Tauri v2 (`@tauri-apps/api ^2`, `tauri 2.10.3`) |
| **React Sürüm Ailesi** | React v18 (`^18`) |
| **TypeScript Sürüm Ailesi** | TypeScript v5 (`^5`) |
| **CI Node.js Sürümü** | `22.20.0` (GitHub Actions `ci.yml` / `releases.yml`) |
| **CI Rust Toolchain Sürümü** | `1.93.0` (dtolnay/rust-toolchain) |

---

## 2. Standart Build ve Doğrulama Komutları

Repository üzerindeki doğrulama ve build işlemleri aşağıdaki komut dizilimi ile yürütülmektedir:

### Frontend Bağımlılık ve Build Komutları
```bash
# Bağımlılıkların kilitli sürümlerle yüklenmesi
npm ci

# Frontend birim testlerinin çalıştırılması (Vitest)
npm test

# Production frontend bundle ve TypeScript tip kontrolü
npm run build
```

### Rust Backend Test ve Analiz Komutları
```bash
# Rust birim ve property testleri
cd src-tauri
cargo test --lib

# Strict linter kontrolü (Warnings = Errors)
cargo clippy --lib -- -D warnings
```

### Güvenlik ve Bağımlılık Denetim Komutları
```bash
# Frontend yüksek seviyeli zafiyet denetimi
npm audit --audit-level=high

# Rust bağımlılık zafiyet denetimi
cargo audit --file src-tauri/Cargo.lock
```

---

## 3. Artifact Envanteri ve Bileşenler

| Artifact / Bileşen | Platform | Dosya Yolu / Konum | Açıklama |
| :--- | :--- | :--- | :--- |
| **Windows Executable (Bundled)** | Windows | `src-tauri/binaries/winws-x86_64-pc-windows-msvc.exe` | Zapret Windows paketi desync binary'si |
| **WinDivert Driver (64-bit Sys)** | Windows | `src-tauri/binaries/WinDivert64.sys` | WinDivert 64-bit kernel sürücüsü |
| **WinDivert DLL** | Windows | `src-tauri/binaries/WinDivert.dll` | WinDivert kullanıcı modu kütüphanesi |
| **Cygwin DLL** | Windows | `src-tauri/binaries/cygwin1.dll` | Winws runtime bağımlılık DLL'i |
| **Linux Binary (Bundled)** | Linux | `src-tauri/binaries/nfqws-x86_64-unknown-linux-gnu` | Zapret Linux NFQUEUE desync binary'si |
| **Built-in Preset JSON** | Cross | `presets/builtin.json` | Uygulama ile gelen varsayılan preset listesi |
| **Remote Preset Template** | Cross | `presets/remote_template.json` | Uzaktan yüklenebilir preset şablon yapısı |
| **Updater Public Key** | Cross | `src-tauri/tauri.conf.json#L67` | Minisign updater açık anahtarı |
| **Security Public Key** | Cross | `SECURITY.md#L49` | Güvenlik bildirimi Minisign açık anahtarı |

---

## 4. SHA-256 Hash Manifestosu

Aşağıdaki hash değerleri 2026-07-29 tarihinde `collect-baseline.node.js` scripti ile hesaplanmış ve doğrulanmıştır:

| Dosya Yolu | SHA-256 Checksum |
| :--- | :--- |
| `src-tauri/binaries/winws-x86_64-pc-windows-msvc.exe` | `2da71e80878dc270ac83f5893ecbb841f9752a57f1da8ff9325636b4346bc632` |
| `src-tauri/binaries/WinDivert64.sys` | `8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2` |
| `src-tauri/binaries/WinDivert.dll` | `c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2` |
| `src-tauri/binaries/cygwin1.dll` | `103104a52e5293ce418944725df19e2bf81ad9269b9a120d71d39028e821499b` |
| `src-tauri/binaries/nfqws-x86_64-unknown-linux-gnu` | `8d3452ce0e0b9d9fed2a3a087b1caecfd39a910b7a31b304078fcbed3ea0e33c` |
| `presets/builtin.json` | `f897e2b443b710ff46dcc27e58f653adfb255b9fce10581bd0431ee7fb82a853` |
| `presets/remote_template.json` | `0f93fe72f68416a7cf0d2d4a0a7eb3dd5d2eb75e46233b42bdbc3bf6202b9187` |
| `src-tauri/tauri.conf.json` | `4d662833718663127da23f4df2521086cd05129d1f5a0d785d1004e393c9fd19` |
| `package.json` | `54cf95f101ee29864265b996fda367b937685d8ecbff262f619a3f851dc3d611` |
| `src-tauri/Cargo.toml` | `36fba27d70b962992e1acbece2a7f4806bfcaf202d9803480ba538dd43d7ab64` |
| `package-lock.json` | `33f1f50e8ca63559939b0372468b64dec708c74ede122a2a5153f8b4039ac021` |
| `src-tauri/Cargo.lock` | `d7e0d1008eb89e386341ce59032d7afc4930e7bf7d1b8b06f3465b58b99925ae` |

---

## 5. Doğrulama Anahtarları (Cryptographic Keys)

### Tauri Updater Minisign Public Key (`tauri.conf.json`)
```text
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDdDNUIyQTk1MkIzRTNDOEUKUldTT1BENHJsU3BiZkdlcUJFK0pOejRXUGJJNUUxQXUwZkZtaVp3aGQxU1FSa3JRVDRSdlB0d1YK
```

### Security Minisign Public Key (`SECURITY.md`)
```text
untrusted comment: minisign public key: 2A7CBD213C2CD2E8
RWTo0iw8Ib18KoSGwlXjG4Hlz+oMjaFhN6077H5nNlTH6KuJogHeUra1
```
