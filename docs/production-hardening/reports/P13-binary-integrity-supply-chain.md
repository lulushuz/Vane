# P13 — Binary Integrity, Artifact Trust, Supply-Chain Security ve Güvenli Çalıştırma Kapısı

## Executive Summary

P13 aşamasında Vane DPI uygulamasının çalıştırdığı ve bağımlı olduğu tüm native ikili dosyalar (`winws`, `nfqws`, `WinDivert64.sys`, `WinDivert.dll`, `cygwin1.dll`) ve içerik dosyaları (`builtin.json`, `remote_template.json`) için uçtan uca **Binary Integrity & Artifact Trust** mimarisi kurulmuştur.

Hiçbir native ikili veya dinamik kütüphane gömülü güvenilir manifest doğrulaması yapılmadan çalıştırılamaz. Bütünlük doğrulaması (size, SHA-256, path containment, symlink/reparse-point engelleme, TOCTOU kontrolü) başarısız olduğunda sistem **fail-closed** olarak motoru ve optimizer'ı başlatmayı reddeder. PATH veya çalışma dizininden fallback arama yolları tamamen engellenmiştir.

---

## 1. P12 Kapanış ve Ön Kontrol (P12 Preflight)

| P12 Alanı | Production Integration | Test | Sonuç |
| :--- | :---: | :---: | :--- |
| **Unified candidate lifecycle** | Integration | `optimizer_session_tests.rs` | VERIFIED |
| **No direct engine spawn** | Scan (`git grep`) | 0 occurrences found | VERIFIED |
| **Original stopped restore** | Integration | `group_h01_restore_guarantee` | VERIFIED |
| **Original running restore** | Integration | `group_h01_restore_guarantee` | VERIFIED |
| **Cancel restore** | Integration | `cancel_optimizer` IPC | VERIFIED |
| **Failure restore** | Integration | `FakeRuntime` candidate failure | VERIFIED |
| **No persistence** | Storage isolation | `optimizer.test.ts` | VERIFIED |
| **No automatic winner apply** | Facade decoupling | `start_auto_optimize` | VERIFIED |
| **Hostname targets** | DNS measurement | `default_measurement_targets` | VERIFIED |
| **Full Rust test suite** | Cargo test / check | 229 passed (P12) | VERIFIED |

---

## 2. P13 Temel Başarımları ve Güvenlik Sözleşmesi

### 2.1 Embedded Trusted Manifest Anchor
- `src-tauri/security/native-artifacts.json` ve `src-tauri/security/content-artifacts.json` manifestleri oluşturulmuştur.
- Manifestler runtime'da değiştirilebilir disk JSON'ı olarak okunmak yerine `include_str!` makrosu ile güvenilir Vane ikilisinin içine gömülmüştür (`EMBEDDED_NATIVE_MANIFEST` ve `EMBEDDED_CONTENT_MANIFEST`).

### 2.2 Strict Path Containment & Reparse Point Protection
- `src-tauri/src/security/artifact_path.rs`:
  - Göreli yollarda parent traversal (`..`) ve mutlak yol (`absolute`) kullanımı reddedilir.
  - `symlink_metadata` ile sembolik linkler reddedilir.
  - Windows'ta `FILE_ATTRIBUTE_REPARSE_POINT` (0x400) ile NTFS reparse point ve junction yönlendirmeleri engellenir.
  - Canonicalization sonrası dosyanın `resource_root` içinde bulunduğu doğrulanır.
  - Dosyanın normal dosya (`is_file()`) olduğu garanti edilir.

### 2.3 TOCTOU-Safe Streaming SHA-256 Verification
- `src-tauri/src/security/artifact_integrity.rs`:
  - `Sha256ArtifactIntegrityVerifier` akış tabanlı (streaming) 64 KB arabellek ile bellek dostu SHA-256 hesaplar.
  - Dosya açıldıktan hemen sonra alınan `metadata` (boyut, mtime) ile okuma sonrası alınan `metadata` karşılaştırılarak TOCTOU (Time-of-Check to Time-of-Use) tahrifatı engellenir.
  - Linux'ta `mode` kontrolleri ile executable bit zorunluluğu, world-writable ve setuid/setgid engellemesi uygulanır.

