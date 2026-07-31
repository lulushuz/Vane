# Version Realignment: 2.x → 1.0.0-rc.1

## Genel Bakış

Vane, `P00–P15` production-hardening programının tamamlanmasıyla birlikte ilk gerçek production-ready mimarisine ulaşmıştır. Bu süreç sırasında kullanılan eski `2.x` sürüm numaraları, projenin henüz geliştirme aşamasındayken atanmış *legacy development build* numaralarıdır.

Bu nedenle resmî sürümleme `1.0.0-rc.1` ile yeniden başlatılmaktadır.

**Doğru ifadeler:**
- Versioning realignment
- Official versioning restart
- Legacy development release line

**Yanlış ifadeler:**
- Git geçmişi sıfırlandı
- Eski proje silindi
- 2.x sürümleri sahteydi

---

## Mevcut `2.x` Kullanıcıları İçin Etki

### Otomatik Güncelleme (Auto-Updater)

Tauri updater SemVer karşılaştırması kullanır. `1.0.0-rc.1`, semver açısından `2.1.4`'ten **düşük** olarak değerlendirilir.

```text
Otomatik 2.1.4 → 1.0.0-rc.1 geçişi: DESTEKLENMEZ
Manuel kurulum: GEREKLİ
```

Eski `2.x` kullanıcıları bu sürümü otomatik güncelleyici üzerinden **alamaz**. Yeni NSIS yükleyicisi manuel olarak çalıştırılmalıdır.

### Uygulama Ayarlarının Korunması

Manuel kurulumda aşağıdaki verilerin durumu:

| Veri | Durum | Not |
|---|---|---|
| Settings (genel) | Korunur | Aynı `com.vane.dpi` identifier |
| Pattern domains / Whitelist / Blacklist | Korunur | Rust backend aynı yolda okur |
| Seçili preset | Korunur | builtin preset ID'leri değişmedi |
| Advanced config | Korunur | JSON schema geriye uyumlu |
| DNS config | Korunur | Forwarder ayarları korunur |
| Diagnostics geçmişi | Korunmaz | Event store yeni kurulumda sıfırlanır |

> [!IMPORTANT]
> Bu tablo teorik değerlendirmeye dayanmaktadır. Gerçek davranış temiz Windows 11 VM ortamında acceptance testi sırasında doğrulanmalıdır (`NOT EXECUTED`).

### NSIS Installer Davranışı

NSIS yükleyicisi aynı `productName = "Vane"` ve `identifier = "com.vane.dpi"` kullandığından:

- **Upgrade senaryosu (2.x üzerine 1.0.0-rc.1):** NSIS genellikle düşük sürümü "downgrade" olarak ele alır. Sessizce eski sürümü kaldırıp yeni sürümü kurabilir veya engel çıkarabilir.
- **Kullanıcı verisi:** Program Files dışındaki AppData klasörü (`%APPDATA%\com.vane.dpi`) genellikle NSIS kaldırma işleminden etkilenmez, ancak bu davranış doğrulanmamıştır.

```text
NSIS downgrade VM testi: NOT EXECUTED
Kullanıcı verisi korunması VM testi: NOT EXECUTED
```

---

## Sürümleme Stratejisi

```text
Mevcut:        1.0.0-rc.1  →  Unsigned release candidate
Sonraki adım:  Clean Windows VM acceptance
               Production code signing
               1.0.0  →  Signed stable release

Bug fix:       1.0.1, 1.0.2
Yeni özellik:  1.1.0
Büyük kırıcı:  2.0.0
```

---

## Git Geçmişi

Eski `2.x` commit geçmişi ve tagleri **korunmaktadır**. Hiçbir commit silinmemiş, force push yapılmamıştır.

```bash
# Mevcut tagleri görüntüle
git tag -l

# Eski v2.1.4 tagine bak
git show v2.1.4
```
