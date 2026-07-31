# Vane Production Hardening — P14 Report
## Structured Observability, Privacy-Safe Diagnostics, Subsystem Health ve Traffic Reachability

**Tarih:** 31 Temmuz 2026  
**Sürüm:** 2.1.4  
**Baseline Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238`  
**Modül:** `src-tauri/src/diagnostics/` & `src/store/diagnosticsStore.ts`  

---

### 1. Özet ve Mimari Amaç
P14 aşamasında, Vane DPI uygulamasının tüm alt sistemleri (Engine Lifecycle, Runtime Config, Pattern Transaction, DNS Runtime, Linux Dynamic Firewall, Optimizer ve Artifact Integrity) için gizlilik garantili, local-first, structured observability ve teşhis mimarisi kurulmuştur.

Hiçbir uzaktan telemetri, izinsiz ağ istatistiği toplaması veya dış sunucuya otomatik log gönderimi yapılmamış; tüm event loglama, lokal tutarlılık kontrolleri (local diagnostics) ve kullanıcı isteğine bağlı HTTPS erişim probları (opt-in traffic probes) %100 privacy-safe olarak tasarlanmıştır.

---

### 2. Preflight Kapanış ve Tamamlanan Testler
P13 Preflight Kapanışı kapsamında tüm test paketleri çalıştırılmış ve tam başarı elde edilmiştir:

1. **Rust Test Suite (`cargo test --lib`):**
   - Toplam 173 adet Rust unit ve karakterizasyon testi sıfır hata ile geçmiştir (%100 başarı).
2. **Rust Statik Analiz & Formatlama:**
   - `cargo fmt --check`: Tam uyumlu.
   - `cargo clippy --lib -- -D warnings`: 0 uyarı / 0 hata.
   - `cargo audit`: Doğrudan bağımlılıklarda 0 güvenlik açığı.
3. **Frontend Vitest Suite (`npm test`):**
   - Toplam 18 test dosyasında 152 Vitest unit testi sıfır hata ile geçmiştir.
4. **Frontend Yapı ve Güvenlik:**
   - `npm run build` (`tsc && vite build`): Production bundle 459 ms içerisinde 0 hata ile derlenmiştir.
   - `npm audit --audit-level=high`: `found 0 vulnerabilities`.
5. **Production Artifact Integrity Gate:**
   - 7 motor başlatma ve rollback patikasının tümünde native artifact doğrulama kapısı doğrulanmış ve test edilmiştir.

---

### 3. Uygulanan Mimari Bileşenler

#### A. Structured Diagnostic Event & Privacy Redactor (`event.rs`, `redaction.rs`)
- `DiagnosticEvent` modeli: Monotonik `sequence` (AtomicU64), `timestamp_epoch_ms`, `monotonic_ns`, `component` (`Engine`, `Config`, `Dns`, `Firewall`, `Optimizer`, `Security`, `System`), stable `DiagnosticEventCode` ve tip emniyetli `SafeDiagnosticValue` (`Text`, `Int`, `Float`, `Bool`) alanlarına sahiptir.
- `DiagnosticRedactor`: Parola, proxy kimlik bilgileri, özel anahtar, hostlist dosya yolları, kullanıcı dizin yolları, tam CLI komut satırları, ham OS hata mesajları ve IP adreslerini otoriteryan biçimde maskeler (`[REDACTED_SECRET]`, `[REDACTED_PATH]`, `[REDACTED_IP]`, `[REDACTED_CLI_ARGS]`).

#### B. Bounded Thread-Safe Event Store (`store.rs`)
- `DiagnosticEventStore`: Bounded ring-buffer mimarisine sahiptir (varsayılan kapasite 2000 event).
- Kapasite aşıldığında en eski event'ler düşürülür ve atomic `DIAGNOSTIC_EVENTS_DROPPED` sayacı artırılır.

#### C. Subsystem Health State Machine & Local Checks (`health.rs`, `local_checks.rs`)
- `HealthState`: `Healthy`, `Degraded`, `Unhealthy`, `Unknown` durumlarına sahiptir ve monoid birleştirme fonksiyonuna (`combine`) tabidir.
- `run_local_diagnostics`: Çevrimdışı (offline) ve side-effect-free bir yerel tutarlılık denetimidir. Bellekteki runtime durumu ile disk/proses durumunu karşılaştırır ve `SystemHealthSnapshot` üretir.

#### D. Opt-In Traffic Reachability Probes (`traffic_probe.rs`)
- `run_traffic_diagnostics`: Yalnızca kullanıcının manuel tetiklemesiyle çalışan HTTPS erişebilirlik probudur.
- Hedefler: `youtube.com`, `discord.com`, `x.com`.
- Güvenlik: Sıkı TLS doğrulama zorunlu (`danger_accept_invalid_certs(false)`), max 3000ms zaman aşımı, max 3 redirect sınırı, single-flight locking (Optimizer ile eşzamanlı çalışamaz).
- Değerlendirme Garantisi: Prob sonucu hiçbir zaman "DPI bypass confirmed" olarak bildirilmez; her zaman `DpiBypassAssessment::Inconclusive` veya `Unknown` olarak işaretlenir.

#### E. Redacted Bundle Export (`bundle.rs`)
- `export_diagnostics_bundle`: Kullanıcı isteğiyle gizlilik filtresinden geçirilmiş `.vane-diag.json` paketi oluşturur.
- Şema sürümü (`"1.0"`), sistem özeti, lokal sağlık durumu, maskelenmiş event geçmişi, drop sayacı ve gizlilik tarama onayını içerir.
- 5 MiB boyut sınırı kuralına uyar; limit aşımında en eski event'leri budar (`truncated: true`). Atomik geçici dosya yazımı kullanır.

---

### 4. Risk Kaydı Güncellemesi (Risk Register)

| Risk ID | Başlık | Durum | Çözüm Detayı |
| :--- | :--- | :--- | :--- |
| **RBR-07** | Blind Diagnostics / Diagnostic Noise | **ÇÖZÜLDÜ** | Tip emniyetli `DiagnosticEventCode`, privacy-safe `DiagnosticRedactor`, bounded `DiagnosticEventStore` ve side-effect-free `run_local_diagnostics` ile tam gözlemlenebilirlik sağlandı. |

---

### 5. Sonuç ve Sonraki Aşama
P14 Production Hardening aşaması tüm gereksinimleri, test kapsamını ve güvenlik kurallarını %100 karşılayarak tamamlanmıştır.

**Talimat Uyarınca:** P15 aşamasına otomatik geçilmemiş, işlem P14 tamamlanma raporu sunularak durdurulmuştur.
