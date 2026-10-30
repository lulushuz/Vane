<p align="center">
  <img src="src-tauri/icons/icon.png" width="160" alt="Vane Logo">
</p>

<h1 align="center">Vane DPI</h1>

<p align="center">
  <strong>Gelişmiş DPI Engel Aşma ve Şifreli DNS Kontrol Merkezi — Zapret Motoru ile Güçlendirilmiştir</strong>
</p>

<p align="center">
  <a href="README.md"><img src="https://img.shields.io/badge/English-EN-blue.svg" alt="en"></a>
  <a href="README.ru.md"><img src="https://img.shields.io/badge/Русский-RU-blue.svg" alt="ru"></a>
  <a href="README.zh.md"><img src="https://img.shields.io/badge/简体中文-ZH-yellow.svg" alt="zh"></a>
  <a href="README.fa.md"><img src="https://img.shields.io/badge/فارسی-FA-green.svg" alt="fa"></a>
  <img src="https://img.shields.io/github/actions/workflow/status/lulushuz/Vane/releases.yml?style=flat-square&label=derleme" alt="Derleme Durumu">
  <img src="https://img.shields.io/github/license/lulushuz/Vane?style=flat-square&color=blue" alt="Lisans">
  <img src="https://img.shields.io/github/v/release/lulushuz/Vane?style=flat-square&color=blue&label=sürüm" alt="Sürüm">
  <img src="https://img.shields.io/discord/luppux?style=flat-square&logo=discord&color=5865F2" alt="Discord">
</p>

<p align="center">
  <img src="docs/assets/preview.png" width="720" alt="Vane Masaüstü Arayüzü Önizlemesi">
</p>

---

## İçindekiler

