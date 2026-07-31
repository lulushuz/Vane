# Vane Production Hardening — P05 Raporu

## Low-Risk Deterministic Fixes: Preset Import/Export, DoQ Temizliği ve Capability Dokümantasyonu

**Tarih:** 2026-07-29  
**Sürüm:** `2.1.4`  
**Baseline Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238`  
**Aşama:** P05 (Low-Risk Deterministic Fixes)

---

## 1. Yönetici Özeti

P05 aşamasında, alt seviye motor çalışma mekanizmalarına dokunulmaksızın, deterministik, yüksek güvenlikli ve düşük riskli 3 grup düzeltme ve dokümantasyon hizalaması başarıyla tamamlanmıştır:

1. **P05-A (Preset Import/Export Uzantı Hizalaması):**
   - Frontend `save` diyalog filtreleri varsayılan olarak `.vane` (Kanonik format) yapıldı.
   - Dışa aktarma patikalarında `.vane` uzantısı otomatik olarak tamamlanır ve büyük/küçük harf duyarsız (`.vane` / `.VANE`) kabul edilir.
   - İçe aktarma (`AdvancedView` ve `CustomPresetView`) mekanizmaları hem kanonik `.vane` hem de eski (legacy) `.json` formatlarını sorunsuz okur.
   - `BR-02` reproducer testi başarıyla güncellendi (`BR-02 resolved`).

2. **P05-B (Güvenli DoQ Temizliği ve Hydration Migrasyonu):**
   - Desteklenmeyen `doq` (DNS-over-QUIC) seçeneği frontend `DnsProtocol` tipinden (`'doh' | 'dot'`) ve `DnsView` seçim listesinden kaldırıldı.
   - `migratePersistedEngineState` fonksiyonuna idempotent state migrasyonu eklendi: Önceden diske `dnsProtocol: 'doq'` olarak yazılmış kullanıcı ayarları, tüm diğer DNS ayarları (sağlayıcı, adblock, cache, proxy, killswitch vb.) korunarak otomatik ve kayıpsız bir şekilde `'doh'` değerine dönüştürülür.
   - Frontend IPC çağırma noktalarındaki (`engineStore.ts`, `SafetyProxyView.tsx`) sessiz `doq` -> `doh` coercion mantıkları kaldırıldı.
   - Arka plan IPC / doğrulama katmanında doğrudan ham `doq` gönderimi durumunda sessizce DoH'a çevrilmek yerine açık `UnsupportedDnsProtocol("doq")` hatası verilmesi sağlandı.
   - `BR-03` reproducer testi başarıyla güncellendi (`BR-03 resolved`).

3. **P05-C (Dokümantasyon ve Capability Hizalaması):**
   - `README.md` ve `README.tr.md` dosyaları taranarak gerçek kod yetenekleriyle hizalandı.
   - DoQ iddiası kaldırıldı ve `❌ Not supported (Use DoH or DoT)` olarak işaretlendi.
   - Yanıltıcı WFP (Windows Filtering Platform) callout driver iddiası kaldırıldı, Windows Firewall (`netsh`) kuralları & WinDivert sürücüsü olarak güncellendi.
   - Linux desteği `⚠️ Experimental` (Deneysel) olarak açıkça belirtildi.
   - `src/test/documentation.test.ts` eklenerek dokümantasyon iddiaları otomatize vitest suite'e bağlandı.
   - Risk kaydındaki `R-02` ve `R-14` riskleri çözüldü (`Mitigated in P05 / Resolved`).

---

## 2. Test ve Doğrulama Sonuçları

### Frontend Vitest Suite
```text
RUN  v4.1.10 C:/Users/Lulushu/Desktop/WinDPI

 ✓ src/test/documentation.test.ts (4 tests)
 ✓ src/utils/presetValidator.test.ts (7 tests)
 ✓ src/utils/argsParser.test.ts (6 tests)
 ✓ src/types/ipc.test.ts (2 tests)
 ✓ src/store/persistence.test.ts (7 tests)
 ✓ src/test/advancedConfig.test.ts (29 tests)
 ✓ src/test/patternDnsSync.test.ts (15 tests)
 ✓ src/test/storePersistence.test.ts (13 tests)
 ✓ src/test/engineLifecycle.test.ts (19 tests)
 ✓ src/test/bugReproducers.test.ts (8 tests)
 ✓ src/test/presetImportExport.test.ts (13 tests)
 ✓ src/store/revisionGate.test.ts (2 tests)

 Test Files  12 passed (12)
      Tests  125 passed (125)
```

### Frontend Build
- `npm run build`: **PASSED** (0 TypeScript / Vite uyarısı/hatası)

### Rust Backend Test Suite & Quality Gates
- `cargo fmt --check`: **PASSED**
- `cargo test --lib`: **179 passed, 0 failed, 0 ignored**
- `cargo clippy --lib -- -D warnings`: **PASSED (0 warning)**

---

## 3. Çözülen Riskler ve Değiştirilen Dosyalar

| Risk ID | Başlık | Çözüm |
| :--- | :--- | :--- |
| **R-02** | Preset format uyuşmazlığı | Kanonik `.vane` uzantısı zorunlu kılındı, `.json` import uyumluluğu korundu |
| **R-14** | DoQ sessiz dönüşüm riski | DoQ UI'dan kaldırıldı, disk migrasyonu eklendi, ham IPC'de açık hata dönüldü |

### Oluşturulan / Değiştirilen Dosyalar
- `src-tauri/src/commands.rs` — `export_preset` büyük/küçük harf duyarsız `.vane` kontrolü.
- `src/views/AdvancedView.tsx` — `.vane` export varsayılan filtresi, hata yakalama ve `.vane,.json` import kabulü.
- `src/views/CustomPresetView.tsx` — `.vane,.json` import kabulü.
- `src/views/DnsView.tsx` — `doq` seçeneğinin UI dropdown'ından kaldırılması.
- `src/views/SafetyProxyView.tsx` — `doq` coercion branch'inin temizlenmesi.
- `src/store/engineStore.ts` — `DnsProtocol` tipinin `'doh' | 'dot'` yapılması ve coercion silinmesi.
- `src/store/persistence.ts` — Hydration aşamasında `doq` -> `doh` otomatik migrasyonu.
- `src/test/bugReproducers.test.ts` — BR-02 ve BR-03 testlerinin çözülmüş davranışa güncellenmesi.
- `src/test/presetImportExport.test.ts` — `.vane` kanonik export testlerinin güncellenmesi.
- `src/test/engineLifecycle.test.ts` & `src/test/patternDnsSync.test.ts` — DoQ temizliği sonrası test hizalaması.
- `src/test/documentation.test.ts` — Dokümantasyon yetenek iddialarının otomatik vitest suite ile doğrulanması.
- `README.md` & `README.tr.md` — DoQ, WFP, Linux status ve preset format güncellemeleri.
- `docs/production-hardening/02-risk-register.md` — R-02 ve R-14 güncellemeleri.
- `docs/production-hardening/03-test-matrix.md` — P05 test matrisinin eklenmesi.

---

## 4. Sonraki Aşama
P05 başarıyla tamamlanmıştır. P06 aşamasına geçmeye hazırdır.
