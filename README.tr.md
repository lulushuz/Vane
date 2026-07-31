<p align="center">
  <img src="src-tauri/icons/icon.png" width="160" alt="Vane Logo">
</p>

<h1 align="center">Vane DPI</h1>

<p align="center">
  <strong>Gelişmiş DPI Engeli Aşma ve Şifreli DNS Kontrol Merkezi — Zapret ile Güçlendirilmiş</strong>
</p>

<p align="center">
  <a href="README.md"><img src="https://img.shields.io/badge/lang-en-red.svg" alt="en"></a>
  <img src="https://img.shields.io/github/actions/workflow/status/lulushuz/Vane/releases.yml?style=flat-square&label=derleme" alt="Derleme Durumu">
  <img src="https://img.shields.io/github/license/lulushuz/Vane?style=flat-square&color=blue" alt="Lisans">
  <img src="https://img.shields.io/badge/sürüm-1.0.0--rc.1-orange?style=flat-square" alt="Release Candidate">
  <img src="https://img.shields.io/discord/luppux?style=flat-square&logo=discord&color=5865F2" alt="Discord">
</p>

> **⚠️ Release Candidate — Yalnızca Test Amaçlıdır**
>
> | | |
> |---|---|
> | **Mevcut release candidate** | `1.0.0-rc.1` — imzasız, yalnızca test |
> | **Kararlı sürüm** | Henüz yayınlanmadı |
> | **Eski geliştirme hattı** | `2.x` (Git geçmişinde korunmaktadır) |
> | **Production imzalama** | Beklemede |
>
> Önceki `2.x` yapıları geliştirme aşaması anlık görüntüleriydi. Resmî production sürümlemesi,
> sertleştirilmiş P00–P15 mimarisi üzerine inşa edilen `1.0.0-rc.1` ile başlamaktadır.
> Mevcut `2.x` kullanıcıları bu sürümü **manuel olarak** kurmalıdır — otomatik güncelleyici
> SemVer düşürme işlemi sunamaz. Bkz. [version-realignment.md](docs/release/version-realignment.md).

---

## İçindekiler

