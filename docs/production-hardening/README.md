# Vane Production Hardening

Bu dizin, **Vane** uygulamasını prototip seviyesinden production seviyesine taşımak amacıyla yürütülen hardening (güvenlik ve kararlılık sertleştirme) sürecinin kılavuzunu, kurallarını ve safha dokümantasyonunu içerir.

---

## 1. Amaç ve Kapsam

Vane, DPI (Deep Packet Inspection) engellemelerini aşmak için Windows WinDivert ve Linux NFQUEUE / nftables sürücü seviyesi altyapılarını kullanan yüksek yetkili bir masaüstü uygulamasıdır. Production hardening sürecinin ana hedefleri:

1. **Güvenli Çalışma Zemini:** Sistemin mevcut yeteneklerini, davranışlarını ve sınırlarını eksiksiz belgelemek.
2. **Deterministic Process Ownership:** `winws` / `nfqws` ve WinDivert gibi kritik bileşenlerin yetkisiz / sızıntı yapmayacak şekilde kontrol altına alınması.
3. **Fail-Closed & Safe Rollback:** Hata durumlarında ağ erişiminin yetkisiz açık kalmaması ve sistem DNS/firewall ayarlarının otomatik eski haline dönebilmesi.
4. **Platform Önceliği:** Production kalitesinin öncelikle **Windows 10 ve Windows 11** üzerinde tam garantilenmesi; Linux platformunun şimdilik **experimental** statüde tutulması.

---

## 2. Production Hardening Çalışma Kuralları

1. **Tek Yönlü Safha İlerlemesi:** Bir aşamanın (P-stage) tüm başarı kriterleri (Exit Criteria) ve kapıları (Gates) tamamlanıp doğrulanmadan bir sonraki aşamaya geçilemez.
2. **Feature Freeze Politikası:** Production hardening sürecinde hiçbir yeni özellik, yeni preset, yeni UI bileşeni veya yeni network davranışı eklenemez.
3. **Küçük ve Sorumluluğu Belirli PR'lar:** Her PR yalnızca tek bir hardening aşamasına odaklanmalı ve ilgili doküman / test güncellemelerini barındırmalıdır.
4. **Test-First Yaklaşımı:** Herhangi bir refaktör veya düşük riskli bug fix öncesinde ilgili davranış karakterizasyon testleri (frontend, IPC contract ve Rust integration) yazılmalıdır.
5. **Rollback Zorunluluğu:** Uygulama çökse veya beklenmedik şekilde kapansa dahi sistem DNS veya Firewall kuralları asla bozuk veya engellenmiş durumda bırakılamaz.
6. **Windows-First Kararı:** Windows 10/11 platformu birincil production hedefidir. Linux desteği mevcut kodda deneyseldir (experimental) ve stable release kapsamını bloklamaz.

---

## 3. Aşamaların Uygulanma Sırası (Phase Sequence)

| Safha | Adı | Tanım |
| :--- | :--- | :--- |
| **P00** | **Baseline, Capability Matrix & Grounding** | Mevcut durumun dondurulması, hash manifesti, yetenek matrisi, risk kütüğü ve kapıların tanımlanması. |
| **P01** | **Frontend Characterization Tests** | Component, Store ve Serializer seviyesinde frontend birim testlerinin tamamlanması. |
| **P02** | **Rust Characterization Tests** | Sanitizer, Domain Canonicalization, Preset Loader ve Settings persistence birim testleri. |
| **P03** | **Engine Launch Planner** | Komut satırı argüman oluşturma ve doğrulama katmanının izole edilmesi. |
| **P04** | **Runtime Configuration Contract** | Rust ve TypeScript arasındaki IPC DTO ve schema tip uyumunun otomasyona bağlanması. |
| **P05** | **Low-Risk Bug Fixes** | Risk register'da tanımlanan düşük riskli mantık hatalarının düzeltilmesi. |
| **P06** | **Pattern Transaction** | Site listesi senkronizasyonu ve atomik disk persistance yarış durumlarının engellenmesi. |
| **P07** | **Process Lifecycle & Ownership** | Child process PID takibi, Job Object (Windows) ve sızmayan temizlik mekanizması. |
| **P08** | **Preset Validation Pipeline** | Dahili ve harici tüm presetlerin semantic validation ve phase-order kontrolünden geçirilmesi. |
| **P09** | **Advanced Configuration Hardening** | Desteklenmeyen parametrelerin temizlenmesi ve wssize/TTL modellerinin doğrulanması. |
| **P10** | **DNS & Kill Switch Hardening** | System DNS ve Kill Switch için fail-closed, watchdog ve atomic rollback sağlanması. |
| **P11** | **Linux Platform Layer Isolation** | Linux nftables / NFQUEUE katmanının izole edilip experimental olarak işaretlenmesi. |
| **P12** | **Optimizer Safety & Isolation** | Optimizer'ın EngineManager üzerinden çalıştırılması ve dinamik hedef tespiti. |
| **P13** | **Security & Supply Chain** | Binary imza doğrulama, Minisign entegrasyonu, SBOM ve dependency audit kapıları. |
| **P14** | **Observability & Logging** | Structured logging, IPC event batching ve runtime metric takibi. |
| **P15** | **CI/CD & Automated Verification** | Privileged test harness integration ve GitHub Actions workflow sertleştirilmesi. |
| **P16** | **Release Candidates & Validation** | Full acceptance testing, manual network matrix ve production release adayları. |

---

## 4. Doküman Haritası

- [00-baseline.md](docs/production-hardening/00-baseline.md) — Dondurulan mevcut repository durumu, sürümler, komutlar, binary ve statik dosya hash manifesti.
- [01-capability-matrix.md](docs/production-hardening/01-capability-matrix.md) — Gerçek koda dayalı özellik yetenek matrisi (Frontend / Backend / Runtime / Platform ayrımı).
- [02-risk-register.md](docs/production-hardening/02-risk-register.md) — Kodda tespit edilen risklerin önem dereceleri, etkileri ve çözüm aşaması planı.
- [03-test-matrix.md](docs/production-hardening/03-test-matrix.md) — Acceptance ve regression test senaryoları (Unit, Contract, Integration, Privileged, Packaged, Manual).
- [04-release-gates.md](docs/production-hardening/04-release-gates.md) — Production release öncesi geçilmesi zorunlu olan Gate 1-5 kriterleri ve durumları.
- [05-known-gaps.md](docs/production-hardening/05-known-gaps.md) — Onaylanmış kusurlar, henüz doğrulanmamış davranışlar ve dokümante edilip uygulanmamış özellikler.
