# P02 Risk Register (Risk Kütüğü)

Bu doküman, Vane kod tabanında tespit edilen tüm bilinen mimari, güvenlik, yarış durumu (race condition), platform ve runtime risklerini kayıt altına alır.

---

## 1. Risk Değerlendirme Dereceleri

- **Critical:** Sistem çökmesine, yetkisiz kod çalıştırılmasına, veri kaybına veya ağ bağlantısının tamamen kilitlenip manuel müdahalesiz düzelmemesine yol açan riskler.
- **High:** Güvenlik açığına, kısıtlamaların (fail-closed) ihlaline, arka planda yetkisiz sızan child process'lere veya platform uyumsuzluklarına yol açan riskler.
- **Medium:** Kullanıcı arayüzü desenkronizasyonuna, yanlış durum gösterimine, optimizasyon hatalarına veya eksik doğrulamalara yol açan riskler.
- **Low:** UI gösterim veya loglama tutarsızlıkları, minor performans kayıpları.
- **Informational:** İyileştirme önerileri ve dokümantasyon hizalama farkları.

---

## 2. Kayıtlı Riskler Listesi

### R-01: Pattern Cache ile Disk Source-of-Truth Yarışı (Race Condition)
- **Risk ID:** R-01
- **Başlık:** Pattern bellek önbelleği ile disk ayarları arasındaki senkronizasyon yarışı
- **Kategori:** Data Integrity / Concurrency
- **Önem Derecesi:** Critical
- **Olasılık:** High
- **Etki:** High
- **Mevcut Kanıt:** [engineStore.ts:L110](src/store/engineStore.ts#L110), [settings.rs:L100](src-tauri/src/settings.rs#L100)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Pattern / Settings Persistence
- **Olası Kullanıcı Etkisi:** Kullanıcı site ekleyip sildiğinde UI güncellenir ancak disk yazımı veya Rust runtime bellek önbelleği eşzamanlı güncellenmezse engelleme listesi eski durumuna dönebilir veya bypass çalışmayabilir.
- **Önerilen Çözüm Aşaması:** P06 (Pattern Transaction)
- **Bağımlı Olduğu İşler:** P04 (Runtime Configuration Contract)
- **Regression Riski:** Medium
- **Doğrulama Yöntemi:** Concurrent add/delete domain integration testleri ve disk atomic write assertion.
- **Durum:** Resolved in P06 (RuntimeConfigState bellek snapshot'ı tek source-of-truth yapıldı; restart işlemlerinde disk/cache okumaları ortadan kaldırıldı)

---

### R-02: `.json` ve `.vane` Export/Import Format Uyuşmazlığı
- **Risk ID:** R-02
- **Başlık:** Preset dışa ve içe aktarma formatlarındaki tip ve imza doğrulama ayrışması
- **Kategori:** Data Validation / Security
- **Önem Derecesi:** High
- **Olasılık:** Medium
- **Etki:** Medium
- **Mevcut Kanıt:** [CustomPresetView.tsx:L40](src/views/CustomPresetView.tsx#L40), [presetValidator.ts:L20](src/utils/presetValidator.ts#L20)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Preset Import/Export
- **Olası Kullanıcı Etkisi:** Kullanıcının dışa aktardığı `.vane` uzantılı imzalı dosyalar raw `.json` import mekanizması tarafından reddedilebilir veya imzasız `.json` yüklenirken güvenlik uyarısı atlanabilir.
- **Önerilen Çözüm Aşaması:** P08 (Preset Validation Pipeline)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Preset import/export roundtrip unit testleri.
- **Durum:** Mitigated in P05 / Resolved (.vane canonical extension enforced, legacy .json import supported)


---

### R-03: Linux'ta `--wf-*` WinDivert Argümanlarının Sessizce Atılması
- **Risk ID:** R-03
- **Başlık:** Windows WinDivert parametrelerinin Linux `nfqws` üzerinde çalışmaması ve argüman kırpılması
- **Kategori:** Platform Compatibility / Runtime Failure
- **Önem Derecesi:** High
- **Olasılık:** High
- **Etki:** High
- **Mevcut Kanıt:** [sanitizer.rs:L10](src-tauri/src/engine/sanitizer.rs#L10), [manager.rs:L850](src-tauri/src/engine/manager.rs#L850)
- **Etkilenen Platform:** Linux
- **Etkilenen Bileşen:** Engine Manager / Launcher
- **Olası Kullanıcı Etkisi:** Linux kullanıcıları varsayılan presetleri çalıştırdığında WinDivert filtre argümanları (`--wf-tcp`) Linux `nfqws` başlatıcı tarafından reddedilir veya sessizce yutulur, desync çalışmaz.
- **Önerilen Çözüm Aşaması:** P11 (Linux Platform Layer Isolation)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Linux platform argument translation unit testleri.
- **Durum:** Identified

---

### R-04: Linux Firewall Kurallarının Yalnızca TCP 80/443 Yakalaması
- **Risk ID:** R-04
- **Başlık:** Linux nftables / iptables yönlendirmesinin UDP (QUIC) ve özel portları hariç tutması
- **Kategori:** Network / Filtering Incompleteness
- **Önem Derecesi:** High
- **Olasılık:** High
- **Etki:** High
- **Mevcut Kanıt:** [router.rs:L30](src-tauri/src/network/router.rs#L30)
- **Etkilenen Platform:** Linux
- **Etkilenen Bileşen:** Network Router / NFQUEUE
- **Olası Kullanıcı Etkisi:** HTTP/3 (QUIC) trafiği veya özel portlardaki HTTP/HTTPS istekleri Linux ortamında NFQUEUE kuyruğuna alınmaz ve desync edilmeden sansür kutularına takılır.
- **Önerilen Çözüm Aşaması:** P11 (Linux Platform Layer Isolation)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Medium
- **Doğrulama Yöntemi:** Linux nftables rule assertion testleri.
- **Durum:** Identified

---

### R-05: Linux Ortamında Global `killall nfqws` Kullanımı
- **Risk ID:** R-05
- **Başlık:** Engine durdurulurken Linux'ta sistem genelindeki tüm `nfqws` süreçlerinin sonlandırılması
- **Kategori:** Process Safety / Side Effects
- **Önem Derecesi:** High
- **Olasılık:** Low
- **Etki:** High
- **Mevcut Kanıt:** [process.rs:L40](src-tauri/src/engine/process.rs#L40)
- **Etkilenen Platform:** Linux
- **Etkilenen Bileşen:** Engine Process Lifecycle
- **Olası Kullanıcı Etkisi:** Vane kapatıldığında sistemde çalışan Vane harici başka `nfqws` süreçleri varsa onlar da zorla kapatılır.
- **Önerilen Çözüm Aşaması:** P07 (Process Lifecycle & Ownership)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Child Process PID tracking testleri.
- **Durum:** Resolved in P07 (Global killall kaldırıldı; yalnız Vane child process PID'si sonlandırılıyor)

---

### R-06: Windows Global Process ve Driver Cleanup Yan Etkileri
- **Risk ID:** R-06
- **Başlık:** Windows'ta `taskkill /IM winws.exe` veya sürücü durdurmanın diğer uygulamaları etkilemesi
- **Kategori:** Process Safety / Side Effects
- **Önem Derecesi:** High
- **Olasılık:** Medium
- **Etki:** High
- **Mevcut Kanıt:** [manager.rs:L390](src-tauri/src/engine/manager.rs#L390)
- **Etkilenen Platform:** Windows
- **Etkilenen Bileşen:** Engine Manager / WinDivert
- **Olası Kullanıcı Etkisi:** Vane kapatıldığında bağımsız çalışan başka bir Zapret/WinDivert uygulaması sonlandırılabilir.
- **Önerilen Çözüm Aşaması:** P07 (Process Lifecycle & Ownership)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Specific PID process termination assertion.
- **Durum:** Resolved in P07 (Global taskkill /IM kaldırıldı; Job Object ve owned child process handle ile yalnız Vane süreci yönetiliyor)

---

### R-07: Optimizer'ın EngineManager Dışından Doğrudan Süreç Başlatması
- **Risk ID:** R-07
- **Başlık:** Optimizer modülünün child process'leri `EngineManager` dışından ham `std::process::Command` ile açması
- **Kategori:** Resource Leak / Process Ownership
- **Önem Derecesi:** High
- **Olasılık:** High
- **Etki:** High
- **Mevcut Kanıt:** [optimizer.rs:L80](src-tauri/src/engine/optimizer.rs#L80)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Optimizer
- **Olası Kullanıcı Etkisi:** Optimizer testi iptal edildiğinde veya çöktüğünde arkada yetim (zombie/orphan) `winws.exe` süreçleri kalabilir ve ağ portlarını/WinDivert sürücüsünü meşgul tutabilir.
- **Önerilen Çözüm Aşaması:** P12 (Optimizer Safety & Isolation)
- **Bağımlı Olduğu İşler:** P07 (Process Lifecycle & Ownership)
- **Regression Riski:** Medium
- **Doğrulama Yöntemi:** Optimizer cancellation orphan process check test.
- **Durum:** Identified

---

### R-08: Optimizer'ın Sabit CDN IP Adresleri Kullanması
- **Risk ID:** R-08
- **Başlık:** Latans ve bağlantı testinde dinamik çözümleme yerine sabit IP adreslerine bağımlılık
- **Kategori:** Reliability / Network Testing
- **Önem Derecesi:** Medium
- **Olasılık:** Medium
- **Etki:** Medium
- **Mevcut Kanıt:** [optimizer.rs:L140](src-tauri/src/engine/optimizer.rs#L140)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Optimizer
- **Olası Kullanıcı Etkisi:** ISP tarafından sabit CDN IP'leri engellendiğinde veya rotası değiştiğinde optimizer tüm presetleri başarısız puanlayabilir.
- **Önerilen Çözüm Aşaması:** P12 (Optimizer Safety & Isolation)
- **Bağımlı Olduğu İşler:** P10 (DNS Hardening)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Dynamic DNS target resolution unit test.
- **Durum:** Identified

---

### R-09: Built-in Presetlerin Aynı Validation Pipeline'dan Geçmemesi
- **Risk ID:** R-09
- **Başlık:** Dahili presetlerin `sanitizer` ve `presetValidator` süzgecinden geçirilmeden doğrudan çalıştırılması
- **Kategori:** Security / Data Integrity
- **Önem Derecesi:** High
- **Olasılık:** Low
- **Etki:** High
- **Mevcut Kanıt:** [loader.rs:L30](src-tauri/src/config/loader.rs#L30), [builtin.json](presets/builtin.json)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Preset Loader
- **Olası Kullanıcı Etkisi:** Dahili preset dosyasında hatalı veya desteklenmeyen bir parametre unutulduğunda runtime seviyesinde sessiz çökme oluşabilir.
- **Önerilen Çözüm Aşaması:** P08 (Preset Validation Pipeline)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Unified corpus validation unit test.
- **Durum:** Identified

---

### R-10: Preset Phase Order Doğrulamasının Bulunmaması
- **Risk ID:** R-10
- **Başlık:** Zapret argümanlarının çalışma sırasına (phase order) uygunluğunun kontrol edilmemesi
- **Kategori:** Logic / Validation Missing
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Medium
- **Mevcut Kanıt:** [sanitizer.rs:L43](src-tauri/src/engine/sanitizer.rs#L43), [presetValidator.ts:L30](src/utils/presetValidator.ts#L30)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Preset Validation
- **Olası Kullanıcı Etkisi:** Yanlış sırayla verilen argümanlar (örneğin split öncesi fooling verilmesi) `winws` tarafından paketlerin işlenmeden pas geçilmesine neden olur.
- **Önerilen Çözüm Aşaması:** P08 (Preset Validation Pipeline)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Medium
- **Doğrulama Yöntemi:** Semantic phase-order parser unit test.
- **Durum:** Resolved in P08 (Birleşik validate_preset pipeline'ı ile semantik desync faz sırası zorunlu kılındı)

---

### R-11: `https-sni-ghost` Phase Order Sıralama Problemi
- **Risk ID:** R-11
- **Başlık:** `builtin.json` içindeki `https-sni-ghost` modunda argüman sırasından kaynaklı uyumsuzluk
- **Kategori:** Configuration / Preset Integrity
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Medium
- **Mevcut Kanıt:** [builtin.json#L42](presets/builtin.json#L42)
- **Etkilenen Platform:** Windows
- **Etkilenen Bileşen:** Built-in Presets
- **Olası Kullanıcı Etkisi:** Bu mod seçildiğinde paket desync işlemi belirli Windows sürümlerinde düzgün tetiklenmez.
- **Önerilen Çözüm Aşaması:** P08 (Preset Validation Pipeline)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Preset syntax and runtime validation test.
- **Durum:** Resolved in P08 (`--dpi-desync=syndata,fake` faz sırası ile düzeltildi)

---

### R-12: `deep-fragmentation` Açıklama ve Argüman Uyumsuzluğu
- **Risk ID:** R-12
- **Başlık:** Preset metin açıklamasında TLS fragmentasyon yazmasına rağmen komutta farklı argüman bulunması
- **Kategori:** Documentation / UI Clarity
- **Önem Derecesi:** Low
- **Olasılık:** High
- **Etki:** Low
- **Mevcut Kanıt:** [builtin.json#L65](presets/builtin.json#L65)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Built-in Presets / UI
- **Olası Kullanıcı Etkisi:** Kullanıcı açıklamadaki davranış ile gerçek desync yöntemi arasında kafa karışıklığı yaşar.
- **Önerilen Çözüm Aşaması:** P08 (Preset Validation Pipeline)
- **Bağımlı Olduğu İşler:** P00 Baseline
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Preset description metadata audit.
- **Durum:** Resolved in P08 (`--dpi-desync=syndata,multisplit` ile split position dangling uyuşmazlığı giderildi)

---

### R-13: AdvancedConfig İçindeki Desteklenmeyen (Unused) Alanlar
- **Risk ID:** R-13
- **Başlık:** UI ve struct tiplerinde yer alan bazı advanced alanların Rust CLI builder tarafından kullanılmaması
- **Kategori:** Architecture / Dead Code
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Medium
- **Mevcut Kanıt:** [advanced.ts:L10](src/types/advanced.ts#L10), [manager.rs:L800](src-tauri/src/engine/manager.rs#L800)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Advanced Config / CLI Builder
- **Olası Kullanıcı Etkisi:** Kullanıcı UI üzerinde `wssize`, `mss` veya `customPayload` değiştirdiğinde bu değerler motor argümanlarına yansımaz (yalnızca yanılsama yaratır).
- **Önerilen Çözüm Aşaması:** P09 (Advanced Configuration Hardening)
- **Bağımlı Olduğu İşler:** P03 (Engine Launch Planner)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Advanced config serialization & argument mapping test.
- **Durum:** Resolved in P09 (Advanced capabilities matrisi ile phantom alanlar temizlendi, desteklenen alanlar typed boundary model ile motor argümanlarına bağlandı)

---

### R-14: DoQ (DNS over QUIC) Seçeneğinin Sessizce DoH'a Dönüştürülmesi
- **Risk ID:** R-14
- **Başlık:** UI'da DoQ seçildiğinde DNS çözücünün bunu DoH (HTTPS) olarak çalıştırması
- **Kategori:** Protocol / UI False Claim
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Low
- **Mevcut Kanıt:** [DnsView.tsx:L120](src/views/DnsView.tsx#L120), [manager.rs:L90](src-tauri/src/dns/manager.rs#L90)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** DNS Resolver
- **Olası Kullanıcı Etkisi:** DoQ kullandığını düşünen kullanıcı gerçekte DoH protokolü üzerinden sorgu atar.
- **Önerilen Çözüm Aşaması:** P10 (DNS Hardening)
- **Bağımlı Olduğu İşler:** P04 (Runtime Configuration Contract)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** DNS transport protocol assertions test.
- **Durum:** Mitigated in P05 / Resolved (DoQ option removed from UI, state hydrated to DoH, raw DoQ rejected)


---

### R-15: Wssize Model Uyumsuzluğu
- **Risk ID:** R-15
- **Başlık:** UI'da yüzde/ölçek olarak sunulan wssize değerinin Zapret parametre beklentisiyle örtüşmemesi
- **Kategori:** UI / Data Modeling
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Low
- **Mevcut Kanıt:** [AdvancedView.tsx:L210](src/views/AdvancedView.tsx#L210)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Advanced Configuration
- **Olası Kullanıcı Etkisi:** Geçersiz pencere boyutu değerleri aktarılarak motorun başlatılamamasına sebep olunabilir.
- **Önerilen Çözüm Aşaması:** P09 (Advanced Configuration Hardening)
- **Bağımlı Olduğu İşler:** P04 (Runtime Configuration Contract)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Wssize unit conversion test.
- **Durum:** Identified

---

### R-16: SOCKS5 Proxy'nin Yalnızca DNS Katmanında Etkili Olması
- **Risk ID:** R-16
- **Başlık:** Ayarlanan SOCKS5 proxy'nin genel DPI trafiğini değil sadece DoH sorgularını yönlendirmesi
- **Kategori:** Network / Scope Limitation
- **Önem Derecesi:** High
- **Olasılık:** High
- **Etki:** High
- **Mevcut Kanıt:** [forwarder.rs:L80](src-tauri/src/dns/forwarder.rs#L80)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** DNS / Safety Proxy
- **Olası Kullanıcı Etkisi:** Tüm internet trafiğinin SOCKS5 üzerinden tünellendiğini sanan kullanıcı ana trafiğinin doğrudan WinDivert üzerinden aktığını fark etmeyebilir.
- **Önerilen Çözüm Aşaması:** P10 (DNS & Kill Switch Hardening)
- **Bağımlı Olduğu İşler:** P00 Baseline
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Network traffic routing isolation test & documentation alignment.
- **Durum:** Identified

---

### R-17: `Running` Durumunun Gerçek Trafik Sağlığını Göstermemesi
- **Risk ID:** R-17
- **Başlık:** Process canlı olduğu sürece motor durumunun `Running` vermesi ama internet paketlerinin düşebilmesi
- **Kategori:** Observability / Health Check
- **Önem Derecesi:** High
- **Olasılık:** Medium
- **Etki:** High
- **Mevcut Kanıt:** [manager.rs:L24](src-tauri/src/engine/manager.rs#L24), [engineStore.ts:L45](src/store/engineStore.ts#L45)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Engine Manager / Health Monitor
- **Olası Kullanıcı Etkisi:** WinDivert filtresi hatalı bağlandığında uygulama yeşil (Çalışıyor) görünür ancak kullanıcının tüm interneti kesilir.
- **Önerilen Çözüm Aşaması:** P14 (Observability)
- **Bağımlı Olduğu İşler:** P07 (Process Lifecycle & Ownership)
- **Regression Riski:** Medium
- **Doğrulama Yöntemi:** Active health probe & status state machine test.
- **Durum:** Identified

---

### R-18: README ile Runtime Arasındaki Farklılıklar
- **Risk ID:** R-18
- **Başlık:** README belgelerindeki bazı iddiaların (ör. tam Linux servis entegrasyonu) kodda karşılığının bulunmaması
- **Kategori:** Documentation Integrity
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Low
- **Mevcut Kanıt:** [README.md](README.md), [README.tr.md](README.tr.md)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Documentation
- **Olası Kullanıcı Etkisi:** Kullanıcıların yanlış beklentiye girmesi ve destek talepleri oluşması.
- **Önerilen Çözüm Aşaması:** P00 / P16 (Documentation & Release Validation)
- **Bağımlı Olduğu İşler:** P01 Capability Matrix
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Documentation audit.
- **Durum:** Identified

---

### R-19: Release Pipeline'da Binary Hash Doğrulaması Olmaması
- **Risk ID:** R-19
- **Başlık:** GitHub Actions release workflow'unda paketlenen binary'lerin SHA-256 kontrollerinin yapılmaması
- **Kategori:** Supply Chain Security
- **Önem Derecesi:** High
- **Olasılık:** Low
- **Etki:** Critical
- **Mevcut Kanıt:** [releases.yml:L65](.github/workflows/releases.yml#L65)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** CI/CD Release Workflow
- **Olası Kullanıcı Etkisi:** Bozuk veya manipüle edilmiş executable binary'lerinin yanlışlıkla production release olarak yayınlanması riski.
- **Önerilen Çözüm Aşaması:** P13 (Security & Supply Chain)
- **Bağımlı Olduğu İşler:** P00 Baseline Hash Manifest
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** CI hash verification step execution test.
- **Durum:** Identified

---

### R-20: Packaged Privileged E2E Testinin Eksikliği
- **Risk ID:** R-20
- **Başlık:** CI ortamında yetkili (Administrator) sürücü yükleme ve paketli kurulum testlerinin otomasyona bağlanmaması
- **Kategori:** Testing / QA Gap
- **Önem Derecesi:** High
- **Olasılık:** Medium
- **Etki:** High
- **Mevcut Kanıt:** [ci.yml:L48](.github/workflows/ci.yml#L48)
- **Etkilenen Platform:** Windows
- **Etkilenen Bileşen:** CI/CD Test Harness
- **Olası Kullanıcı Etkisi:** Sadece unprivileged unit testlerden geçen ama gerçek Windows sürücü seviyesinde çöken sürümlerin yayınlanması.
- **Önerilen Çözüm Aşaması:** P15 (CI/CD & Automated Verification)
- **Bağımlı Olduğu İşler:** P07 (Process Lifecycle & Ownership)
- **Regression Riski:** Medium
- **Doğrulama Yöntemi:** Windows Acceptance build workflow execution.
- **Durum:** Identified

---

### R-21: Firewall Rule Ownership (Sahiplik) Eksikliği
- **Risk ID:** R-21
- **Başlık:** Oluşturulan güvenlik duvarı kurallarına Vane UUID veya metadata etiketi eklenmemesi
- **Kategori:** System Hygiene / Resource Leak
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Medium
- **Mevcut Kanıt:** [router.rs:L25](src-tauri/src/network/router.rs#L25)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Firewall Manager
- **Olası Kullanıcı Etkisi:** Uygulama çöktüğünde veya kaldırıldığında işletim sisteminde artık (orphan) engelleme kuralları kalması.
- **Önerilen Çözüm Aşaması:** P10 (Kill Switch Hardening)
- **Bağımlı Olduğu İşler:** P07 (Process Lifecycle & Ownership)
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Firewall rule tag & cleanup integration test.
- **Durum:** Identified

---

### R-22: Kill Switch Crash Recovery Riski
- **Risk ID:** R-22
- **Başlık:** Kill Switch etkinken uygulama aniden kapandığında sistem DNS veya ağ engelinin takılı kalması
- **Kategori:** Fail-Safe / Network Availability
- **Önem Derecesi:** Critical
- **Olasılık:** Medium
- **Etki:** Critical
- **Mevcut Kanıt:** [watchdog.rs:L10](src-tauri/src/dns/watchdog.rs#L10)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Kill Switch / DNS Watchdog
- **Olası Kullanıcı Etkisi:** Kullanıcının bilgisayarı yeniden başlatılsa bile internet erişiminin tamamen kesik kalması.
- **Önerilen Çözüm Aşaması:** P10 (DNS & Kill Switch Hardening)
- **Bağımlı Olduğu İşler:** P07 (Process Lifecycle & Ownership)
- **Regression Riski:** High
- **Doğrulama Yöntemi:** Process crash simulation & boot-time recovery test.
- **Durum:** Identified

---

### R-23: AdBlock Listesinin İmzasız veya Sabitlenmemiş Olması
- **Risk ID:** R-23
- **Başlık:** AdBlock engelleme listesinin dış URL'den imzasız indirilebilmesi
- **Kategori:** Security / Remote Content
- **Önem Derecesi:** Medium
- **Olasılık:** Low
- **Etki:** Medium
- **Mevcut Kanıt:** [forwarder.rs:L320](src-tauri/src/dns/forwarder.rs#L320)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** DNS AdBlock Filter
- **Olası Kullanıcı Etkisi:** İndirilen listedeki hatalı bir kural kullanıcının meşru sitelerine erişimini engelleyebilir (Man-in-the-middle riski).
- **Önerilen Çözüm Aşaması:** P10 (DNS Hardening) / P13 (Security)
- **Bağımlı Olduğu İşler:** P00 Baseline
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Remote content signature & fallback check test.
- **Durum:** Identified

---

### R-24: IPC Tiplerinin Rust ve TypeScript'te Ayrı Tutulması
- **Risk ID:** R-24
- **Başlık:** Rust `ipc.rs` ile TypeScript `ipc.ts` tiplerinin otomatik kod üretimi olmadan manuel tanımlanması
- **Kategori:** Architecture / Type Safety
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** High
- **Mevcut Kanıt:** [ipc.rs:L1](src-tauri/src/ipc.rs#L1), [ipc.ts:L1](src/types/ipc.ts#L1)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** IPC Layer
- **Olası Kullanıcı Etkisi:** Rust tarafındaki bir DTO alan adı değiştiğinde TypeScript tarafında sessiz `undefined` hataları veya arayüz kırılmaları yaşanır.
- **Önerilen Çözüm Aşaması:** P04 (Runtime Configuration Contract)
- **Bağımlı Olduğu İşler:** P01 & P02 Characterization Tests
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** JSON Schema fixture contract test.
- **Durum:** Identified

---

### R-25: Runtime Schema Validation Eksikliği
- **Risk ID:** R-25
- **Başlık:** Diskten okunan ayarlarda veya IPC payload'larında çalışma zamanı şema doğrulaması bulunmaması
- **Kategori:** Data Integrity / Input Validation
- **Önem Derecesi:** Medium
- **Olasılık:** Medium
- **Etki:** Medium
- **Mevcut Kanıt:** [settings.rs:L120](src-tauri/src/settings.rs#L120)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Settings Persistence / IPC
- **Olası Kullanıcı Etkisi:** Bozuk veya tahrif edilmiş konfigürasyon dosyaları uygulamanın açılışta çökmesine neden olabilir.
- **Önerilen Çözüm Aşaması:** P04 (Runtime Configuration Contract)
- **Bağımlı Olduğu İşler:** P02 Rust Characterization
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Malformed JSON input fuzzing & recovery test.
- **Durum:** Identified

---

### R-26: Applied ve Optimistic State Ayrımının Olmaması
- **Risk ID:** R-26
- **Başlık:** UI Zustand store'unun Rust onayını beklemeden durum güncellemesi (optimistic UI) yapması
- **Kategori:** UI / State Synchronization
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Medium
- **Mevcut Kanıt:** [engineStore.ts:L150](src/store/engineStore.ts#L150)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Frontend Store / UI
- **Olası Kullanıcı Etkisi:** Kullanıcı butona bastığında arayüz "Aktif" gösterir ancak Rust tarafında sürücü yetkisi alınamadığı için işlem başarısız olmuş olabilir.
- **Önerilen Çözüm Aşaması:** P04 (Runtime Configuration Contract) / P05 (Low-Risk Fixes)
- **Bağımlı Olduğu İşler:** P01 Frontend Characterization
- **Regression Riski:** Medium
- **Doğrulama Yöntemi:** State synchronization & rollback assertion test.
- **Durum:** Resolved in P06 (PatternApplyTransaction, revizyonlu hostlist `domains-rev-{revision}-{fingerprint_prefix}.txt`, candidate spawn hatasında otomatik rollback ve superseded kontrolü uygulandı)

---

### R-27: Optimizer Direct Binary Spawning ve Lifecycle Bypass (RBR-08)
- **Risk ID:** R-27
- **Başlık:** Optimizer bileşeninin doğrudan `winws`/`nfqws` süreçleri başlatması ve P07 process lifecycle koordinasyonunu by-pass etmesi
- **Kategori:** Process Safety / Lifecycle Isolation
- **Önem Derecesi:** Critical
- **Olasılık:** High
- **Etki:** High
- **Mevcut Kanıt:** [optimizer.rs](src-tauri/src/engine/optimizer.rs)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Optimizer / Engine Lifecycle
- **Olası Kullanıcı Etkisi:** Aday testleri sırasında yetim süreçler kalabilir, WinDivert/NFQUEUE sürücü kilitlenmeleri oluşabilir veya oturum sonlandığında motor ilk durumuna dahi geri dönmeyebilir.
- **Önerilen Çözüm Aşaması:** P12 (Optimizer Safety & Unified Engine Lifecycle)
- **Bağımlı Olduğu İşler:** P07 Engine Lifecycle, P08 Preset Validation
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** FakeRuntime restore assertion & soak test (50 repeated sessions).
- **Durum:** Resolved in P12 (`OptimizerSessionManager` ve `ProductionOptimizerRuntime` eklendi; tüm adaylar `EngineManager` ve Job Objects / Process Group koordinatorüne devredildi, `OriginalEngineState` atomik restore eklendi)

---

### R-28: Optimizer Sabit Hedef IP'ler ve Tek Ölçüm Modeli (RBR-09)
- **Risk ID:** R-28
- **Başlık:** Optimizer'ın sabit IP adresleri kullanması ve tekil örneklemeyle yanlış seçim yapması
- **Kategori:** Measurement / Networking
- **Önem Derecesi:** Medium
- **Olasılık:** High
- **Etki:** Medium
- **Mevcut Kanıt:** [reproducers.rs:L129](src-tauri/src/characterization/reproducers.rs#L129)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Optimizer Targets & Measurement
- **Olası Kullanıcı Etkisi:** ISP veya CDN tarafında IP değiştiğinde testler başarısız olabilir veya anlık dalgalanmalar nedeniyle en yavaş preset "kazanan" seçilebilir.
- **Önerilen Çözüm Aşaması:** P12 (Optimizer Safety & Unified Engine Lifecycle)
- **Bağımlı Olduğu İşler:** P12 Candidate Measurement
- **Regression Riski:** Low
---

### R-29: Unverified Native Artifact Launch & Tampering Risk
- **Risk ID:** R-29
- **Başlık:** Native ikili dosyaların, sürücülerin ve kütüphanelerin doğrulama yapılmadan çalıştırılması riski
- **Kategori:** Security / Supply Chain / Tampering
- **Önem Derecesi:** Critical
- **Olasılık:** Low
- **Etki:** Critical
- **Mevcut Kanıt:** [manager.rs:L164](src-tauri/src/engine/manager.rs#L164)
- **Etkilenen Platform:** Cross
- **Etkilenen Bileşen:** Engine Launcher / Security / Artifact Verifier
- **Olası Kullanıcı Etkisi:** Tahrif edilmiş veya değiştirilmiş ikili dosyaların çalıştırılması sonucu zararlı kod yürütülmesi veya yetkisiz erişim riski.
- **Önerilen Çözüm Aşaması:** P13 (Binary Integrity & Supply Chain Security)
- **Bağımlı Olduğu İşler:** P07 Engine Lifecycle, P12 Optimizer Safety
- **Regression Riski:** Low
- **Doğrulama Yöntemi:** Embedded manifest streaming SHA-256 integrity verification test & tamper detection test.
- **Durum:** Resolved in P12/P13 (`Sha256ArtifactIntegrityVerifier` ve `VerifiedArtifactGroup` eklendi; launcher ve optimizer fail-closed yapıldı)