- [1. Vane Nedir?](#1-vane-nedir)
  - [Vane Neleri Otomatikleştirir?](#vane-neleri-otomatikleştirir)
- [2. Temel Sistem Özellikleri](#2-temel-sistem-özellikleri)
  - [2.1. DNS Guard: Yerel Şifreli DNS Çözücü](#21-dns-guard-yerel-şifreli-dns-çözücü)
  - [2.2. AdBlock DNS Filtreleme](#22-adblock-dns-filtreleme)
  - [2.3. Çift Yığın (Dual-Stack) DNS Kill Switch](#23-çift-yığın-dual-stack-dns-kill-switch)
  - [2.4. Otomatik Kurtarma Watchdog Gözlemcisi](#24-otomatik-kurtarma-watchdog-gözlemcisi)
  - [2.5. Preset Optimize Edici (Optimizer)](#25-preset-optimize-edici-optimizer)
  - [2.6. SOCKS5 Upstream Proxy](#26-socks5-upstream-proxy)
  - [2.7. Uzaktan Preset Senkronizasyonu](#27-uzaktan-preset-senkronizasyonu)
  - [2.8. Etiketli Günlük Konsolu](#28-etiketli-günlük-konsolu)
  - [2.9. Platforma Duyarlı Otomatik Başlatma](#29-platforma-duyarlı-otomatik-başlatma)
  - [2.10. Ağ Değişikliği Algılama](#210-ağ-değişikliği-algılama)
- [3. Güvenlik ve Sistem Mimarisi](#3-güvenlik-ve-sistem-mimarisi)
  - [3.1. Tedarik Zinciri ve TOCTOU İkili Doğrulaması](#31-tedarik-zinciri-ve-toctou-i̇kili-doğrulaması)
  - [3.2. Yetki İzolasyonu ve Korumalı Alan Bütünlüğü](#32-yetki-i̇zolasyonu-ve-korumalı-alan-bütünlüğü)
  - [3.3. IPC ve WebView Sertleştirme](#33-ipc-ve-webview-sertleştirme)
  - [3.4. Gizlilik Öncelikli Tanılama Paketi](#34-gizlilik-öncelikli-tanılama-paketi)
- [4. Yerleşik Presetler ve Hızlı Engel Aşma Rehberi](#4-yerleşik-presetler-ve-hızlı-engel-aşma-rehberi)
- [5. Linux ve Windows Güvenlik Duvarı Mimarisi](#5-linux-ve-windows-güvenlik-duvarı-mimarisi)
  - [5.1. Windows: WinDivert Hattı](#51-windows-windivert-hattı)
  - [5.2. Linux: Birincil nftables Arka Ucu](#52-linux-birincil-nftables-arka-ucu)
  - [5.3. Linux: iptables Yedek Yaşam Döngüsü](#53-linux-iptables-yedek-yaşam-döngüsü)
  - [5.4. Yetim Kalan Filtre Kurtarma (Orphan Filter Recovery)](#54-yetim-kalan-filtre-kurtarma-orphan-filter-recovery)
- [6. Kurulum ve Kaynak Koddan Derleme](#6-kurulum-ve-kaynak-koddan-derleme)
  - [6.1. Sistem Gereksinimleri](#61-sistem-gereksinimleri)
  - [6.2. Kurulum Paketleri](#62-kurulum-paketleri)
  - [6.3. Derleme Adımları](#63-derleme-adımları)
  - [6.4. Doğrulama Kapıları (Testler)](#64-doğrulama-kapıları-testler)
- [7. Sorun Giderme, Sınırlamalar, Lisans ve Emeği Geçenler](#7-sorun-giderme-sınırlamalar-lisans-ve-emeği-geçenler)
  - [7.1. Sorun Giderme](#71-sorun-giderme)
  - [7.2. Teknik Sınırlamalar](#72-teknik-sınırlamalar)
  - [7.3. Emeği Geçenler ve Lisans](#73-emeği-geçenler-ve-lisans)
  - [7.4. Topluluk ve Destek](#74-topluluk-ve-destek)

---

## 1. Vane Nedir?

Vane, açık kaynaklı [zapret](https://github.com/bol-van/zapret) paket denetimi atlatma motorunu yöneten modern ve grafiksel bir masaüstü uygulamasıdır. Düşük seviyeli `winws` (Windows) ve `nfqws` (Linux) arka plan süreçlerini; Tauri v2, Rust, React 19 ve Zustand üzerine kurulu hafif bir kontrol paneline dönüştürür.

**Vane bir VPN değildir.** Trafiğinizi uzak bir sunucuya veya ticari proxy havuzuna yönlendirmez. Bunun yerine doğrudan yerel bilgisayarınızda çalışarak, giden bağlantıların el sıkışma paketlerini çekirdek seviyesinde mikro manipülasyonlara tabi tutar. Bu işlem, servis sağlayıcıların (İSS) Derin Paket İnceleme (DPI) donanımlarını yanıltır; bant genişliğinizde veya gecikme (ping) sürenizde kayıp yaşamadan engelli servislere erişmenizi sağlar.

### Vane Neleri Otomatikleştirir?

| Görev | Manuel Zapret Kurulumu | Vane ile |
|-------|------------------------|----------|
| **İkili Bütünlüğü** | Kontrolsüz yerel dosyalar | Minisign imzası + SHA-256 + çalıştırma öncesi TOCTOU doğrulaması |
| **Motor Başlatma** | 20+ karmaşık CLI bayrağı | Tek tıkla görsel başlatma ve durum takibi |
| **DNS Şifreleme** | Karmaşık yerel stub ayarları | Dahili DoH / DoT şifreli DNS ve işlemsel geri alma (rollback) |
| **DNS Sızıntı Koruması**| Harici araçlar veya kontrolsüz sızıntı | Çift Yığın (IPv4 + IPv6) Kill Switch |
| **Güvenlik Duvarı** | Manuel sürücü/iptables komutları | Otomatik nftables / iptables / WinDivert kural yönetimi |
| **Çökme & Filtre Temizliği** | Kalan kurallar ve kopan internet | Windows Job Object + Linux Yetim Filtre Kurtarma (`linux-engine-filter.json`) |
| **Preset Yönetimi** | Metin dosyalarını elle düzenleme | Görsel düzenleyici, `.vane` paketi içe/dışa aktarma, imzalı uzak senkronizasyon |
| **Ağ Tanılaması** | Manuel curl, ping ve traceroute | Tek tıkla bağlantı sağlığı testi ve sansürlenmiş hata raporu dışa aktarma |

---

## 2. Temel Sistem Özellikleri

### 2.1. DNS Guard: Yerel Şifreli DNS Çözücü

DNS Guard, `127.0.0.127:5353` adresinde çalışan gömülü bir yerel DNS çözücüdür. İşletim sistemi ve tarayıcılar tarafından gönderilen standart açık metinli UDP/53 DNS sorgularını yakalayarak şifrelenmiş kanallara aktarır.

| Sağlayıcı | Protokol | Uç Nokta | Durum |
|-----------|----------|----------|-------|
| **Cloudflare** | DoH / DoT | `https://cloudflare-dns.com/dns-query` | ✅ Destekleniyor |
| **Google** | DoH / DoT | `https://dns.google/dns-query` | ✅ Destekleniyor |
| **AdGuard** | DoH | `https://dns.adguard.com/dns-query` | ⏳ Planlandı |
| **Özel (Custom)** | DoH / DoT | Kullanıcı tanımlı URL veya IP | ⏳ Planlandı |

**Protokol ve Yetenek Özeti:**

| Özellik | Durum |
|---------|-------|
| DNS-over-HTTPS (DoH) | ✅ Destekleniyor |
| DNS-over-TLS (DoT) | ✅ Destekleniyor |
| DNS-over-QUIC | ❌ Desteklenmiyor (DoH veya DoT kullanın) |
| Bellek içi DNS önbelleği | ✅ Destekleniyor (TTL uyumlu) |
| Yerel alan adı yedek desteği (`.local`, `.lan`) | ✅ Destekleniyor |
| Eş zamanlılık sınırı | 100 paralel asenkron işlem |
| DoH için SOCKS5 proxy desteği | ✅ Destekleniyor |
| Hata anında işlemsel geri alma | ✅ Destekleniyor (`DnsTransactionManager`) |

> [!NOTE]
> Kanonik preset dosyaları `.vane` uzantısını kullanır. Eski `.json` dosyaları da geriye dönük uyumluluk kapsamında içe aktarılabilir.

### 2.2. AdBlock DNS Filtreleme

DNS Guard, 100.000'den fazla reklam, telemetri ve zararlı yazılım alan adını içeren [StevenBlack hosts listesini](https://github.com/StevenBlack/hosts) entegre eder. Engellenen bir alan adı sorgulandığında, ağa istek çıkarılmadan doğrudan `0.0.0.0` (NXDOMAIN eşdeğeri) yanıtı verilir.

### 2.3. Çift Yığın (Dual-Stack) DNS Kill Switch

Klasik DNS şifreleme yöntemlerinde, sistemdeki bazı uygulamalar doğrudan harici bir DNS sunucusunun açık UDP 53 portuna sorgu göndererek sızıntıya neden olabilir. Vane Çift Yığın Kill Switch ile bu sızıntıları engeller:
- **Windows:** `netsh advfirewall` üzerinden tüm IPv4 (`0.0.0.0-127.0.0.0,127.0.0.2-255.255.255.255`) ve IPv6 (`::-::0,::2-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff`) açık DNS trafiğini engeller; yerel döngü trafiğine (`127.0.0.1` ve `::1`) dokunmaz.
- **Linux:** Yetkili çalıştırıcı üzerinden `OUTPUT` zincirine eklenen `iptables` ve `ip6tables` kurallarıyla yerel döngü haricindeki tüm düz metin 53 portu isteklerini sessizce düşürür.

### 2.4. Otomatik Kurtarma Watchdog Gözlemcisi

Watchdog arka plan süreci, yapılandırılan hedef alan adlarına (varsayılan: `discord.com`) yönelik bağlantı sağlığını sürekli test eder. Motor çöktüğünde veya mevcut parametreler bağlantı sağlayamaz hale geldiğinde motoru yeniden başlatır veya optimize ediciyi tetikler.

### 2.5. Preset Optimize Edici (Optimizer)

Optimize edici, kütüphanenizdeki tüm presetleri gerçek dünya HTTP hedeflerine karşı otomatik olarak test eder. En hızlı ve kesintisiz yanıt veren preseti belirleyerek tek tıkla o yapılandırmaya geçmenizi sağlar.

### 2.6. SOCKS5 Upstream Proxy

DNS-over-HTTPS uç noktalarının dahi engellendiği sıkı sansürlü ağlarda, DNS Guard'ın şifreli sorguları kimlik doğrulamalı bir SOCKS5 proxy üzerinden tünellenebilir.

### 2.7. Uzaktan Preset Senkronizasyonu

Güvenilir preset tanımlarını uzak depolardan (GitHub Gist veya CDN) otomatik olarak çekebilirsiniz:
- Açılışta yerel önbellekten sıfır ağ gecikmesi ile yüklenir.
- Güncellemeler arka planda Minisign kriptografik imzalarıyla doğrulanır.
- Uzak presetler yerleşik varsayılan presetlerin üzerine yazamaz (çakışma koruması).

### 2.8. Etiketli Günlük Konsolu

Vane, süreç çıktısını gerçek zamanlı olarak kategorize eder:
- `[MOTOR]`: Zapret paket motoru olayları.
- `[DNS]`: DNS Guard sorgu ve önbellek işlemleri.
- `[ADBLOCK]`: Reklam ve izleyici engelleme bildirimleri.
- `[GÜVENLİK]`: Yetki kontrolleri, sanitizasyon ve bütünlük doğrulamaları.
- `[SİSTEM]`: Otomatik başlatma ve ağ arabirimi geçişleri.
- `[HATA]`: Kurtarılamayan hatalar ve kritik uyarılar.

### 2.9. Platforma Duyarlı Otomatik Başlatma

Vane, otomatik başlatma mantığını işletim sistemine göre uyarlar:
- **Windows:** En yüksek ayrıcalık seviyesinde (`/rl highest`) Görev Zamanlayıcı görevi kaydeder; böylece her açılışta WinDivert sürücüsü için UAC onayı sormaz.
- **Linux:** `systemd --user` servisi (`vane.service`) olarak root gerektirmeksizin kullanıcı oturumuna entegre olur.

### 2.10. Ağ Değişikliği Algılama

Vane, ağ arabirimi değişikliklerini anlık olarak dinler (Windows'ta `WM_DEVICECHANGE`, Linux'ta netlink yönlendirme soketi). Wi-Fi'dan kablolu ağa geçildiğinde veya yeni bir adaptör takıldığında filtreler kesintisiz biçimde yeni arayüze uygulanır.

---

## 3. Güvenlik ve Sistem Mimarisi

```
+--------------------------------------------------------------------------+
|                             Vane Mimarisi                                |
+--------------------------------------------------------------------------+
|  [ Web UI: React 19 + Zustand ]                                          |
|         |                                                                |
|  [ Tauri IPC Köprüsü ] -- (Sıkı CSP, Yol & SSRF Beyaz Listesi)           |
|         |                                                                |
|  [ Rust Çekirdeği: EngineManager, DnsTransactionManager, Diagnostics ]   |
|         |                                                                |
|   +-----+--------------------------------+                               |
|   | (Windows)                            | (Linux)                       |
|   v                                      v                               |
| [ WinDivert64.sys Sürücüsü ]       [ nftables / iptables Motoru ]        |
| [ winws.exe Süreci ]               [ nfqws Süreci (İzole setcap) ]       |
| [ Windows Job Object (Otomatik Son)] [ Yetim Filtre Kurtarma Takibi ]    |
+--------------------------------------------------------------------------+
```

### 3.1. Tedarik Zinciri ve TOCTOU İkili Doğrulaması

Vane, derleme anında ikili dosyaların SHA-256 özetlerini bildirime (`ContentArtifactManifest`) gömer. `winws.exe`, `WinDivert64.sys` veya `nfqws` süreçleri başlatılmadan hemen önce `Sha256ArtifactIntegrityVerifier` dosyayı diskten okur, boyutunu ve özetini doğrular.

Bu kontrol doğrudan süreç çalıştırılmadan milisaniyeler önce yapıldığından, Denetim Zamanı - Kullanım Zamanı (TOCTOU) yarış durumları ve dosya değiştirme saldırıları engellenir.

### 3.2. Yetki İzolasyonu ve Korumalı Alan Bütünlüğü

- **Linux Yetki Sınırlandırması:** `pkexec setcap cap_net_admin,cap_net_raw+ep` yetkisi yalnızca izole edilmiş `nfqws` ikili dosyasına atanır. Asla ana Tauri ikilisine (`current_exe`) yetki verilmez. WebKitGTK ikililerine capability atanması `AT_SECURE=1` bayrağını etkinleştirerek bubblewrap (`bwrap`) render koruma alanının çökmesine yol açar. Vane, ana uygulamayı tamamen yetkisiz tutarak korur.
- **Parametrik Komut Çalıştırma:** Kabuk enterpolasyonu (`sh -c`) tamamen kaldırılmıştır; tüm işlemler parametre dizileriyle doğrudan `pkexec` üzerinden yürütülür.
- **Windows FFI İzolasyonu:** `unsafe` Rust kodları yalnızca 3 RAII modülüyle sınırlandırılmıştır (Job Object yönetimi `job.rs`, süreç belirteci yetki sorgusu ve servis yönetimi). Linux çekirdek mantığında hiçbir `unsafe` blok bulunmaz.

### 3.3. IPC ve WebView Sertleştirme

- **Minimum İzin Politikası:** `default.json` yapılandırmasında `shell:allow-execute`, `fs:allow-read`, `fs:allow-write` gibi tehlikeli izinler kapalıdır.
- **Dizin Atlama (Path Traversal) Koruması:** `resolve_safe_export_path`, dışa aktarma işlemlerini yalnızca izin verilen kullanıcı dizinleriyle (İndirilenler, Belgeler, Masaüstü, AppData) ve `.vane` / `.json` uzantılarıyla sınırlar.
- **SSRF Koruması:** `get_engine_health` komutu en fazla 16 hedefle (`MAX_HEALTH_TARGETS`) sınırlandırılmıştır ve `is_public_ip` ile özel ağ IP'lerine sorgu atılması önlenir.
- **Hostlist Doğrulaması:** `--hostlist` parametreleri yalnızca uygulama dizinindeki `.txt` dosyalarını kabul eder.

### 3.4. Gizlilik Öncelikli Tanılama Paketi

Tanılama paketi oluşturulurken `redaction.rs` motoru şu verileri otomatik olarak maskeler:
- Genel IPv4 ve IPv6 adresleri.
- Kullanıcı adları, ev dizini ve mutlak sistem yolları.
- SOCKS5 kimlik bilgileri, API anahtarları ve yetkilendirme başlıkları.

Oluşturulan tanılama paketleri, sembolik bağ saldırılarını önlemek amacıyla PID ve atomik sayaç içeren geçici dosyalarla (`.tmp-<pid>-<seq>`) atomik olarak yazılır (`create_new(true)`).

---

## 4. Yerleşik Presetler ve Hızlı Engel Aşma Rehberi

Vane, farklı engel türlerine göre optimize edilmiş hazır presetlerle gelir:

| Preset ID | Preset Adı | Hedef Protokoller | Temel Evasion Stratejisi | Önerilen Kullanım Alanı |
|-----------|------------|-------------------|--------------------------|-------------------------|
| `tr-1` | **TR Standard** | TCP 80, 443 | `fake,multidisorder` + `autottl` + `badseq` | Genel web siteleri ve haber portalları |
| `tr-2` | **TR Aggressive** | TCP 80, 443 | `fake,multidisorder` + `md5sig` + sabit TTL | Durumsal TCP yeniden birleştiren İSS'ler |
| `tr-3` | **TR Fragment** | TCP 80, 443 | `multisplit` + `fakedsplit` | Sıkı TLS denetimi uygulayan ağlar |
| `tr-4` | **TR Desync-HTTPS**| TCP 443 | Özel SNI kayıt bölme | Yalnızca HTTPS trafiğini denetleyen ağlar |
| `tr-5` | **TR QUIC** | UDP 443 | `fakeknown` + `udplen` | YouTube 4K akışları ve QUIC protokolü |
| `discord-voip` | **Discord & VoIP**| UDP 50000-65535 | UDP sahte paket enjeksiyonu | Discord sesli sohbet ve oyun ses trafiği |

> [!TIP]
> **Ayrıntılı Teknik Başvuru:** Paket akış şemaları, TCP segmentasyon yöntemleri, yanıltma modları (`badsum`, `badseq`, `md5sig`) ve tam parametre listesi için [DPI ve Zapret Dahili Protokol Referansı](docs/DPI_AND_ZAPRET_INTERNALS.tr.md) kılavuzunu inceleyebilirsiniz.

---

## 5. Linux ve Windows Güvenlik Duvarı Mimarisi

```
+--------------------------------------------------------------------------+
|                        Linux Paket İşleme Döngüsü                        |
+--------------------------------------------------------------------------+
| Giden Paket                                                              |
|       |                                                                  |
|       v                                                                  |
| [ nftables / iptables OUTPUT ]                                           |
|   |-- (oifname "lo" return) -------------> Yerel döngüyü atla            |
|   |-- (mark 0x40000000 return) ----------> nfqws tarafından üretileni atla|
|   +-- (tcp dport 80,443 / udp 50000-65535) -> NFQUEUE num 200 (bypass)   |
|                                                     |                    |
|                                                     v                    |
|                                            [ nfqws süreci ]              |
|                                            Desync uygula & geri enjekte et|
+--------------------------------------------------------------------------+
```

### 5.1. Windows: WinDivert Hattı

Windows platformunda Vane, imzalı `WinDivert64.sys` çekirdek sürücüsüyle çalışan `winws.exe` sürecini yönetir. Süreç ömrü `KILL_ON_JOB_CLOSE` bayrağıyla bir Windows Job Object nesnesine bağlanır. Vane kapandığında veya çöktüğünde işletim sistemi sürücü filtrelerini ve süreci otomatik olarak temizler.

### 5.2. Linux: Birincil nftables Arka Ucu

Linux çekirdeği 5.x ve üzeri sistemlerde Vane birincil olarak `nftables` kullanır:
- **Atomik Kurallar:** Kurallar işlem garantili (transactional) uygulanır.
- **Yerel Döngü Muafiyeti:** `oifname "lo" return` kuralı sayesinde yerel DNS sorguları (`127.0.0.127:5353`) ve loopback trafiği bozulmaz.
- **PortRange Korunumu:** `50000-65535` gibi geniş port aralıkları tekil porta indirgenmeden tam aralık olarak korunur.

```nftables
table inet vane_mangle {
    chain output {
        type filter hook output priority mangle; policy accept;
        oifname "lo" return
        mark 0x40000000 return
        tcp dport { 80, 443 } queue num 200 bypass
        udp dport { 443, 50000-65535 } queue num 200 bypass
    }
}
```

### 5.3. Linux: iptables Yedek Yaşam Döngüsü

`nftables` bulunmayan sistemlerde Vane `iptables` zincirlerini devreye sokar:
1. **Özel Zincir Oluşturma:** `VANE_OUTPUT` ve `VANE_INPUT` zincirleri oluşturulur.
2. **Döngü Engelleme:** `nfqws` tarafından üretilen paketlerin tekrar yakalanmasını önlemek için `mark 0x40000000/0x40000000 -j ACCEPT` kuralı eklenir.
3. **Kuyruk Yönlendirme:** Hedef portlar `NFQUEUE --queue-num 200 --queue-bypass` hedefine aktarılır.
4. **Temizleme:** Kapanışta `OUTPUT` bağlantısı kaldırılır, kurallar temizlenir ve zincir silinir.

### 5.4. Yetim Kalan Filtre Kurtarma (Orphan Filter Recovery)

Uygulama çalışırken elektrik kesintisi veya sistem çökmesi yaşanırsa:
- Aktif filtre parametreleri `linux-engine-filter.json` dosyasında tutulur.
- Açılışta dosya okunur, zincir isimleri `^[a-zA-Z0-9_-]{1,32}$` kuralına göre doğrulanır ve eski kurallar temizlenir.

---

## 6. Kurulum ve Kaynak Koddan Derleme

### 6.1. Sistem Gereksinimleri

| Platform | Minimum İşletim Sistemi | Bağımlılıklar | Yetki Seviyesi |
|----------|-------------------------|---------------|----------------|
| **Windows** | Windows 10 (1809+) veya Windows 11 | Yok (WinDivert sürücüsü gömülüdür) | Yönetici (UAC izni) |
| **Linux** | Ubuntu 22.04+, Debian 12+, Fedora 38+, Arch | `libwebkit2gtk-4.1`, `nftables` veya `iptables` | Standart kullanıcı (`pkexec` ile nfqws yetkilendirilir) |

### 6.2. Kurulum Paketleri

Resmi derlemeleri [GitHub Releases](https://github.com/lulushuz/Vane/releases) sayfasından edinebilirsiniz:
- **Windows:** `Vane_0.2.0_x64-setup.exe` veya `.msi` kurulum paketini çalıştırın.
- **Linux:** `vane_0.2.0_amd64.deb` paketini yükleyin veya `.AppImage` dosyasını çalıştırın:

```bash
# Debian / Ubuntu kurulumu
sudo dpkg -i vane_0.2.0_amd64.deb

# AppImage çalıştırma
chmod +x Vane_0.2.0_amd64.AppImage
./Vane_0.2.0_amd64.AppImage
```

### 6.3. Derleme Adımları

Kaynak koddan derlemek için:

```bash
# 1. Depoyu klonlayın
git clone https://github.com/lulushuz/Vane.git
cd Vane

# 2. Frontend bağımlılıklarını kurun
npm install

# 3. Geliştirici modunda başlatın
npm run tauri dev

# 4. Üretim sürümünü derleyin
npm run tauri build
```

### 6.4. Doğrulama Kapıları (Testler)

Kod tabanında değişiklik yaparken tüm test paketlerinin başarıyla geçtiğinden emin olun:

```bash
# Rust test paketi (372 birim ve karakterizasyon testi)
cd src-tauri
cargo test --all-targets

# Rust statik kod analizi (Clippy)
cargo clippy --all-targets -- -D warnings

# Frontend birim ve sözleşme testleri
cd ..
npm test -- --run

# TypeScript tip denetimi
npx tsc --noEmit
```

---

## 7. Sorun Giderme, Sınırlamalar, Lisans ve Emeği Geçenler

### 7.1. Sorun Giderme

| Belirti | Olası Neden | Çözüm Adımı |
|---------|-------------|-------------|
| **Windows'ta motor başlamıyor** | Başka bir WinDivert aracı açık | GoodbyeDPI veya benzeri yazılımları kapatın |
| **DNS çözümlemesi çalışmıyor** | DoH uç noktası İSS tarafından engelli | DNS ayarlarından SOCKS5 upstream proxy özelliğini açın |
| **Linux'ta başlatma sırasında korumalı alan hatası** | Ana ikiliye capability atanmış | `setcap` komutunun yalnızca `nfqws` ikilisine verildiğinden emin olun |
| **Discord sesli sohbette takılma** | UDP port aralıkları kapalı | `discord-voip` presetini seçin |
| **Gezinirken yüksek gecikme** | Pencere boyutu (`wssize`) çok düşük | `--wssize` değerini artırın veya standart presetlere dönün |
| **Linux'ta yeniden başlatma sonrası kural kalması** | Önceki oturum düzensiz kapandı | Vane açılışta eski kuralları otomatik olarak temizler |

### 7.2. Teknik Sınırlamalar

Paket manipülasyonu mantıksal ağ sınırları dahilinde çalışır:
- **IP Düzeyinde Engelleme:** İSS hedef sunucunun IP adresine giden trafiği tamamen düşürüyorsa, paket manipülasyonu bağlantıyı kurtaramaz. Bu durumda VPN veya proxy kullanılmalıdır.
- **Şeffaf TCP Proxy:** İSS trafiği sonlandırıp kendi vekili üzerinden sıfırdan kuruyorsa gönderilen sahte paketler etkisiz kalır.
- **Kök Sertifika Denetimi:** Kurumsal ağlarda istemciye yüklenen özel CA sertifikalarıyla yapılan SSL incelemeleri paket manipülasyonuyla aşılamaz.

### 7.3. Emeği Geçenler ve Lisans

- **[zapret](https://github.com/bol-van/zapret)** (bol-van): Paket manipülasyon çekirdeği (`winws` ve `nfqws`).
- **[WinDivert](https://reqrypt.org/windivert.html)** (basil00): Windows kullanıcı alanı paket yakalama sürücüsü.
- **[Tauri](https://tauri.app)**: Güvenli masaüstü uygulama çatısı.
- **[StevenBlack/hosts](https://github.com/StevenBlack/hosts)**: Reklam ve izleyici engelleme listesi.
- **[Minisign](https://jedisct1.github.io/minisign/)**: Kriptografik imza doğrulama kütüphanesi.

Bu proje **GNU General Public License v3.0** (`GPL-3.0`) ile lisanslanmıştır. Detaylar için [LICENSE](LICENSE) dosyasına bakabilirsiniz.

### 7.4. Topluluk ve Destek

- **Hata Bildirimi ve Tartışmalar:** [GitHub Issues](https://github.com/lulushuz/Vane/issues)
- **Discord Topluluğu:** [discord.gg/luppux](https://discord.gg/luppux)
- **Güvenlik Bildirimleri:** Güvenlik açıkları için [SECURITY.md](SECURITY.md) dosyasındaki prosedürü izleyiniz.