- [1. Vane Nedir?](#1-vane-nedir)
- [2. Zapret Nedir?](#2-zapret-nedir)
- [3. Derin Paket İnceleme (DPI) Nasıl Çalışır?](#3-derin-paket-inceleme-dpi-nasıl-çalışır)
  - [3.1. Pasif DPI ve Aktif DPI](#31-pasif-dpi-ve-aktif-dpi)
  - [3.2. SNI ve Hostname Ayıklama Süreci](#32-sni-ve-hostname-ayıklama-süreci)
  - [3.3. Blok Enjeksiyonu — RST ve HTTP Yönlendirme](#33-blok-enjeksiyonu--rst-ve-http-yönlendirme)
  - [3.4. DNS Zehirlenmesi ve Ele Geçirme](#34-dns-zehirlenmesi-ve-ele-geçirme)
  - [3.5. Derin Parmak İzi ve Davranış Analizi](#35-derin-parmak-i̇zi-ve-davranış-analizi)
- [4. Zapret Mimarisi — nfqws / winws Çekirdeği](#4-zapret-mimarisi--nfqws--winws-çekirdeği)
  - [4.1. WinDivert (Windows)](#41-windivert-windows)
  - [4.2. NFQUEUE (Linux)](#42-nfqueue-linux)
  - [4.3. Paket İşleme Hattı](#43-paket-i̇şleme-hattı)
- [5. DPI Desync Stratejileri](#5-dpi-desync-stratejileri)
  - [5.1. Desync Faz Sistemi — Kombo Sıralaması](#51-desync-faz-sistemi--kombo-sıralaması)
  - [5.2. TCP Segmentasyon Yöntemleri](#52-tcp-segmentasyon-yöntemleri)
  - [5.3. fakedsplit ve fakeddisorder — altorder Tablosu](#53-fakedsplit-ve-fakeddisorder--altorder-tablosu)
  - [5.4. Bölme Konum İşaretçileri](#54-bölme-konum-i̇şaretçileri)
  - [5.5. Sahte Paket Enjeksiyonu](#55-sahte-paket-enjeksiyonu)
  - [5.6. Fooling (Yanıltma) Modları](#56-fooling-yanıltma-modları)
  - [5.7. Sahte Paket İçeriği Özelleştirme](#57-sahte-paket-i̇çeriği-özelleştirme)
  - [5.8. Sıra Numarası Çakıştırma (seqovl)](#58-sıra-numarası-çakıştırma-seqovl)
  - [5.9. IP ID Atama Şemaları](#59-ip-id-atama-şemaları)
  - [5.10. SYNDATA Modu](#510-syndata-modu)
  - [5.11. Orijinal Paket Değişikliği](#511-orijinal-paket-değişikliği)
  - [5.12. Yinelenen Paket Enjeksiyonu](#512-yinelenen-paket-enjeksiyonu)
  - [5.13. Sunucu Tarafı Pencere Manipülasyonu (wssize)](#513-sunucu-tarafı-pencere-manipülasyonu-wssize)
  - [5.14. UDP / QUIC Desync](#514-udp--quic-desync)
  - [5.15. IP Parçalama (Fragmentation)](#515-ip-parçalama-fragmentation)
- [6. tpws — Şeffaf TCP Proxy Modu](#6-tpws--şeffaf-tcp-proxy-modu)
- [7. Desync Parametre Referans Tablosu](#7-desync-parametre-referans-tablosu)
- [7. Parçalanmış El Sıkışma ve Kyber Desteği](#7-parçalanmış-el-sıkışma-ve-kyber-desteği)
- [8. Bağlantı Takibi (Conntrack)](#8-bağlantı-takibi-conntrack)
- [9. IP Önbelleği Yönetimi](#9-ip-önbelleği-yönetimi)
- [10. Vane Sistem Özellikleri](#10-vane-sistem-özellikleri)
  - [10.1. DNS Koruma — Yerel DoH / DoT Yönlendiricisi](#101-dns-koruma--yerel-doh--dot-yönlendiricisi)

  - [10.2. AdBlock DNS Filtresi](#102-adblock-dns-filtresi)
  - [10.3. Kill Switch — DNS Sızıntı Koruması](#103-kill-switch--dns-sızıntı-koruması)
  - [10.4. Otomatik Kurtarma Watchdog'u](#104-otomatik-kurtarma-watchdogu)
  - [10.5. Preset Optimize Edici](#105-preset-optimize-edici)
  - [10.6. SOCKS5 Üst Düzey Proxy](#106-socks5-üst-düzey-proxy)
  - [10.7. Uzak Preset Senkronizasyonu](#107-uzak-preset-senkronizasyonu)
  - [10.8. Etiketli Çıktılı Log Konsolu](#108-etiketli-çıktılı-log-konsolu)
  - [10.9. Otomatik Başlatma ve Sistem Tepsisi](#109-otomatik-başlatma-ve-sistem-tepsisi)
  - [10.10. Ağ Değişikliği Algılama](#1010-ağ-değişikliği-algılama)
- [11. Yerleşik Presetler](#11-yerleşik-presetler)
- [12. Gelişmiş Yapılandırma — Tam Parametre Tablosu](#12-gelişmiş-yapılandırma--tam-parametre-tablosu)
- [13. Pratik Engel Aşma Stratejileri](#13-pratik-engel-aşma-stratejileri)
- [14. Güvenlik Duvarı Kurulumu — Linux (Iptables / Nftables)](#14-güvenlik-duvarı-kurulumu--linux-iptables--nftables)
- [15. Güvenlik Mimarisi](#15-güvenlik-mimarisi)
- [16. Kurulum](#16-kurulum)
- [17. Kaynak Koddan Derleme](#17-kaynak-koddan-derleme)
- [18. Sorun Giderme](#18-sorun-giderme)
- [19. Sınırlamalar ve Engel Aşmanın Yetersiz Kaldığı Durumlar](#19-sınırlamalar-ve-engel-aşmanın-yetersiz-kaldığı-durumlar)
- [20. Emeği Geçenler ve Lisans](#20-emeği-geçenler-ve-lisans)
- [21. Topluluk](#21-topluluk)

---

## 1. Vane Nedir?

Vane, [zapret](https://github.com/bol-van/zapret) DPI engeli aşma motorunun modern bir masaüstü uygulaması kontrolcüsüdür. Windows'ta `winws`, Linux'ta `nfqws` süreçlerini Tauri v2 + Rust + React/TypeScript tabanlı güvenli bir arayüzle yönetir.

**Vane bir VPN değildir.** Trafiğinizi uzak bir sunucuya yönlendirmez. Bunun yerine, İSS'nin Derin Paket İnceleme sistemlerini yanıltmak amacıyla giden ve gelen ağ paketlerini çekirdek düzeyinde değiştirir. Tüm trafik kendi bağlantınız üzerinde kalır — yalnızca paket yapısı değiştirilir.

### Vane'in Otomatikleştirdiği İşlemler

| İşlem | Vane Olmadan | Vane ile |
|-------|-------------|----------|
| Motor başlatma | 20+ argümanla elle CLI | Tek tıklama |
| DNS şifreleme | Manuel DoH/DoT yapılandırması | Dahili DNS Guard |
| Güvenlik duvarı kuralları | Elle iptables/netsh kurulumu | Otomatik |

| DNS sızıntı koruması | Harici araçlar gerekli | Entegre Kill Switch |
| Preset yönetimi | Metin dosyaları | Görsel düzenleyici + uzak senkronizasyon |
| İkili dosya bütünlüğü | Doğrulanmıyor | Başlangıçta SHA-256 doğrulaması |
| Çökmede süreç temizliği | Elle | Windows Job Object / SIGTERM |

---

## 2. Zapret Nedir?

[Zapret](https://github.com/bol-van/zapret), bol-van tarafından geliştirilen açık kaynaklı bir DPI engeli aşma kütüphanesi ve daemon sürecidir. Giden (ve bazı durumlarda gelen) TCP/UDP paketlerini yakalayarak yapılandırılabilir manipülasyonlar uygular ve yeniden enjekte eder.

### Temel Daemon'lar

| Daemon | Platform | Mekanizma | Notlar |
|--------|----------|-----------|--------|
| `winws` | Windows | WinDivert çekirdek sürücüsü | Yönetici yetkisi gerektirir |
| `nfqws` | Linux | NFQUEUE netfilter hedefi | iptables/nftables kuralları gerektirir |
| `tpws` | Linux | SOCKS5 şeffaf proxy | Çekirdek modülü gerekmez |

### Desteklenen Protokoller

| Protokol | Port | Taşıma | Aşma Yöntemleri |
|----------|------|--------|-----------------|
| HTTP | 80 | TCP | Bölme, bozuk düzen, sahte |
| HTTPS (TLS 1.2/1.3) | 443 | TCP | Bölme, sahte, seqovl, wssize |
| QUIC (HTTP/3) | 443 | UDP | Sahte, udplen, fakeknown |
| VoIP / Discord RTP | 50000-65535 | UDP | Sahte, udplen |
| DoH (HTTPS üzerinden DNS) | 443 | TCP | DNS Guard tarafından yönetilir |

---

## 3. Derin Paket İnceleme (DPI) Nasıl Çalışır?

Etkili engel aşma stratejileri geliştirmek için İSS'lerin kullandığı denetim sistemlerinin mimarisini anlamak kritik önem taşır.

### 3.1. Pasif DPI ve Aktif DPI

| Özellik | Pasif DPI | Aktif DPI |
|---------|-----------|-----------|
| Yerleşim | Ayna portu / optik tap | Veri yolunun içinde (inline) |
| Paket düşürebilir mi | ❌ Hayır | ✅ Evet |
| Paket geciktirebilir mi | ❌ Hayır | ✅ Evet |
| Bloklama yöntemi | RST enjeksiyonu / HTTP yönlendirme | Düşürme, TCP sıfırlama, proxy kesişmesi |
| Aşma zorluğu | Düşük — Orta | Yüksek |

**Pasif DPI**, trafiğin bir kopyasını alır. Gerçek sunucu yanıtından önce müşteriye sahte bir TCP RST veya HTTP 302 enjekte etmeye çalışır. Sahte paket önce ulaşırsa bağlantı kopar.

**Aktif DPI** doğrudan veri yolunun üzerinde konumlanır. TCP akışlarını yeniden oluşturabilir, yeniden birleştirilmiş yükler üzerinde regex eşleştirmesi uygulayabilir ve bağlantıları tamamlanmadan engelleyebilir.

### 3.2. SNI ve Hostname Ayıklama Süreci

```
İstemci → [TCP SYN] → [TCP SYN-ACK] → [TLS ClientHello]
                                              ↑
                              DPI burada SNI uzantısını okur
                              "server_name: engellendi.example.com"
```

| Bağlantı Tipi | İncelenen Başlık | Paketteki Konumu |
|--------------|------------------|------------------|
| HTTP/1.1 | `Host:` başlığı | Düz metin, TCP yükü |
| HTTPS (TLS) | `server_name` SNI uzantısı | ClientHello, şifreleme öncesi düz metin |
| HTTP/2 TLS üzerinde | ClientHello'daki SNI | HTTPS ile aynı |
| QUIC (HTTP/3) | QUIC CRYPTO SNI | İlk QUIC CRYPTO çerçevesi |

### 3.3. Blok Enjeksiyonu — RST ve HTTP Yönlendirme

DPI sensörü yasaklı bir alan adı tespit ettiğinde:

1. Sunucunun kaynak IP ve portunuyla **sahte TCP RST** paketi üretir (pasif DPI).
2. HTTP bağlantıları için İSS uyarı sayfasını gösteren **sahte HTTP 302 yönlendirme** paketi enjekte edebilir.
3. Yarış koşulu: enjekte edilen RST/yönlendirme gerçek sunucu yanıtından **önce** istemciye ulaşırsa işletim sistemi soketi kapatır.
4. Aktif DPI'lar gerçek paketi tamamen düşürür ve kendi RST'lerini gönderir — yarış koşuluna gerek yoktur.

### 3.4. DNS Zehirlenmesi ve Ele Geçirme

TCP bağlantısı kurulmadan önce alan adının çözümlenmesi gerekir. İSS'ler DNS'i iki yaygın yolla ele geçirir:

| Yöntem | Mekanizma | Sonuç |
|--------|-----------|-------|
| DNS Ele Geçirme | UDP/53'ü keser, sahte IP döndürür | İstemci engel portalına bağlanır |
| DNS Zehirlenmesi | Yarış koşulunda yanlış DNS yanıtı enjekte eder | Ele geçirmeyle aynı |
| DNS Engelleme | DNS isteğini tamamen düşürür | Bağlantı zaman aşımına uğrar |
| Şeffaf DNS Proxy | Tüm DNS'i İSS çözümleyicisinden geçirir | İSS sansürlü yanıtlar döndürür |

**Vane'in DNS Guard'ı**, tüm DNS sorgularını şifreli DoH/DoT tünelleri üzerinden yönlendirerek bunların tamamını çözer.


### 3.5. Derin Parmak İzi ve Davranış Analizi

Gelişmiş DPI sistemleri SNI okumayı davranışsal parmak iziyle destekler:

| Teknik | Açıklama | Karşı Tedbir |
|--------|----------|--------------|
| JA3/JA3S parmak izi | TLS ClientHello parametrelerinin hash'i | TLS uzantı sıralamasını değiştirme |
| TCP parmak izi | TCP seçenek sıralamasıyla OS tespiti | `--dpi-desync-ttl` + bölme |
| Akış uzunluk analizi | Paket boyutuyla VPN/proxy tespiti | `--udplen`, yük doldurma |
| Zamanlama korelasyonu | Şifreli akışları bilinen kalıplarla eşleştirme | Bozuk düzen moduyla kaos enjeksiyonu |

---

## 4. Zapret Mimarisi — nfqws / winws Çekirdeği

### 4.1. WinDivert (Windows)

Windows'ta `winws`, [WinDivert](https://reqrypt.org/windivert.html) çekirdek sürücüsünü kullanır:

```
Uygulama → TCP Yığını → WinDivert (çekirdek sürücüsü paketi yakalar)
                                   ↓
                         winws.exe (kullanıcı alanı işleme)
                                   ↓
                         WinDivert yeniden enjekte → Router → İnternet
```

WinDivert, hangi paketlerin ele geçirileceğini belirten bir filtre ifadesiyle Windows çekirdek sürücüsü (`WinDivert64.sys`) olarak kurulur.

### 4.2. NFQUEUE (Linux)

Linux'ta `nfqws`, çekirdeğin NFQUEUE netfilter hedefini kullanır:

```
Uygulama → TCP Yığını → iptables NFQUEUE kuralı → NFQUEUE (çekirdek)
                                                         ↓
                                             nfqws (kullanıcı alanı işleme)
                                                         ↓
                                         NF_ACCEPT / NF_DROP → Router → İnternet
```

### 4.3. Paket İşleme Hattı

```
Gelen Paket
      │
      ├─ TCP mi? ──→ TCP bayraklarını, sıra numaralarını ayrıştır
      │                       │
      │                       ├─ SYN mi? ──→ SYNDATA modu
      │                       │
      │                       └─ Uygulama verisi mi? ──→ Protokol ayrıştır
      │                               │
      │                               ├─ HTTP → Host: başlığını bul
      │                               ├─ TLS  → ClientHello + SNI bul
      │                               └─ QUIC → CRYPTO SNI bul
      │
      ├─ UDP mi? ──→ QUIC / VoIP işleyicisi
      │
      └─ Desync stratejisi uygula → Değiştirilmiş paketleri yeniden enjekte et
```

---

## 5. DPI Desync Stratejileri

### 5.1. Desync Faz Sistemi — Kombo Sıralaması

`--dpi-desync` parametresi **virgülle ayrılmış en fazla 3 mod** alır. Her mod belirli bir faza aittir ve **artan faz sırasında** yazılmalıdır.

| Faz | Ne Zaman Çalışır | Kullanılabilir Modlar |
|-----|------------------|-----------------------|
| **Faz 0** | TCP bağlantısı kurulurken (SYN/SYN-ACK) | `synack`, `syndata`, `--wssize` |
| **Faz 1** | Orijinal veriden önce — önce sahteler gönderilir | `fake`, `fakeknown`, `rst`, `rstack`, `hopbyhop`, `destopt`, `ipfrag1` |
| **Faz 2** | Orijinal veri değiştirilmiş biçimde gönderilir | `multisplit`, `multidisorder`, `fakedsplit`, `fakeddisorder`, `hostfakesplit`, `ipfrag2`, `udplen`, `tamper` |

**Kurallar:**
- **Faz 1 + Faz 2** kombinasyonu yapılabilir: `fake,multidisorder`
- Aynı fazdaki iki mod birleştirilemez: ❌ `fake,fakeknown`
- Faz 0 modları (`synack`, `syndata`) Faz 1 + Faz 2 öncesine eklenebilir: `syndata,fake,multisplit`

**Örnek kombolar:**
```
fake,multidisorder          → Faz 1 + Faz 2 (en yaygın)
fake,multisplit             → Faz 1 + Faz 2
syndata,fake,fakedsplit     → Faz 0 + Faz 1 + Faz 2
```

> ⚠️ Modları yanlış sırayla yazarsanız (örn. `multidisorder,fake`) zapret yapılandırmayı reddeder.

---

### 5.2. TCP Segmentasyon Yöntemleri

TCP desync, DPI donanımının kaynak kısıtlamalarını kullanır. Bir TCP akışı olağandışı parçalara bölündüğünde veya yeniden sıralandığında, DPI'nın yeniden birleştirme arabelleği bağlantı kurulmadan önce SNI'yi işleyemeyebilir.

| Yöntem | Faz | Açıklama | Uyumluluk |
|--------|-----|----------|-----------|
| `split` | 2 | TCP yükünü tek konumda böler | Çok Yüksek |
| `split2` | 2 | İki konumda böler | Yüksek |
| `disorder` | 2 | Böler ve segmentleri ters sırayla gönderir | Yüksek |
| `disorder2` | 2 | İki konumda bozuk düzen | Orta-Yüksek |
| `fakedsplit` | 2 | Bölme + etrafında sahte paketler | Yüksek |
| `fakeddisorder` | 2 | Ters sıra bölme + sahte paketler | Orta-Yüksek |
| `multisplit` | 2 | Listede belirtilen N konumda bölme | Yüksek |
| `multidisorder` | 2 | Çok konumlu bozuk düzen | Yüksek |
| `hostfakesplit` | 2 | Hostname alanı etrafında sahte hostlu bölme | Orta |
| `fake` | 1 | Gerçekten önce sahte TLS/HTTP/QUIC paketi gönder | Yüksek |
| `fakeknown` | 1 | Protokol-bilinçli sahte paket (QUIC Initial, WireGuard…) | Yüksek |
| `rst` | 1 | Gerçekten önce sahte RST paketi gönder | Orta |
| `rstack` | 1 | Gerçekten önce sahte RST+ACK paketi gönder | Orta |
| `syndata` | 0 | SYN paketinin içine veri yükü ekle | Orta |
| `synack` | 0 | TCP split handshake manipülasyonu | Düşük |
| `ipfrag1` | 1 | Sahte paketlere IPv6 Fragment başlığı ekle (yalnızca IPv6) | Orta |
| `ipfrag2` | 2 | Orijinal paketleri IP katmanında parçala | Orta |

**Bozuk düzen (disorder) nasıl çalışır:**

```
Orijinal:    [Seg1: 1-10. baytlar][Seg2: 11-20. baytlar][Seg3: 21-30. baytlar]
             İçeriği: "example.com" (engellenen SNI)

Bozuk düzen: [Seg2][Seg3][Seg1]
Sunucu:      Seg2, Seg3'ü arabellekte tutar, Seg1'i alınca doğru sırayla birleştirir
DPI:         Yeniden birleştiremez → SNI'yi göz ardı eder
```

---

### 5.3. fakedsplit ve fakeddisorder — altorder Tablosu

`fakedsplit` ve `fakeddisorder`, gerçek TCP segmentlerinin etrafına sahte yem paketleri yerleştirir. Sahtelerin ve gerçek segmentlerin **tam gönderim sırası**, `altorder=N` değiştiriciyle kontrol edilir:

```
--dpi-desync-fakedsplit-mod=altorder=N
```

#### fakedsplit — Çok paketli mod (bölme konumu tanımlıyken)

| altorder | Gönderilen paket sırası |
|----------|--------------------------|
| `0` (varsayılan) | `sahte[1.]` → `gerçek[1.]` → `sahte[1.]` → `sahte[2.]` → `gerçek[2.]` → `sahte[2.]` |
| `1` | `gerçek[1.]` → `sahte[1.]` → `sahte[2.]` → `gerçek[2.]` → `sahte[2.]` |
| `2` | `gerçek[1.]` → `sahte[2.]` → `gerçek[2.]` → `sahte[2.]` |
| `3` | `gerçek[1.]` → `sahte[2.]` → `gerçek[2.]` |

#### fakeddisorder — Çok paketli mod (ters sıra + sahteler)

| altorder | Gönderilen paket sırası |
|----------|--------------------------|
| `0` (varsayılan) | `sahte[2.]` → `gerçek[2.]` → `sahte[2.]` → `sahte[1.]` → `gerçek[1.]` → `sahte[1.]` |
| `1` | `gerçek[2.]` → `sahte[2.]` → `sahte[1.]` → `gerçek[1.]` → `sahte[1.]` |
| `2` | `gerçek[2.]` → `sahte[1.]` → `gerçek[1.]` → `sahte[1.]` |
| `3` | `gerçek[2.]` → `sahte[1.]` → `gerçek[1.]` |

#### Tek paketli mod (bölme konumu tanımsız)

| altorder | Sıra |
|----------|------|
| `0` | `sahte` → `gerçek` → `sahte` |
| `8` | `gerçek` → `sahte` |
| `16` | Yalnızca `gerçek` (sahteler devre dışı) |

Değerler toplanabilir — çok paketli `altorder=N` + tek paketli `0`, `8` veya `16`. Çoğu ISP için `altorder=0` (varsayılan) veya `altorder=1` yeterlidir.

> 💡 **Örnek özel preset:** `--dpi-desync=fake,fakedsplit --dpi-desync-fakedsplit-mod=altorder=1 --dpi-desync-fooling=badseq`

---

### 5.4. Bölme Konum İşaretçileri

| İşaretçi | Çözümlediği Konum | Geçerli Protokol |
|----------|-------------------|------------------|
| `method` | HTTP metodunun başlangıcı (GET, POST…) | HTTP |
| `host` | Hostname alanının başlangıcı | HTTP, TLS |
| `endhost` | Hostname'in son karakterinden sonraki bayt | HTTP, TLS |
| `sld` | İkinci düzey etki alanının başlangıcı | HTTP, TLS |
| `endsld` | SLD'nin sonundan sonraki bayt | HTTP, TLS |
| `midsld` | İkinci düzey etki alanının ortası | HTTP, TLS |
| `sniext` | TLS SNI uzantı veri alanı | Yalnızca TLS |
| `0`, `1`, `N` | Yük başlangıcından mutlak bayt ofseti | Herhangi |
| `method+N` | Metod işaretçisi + N ofseti | HTTP |
| `host-N` | Host işaretçisi − N ofseti | HTTP, TLS |

**Örnek:**
```
--dpi-desync-split-pos=method+2,midsld
```
HTTP istekleri için `method+2`, TLS bağlantıları için `midsld` konumunda böler.

### 5.3. Sahte Paket Enjeksiyonu

Sahte paket enjeksiyonu, DPI'nın bloklama mantığını tetiklemek için tasarlanmış bir yem paketi gönderir. DPI sahte yükü işledikten sonra oturumun zaten ele alındığını varsayar ve takibi durdurur. Gerçek paket daha sonra gönderilir.

```
Zaman çizelgesi:
  T=0ms  → İstemci sahte paket gönderir (yasaklı SNI içerir)
             DPI: "Bu blocked.example.com, bloklama kuralını etkinleştir"
  T=1ms  → İstemci gerçek parçalanmış paketleri gönderir (tam SNI görünmez)
             DPI: "Bu oturumu zaten işledim, yoksayıyorum"
  T=2ms  → Sunucu gerçek paketleri birleştirir, bağlantı başarılı
```

**Kritik gereksinim**: Sahte paketin gerçek sunucuya ulaşmaması gerekir (aksi halde sunucu bağlantıyı keser). Bu **fooling modları** ile sağlanır.

### 5.6. Fooling (Yanıltma) Modları

| Mod | Mekanizma | Güvenilirlik | Notlar |
|-----|-----------|--------------|--------|
| `ttl` | TTL'yi hedefe ulaşmadan dolacak kadar düşük ayarla | Yüksek | Atlama sayısı gerektirir; TTL yeniden yazan routerlarla çalışmaz |
| `autottl` | Sunucunun gelen TTL'ini ölçer, atlama mesafesini otomatik hesaplar | Çok Yüksek | En iyi varsayılan seçim |

### 5.5. Sahte Paket İçeriği Özelleştirme

| Seçenek | Etkisi |
|---------|--------|
| `rnd` | Her istekte TLS Rastgele ve Oturum ID alanlarını rastgele oluşturur |
| `rndsni` | SNI uzantısını rastgele SLD + TLD ile değiştirir |
| `dupsid` | Gerçek ClientHello'nun Oturum ID'sini sahte pakete kopyalar |
| `sni=domain` | Sahte paketteki SNI'yi `domain` ile değiştirir |
| `padencap` | Sahte doldurma uzantısını gerçek paketin boyutuna göre genişletir |
| `oob` | Sahte pakette bant dışı veri (TCP Urgent bayrağı) gönderir |

### 5.6. Sıra Numarası Çakıştırma (seqovl)

`seqovl` tekniği, durumsal DPI motorlarını yanıltmak için kasıtlı olarak örtüşen TCP sıra aralıkları oluşturur.

```
Sahte segment:  [seq=100, len=10] → yük: çöp_veri
Gerçek segment: [seq=105, len=10] → yük: gerçek_sni_verisi

DPI görür: çöp_veri (ilk alınan, standart olarak kaydedilir)
Sunucu:    OS uygulamasına bağlı olarak ikinci alınan veya sonraki verilere öncelik verebilir
```

> ⚠️ Windows tabanlı sunucular çakışmaları Linux/BSD sunucuları gibi işlemez. `seqovl` güvenilirliği Windows kaynaklı sitelerde değişkenlik gösterir.

### 5.7. IP ID Atama Şemaları

| Mod | Açıklama | Kullanım Alanı |
|-----|----------|----------------|
| `seq` | Her enjekte edilen paket için IP ID'yi artır | Varsayılan |
| `seqgroup` | Sahte segmentin IP ID'sini orijinal segmentiyle eşleştir | Durumsal DPI aşma |
| `rnd` | Her pakete rastgele IP ID | Parmak izi önleme |
| `zero` | IP ID'yi 0'a zorla | Yalnızca Linux/BSD sunucular |

### 5.10. SYNDATA Modu

Normalde TCP SYN paketleri yük taşımaz. SYNDATA, SYN paketinin içine veri yükü ekler.

```
Standart SYN:  [SYN bayrağı][veri yok]
SYNDATA SYN:   [SYN bayrağı][16 null bayt veya özel yük]
```

- Hedef OS, SYN yükünü **TCP Fast Open (TFO)** etkin olmadıkça yoksayar.
- DPI ise SYN yükünü ayrıştırmaya çalışarak oturum durum makinesi gerçek el sıkışmayla senkronunu kaybeder.

### 5.11. Orijinal Paket Değişikliği

**Gerçek** veri paketlerinin TTL ve IP ID alanlarını değiştirebilirsiniz (yalnızca sahteler için değil):

| Parametre | İşlevi |
|-----------|--------|
| `--orig-ttl=N` | Gerçek paket TTL'ini N'e ayarla |
| `--orig-autottl` | Gerçek paket için TTL'yi otomatik hesapla |

Bu, DPI'ı gerçek ve sahte akışları karşılaştırırken hatalı atlama mesafeleri hesaplamaya zorlar.

### 5.12. Yinelenen Paket Enjeksiyonu

`--dup=N`, gerçek paketten önce N adet yinelenen kopya gönderir.

| Parametre | İşlevi |
|-----------|--------|
| `--dup=1` | Gerçek paketten önce 1 yinelenen gönder |
| `--dup-ttl=N` | Yinelemelere N TTL uygula |
| `--dup-autottl` | Yinelemeler için TTL'yi otomatik hesapla |
| `--dup-badseq` | Yinelemelere kötü sıra numarası uygula |

**Amaç**: DPI'ı aynı paketin çelişkili kopyalarını işlemeye zorlayarak oturum takibini bırakmasına neden olur.

### 5.11. Sunucu Tarafı Pencere Manipülasyonu (wssize)

Normalde sunucu büyük bir TCP yanıtı gönderir. `wssize`, el sıkışma sırasında sunucuya bildirilen TCP penceresini yapay olarak kısıtlar:

```
İstemci → [SYN, Pencere=65535] → Sunucu
İstemci ← [SYN-ACK] ← Sunucu
İstemci → [ACK, Pencere=1] → Sunucu (wssize etkin)
Sunucu: "Pencere çok küçük, seferinde 1 bayt göndereceğim"
DPI: "Tam ServerHello'yu okuyamıyorum — denetimi bırakıyorum"
```

> Not: İlk el sıkışma tamamlandıktan sonra Vane'in conntrack modülü pencere kısıtlamasını kaldırarak tam indirme hızını geri yükler.

### 5.12. UDP / QUIC Desync

QUIC (HTTP/3), UDP kullanır ve bağlantının SNI'sini ilk QUIC CRYPTO çerçevesinde taşır.

| Yöntem | Açıklama |
|--------|----------|
| `fake` | Gerçek paketten önce sahte QUIC paketi enjekte et |
| `fakeknown` | Hazırlanmış CRYPTO çerçevesiyle sahte QUIC Initial paketi |
| `udplen=N` | UDP yük uzunluğunu N bayt artır |
| IPv6 uzantıları | QUIC paketlerine IPv6 Atlama-Atlama uzantı başlıkları ekle |

---

## 6. Desync Parametre Referans Tablosu

Vane'in Gelişmiş sekmesinde sunulan tüm zapret parametrelerinin tam referansı:

| Parametre | Değerler | Varsayılan | Açıklama |
|-----------|---------|----------|----------|
| `--dpi-desync` | `fake`,`multisplit`,`multidisorder`,`fakedsplit`,`fakeddisorder`... | — | Birincil desync yöntemi(leri) |
| `--dpi-desync-split-pos` | İşaretçi veya tamsayı listesi | `2` | Bölme konumu(ları) |
| `--dpi-desync-split-http-req` | `none`,`method`,`host` | `none` | HTTP isteği özel bölme noktası |
| `--dpi-desync-split-tls` | `none`,`sni`,`sniext` | `none` | TLS özel bölme noktası |
| `--dpi-desync-fooling` | `badsum`,`badseq`,`md5sig`,`ts`,`datanoack`,`hopbyhop`,`hopbyhop2` | — | Sahte paket yanıltma modu(ları) |
| `--dpi-desync-autottl` | `[-]N:N-N` | — | TTL sınırlarını otomatik hesapla |
| `--dpi-desync-ttl` | Tamsayı | — | Sahte paketler için sabit TTL |
| `--dpi-desync-repeats` | Tamsayı | `1` | Gerçek paket başına sahte paket sayısı |
| `--dpi-desync-any-protocol` | Bayrak | Kapalı | Tüm TCP bağlantılarına desync uygula |
| `--dpi-desync-cutoff` | `d1`-`d9`, `s1`-`sN` | — | Yalnızca ilk N veri/SYN paketine desync uygula |
| `--wssize` | Tamsayı | — | Sunucuya bildirilen alıcı pencere boyutu |
| `--wf-tcp` | port listesi | — | Ele geçirilecek TCP portları |
| `--wf-udp` | port listesi | — | Ele geçirilecek UDP portları |
| `--ipcache-lifetime` | Saniye | 7200 | IP önbelleği giriş süresi |
| `--dup` | Tamsayı | — | Orijinal başına yinelenen paket sayısı |

---

## 7. Parçalanmış El Sıkışma ve Kyber Desteği

Modern tarayıcılar (Chrome 124+, Firefox 126+) kuantum sonrası anahtar kapsülleme için **ML-KEM (Kyber)** kullanır. Bu, TLS ClientHello boyutunu dramatik biçimde artırır:

| ClientHello Tipi | Tipik Boyut | Tek Pakete Sığar mı? |
|-----------------|------------|----------------------|
| Klasik TLS 1.3 | ~300 bayt | ✅ Evet (MTU ~1500 bayt) |
| TLS 1.3 + Kyber768 | ~1500-2000 bayt | ❌ Hayır, 2+ pakete bölünür |

**Vane'in bu durumu nasıl ele aldığı:**

1. El sıkışmanın ilk TCP segmentini yakalar.
2. `handshake_length > (paket_boyutu - başlıklar)` kontrolüyle çok paketli ClientHello'yu tespit eder.
3. Tam ClientHello alınana kadar tüm parçaları arabellekte tutar.
4. Yapılandırılmış desync stratejisini yeniden birleştirilmiş ileti bloğu üzerinde uygular.
5. Değiştirilmiş segmentleri doğru sırayla yeniden enjekte eder.

---

## 8. Bağlantı Takibi (Conntrack)

Vane'in dahili conntrack modülü, wssize ve parçalanmış el sıkışma desteği gibi çok paketli işlemleri koordine etmek için canlı TCP ve UDP oturumlarını takip eder.

| Özellik | Ayrıntılar |
|---------|------------|
| TCP durum takibi | SYN → ESTABLISHED → FIN / RST |
| UDP akış takibi | Kaynak IP/port + Hedef IP/port anahtarı |
| Etkin olmayan zaman aşımı | 60 saniye (UDP), 120 saniye (TCP established) |
| Tanılama dökümü | Daemon sürecine `SIGUSR1` göndererek conntrack tablosu yazdırılır |

---

## 9. IP Önbelleği Yönetimi

IP önbelleği, önceden hesaplanan atlama mesafelerini saklayarak yeni oturumların ilk paketinden itibaren anlık autottl kalibrasyonunu mümkün kılar.

| Özellik | Değer |
|---------|-------|
| Önbellek anahtarı | Hedef IP + ağ arayüzü |
| Önbellek değeri | Gözlemlenen TTL, hesaplanan atlama mesafesi, hostname |
| Varsayılan ömür | 7200 saniye (2 saat) |
| Geçersiz kılma | `--ipcache-lifetime=N` |
| Tahliye politikası | LRU; kapasite aşıldığında en eski girişler silinir |

---

## 10. Vane Sistem Özellikleri

### 10.1. DNS Koruma — Yerel DoH / DoT Yönlendiricisi

DNS Guard, `127.0.0.127:5353` üzerinde yerel bir çözümleyici çalıştırır. Standart UDP/53 DNS sorgularını şifreli kanallar üzerinden iletir.

| Sağlayıcı | Protokol | Uç Nokta | Durum |
|-----------|----------|----------|-------|
| Cloudflare | DoH / DoT | `https://cloudflare-dns.com/dns-query` | ✅ Destekleniyor |
| Google | DoH / DoT | `https://dns.google/dns-query` | ✅ Destekleniyor |
| AdGuard | DoH | `https://dns.adguard.com/dns-query` | ⏳ Planlanıyor |
| NextDNS | DoH | Yapılandırma üzerinden özel | ⏳ Planlanıyor |
| Özel | DoH | Kullanıcı tanımlı URL | ⏳ Planlanıyor |

**Özellik Özeti:**

| Özellik | Durum |
|---------|-------|
| DNS-over-HTTPS | ✅ Destekleniyor |
| DNS-over-TLS | ✅ Destekleniyor |
| DNS-over-QUIC | ❌ Desteklenmiyor (DoH veya DoT kullanın) |
| Bellek içi DNS önbelleği | ✅ Destekleniyor |
| Önbellek TTL'ye uyum | ✅ Destekleniyor |
| Yerel alan adı yedek desteği (`.local`, `.lan`) | ✅ Destekleniyor |
| Eş zamanlılık limiti (100 paralel istek) | ✅ Destekleniyor |
| DoH sorguları için SOCKS5 proxy | ✅ Destekleniyor |

> [!NOTE]
> **Preset Formatı:** Kanonik preset dosyaları `.vane` uzantısını kullanır. Eski `.json` dosyaları içe aktarma uyumluluğu için desteklenmeye devam eder.

### 10.2. AdBlock DNS Filtresi

DNS Guard, DNS düzeyinde engelleme için [StevenBlack hosts listesini](https://github.com/StevenBlack/hosts) (~100.000+ alan adı) entegre eder.

| Kategori | Engellendi |
|----------|------------|
| Reklam ağları | ✅ |
| Telemetri ve analitik | ✅ |
| Zararlı yazılım / phishing alan adları | ✅ |
| Sosyal medya izleyicileri | İsteğe bağlı |

### 10.3. Kill Switch — DNS Sızıntı Koruması

Kill Switch, yerel işletim sistemi API'leri kullanarak giden UDP/TCP port 53 trafiğini engeller:

| İS | Mekanizma | Durum |
|----|-----------|-------|
| Windows | Windows Firewall (`netsh`) kuralları & WinDivert sürücüsü | ✅ Destekleniyor |
| Linux | iptables / nftables OUTPUT zincir kuralları | ⚠️ Deneysel (Experimental) |


Etkinleştirildiğinde, şifreli DNS Guard tüneli dışındaki tüm DNS sorguları sessizce düşürülür.

### 10.4. Otomatik Kurtarma Watchdog'u

Watchdog, yapılandırılabilir hedef alan adlarına (varsayılan: `discord.com`) bağlantıyı sürekli izler.

| Tetikleyici Koşul | Eylem |
|-------------------|-------|
| HTTP HEAD isteği başarısız | Preset optimize ediciyi çalıştır |
| Optimize edici daha iyi preset bulur | Optimal presete geç |
| ICMP ping zaman aşımı | Uyarı kaydı + yeniden deneme |
| Motor süreci çöküşü | Aynı presetle motoru yeniden başlat |

### 10.5. Preset Optimize Edici

Optimize edici, mevcut ağ için en etkili yapılandırmayı bulmak amacıyla tüm presetleri canlı bağlantı hedeflerine karşı test eder.

**Süreç:**
1. Mevcut motor oturumunu durdur.
2. Öncelik sırasına göre tüm presetleri dene.
3. Her preset için motoru başlat ve test hedefine HTTP HEAD sorgusu çalıştır.
4. HTTP < 400 döndüren ilk preseti seç.
5. Kazanan presete geç ve normal çalışmaya devam et.

### 10.6. SOCKS5 Üst Düzey Proxy

DNS Guard'ın giden DoH sorgularını SOCKS5 proxy üzerinden tünellemeye olanak tanır:

```
DNS Sorgusu → DNS Guard → SOCKS5 Proxy → İnternet → DoH Sunucusu
```

DoH uç noktalarının da engellendiği ortamlarda şifreli DNS arama yolunu İSS'den gizler.

### 10.7. Uzak Preset Senkronizasyonu

Vane, uzak bir JSON uç noktasından (GitHub Gist veya CDN) preset tanımlarını çekebilir:

- Önbelleğe alınmış kopya mevcutsa başlangıçta yüklenir (sıfır ağ I/O).
- Başlangıçtan sonra arka planda yenilenir (engellemesiz).
- Minisign kriptografik imzalarıyla doğrulanır.
- Yerleşik presetlerin üzerine yazamaz (ID çakışma koruması).

### 10.8. Etiketli Çıktılı Log Konsolu

| Etiket | Renk | Kaynak |
|--------|------|--------|
| `[MOTOR]` | Mor | Zapret motor süreci |
| `[DNS]` | Yeşil | DNS Guard yönlendiricisi |
| `[ADBLOCK]` | Kırmızı | DNS filtreleme olayları |
| `[GÜVENLİK]` | Sarı | Yetki kontrolleri, sanitasyon |
| `[SİSTEM]` | Mavi | Otomatik başlatma, ağ değişiklikleri |
| `[HATA]` | Kırmızı | Süreç hataları, kritik arızalar |
| `[UYARI]` | Kehribar | Kritik olmayan uyarılar |

### 10.9. Otomatik Başlatma ve Sistem Tepsisi

Otomatik başlatma etkinleştirildiğinde, Vane bir Windows Görev Zamanlayıcı girişi kaydeder. Başlangıçta:

1. Kalıcı ayarlar dosyasından son etkin preset ID'sini okur.
2. Sistem DNS'inin güvenilir olup olmadığını kontrol eder; değilse Cloudflare DNS uygular.
3. Kaydedilen presetle motoru sessizce başlatır.
4. Ana pencereyi gizler; sistem tepsisi simgesini gösterir.

### 10.10. Ağ Değişikliği Algılama

Vane, ağ adaptörü değişikliklerini algılamak için `WM_DEVICECHANGE` (Windows) veya netlink soket olaylarını (Linux) dinler.

- Yeni bir adaptör bağlandığında `network_changed` olayı gönderilir.
- Ön yüz arayüzü DNS adaptör durumunu ve ağ istatistiklerini yeniler.
- WinDivert, yakalama filtrelerini yeni adaptörlere otomatik olarak uygular — motor yeniden başlatma gerekmez.

---

## 11. Yerleşik Presetler

| ID | Etiket | Strateji | Hedef |
|----|--------|----------|-------|
| `tr-1` | TR Standart | `fake,multidisorder` + `autottl` + `badseq` | Türk İSS'leri |
| `tr-2` | TR Agresif | `fake,multidisorder` + `md5sig` + sabit TTL | Kısıtlayıcı Türk İSS'leri |
| `tr-3` | TR Fragment | `multisplit` + `fakedsplit` | Parçalanma tabanlı aşma |
| `tr-4` | TR Desync-HTTPS | Özel bölme ile HTTPS'e özgü desync | Yalnızca HTTPS denetimi |
| `tr-5` | TR QUIC | UDP 443 + `fakeknown` | YouTube QUIC akışları |
| `discord-voip` | Discord & VoIP Düzeltmesi | UDP 50000-65535 sahte enjeksiyonu | Sesli sohbet kararlılığı |

---

## 12. Gelişmiş Yapılandırma — Tam Parametre Tablosu

### DPI Desync

| Ayar | Parametre | Değerler | Açıklama |
|------|-----------|---------|----------|
| Desync Yöntemi | `--dpi-desync` | `fake`, `multisplit`, `multidisorder`, `fakedsplit`... | Birincil aşma yöntemi |
| Bölme Konumu | `--dpi-desync-split-pos` | İşaretçi veya tamsayı listesi | TCP yükünün nerede kesileceği |
| HTTP Bölme Hedefi | `--dpi-desync-split-http-req` | `none`, `method`, `host` | HTTP'ye özgü bölme çıpası |
| TLS Bölme Hedefi | `--dpi-desync-split-tls` | `none`, `sni`, `sniext` | TLS'ye özgü bölme çıpası |
| Yanıltma Modu | `--dpi-desync-fooling` | `badsum`, `badseq`, `md5sig`, `ts`, `datanoack`... | Sahteyi sunucuya ulaşmaktan engelle |
| Otomatik TTL | `--dpi-desync-autottl` | `[-]N:N-N` | TTL'yi dinamik olarak kalibre et |
| Sabit TTL | `--dpi-desync-ttl` | Tamsayı | Sabit sahte TTL değeri |
| Sahte Tekrarlar | `--dpi-desync-repeats` | Tamsayı | Sahte paket sayısı |
| Herhangi Protokol | `--dpi-desync-any-protocol` | Bayrak | Tüm TCP'ye uygula |
| Kesim Noktası | `--dpi-desync-cutoff` | `d1`-`d9`, `s1`-`sN` | İlk N pakete desync uygula |

### Paket ve Trafik

| Ayar | Parametre | Açıklama |
|------|-----------|----------|
| TCP Alıcı Penceresi | `--wssize` | Sunucuya bildirilen alıcı penceresini kısıtla |

### Protokol ve Portlar

| Ayar | Parametre | Örnek | Açıklama |
|------|-----------|-------|----------|
| TCP Portları | `--wf-tcp` | `80,443` | Ele geçirilecek TCP portları |
| UDP Portları | `--wf-udp` | `443` | Ele geçirilecek UDP portları |
| QUIC UDP | `--wf-udp=443` | Bayrak | QUIC aşmasını etkinleştir |

---

## 13. Pratik Engel Aşma Stratejileri

### Strateji 1: Temel Bölme (Güvenli, Maksimum Uyumluluk)

```
--wf-tcp=80,443 --dpi-desync=split --dpi-desync-split-pos=2
```

### Strateji 2: AutoTTL Sahte + Bozuk Düzen (Varsayılan — Türkiye)

```
--wf-tcp=80,443 --wf-udp=443 --dpi-desync=fake,multidisorder
--dpi-desync-autottl=-1:3-20 --dpi-desync-fooling=badseq
--dpi-desync-any-protocol --dpi-desync-cutoff=d3
```

**Mekanizma:**
1. TTL kalibrasyonu için sunucu TTL'sini ölçer.
2. Hesaplanan TTL ve kötü sıra numarasıyla sahte paket gönderir.
3. Kalan gerçek segmentleri bozuk düzen sırasıyla gönderir.
4. Oturum başına yalnızca ilk 3 veri paketine uygulanır.

### Strateji 3: MD5 İmzası + Bölme (Yoğun İSS Aşması)

```
--wf-tcp=80,443 --wf-udp=443 --dpi-desync=fake,multidisorder
--dpi-desync-fooling=md5sig --dpi-desync-autottl
--dpi-desync-split-pos=4 --dpi-desync-any-protocol
```

### Strateji 4: QUIC + VoIP Aşması

```
--wf-udp=443,50000-65535 --dpi-desync=fake --dpi-desync-repeats=2
--dpi-desync-fooling=badseq --dpi-desync-any-protocol
```

### Strateji 5: wssize (Sunucu Tarafı Sertifika Gizleme)

```
--wf-tcp=443 --wssize=1:6 --dpi-desync=split --dpi-desync-split-pos=2
```

### Strateji 6: Fragment Yöntemi

```
--wf-tcp=80,443 --dpi-desync=multisplit,fakedsplit
--dpi-desync-split-pos=2,4 --dpi-desync-fooling=badseq
```

---

## 14. Güvenlik Duvarı Kurulumu — Linux (Iptables / Nftables)

### Iptables

```bash
# Giden TCP (HTTP + HTTPS)
iptables -A OUTPUT -p tcp -m multiport --dports 80,443 \
  -j NFQUEUE --queue-num 200 --queue-bypass

# Gelen TCP (autottl için sunucu TTL gözlemi gerektirir)
iptables -A INPUT -p tcp -m multiport --sports 80,443 \
  -j NFQUEUE --queue-num 200 --queue-bypass

# Giden QUIC (HTTP/3)
iptables -A OUTPUT -p udp --dport 443 \
  -j NFQUEUE --queue-num 200 --queue-bypass
```

### Nftables

```nftables
table ip vane_mangle {
    chain output {
        type filter hook output priority mangle; policy accept;
        tcp dport { 80, 443 } queue num 200
        udp dport 443 queue num 200
    }
    chain input {
        type filter hook input priority mangle; policy accept;
        tcp sport { 80, 443 } queue num 200
    }
}
```

> Vane, Linux'ta motor başlatıldığında bu kuralları otomatik olarak uygular.

---

## 15. Güvenlik Mimarisi

| Kontrol | Mekanizma | Konum |
|---------|-----------|-------|
| IPC beyaz listesi | Tüm Tauri komutları URL şemalarını ve preset ID'lerini doğrular | `commands.rs` |
| Argüman sanitasyonu | Katı beyaz liste — yalnızca bilinen zapret parametreleri kabul edilir | `sanitizer.rs` |
| Kabuk enjeksiyonu önleme | Linux kök sarmalayıcısında tüm argümanlara tek tırnak kaçış işlemi | `manager.rs` |
| İkili dosya bütünlüğü | Çalışmadan önce `winws.exe` / `nfqws` SHA-256 doğrulaması | `manager.rs` |
| Süreç izolasyonu | `KILL_ON_JOB_CLOSE` ile Windows İş Nesnesi | `job.rs` |
| Kapasite kısıtlaması | WebView'de `fs:write`, `fs:read` veya `shell:execute` yok | `capabilities/default.json` |
| İçerik Güvenlik Politikası | `script-src 'self'` — harici komut dosyası yok | `tauri.conf.json` |
| Güncelleyici imzası | Kurulumdan önce Minisign imza doğrulaması | `updater.rs` |
| Preset ID doğrulaması | Yalnızca alfanümerik + `-` + `_`, maksimum uzunluk uygulanır | `loader.rs` |
| DNS sızıntı önleme | Kill Switch giden UDP/TCP 53'ü engeller | `dns/mod.rs` |

---

## 16. Kurulum

### Windows

1. [Releases](https://github.com/lulushuz/Vane/releases) sayfasından en son `.msi` yükleyicisini indirin.
2. Yükleyiciyi Yönetici olarak çalıştırın.
3. Vane'i Başlat Menüsünden başlatın.

> Vane, WinDivert çekirdek sürücüsünü yüklemek için Yönetici yetkisi gerektirir.

### Linux

1. [Releases](https://github.com/lulushuz/Vane/releases) sayfasından `.deb` (Debian/Ubuntu) veya `.AppImage` dosyasını indirin.
2. `sudo dpkg -i vane_*.deb` veya `chmod +x Vane_*.AppImage && ./Vane_*.AppImage` ile kurun.
3. Vane'i başlatın; motor başlatıldığında `pkexec` (PolicyKit) kök yükseltmesi isteyecektir.

---

## 17. Kaynak Koddan Derleme

### Gereksinimler

| Araç | Sürüm |
|------|-------|
| Node.js | LTS (20+) |
| npm | 10+ |
| Rust | Kararlı (2021 sürümü) |
| Tauri CLI | v2 |

### Derleme Adımları

```bash
# Depoyu klonla
git clone https://github.com/lulushuz/Vane.git
cd Vane

# Ön yüz bağımlılıklarını yükle
npm install

# Geliştirme modu (sıcak yenileme)
npm run tauri dev

# Üretim derlemesi
npm run tauri build
```

### Backend Testleri

```bash
cd src-tauri
cargo test
cargo clippy
```

---

## 18. Sorun Giderme

| Sorun | Olası Neden | Çözüm |
|-------|-------------|-------|
| Motor başlamıyor | WinDivert sürücü çakışması | Diğer DPI araçlarını kapat (GoodbyeDPI vb.) |
| DNS sızıntısı tespit edildi | Kill Switch devre dışı | DNS sekmesinde Kill Switch'i etkinleştir |
| Günlüklerde `[HATA]` | Yönetici olarak çalışmıyor | Vane'i Yönetici olarak yeniden başlat |
| Etkinleştirme sonrası yüksek gecikme | wssize tüm bağlantılarda etkin | wssize'ı devre dışı bırak veya belirli portlarla sınırla |
| QUIC akışları hâlâ engelleniyor | QUIC aşması etkin değil | Gelişmiş sekmede UDP 443'ü etkinleştir |
| Motor başlayıp hemen duruyor | İkili dosya hash uyuşmazlığı | Vane yükleyicisini yeniden indir |
| Uzak presetler yüklenmiyor | Güvenlik duvarı GitHub CDN'i engelliyor | Ağı kontrol et veya preset senkronizasyonunu devre dışı bırak |

---

## 19. Sınırlamalar ve Engel Aşmanın Yetersiz Kaldığı Durumlar

| Senaryo | Neden | Alternatif |
|---------|-------|------------|
| IP düzeyinde engelleme | Hedef IP yönlendirme katmanında engelleniyor | VPN veya proxy |
| Şeffaf TCP proxy | İSS TCP'yi tamamen sonlandırıp yeniden oluşturuyor | VPN |
| Aktif yoklama | İSS meşruiyeti doğrulamak için yoklama paketleri gönderiyor | Yoklama dirençli VPN |
| TLS 1.2 parmak izi | Statik şifre grubu siparişleri tespit ediliyor | İstemci OS / TLS kütüphanesini güncelle |
| MITM sertifika denetimi | İSS kendi CA'sını OS güven deposuna yüklüyor | Güvenilmeyen CA'ları OS'den kaldır |
| Tam engelleme | TCP SYN yönlendirme düzeyinde düşürülüyor | VPN |

---

## 20. Emeği Geçenler ve Lisans

- **[zapret](https://github.com/bol-van/zapret)** — bol-van tarafından: Temel DPI engeli aşma motoru.
- **[WinDivert](https://reqrypt.org/windivert.html)** — basil00 tarafından: Windows çekirdek paketi yakalama sürücüsü.
- **[Tauri](https://tauri.app)** — Güvenli masaüstü uygulama çerçevesi.
- **[Minisign](https://jedisct1.github.io/minisign/)** — Kriptografik imza doğrulama.
- **[StevenBlack/hosts](https://github.com/StevenBlack/hosts)** — AdBlock hosts listesi.

**GPL-3.0 Lisansı** altında lisanslanmıştır — ayrıntılar için [LICENSE](LICENSE) dosyasına bakın.

---

## 21. Topluluk

| Kanal | Bağlantı |
|-------|----------|
| 🐛 Hata Bildirimleri | [GitHub Issues](https://github.com/lulushuz/Vane/issues) |
| 💡 Özellik İstekleri | [GitHub Issues](https://github.com/lulushuz/Vane/issues) |
| 💬 Discord (Kişisel) | [luppux](https://discord.com/users/852103749228036136) |
| 💬 Discord (Topluluk) | [discord.gg/luppux](https://discord.gg/luppux) |
| 📧 Güvenlik Bildirimleri | alp@archey.com.tr |
| 📋 Katkı Sağlama | [CONTRIBUTING.md](CONTRIBUTING.md) |
| 🔒 Güvenlik Politikası | [SECURITY.md](SECURITY.md) |
| 📝 Değişim Günlüğü | [CHANGELOG.md](CHANGELOG.md) |
