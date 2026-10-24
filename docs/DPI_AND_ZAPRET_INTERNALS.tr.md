# DPI ve Zapret Dahili Protokol Referansı

Bu teknik doküman; Derin Paket İnceleme (DPI) mekanizmalarını, paket manipülasyon stratejilerini ve düşük seviyeli zapret (`winws` ve `nfqws`) engeli aşma tekniklerini ayrıntılarıyla açıklamaktadır.

## İçindekiler

- [1. Zapret Nedir?](#1-zapret-nedir)
  - [1.1. Temel Daemon'lar](#11-temel-daemonlar)
  - [1.2. Desteklenen Protokoller](#12-desteklenen-protokoller)
- [2. Derin Paket İnceleme (DPI) Nasıl Çalışır?](#2-derin-paket-i̇nceleme-dpi-nasıl-çalışır)
  - [2.1. Pasif DPI ve Aktif DPI](#21-pasif-dpi-ve-aktif-dpi)
  - [2.2. SNI ve Hostname Ayıklama Süreci](#22-sni-ve-hostname-ayıklama-süreci)
  - [2.3. Bloklama Yöntemleri: RST ve HTTP Yönlendirme](#23-bloklama-yöntemleri-rst-ve-http-yönlendirme)
  - [2.4. DNS Zehirlenmesi ve Ele Geçirme](#24-dns-zehirlenmesi-ve-ele-geçirme)
  - [2.5. Derin Parmak İzi ve Davranışsal Analiz](#25-derin-parmak-i̇zi-ve-davranışsal-analiz)
- [3. Zapret Mimarisi — nfqws / winws Çekirdeği](#3-zapret-mimarisi--nfqws--winws-çekirdeği)
  - [3.1. WinDivert (Windows)](#31-windivert-windows)
  - [3.2. NFQUEUE (Linux)](#32-nfqueue-linux)
  - [3.3. Paket İşleme Hattı](#33-paket-i̇şleme-hattı)
- [4. DPI Desync Stratejileri](#4-dpi-desync-stratejileri)
  - [4.1. Desync Aşama Sistemi — Kombinasyon Sıralaması](#41-desync-aşama-sistemi--kombinasyon-sıralaması)
  - [4.2. TCP Segmentasyon Yöntemleri](#42-tcp-segmentasyon-yöntemleri)
  - [4.3. fakedsplit ve fakeddisorder — altorder Tablosu](#43-fakedsplit-ve-fakeddisorder--altorder-tablosu)
  - [4.4. Bölme Konumu İşaretçileri](#44-bölme-konumu-i̇şaretçileri)
  - [4.5. Sahte Paket Enjeksiyonu](#45-sahte-paket-enjeksiyonu)
  - [4.6. Yanıltma Modları (Fooling Modes)](#46-yanıltma-modları-fooling-modes)
  - [4.7. Özel Sahte Yükler](#47-özel-sahte-yükler)
  - [4.8. Sıra Numarası Çakışması (seqovl)](#48-sıra-numarası-çakışması-seqovl)
  - [4.9. IP ID Atama Düzenleri](#49-ip-id-atama-düzenleri)
  - [4.10. SYNDATA Modu](#410-syndata-modu)
  - [4.11. Orijinal Paket Değiştirme](#411-orijinal-paket-değiştirme)
  - [4.12. Çift Paket Enjeksiyonu](#412-çift-paket-enjeksiyonu)
  - [4.13. Sunucu Tarafı Pencere Boyutu Manipülasyonu (wssize)](#413-sunucu-tarafı-pencere-boyutu-manipülasyonu-wssize)
  - [4.14. UDP / QUIC Desync](#414-udp--quic-desync)
  - [4.15. IP Parçalama (Fragmentation)](#415-ip-parçalama-fragmentation)
- [5. tpws — Şeffaf TCP Proxy Modu](#5-tpws--şeffaf-tcp-proxy-modu)
  - [5.1. nfqws ve tpws Karşılaştırması](#51-nfqws-ve-tpws-karşılaştırması)
  - [5.2. tpws Ne Zaman Kullanılmalıdır?](#52-tpws-ne-zaman-kullanılmalıdır)
  - [5.3. tpws Özellikleri](#53-tpws-özellikleri)
  - [5.4. tpws Trafik Yönlendirme (Linux)](#54-tpws-trafik-yönlendirme-linux)
- [6. Desync Parametre Referans Tablosu](#6-desync-parametre-referans-tablosu)
- [7. Parçalanmış El Sıkışma ve Kyber Desteği](#7-parçalanmış-el-sıkışma-ve-kyber-desteği)
- [8. Bağlantı Takibi (Conntrack)](#8-bağlantı-takibi-conntrack)
- [9. IP Önbelleği Yönetimi](#9-ip-önbelleği-yönetimi)
- [10. Gelişmiş Yapılandırma — Tam Parametre Tablosu](#10-gelişmiş-yapılandırma--tam-parametre-tablosu)
- [11. Pratik Engel Aşma Stratejileri](#11-pratik-engel-aşma-stratejileri)

---

## 1. Zapret Nedir?

[Zapret](https://github.com/bol-van/zapret), bol-van tarafından geliştirilen açık kaynaklı bir DPI engeli aşma kütüphanesi ve daemon sürecidir. Giden (ve bazı durumlarda gelen) TCP/UDP paketlerini yakalayarak yapılandırılabilir manipülasyonlar uygular ve yeniden enjekte eder.

### 1.1. Temel Daemon'lar

| Daemon | Platform | Mekanizma | Notlar |
|--------|----------|-----------|--------|
| `winws` | Windows | WinDivert çekirdek sürücüsü | Yönetici yetkisi gerektirir |
| `nfqws` | Linux | NFQUEUE netfilter hedefi | iptables/nftables kuralları gerektirir |
| `tpws` | Linux | SOCKS5 şeffaf proxy | Çekirdek modülü gerekmez |

### 1.2. Desteklenen Protokoller

| Protokol | Port | Taşıma | Aşma Yöntemleri |
|----------|------|--------|-----------------|
| HTTP | 80 | TCP | Bölme, bozuk düzen, sahte |
| HTTPS (TLS 1.2/1.3) | 443 | TCP | Bölme, sahte, seqovl, wssize |
| QUIC (HTTP/3) | 443 | UDP | Sahte, udplen, fakeknown |
| VoIP / Discord RTP | 50000-65535 | UDP | Sahte, udplen |
| DoH (HTTPS üzerinden DNS) | 443 | TCP | DNS Guard tarafından yönetilir |

---

## 2. Derin Paket İnceleme (DPI) Nasıl Çalışır?

Etkili engel aşma stratejileri geliştirmek için İSS'lerin kullandığı denetim sistemlerinin mimarisini anlamak kritik önem taşır.

### 2.1. Pasif DPI ve Aktif DPI

| Özellik | Pasif DPI | Aktif DPI |
|---------|-----------|-----------|
| Yerleşim | Ayna portu / optik tap | Veri yolunun içinde (inline) |
| Paket düşürebilir mi | ❌ Hayır | ✅ Evet |
| Paket geciktirebilir mi | ❌ Hayır | ✅ Evet |
| Bloklama yöntemi | RST enjeksiyonu / HTTP yönlendirme | Düşürme, TCP sıfırlama, proxy kesişmesi |
| Aşma zorluğu | Düşük — Orta | Yüksek |

**Pasif DPI**, trafiğin bir kopyasını alır. Gerçek sunucu yanıtından önce müşteriye sahte bir TCP RST veya HTTP 302 enjekte etmeye çalışır. Sahte paket önce ulaşırsa bağlantı kopar.

**Aktif DPI** doğrudan veri yolunun üzerinde konumlanır. TCP akışlarını yeniden oluşturabilir, yeniden birleştirilmiş yükler üzerinde regex eşleştirmesi uygulayabilir ve bağlantıları tamamlanmadan engelleyebilir.

### 2.2. SNI ve Hostname Ayıklama Süreci

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

### 2.3. Blok Enjeksiyonu — RST ve HTTP Yönlendirme

DPI sensörü yasaklı bir alan adı tespit ettiğinde:

1. Sunucunun kaynak IP ve portunuyla **sahte TCP RST** paketi üretir (pasif DPI).
2. HTTP bağlantıları için İSS uyarı sayfasını gösteren **sahte HTTP 302 yönlendirme** paketi enjekte edebilir.
3. Yarış koşulu: enjekte edilen RST/yönlendirme gerçek sunucu yanıtından **önce** istemciye ulaşırsa işletim sistemi soketi kapatır.
4. Aktif DPI'lar gerçek paketi tamamen düşürür ve kendi RST'lerini gönderir — yarış koşuluna gerek yoktur.

### 2.4. DNS Zehirlenmesi ve Ele Geçirme

TCP bağlantısı kurulmadan önce alan adının çözümlenmesi gerekir. İSS'ler DNS'i iki yaygın yolla ele geçirir:

| Yöntem | Mekanizma | Sonuç |
|--------|-----------|-------|
| DNS Ele Geçirme | UDP/53'ü keser, sahte IP döndürür | İstemci engel portalına bağlanır |
| DNS Zehirlenmesi | Yarış koşulunda yanlış DNS yanıtı enjekte eder | Ele geçirmeyle aynı |
| DNS Engelleme | DNS isteğini tamamen düşürür | Bağlantı zaman aşımına uğrar |
| Şeffaf DNS Proxy | Tüm DNS'i İSS çözümleyicisinden geçirir | İSS sansürlü yanıtlar döndürür |

**Vane'in DNS Guard'ı**, tüm DNS sorgularını şifreli DoH/DoT tünelleri üzerinden yönlendirerek bunların tamamını çözer.


### 2.5. Derin Parmak İzi ve Davranış Analizi

Gelişmiş DPI sistemleri SNI okumayı davranışsal parmak iziyle destekler:

| Teknik | Açıklama | Karşı Tedbir |
|--------|----------|--------------|
| JA3/JA3S parmak izi | TLS ClientHello parametrelerinin hash'i | TLS uzantı sıralamasını değiştirme |
| TCP parmak izi | TCP seçenek sıralamasıyla OS tespiti | `--dpi-desync-ttl` + bölme |
| Akış uzunluk analizi | Paket boyutuyla VPN/proxy tespiti | `--udplen`, yük doldurma |
| Zamanlama korelasyonu | Şifreli akışları bilinen kalıplarla eşleştirme | Bozuk düzen moduyla kaos enjeksiyonu |

---

## 3. Zapret Mimarisi — nfqws / winws Çekirdeği

### 3.1. WinDivert (Windows)

Windows'ta `winws`, [WinDivert](https://reqrypt.org/windivert.html) çekirdek sürücüsünü kullanır:

```
Uygulama → TCP Yığını → WinDivert (çekirdek sürücüsü paketi yakalar)
                                   ↓
                         winws.exe (kullanıcı alanı işleme)
                                   ↓
                         WinDivert yeniden enjekte → Router → İnternet
```

WinDivert, hangi paketlerin ele geçirileceğini belirten bir filtre ifadesiyle Windows çekirdek sürücüsü (`WinDivert64.sys`) olarak kurulur.

### 3.2. NFQUEUE (Linux)

Linux'ta `nfqws`, çekirdeğin NFQUEUE netfilter hedefini kullanır:

```
Uygulama → TCP Yığını → iptables NFQUEUE kuralı → NFQUEUE (çekirdek)
                                                         ↓
                                             nfqws (kullanıcı alanı işleme)
                                                         ↓
                                         NF_ACCEPT / NF_DROP → Router → İnternet
```

### 3.3. Paket İşleme Hattı

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

## 4. DPI Desync Stratejileri

### 4.1. Desync Faz Sistemi — Kombo Sıralaması

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

### 4.2. TCP Segmentasyon Yöntemleri

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

### 4.3. fakedsplit ve fakeddisorder — altorder Tablosu

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

### 4.4. Bölme Konum İşaretçileri

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

### 4.3. Sahte Paket Enjeksiyonu

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

### 4.6. Fooling (Yanıltma) Modları

| Mod | Mekanizma | Güvenilirlik | Notlar |
|-----|-----------|--------------|--------|
| `ttl` | TTL'yi hedefe ulaşmadan dolacak kadar düşük ayarla | Yüksek | Atlama sayısı gerektirir; TTL yeniden yazan routerlarla çalışmaz |
| `autottl` | Sunucunun gelen TTL'ini ölçer, atlama mesafesini otomatik hesaplar | Çok Yüksek | En iyi varsayılan seçim |

### 4.5. Sahte Paket İçeriği Özelleştirme

| Seçenek | Etkisi |
|---------|--------|
| `rnd` | Her istekte TLS Rastgele ve Oturum ID alanlarını rastgele oluşturur |
| `rndsni` | SNI uzantısını rastgele SLD + TLD ile değiştirir |
| `dupsid` | Gerçek ClientHello'nun Oturum ID'sini sahte pakete kopyalar |
| `sni=domain` | Sahte paketteki SNI'yi `domain` ile değiştirir |
| `padencap` | Sahte doldurma uzantısını gerçek paketin boyutuna göre genişletir |
| `oob` | Sahte pakette bant dışı veri (TCP Urgent bayrağı) gönderir |

### 4.6. Sıra Numarası Çakıştırma (seqovl)

`seqovl` tekniği, durumsal DPI motorlarını yanıltmak için kasıtlı olarak örtüşen TCP sıra aralıkları oluşturur.

```
Sahte segment:  [seq=100, len=10] → yük: çöp_veri
Gerçek segment: [seq=105, len=10] → yük: gerçek_sni_verisi

DPI görür: çöp_veri (ilk alınan, standart olarak kaydedilir)
Sunucu:    OS uygulamasına bağlı olarak ikinci alınan veya sonraki verilere öncelik verebilir
```

> ⚠️ Windows tabanlı sunucular çakışmaları Linux/BSD sunucuları gibi işlemez. `seqovl` güvenilirliği Windows kaynaklı sitelerde değişkenlik gösterir.

### 4.7. IP ID Atama Şemaları

| Mod | Açıklama | Kullanım Alanı |
|-----|----------|----------------|
| `seq` | Her enjekte edilen paket için IP ID'yi artır | Varsayılan |
| `seqgroup` | Sahte segmentin IP ID'sini orijinal segmentiyle eşleştir | Durumsal DPI aşma |
| `rnd` | Her pakete rastgele IP ID | Parmak izi önleme |
| `zero` | IP ID'yi 0'a zorla | Yalnızca Linux/BSD sunucular |

### 4.10. SYNDATA Modu

Normalde TCP SYN paketleri yük taşımaz. SYNDATA, SYN paketinin içine veri yükü ekler.

```
Standart SYN:  [SYN bayrağı][veri yok]
SYNDATA SYN:   [SYN bayrağı][16 null bayt veya özel yük]
```

- Hedef OS, SYN yükünü **TCP Fast Open (TFO)** etkin olmadıkça yoksayar.
- DPI ise SYN yükünü ayrıştırmaya çalışarak oturum durum makinesi gerçek el sıkışmayla senkronunu kaybeder.

### 4.11. Orijinal Paket Değişikliği

**Gerçek** veri paketlerinin TTL ve IP ID alanlarını değiştirebilirsiniz (yalnızca sahteler için değil):

| Parametre | İşlevi |
|-----------|--------|
| `--orig-ttl=N` | Gerçek paket TTL'ini N'e ayarla |
| `--orig-autottl` | Gerçek paket için TTL'yi otomatik hesapla |

Bu, DPI'ı gerçek ve sahte akışları karşılaştırırken hatalı atlama mesafeleri hesaplamaya zorlar.

### 4.12. Yinelenen Paket Enjeksiyonu

`--dup=N`, gerçek paketten önce N adet yinelenen kopya gönderir.

| Parametre | İşlevi |
|-----------|--------|
| `--dup=1` | Gerçek paketten önce 1 yinelenen gönder |
| `--dup-ttl=N` | Yinelemelere N TTL uygula |
| `--dup-autottl` | Yinelemeler için TTL'yi otomatik hesapla |
| `--dup-badseq` | Yinelemelere kötü sıra numarası uygula |

**Amaç**: DPI'ı aynı paketin çelişkili kopyalarını işlemeye zorlayarak oturum takibini bırakmasına neden olur.

### 4.11. Sunucu Tarafı Pencere Manipülasyonu (wssize)

Normalde sunucu büyük bir TCP yanıtı gönderir. `wssize`, el sıkışma sırasında sunucuya bildirilen TCP penceresini yapay olarak kısıtlar:

```
İstemci → [SYN, Pencere=65535] → Sunucu
İstemci ← [SYN-ACK] ← Sunucu
İstemci → [ACK, Pencere=1] → Sunucu (wssize etkin)
Sunucu: "Pencere çok küçük, seferinde 1 bayt göndereceğim"
DPI: "Tam ServerHello'yu okuyamıyorum — denetimi bırakıyorum"
```

> Not: İlk el sıkışma tamamlandıktan sonra Vane'in conntrack modülü pencere kısıtlamasını kaldırarak tam indirme hızını geri yükler.

### 4.12. UDP / QUIC Desync

QUIC (HTTP/3), UDP kullanır ve bağlantının SNI'sini ilk QUIC CRYPTO çerçevesinde taşır.

| Yöntem | Açıklama |
|--------|----------|
| `fake` | Gerçek paketten önce sahte QUIC paketi enjekte et |
| `fakeknown` | Hazırlanmış CRYPTO çerçevesiyle sahte QUIC Initial paketi |
| `udplen=N` | UDP yük uzunluğunu N bayt artır |
| IPv6 uzantıları | QUIC paketlerine IPv6 Atlama-Atlama uzantı başlıkları ekle |

---


---

## 5. tpws — Şeffaf TCP Proxy Modu

`tpws`, `nfqws`/`winws` için kullanıcı alanında (user space) çalışan bir alternatiftir. Çekirdek kancaları yerine **SOCKS5 şeffaf proxy** olarak çalışır. Vane, bu modu Gelişmiş sekmesindeki **TPWS Proxy Modu** seçeneği ile sunar.

### 5.1. nfqws ve tpws Karşılaştırması

| Özellik | nfqws / winws | tpws |
|---------|---------------|------|
| Katman | Paket (çekirdek + kullanıcı alanı) | Akış (TCP proxy) |
| Çekirdek modülü gereksinimi | Evet (WinDivert / NFQUEUE) | Hayır |
| IP başlıklarını değiştirme | Evet (TTL, IP ID, IP bayrakları) | Hayır |
| IP parçalama (fragmentation) | Evet | Hayır |
| Kötü TTL ile sahte gönderme | Evet | Hayır |
| TCP segmentasyonu | Evet | Evet (`--split-pos`) |
| TLS kayıt bölme (record split) | Hayır | Evet (`--tlsrec`) |
| MSS manipülasyonu | Evet (`--mss`) | Evet (`--mss`) |
| Gerekli yetki | Root / Yönetici | Standart kullanıcı |
| Performans | Daha yüksek (çekirdek yolu) | Biraz daha düşük |

### 5.2. tpws Ne Zaman Kullanılmalıdır?

- **Çekirdek sürücüsünün yüklenemediği** kısıtlı Linux ortamlarında.
- Akış düzeyinde **basit TCP bölme** yönteminin İSS engeli için yeterli olduğu durumlarda.
- Sistemde **root/yönetici yetkisi vermekten kaçınmak** istendiğinde.
- Sürücü çakışması nedeniyle nfqws/winws başlatılamadığında **yedek çözüm** olarak.

### 5.3. tpws Özellikleri

**TLSREC — TLS Kayıt Bölme:**
tpws, TCP segment sınırı yerine doğrudan TLS kayıt katmanında bölme yapabilir. TCP akışını yeniden birleştiren ancak TLS kayıtlarını ayrıştırmayan DPI sistemlerini yanıltır:
```
--tlsrec=sni
```

**MSS — Maksimum Segment Boyutu:**
```
--mss=256
```
TCP el sıkışması sırasında küçük bir MSS bildirerek işletim sistemi çekirdeğini küçük segmentler göndermeye zorlar.

### 5.4. tpws Trafik Yönlendirme (Linux)

iptables ile (1080 portunu dinleyen tpws sürecine):
```bash
iptables -A OUTPUT -p tcp -m multiport --dports 80,443 -j REDIRECT --to-ports 1080
```

nftables ile:
```nftables
table ip tpws_redir {
    chain output {
        type nat hook output priority -100;
        tcp dport { 80, 443 } redirect to :1080
    }
}
```

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



## 10. Gelişmiş Yapılandırma — Tam Parametre Tablosu

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

## 11. Pratik Engel Aşma Stratejileri

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

