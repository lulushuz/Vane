# P11 — Linux Platform Layer Isolation, Dynamic NFQUEUE Filter Planner ve Rule Ownership Tamamlama Raporu

**Tarih:** 2026-07-30  
**Sürüm:** 2.1.4  
**Aşama:** P11  

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
- **Before P11:** 14 test dosyası, 137 test geçti (0 hata)
- **After P11:** 15 test dosyası, 139 test geçti (0 hata)

### Rust Backend
- **Before P11:** 215 test geçti (0 hata, 0 atlanan)
- **After P11:** 224 test geçti (0 hata, 0 atlanan)

---

## 3. P10 Kapanış Doğrulaması

| P10 alanı                | Production integration | Test | Sonuç |
| ------------------------ | ---------------------: | ---: | ----- |
| Frontend revision gating |               VERIFIED | PASS | VERIFIED |
| DNS transaction command  |               VERIFIED | PASS | VERIFIED |
| Forwarder lifecycle      |               VERIFIED | PASS | VERIFIED |
| Kill Switch executor     |               VERIFIED | PASS | VERIFIED |
| Partial apply rollback   |               VERIFIED | PASS | VERIFIED |
| Startup recovery         |               VERIFIED | PASS | VERIFIED |
| Legacy migration         |               VERIFIED | PASS | VERIFIED |

---

## 4. Eski ve Yeni Linux Akışı

| Katman | Eski (Legacy) Davranış | Yeni (P11) Davranış |
| :--- | :--- | :--- |
| **Filter Source** | `--wf-tcp` ve `--wf-udp` argümanları düşürülüyordu | Typed `LinuxFilterIntent` ile NFQUEUE kurallarına dönüştürülüyor |
| **TCP Portları** | Sabit TCP 80,443 | Verified Preset / Advanced TCP port listeleri (`PortRange`) |
| **UDP Portları** | Sıfır (Yok) | Verified Preset UDP portları (QUIC UDP 443, Discord 50000-65535) |
| **Shell Launcher** | `sh -c` script string escaping ile inline kural ekleme | Shell'siz `Command` + argv ile `nft` batch stdin veya `iptables` |
| **Rule Ownership** | Yok (`vane_mangle` toplu silme) | `LinuxRuleOwnership` (`vane_tbl_{inst}`, `vane_chn_{instance}_g{gen}`) |
| **Rollback & Recovery** | Yok | Reverse partial apply rollback ve `linux-engine-filter.json` yetim kurtarma |

---

## 5. Linux Filter Plan Mimarisi

```text
VerifiedPreset / VerifiedRuntimeConfig
   │
   ▼
LinuxFilterIntent (parse_port_spec -> TCP & UDP PortRange)
   │
   ▼
probe_linux_capabilities (nftables / iptables / privileges)
   │
   ▼
build_linux_filter_plan
   ├─ Ownership: vane_tbl_{inst} / vane_chn_{instance}_g{gen}
   ├─ IPv4 & IPv6 Rules (queue_num = 200)
   └─ Apply / Rollback / Remove steps
   │
   ▼
SystemLinuxFirewallExecutor
   ├─ Nftables: Single atomic batch via stdin
   └─ Iptables: Step-by-step with reverse partial rollback on error
   │
   ▼
Persist linux-engine-filter.json metadata
```

---

## 6. Sahiplik (Ownership) ve Temizlik Politikası

- **Exact Naming:** Tablolar ve zincirler Vane Kurulum ID (`inst_prefix`) ve Instance ID (`instance_prefix`) ile adlandırılır.
- **Foreign Rule Protection:** Yabancı uygulamalara veya diğer Vane kurulumlarına ait kurallara/tablolara dokunulmaz.
- **Orphan Recovery:** Uygulama veya motor çökmesi durumunda `linux-engine-filter.json` okunarak yalnız aktif kuruluma ait yetim tablolar kaldırılır.

---

## 7. Platform Kabiliyet Matrisi

Privileged canlı VM doğrulama adımına kadar Linux kabiliyetleri frontend ve backend seviyesinde `Experimental` olarak işaretlenmiştir:

- **Linux TCP Filtering:** `Experimental — automated plan/executor tests passed`
- **Linux Custom TCP Ports:** `Experimental — automated plan/executor tests passed`
- **Linux UDP Filtering:** `Experimental — automated plan/executor tests passed`
- **Linux Custom UDP Ports:** `Experimental — automated plan/executor tests passed`

---

## 8. Çalıştırılan Komutlar ve Sonuçları

```text
Command: npm test
Result: PASSED (15 test dosyası, 139 test geçti)

Command: npm run build
Result: PASSED (tsc && vite build temiz)

Command: cd src-tauri; cargo fmt --check
Result: PASSED (Temiz)

Command: cd src-tauri; cargo test --lib
Result: PASSED (224 test geçti)

Command: cd src-tauri; cargo clippy --lib -- -D warnings
Result: PASSED (0 uyarı, temiz)
```

---

## 9. Manuel Acceptance Planı

```text
Linux VM (Privileged): NOT EXECUTED — requires controlled privileged Linux VM
Automated Plan / Executor Verification: PASSED
```

---

## 10. Kesinlikle Yapılmayanlar Teyidi

- P06 Pattern authority bozulmadı.
- P07 process ownership ve Job Object yapısı bozulmadı.
- P08 preset validator bypass edilmedi.
- P09 Advanced capability contract bozulmadı.
- P10 DNS transaction ve Kill Switch bozulmadı.
- Global `killall` veya `iptables -F` / `nft flush` eklenmedi.
- Shell üzerinden kullanıcı kontrollü komut çalıştırılmadı.
- Windows WinDivert davranışı değiştirilmedi.
- Optimizer lifecycle'a taşınmadı (P12).
- Version bump veya release tag oluşturulmadı.

---

## 11. Geçiş Kararı

**READY FOR P12**
