# P08 — Birleşik Preset Validation Pipeline, Semantik Strateji Kontrolü ve Preset Corpus Temizliği Tamamlama Raporu

**Tarih:** 2026-07-29  
**Sürüm:** 2.1.4  
**Aşama:** P08  

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
- **Before P08:** 12 test dosyası, 125 test geçti (0 hata)
- **After P08:** 12 test dosyası, 125 test geçti (0 hata)

### Rust Backend
- **Before P08:** 193 test geçti (0 hata, 0 atlanan)
- **After P08:** 201 test geçti (0 hata, 0 atlanan)

---

## 3. Unified Validation Pipeline Mimarisi

Tüm preset kaynakları (built-in, custom, imported `.vane`, legacy `.json`, remote signed, optimizer candidates) tek authoritative `validate_preset` pipeline'ından geçirilmektedir.

```text
RawPreset / Preset
       │
       ▼
validate_preset(preset, source)
       │
       ├── 1. ID & Structural Validation (Alphanumeric/hyphen/underscore)
       ├── 2. Argument Count (<= 30) & Length (<= 128) Bounds
       ├── 3. Shell Injection & Forbidden Character Sanitization
       ├── 4. Forbidden Hostlist Path Injection Filter (--hostlist=)
       ├── 5. Single-Value Duplicate Argument Check
       ├── 6. Desync Method Parsing & Phase Sequence Check
       ├── 7. Cross-Argument Compatibility (TTL, Split, Repeats)
       └── 8. Platform Support Analysis (Windows / Linux)
       │
       ▼
VerifiedPreset
```

---

## 4. Semantik Phase Modeli ve Zapret Yöntem Sınıflandırması

Zapret desync yöntemleri 3 ana evreye ayrılmıştır:

- **Phase 0 (SYN/SYN-ACK Manipülasyonu):** `syndata`, `rst`, `rstack`
- **Phase 1 (Payload & Bölümleme):** `fake`, `fakeknown`, `split`, `split2`, `multisplit`, `disorder`, `multidisorder`, `hostfake`, `fakedsplit`
- **Phase 2 (Opsiyon & Post-handshake):** `destopt`, `ipfrag1`, `ipfrag2`, `udplen`, `tamper`, `none`

### Kurallar:
1. Azalan faz sırası reddedilir (Örn: `fake,syndata` geçersizdir, Phase 0 olan `syndata` önce gelmelidir: `syndata,fake`).
2. Tek bir strateji içinde aynı desync yöntemi tekrar edemez.
3. `none` yöntemi başka yöntemlerle birleştirilemez.
4. Maksimum desync yöntem sayısı `MAX_DESYNC_METHODS = 3` olarak sınırlandırılmıştır.

---

## 5. Built-in Preset Corpus İncelemesi ve `https-sni-ghost` Düzeltmesi

- **`https-sni-ghost` (RBR-03):** Eski argüman `--dpi-desync=fake,syndata` (Phase 1 -> Phase 0) semantik faz hatası içeriyordu. Yeni geçerli argüman `--dpi-desync=syndata,fake` (Phase 0 -> Phase 1) olarak düzeltildi.
- **`deep-fragmentation`:** `--dpi-desync=syndata` ile birlikte kullanılan `--dpi-desync-split-pos=1` parametresi için split desync yöntemi eksikti; `--dpi-desync=syndata,multisplit` olarak güncellendi.
- **Built-in Corpus CI Testi:** `test_every_builtin_preset_is_structurally_semantically_and_platform_valid` testi ile binary içerisinde gömülü tüm 12 varsayılan preset doğrulandı.

---

## 6. Reproducer Çözüm Sonuçları

- **RBR-02 (Missing preset phase validation):** `RBR-02 resolved: semantic validator rejects descending desync phase order` olarak doğrulandı.
- **RBR-03 (https-sni-ghost invalid phase order):** `RBR-03 resolved: https-sni-ghost uses a semantically valid phase sequence` olarak doğrulandı.

---

## 7. Geçiş Kararı

**READY FOR P09**
