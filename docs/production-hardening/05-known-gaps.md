# P05 Known Gaps (Bilinen Eksikler ve Uyumsuzluklar)

Bu doküman, Vane kod tabanında tespit edilen onaylanmış kusurları, henüz doğrulanmamış davranışları ve dokümante edildiği halde kodda karşılığı olmayan özellikleri özetler.

---

## 1. Onaylanmış Kusurlar (Confirmed Defects)

Gerçek kod ve P01 karakterizasyon testleri ile kesin olarak tespit edilmiş çalışma zamanı veya mantık hataları:

1. **Pattern Cache vs Disk State Desenkronizasyonu**
   - **Açıklama:** Pattern site listesi güncellendiğinde bellek içi önbellek ile diskteki konfigürasyon atomik olarak eşitlenmiyor; yarış durumlarına açık.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [engineStore.ts:L110](src/store/engineStore.ts#L110), [settings.rs:L100](src-tauri/src/settings.rs#L100), [bugReproducers.test.ts:BR-01](src/test/bugReproducers.test.ts)
   - **Çözüleceği Aşama:** **P06 (Pattern Transaction)**

2. **Linux Ortamında WinDivert Filtre Argümanlarının Atılması**
   - **Açıklama:** Windows için yazılan `--wf-tcp=` ve `--wf-udp=` parametreleri sessizce kayboluyordu.
   - **Etkilenen Platform:** Linux
   - **Kanıt Dosyası:** [filter_intent.rs](src-tauri/src/platform/linux/filter_intent.rs)
   - **Çözüleceği Aşama:** **Çözüldü (P11)** — `LinuxFilterIntent` ve `LinuxFilterPlan` üzerinden dinamik NFQUEUE kurallarına dönüştürüldü.

3. **Linux Firewall Yönlendirmesinin Yalnızca TCP 80/443 Olması**
   - **Açıklama:** Linux kuralları varsayılan olarak TCP 80/443 portlarını hardcode ediyordu, UDP (QUIC) hariç tutuluyordu.
   - **Etkilenen Platform:** Linux
   - **Kanıt Dosyası:** [filter_plan.rs](src-tauri/src/platform/linux/filter_plan.rs)
   - **Çözüleceği Aşama:** **Çözüldü (P11)** — Dinamik TCP/UDP port desteği (`PortRange`) ve `Experimental` kabiliyet matrisi ile çözüldü.

4. **Optimizer Modülünün `EngineManager` Dışından Doğrudan Süreç Açması**
   - **Açıklama:** Optimizer ham `std::process::Command` ile `winws.exe` açıyor; bu durum yetim süreç (orphan process) kalmasına yol açıyordu.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [optimizer.rs:L80](src-tauri/src/engine/optimizer.rs#L80)
   - **Çözüleceği Aşama:** **Çözüldü (P12)** — `OptimizerSessionManager` ve `ProductionOptimizerRuntime` üzerinden tüm adaylar `EngineManager` ve Job Objects / Process Group koordinasyonuna devredildi; `OriginalEngineState` atomik restore eklendi.

5. **Firewall ve Kill Switch Kurallarında Sahiplik Metadata'sı Eksikliği**
   - **Açıklama:** Oluşturulan güvenlik duvarı kuralları Vane UUID etiketi içermediği için çökme veya kaldırma durumunda sistemde kalıyordu.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [router.rs:L25](src-tauri/src/network/router.rs#L25)
   - **Çözüleceği Aşama:** **Çözüldü (P10)** — `KillSwitchOwnership` (`installation_id`, `instance_id`, `revision`, `fingerprint`, `rule_ids`) ve atomic `dns-kill-switch.json` yetim recovery mekanizması ile çözüldü.

6. **Non-443 UDP Port Aralıklarının `argsParser` Tarafından Kaybedilmesi (P01 Testlerinde Doğrulandı)**
   - **Açıklama:** `--wf-udp=50000-65535` gibi 443 dışı UDP port süzgeçleri `parseArgsToConfig` ve `serializeConfigToArgs` döngüsünde serileştirmeden düşürülmektedir.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [argsParser.ts:L103](src/utils/argsParser.ts#L103), [bugReproducers.test.ts:BR-06](src/test/bugReproducers.test.ts)
   - **Çözüleceği Aşama:** **P09 (Advanced Configuration Hardening)**

7. **Başlatma Devam Ederken Kapatma Çağrıldığında Motor Durumu Yarışı (P01 Testlerinde Doğrulandı)**
   - **Açıklama:** `startEngine` asenkron işlemi devam ederken (yavaş DNS/IPC sırasında) `stopEngine` çağrılıp durum `stopped` yapılsa dahi, geride kalan `startEngine` tamamlandığında durumu tekrar `running` üzerine yazmaktadır.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [engineStore.ts:L601](src/store/engineStore.ts#L601), [bugReproducers.test.ts:BR-07](src/test/bugReproducers.test.ts)
   - **Çözüleceği Aşama:** **Çözüldü (P07)** — `engineStore.ts` `lifecycleToken` guard'ı ve BR-07 doğrulaması ile çözüldü.

8. **P03 Planner İçin `LaunchBypassInput` Bağlantısı (P04'te `VerifiedRuntimeConfig` İle Tamamlandı)**
   - **Açıklama:** P03'te oluşturulan `EngineLaunchPlanner` saf girdi olarak `LaunchBypassInput` kullanıyordu. P04'te `VerifiedRuntimeConfig::to_launch_bypass_input` ile bu bağlantı sağlandı; ancak config kaynağı henüz P06'da otoriter kılınacaktır.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [runtime_config.rs:L137](src-tauri/src/engine/runtime_config.rs#L137)
   - **Çözüleceği Aşama:** **P06 (Pattern Transaction ve Source-of-Truth)**

9. **Prepared ve Applied State Ayrımının Arayüzde (UI) Henüz Görselleşmemesi (P04)**
   - **Açıklama:** Rust backend içinde `PreparedRuntimeConfig` (planlandı, henüz çalışmadı) ve `AppliedRuntimeConfig` (süreç başladı, PID mevcut) türleri tamamen ayrılmış olup, arayüz IPC seviyesinde bu fark henüz ayrı durumlara eşlenmemektedir.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [runtime_config.rs:L180](src-tauri/src/engine/runtime_config.rs#L180), [runtime_config_tests.rs:e01](src-tauri/src/characterization/runtime_config_tests.rs)
   - **Çözüleceği Aşama:** **P07 (Engine Lifecycle & Process Ownership)**



---

## 2. Henüz Doğrulanmamış Davranışlar (Unverified Behavior)

Kodda var olan ancak gerçek sistem ve ağ ortamlarında geniş kapsamlı test edilmemiş özellikler:

1. **Linux NFQUEUE Sürücü Entegrasyonu**
   - **Açıklama:** Linux üzerinde `nfqws` binary çalıştırması ve `iptables/nftables` paket yönlendirmesinin gerçek Linux dağıtımlarında (Ubuntu, Fedora, Arch) davranışı.
   - **Etkilenen Platform:** Linux
   - **Kanıt Dosyası:** [router.rs:L10](src-tauri/src/network/router.rs#L10)
   - **Çözüleceği Aşama:** **P11 (Linux Platform Layer Isolation)**

2. **IPv6 Kill Switch ve Yönlendirme Paritesi**
   - **Açıklama:** Dual-stack ağlarda IPv6 paketlerinin WinDivert ve yerel DNS ile etkileşimi.
   - **Etkilenen Platform:** Windows / Linux
   - **Kanıt Dosyası:** [forwarder.rs:L150](src-tauri/src/dns/forwarder.rs#L150)
   - **Çözüleceği Aşama:** **P10 (DNS & Kill Switch Hardening)**

3. **Otomatik Sürücü Temizleme (WinDivert Unload)**
   - **Açıklama:** Aniden güç kesintisi veya BSOD durumunda WinDivert64.sys sürücüsünün bir sonraki açılışta recovery davranışı.
   - **Etkilenen Platform:** Windows
   - **Kanıt Dosyası:** [manager.rs:L380](src-tauri/src/engine/manager.rs#L380)
   - **Çözüleceği Aşama:** **P07 (Process Lifecycle & Ownership)**

---

## 3. Dokümante Edilip Uygulanmamış Özellikler (Documented But Not Implemented)

README veya UI arayüzünde gösterilen ancak backend kodunda işlevsel karşılığı bulunmayan özellikler:

1. **DNS-over-QUIC (DoQ) Desteği**
   - **Açıklama:** UI'da ve belgelerde DoQ desteği vadedilmekte, ancak arka planda DoH (HTTPS) protokolüne çevrilmektedir.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [DnsView.tsx:L120](src/views/DnsView.tsx#L120), [manager.rs:L90](src-tauri/src/dns/manager.rs#L90), [bugReproducers.test.ts:BR-03](src/test/bugReproducers.test.ts)
   - **Çözüleceği Aşama:** **P10 (DNS Hardening)**

2. **Advanced Config `wssize`, `mss` ve `customPayload` Parametreleri**
   - **Açıklama:** Arayüzde yer alan ve `advanced.ts` tipinde tanımlanan bu alanlar Rust CLI builder tarafından `winws` komut satırı argümanına dönüştürülmemektedir.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [advanced.ts:L10](src/types/advanced.ts#L10), [manager.rs:L800](src-tauri/src/engine/manager.rs#L800)
   - **Çözüleceği Aşama:** **P09 (Advanced Configuration Hardening)**

3. **SOCKS5 Proxy ile Tüm Trafiği Tünelleme**
   - **Açıklama:** Belgelere göre proxy tüm trafiği kapsıyor görünse de kodda sadece DoH sorguları SOCKS5 üzerinden geçmektedir.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [forwarder.rs:L80](src-tauri/src/dns/forwarder.rs#L80)
   - **Çözüleceği Aşama:** **P10 (DNS Hardening)**

4. **Bundled Binary Checksum Doğrulaması (Release Pipeline)**
   - **Açıklama:** Güvenlik dokümanında binary imzaları vurgulanmakla birlikte CI release workflow'unda derleme öncesi SHA-256 denetimi bulunmamaktadır.
   - **Etkilenen Platform:** Cross-platform
   - **Kanıt Dosyası:** [releases.yml:L65](.github/workflows/releases.yml#L65)
   - **Çözüleceği Aşama:** **P13 (Security & Supply Chain)**
