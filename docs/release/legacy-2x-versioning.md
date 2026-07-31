# Legacy 2.x Development Releases

## English

Vane's previous `2.x` releases (2.0.0 through 2.1.4) were development-stage builds created before the **P00–P15 production-hardening program**.

During that period the project was in active architectural development and had not yet reached its first fully hardened, production-ready state. The version numbers assigned then do not reflect a mature `2.x → 3.x` product line — they were simply incremental development snapshots.

The project's **official production versioning begins with `1.0.0-rc.1`**, which represents the first release candidate built on the hardened P00–P15 architecture.

**Key facts:**
- The Git history, old tags, and old GitHub releases are fully preserved.
- No commits were deleted, rebased, or force-pushed.
- Users of old 2.x builds may need to install `1.0.0-rc.1` manually because SemVer comparison treats `1.0.0` as lower than `2.1.4` — the auto-updater will not offer this as an upgrade.
- All application settings stored in AppData are expected to be preserved on manual reinstall (unverified — pending VM acceptance test).

---

## Türkçe

Vane'in önceki `2.x` sürümleri (2.0.0'dan 2.1.4'e kadar), **P00–P15 production-hardening programı** tamamlanmadan önce oluşturulan geliştirme aşaması yapılarıdır.

O dönemde proje aktif mimari geliştirme sürecindeydi ve henüz tam anlamıyla sertleştirilmiş, production-ready duruma ulaşmamıştı. O sırada atanan sürüm numaraları olgun bir `2.x → 3.x` ürün hattını yansıtmaz — bunlar yalnızca artımlı geliştirme anlık görüntüleriydi.

Projenin **resmî production sürümlemesi `1.0.0-rc.1` ile başlamaktadır**; bu sürüm, sertleştirilmiş P00–P15 mimarisi üzerine inşa edilmiş ilk release candidate'i temsil etmektedir.

**Önemli bilgiler:**
- Git geçmişi, eski tagler ve eski GitHub release'leri tamamen korunmaktadır.
- Hiçbir commit silinmemiş, rebase edilmemiş veya force push yapılmamıştır.
- Eski `2.x` sürümlerini kullanan kullanıcıların `1.0.0-rc.1`'i manuel olarak kurması gerekebilir; SemVer karşılaştırması `1.0.0`'ı `2.1.4`'ten düşük olarak kabul ettiğinden otomatik güncelleyici bu sürümü sunamaz.
- AppData'daki uygulama ayarlarının manuel yeniden kurulumda korunması beklenmektedir (doğrulanmamış — VM kabul testi beklemektedir).
