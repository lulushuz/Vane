# P09 — Typed Advanced Configuration Contract, Gerçek Backend Bağlantısı ve Unsupported Alanların Temizlenmesi Tamamlama Raporu

**Tarih:** 2026-07-29  
**Sürüm:** 2.1.4  
**Aşama:** P09  

---

## 1. Repository Durumu

- **Branch:** `main`
- **Start Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238`
- **End Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238`
- **Baseline Matched:** Evet (2.1.4)
- **Pre-existing Files:** Korundu

---

## 2. Test Sonuçları

### Frontend
- **Before P09:** 12 test dosyası, 125 test geçti (0 hata)
- **After P09:** 13 test dosyası, 133 test geçti (0 hata)

### Rust Backend
- **Before P09:** 201 test geçti (0 hata, 0 atlanan)
- **After P09:** 204 test geçti (0 hata, 0 atlanan)

---

## 3. P08 Kapanış Doğrulaması (Preflight Check)

| Preset yolu | Unified validator | Verified type | Runtime bypass mümkün mü? | Sonuç |
| --- | ---: | ---: | ---: | --- |
| Built-in | Evet (`validate_preset` in `ConfigLoader::new`) | `VerifiedPreset` | Hayır | VERIFIED |
| Custom | Evet (`validate_preset` in `load_custom_presets_from`/`save_custom_preset`) | `VerifiedPreset` | Hayır | VERIFIED |
| `.vane` import | Evet (`validate_preset` in `import_preset`) | `VerifiedPreset` | Hayır | VERIFIED |
| Legacy `.json` | Evet (`validate_preset` in `import_preset`) | `VerifiedPreset` | Hayır | VERIFIED |
| Remote | Evet (`validate_preset` in `load_remote_presets`) | `VerifiedPreset` | Hayır | VERIFIED |
| Optimizer | Evet (`validate_preset` in `run_heuristic_scan`) | `VerifiedPreset` | Hayır | VERIFIED IN P09 PREFLIGHT |
| Runtime launch | Evet (`validate_preset` in `verify_runtime_config`) | `VerifiedRuntimeConfig` | Hayır | VERIFIED IN P09 PREFLIGHT |

---

## 4. Advanced Capabilities & IPC Mimarisi

Rust backend tarafında platform bağımsız deterministic `get_advanced_capabilities` IPC komutu ve `AdvancedCapabilities::for_current_platform()` fonksiyonu oluşturulmuştur.

### Frontend Boundary Model:
```text
AdvancedConfigCandidate
       │
       ▼
validateAdvancedConfig
       │
       ▼
VerifiedAdvancedConfig
       │
       ▼
serializeVerifiedAdvancedConfig
       │
       ▼
RawPreset / Preset
       │
       ▼
validate_preset
       │
       ▼
VerifiedPreset ──► VerifiedRuntimeConfig ──► EngineLaunchPlan
```

---

## 5. BR-06 Çözümü (Non-443 UDP Port Range Loss)

- **Problem:** `--wf-udp=50000-65535` gibi non-443 UDP port aralıkları eski parser tarafından kayboluyordu.
- **Çözüm:** `PortRange[]` ve `VerifiedTrafficFilter` typed modelleri ile hem TCP hem UDP port aralıkları (`50000-65535`, `443`, `53,443`) parse ve serialization round-trip boyunca eksiksiz korunmaktadır.
- **Test:** `BR-06 resolved: non-443 UDP port ranges survive Advanced parse and serialization` testi ile doğrulandı.

---

## 6. Kaldırılan / Sınıflandırılan Unsupported Phantom Alanlar

Bundled Zapret binary'sinde gerçek karşılığı olmayan veya UI'da phantom olarak yer alan alanlar net bir şekilde sınıflandırılmıştır:

1. `mssFix` / `--mss=`: Bundled engine tarafından desteklenmiyor (Disabled / Warning).
2. `fakeTlsSni` / `--dpi-desync-fake-tls-sni=`: Engine desteği yok (Disabled).
3. `bindAddress` / `--bind-addr=`: Engine desteği yok (Disabled).
4. `ipset` / `--ipset=`: Engine desteği yok (Disabled / Quarantined).
5. `tpws` / `--tpws`: Engine desteği yok (Disabled / Quarantined).
6. State Migration: Persisted state içinden bu phantom alanlar `advancedConfigSchemaVersion: 2` geçişi ile güvenle temizlenmektedir.

---

## 7. Cross-Language Fixtures Parity

`src-tauri/fixtures/advanced/` altında 10 ortak test fixture'ı oluşturulmuş ve hem Vitest (`advancedCapabilities.test.ts`) hem de Rust backend (`advanced_contract_tests.rs`) tarafından çalıştırılarak cross-language doğrulama sağlanmıştır:

1. `basic-split.json`
2. `tr-1.json`
3. `tr-2.json`
4. `discord-voip.json`
5. `youtube-quic.json`
6. `non-443-udp.json` (BR-06)
7. `selector-split.json`
8. `invalid-conflicting-ttl.json`
9. `invalid-hostlist-injection.json`
10. `unsupported-fields.json`

---

## 8. Geçiş Kararı

**READY FOR P10**