### 2.4 Engine & Optimizer Unified Verification Gate
- `EngineManager::resolve_binary_path` doğrudan ad-hoc hash kontrolü yerine `Sha256ArtifactIntegrityVerifier::verify_current_platform_group` kullanacak şekilde güncellenmiştir.
- `winws` / `nfqws` başlatılmadan önce sürücü (`WinDivert64.sys`), DLL (`WinDivert.dll`) ve bağımlılıklar (`cygwin1.dll`) atomik bir grup (`VerifiedArtifactGroup`) olarak doğrulanır.
- `ProductionOptimizerRuntime` ve `EngineManager` aynı verifier'ı kullanır. Doğrulama hatasında **fail-closed** uygulanır (hata: `ARTIFACT_INTEGRITY_FAILED`).

### 2.5 Security Status IPC & Diagnostics
- Yeni IPC komutu: `get_artifact_integrity_status` (`ArtifactIntegrityStatusDto`).
- Arayüze hassas kullanıcı yolları sızdırılmadan doğrulama durumu (`verified`, `missing`, `modified`, `invalid_manifest`) sunulur.

### 2.6 Maintenance Script & Build-Time Test Gate
- `scripts/audit/update-artifact-manifest.js` bakım betiği oluşturulmuştur (varsayılan diff mode, `--write` ile deterministik güncelleme).
- Build-time test: `bundled_native_artifacts_match_the_trusted_manifest()` testi ile repository'deki ikili dosyaların manifest ile %100 uyuştuğu derleme seviyesinde garanti edilir.

---

## 3. Test ve Audit Sonuçları

- **Frontend Testleri (Vitest):**
  - `src/test/artifactIntegrity.test.ts` eklendi (FE-01 ile FE-05 arası karakterizasyonlar).
  - 17 test dosyasında 147 test geçti (0 failed).
  - `npm run build` ve `npm audit --audit-level=high` 0 zafiyet ile tamamlandı.

- **Rust Testleri:**
  - `src-tauri/src/characterization/binary_integrity_tests.rs` eklendi (Group A, B, C, D, E).
  - `cargo check --tests` ve `cargo clippy --lib -- -D warnings` 0 hata ve 0 uyarı ile geçti.
  - Toplam geçen Rust test sayısı: **237 passed** (P12: 229 passed).

---

## 4. Dosya Değişiklik Özeti

1. `src-tauri/security/native-artifacts.json` — [NEW] Native artifact manifesti.
2. `src-tauri/security/content-artifacts.json` — [NEW] Content artifact manifesti.
3. `src-tauri/src/security/artifact_manifest.rs` — [NEW] Gömülü manifest modelleri ve şema doğrulayıcı.
4. `src-tauri/src/security/artifact_path.rs` — [NEW] Yol güvenliği, symlink ve reparse point kontrolü.
5. `src-tauri/src/security/artifact_integrity.rs` — [NEW] Sha256ArtifactIntegrityVerifier & TOCTOU kontrolü.
6. `src-tauri/src/security/supply_chain.rs` — [NEW] Content artifact doğrulaması & Public key taxonomy.
7. `src-tauri/src/security/mod.rs` — [NEW] Security modülü re-export yapısı.
8. `src-tauri/src/engine/error.rs` — `ArtifactIntegrityError` varyantı eklendi (`ARTIFACT_INTEGRITY_FAILED`).
9. `src-tauri/src/engine/manager.rs` — `resolve_binary_path` entegrasyonu güncellendi.
10. `src-tauri/src/commands.rs` — `get_artifact_integrity_status` IPC komutu eklendi.
11. `src-tauri/src/lib.rs` — Security modülü ve IPC işleyicisi kaydedildi.
12. `src-tauri/src/characterization/binary_integrity_tests.rs` — [NEW] Rust karakterizasyon testleri.
13. `src/test/artifactIntegrity.test.ts` — [NEW] Vitest test süiti.
14. `scripts/audit/update-artifact-manifest.js` — [NEW] Manifest bakım betiği.
15. `THIRD_PARTY_NOTICES.md` — [NEW] Lisans ve üçüncü taraf bildirimleri.
16. `docs/security/native-artifact-provenance.md` — [NEW] Provenance belgesi.
