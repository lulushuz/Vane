<p align="center">
  <img src="src-tauri/icons/icon.png" width="160" alt="Vane Logo">
</p>

<h1 align="center">Vane DPI</h1>

<p align="center">
  <strong>مرکز کنترل پیشرفته دور زدن DPI و DNS رمزگذاری‌شده (قدرت‌گرفته از Zapret)</strong>
</p>

<p align="center">
  <a href="README.md"><img src="https://img.shields.io/badge/English-EN-blue?style=flat-square&labelColor=black" alt="English"></a>
  <a href="README.tr.md"><img src="https://img.shields.io/badge/Türkçe-TR-red?style=flat-square&labelColor=black" alt="Türkçe"></a>
  <a href="README.ru.md"><img src="https://img.shields.io/badge/Русский-RU-blue?style=flat-square&labelColor=black" alt="Русский"></a>
  <a href="README.zh.md"><img src="https://img.shields.io/badge/简体中文-ZH-yellow?style=flat-square&labelColor=black" alt="简体中文"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/lulushuz/Vane/releases.yml?style=flat-square&label=ساخت" alt="Build Status">
  <img src="https://img.shields.io/github/license/lulushuz/Vane?style=flat-square&color=blue" alt="License">
  <img src="https://img.shields.io/github/v/release/lulushuz/Vane?style=flat-square&color=blue&label=نسخه" alt="Release Version">
  <a href="https://discord.com/users/852103749228036136"><img src="https://img.shields.io/badge/Discord-lulushu-5865F2?style=flat-square&logo=discord&logoColor=white" alt="Discord"></a>
</p>

<p align="center">
  <img src="docs/assets/preview.png" width="580" alt="پیش‌نمایش محیط Vane">
</p>

---

## ۱. Vane چیست؟

برنامه **Vane** یک نرم‌افزار دسکتاپ گرافیکی و متن‌باز است که موتور قدرتمند [zapret](https://github.com/bol-van/zapret) را مدیریت می‌کند. این نرم‌افزار سرویس‌های سطح پایین `winws` در ویندوز و `nfqws` در لینوکس را در قالب یک محیط سبک، زیبا و امن مبتنی بر Tauri v2، Rust و React 19 در اختیار شما قرار می‌دهد.

**نکته بسیار مهم: Vane یک فیلترشکن (VPN) نیست.** این برنامه ترافیک اینترنت شما را به هیچ سرور واسط یا پروکسی تجاری خارجی هدایت نمی‌کند. Vane مستقیماً روی رایانه شما اجرا شده و بسته‌های شروع ارتباط اینترنت (Handshake) را به شکلی دستکاری می‌کند که تجهیزات فیلترینگ DPI ارائه‌دهندگان اینترنت دچار سردرگمی شده و مسدودسازی خنثی شود. بنابراین هیچ‌گونه افت سرعت یا افزایش پینگ نخواهید داشت.

---

## ۲. ویژگی‌های کلیدی

- **DNS Guard محلی و رمزگذاری‌شده:** فعال‌سازی سرویس DoH و DoT روی آدرس `127.0.0.127:5353` به همراه کش رم و بازیابی خودکار.
- **مسدودسازی تبلیغات (AdBlock):** ادغام مستقیم با لیست [StevenBlack](https://github.com/StevenBlack/hosts) جهت مسدودسازی بیش از ۱۰۰،۰۰۰ دامنه رهگیری و تبلیغاتی بدون مصرف ترافیک.
- **سوئیچ قطع اضطراری (Dual-Stack Kill Switch):** مسدودسازی پورت ۵۳ معمولی در IPv4 و IPv6 جهت جلوگیری از لو رفتن درخواست‌های DNS.
- **نگهبان خودکار (Watchdog):** نظارت دائمی بر پایداری ارتباط و راه‌اندازی مجدد در صورت قطع ارتباط سرویس‌های هدف (مانند دیسکورد).
- **بهینه‌ساز پریست‌ها (Preset Optimizer):** آزمایش خودکار تمام تنظیمات برای انتخاب سریع‌ترین پریست با کمترین تاخیر در شبکه شما.
- **پشتیبانی از پروکسی SOCKS5:** امکان عبور درخواست‌های رمزنگاری‌شده DoH از پروکسی در شبکه‌هایی که سرورهای اصلی DNS فیلتر شده‌اند.

---

## ۳. پریست‌های پیش‌فرض

| شناسه | نام پریست | پروتکل‌ها | استراتژی | کاربرد |
|---|---|---|---|---|
| `tr-1` | **TR Standard** | TCP 80, 443 | `fake,multidisorder` | وبگردی عادی و شبکه‌های اجتماعی |
| `tr-2` | **TR Aggressive** | TCP 80, 443 | `fake` + `md5sig` + TTL | شبکه‌های دارای بررسی عمیق TCP |
| `tr-3` | **TR Fragment** | TCP 80, 443 | `multisplit` | باز کردن گواهی‌های سخت‌گیرانه TLS |
| `tr-4` | **TR Desync-HTTPS**| TCP 443 | تقسیم رکورد SNI | شبکه‌های مسدودکننده اختصاصی HTTPS |
| `tr-5` | **TR QUIC** | UDP 443 | `fakeknown` + `udplen` | استریم ویدیو با پروتکل QUIC و YouTube 4K |
| `discord-voip` | **Discord & VoIP**| UDP 50000-65535 | تزریق بسته‌های فیک UDP | مکالمات صوتی دیسکورد و بازی‌های آنلاین |

---

## ۴. دریافت و نصب

نسخه‌های کامپایل‌شده رسمی را می‌توانید از بخش [GitHub Releases](https://github.com/lulushuz/Vane/releases) دانلود کنید:
- **ویندوز:** دریافت و نصب فایل `Vane_0.2.0_x64-setup.exe` یا بسته `.msi`.
- **لینوکس:** دریافت بسته `vane_0.2.0_amd64.deb` یا نسخه قابل حمل `.AppImage`.

```bash
# نصب روی دبیان / اوبونتو:
sudo dpkg -i vane_0.2.0_amd64.deb

# اجرای نسخه پرتابل AppImage:
chmod +x Vane_0.2.0_amd64.AppImage
./Vane_0.2.0_amd64.AppImage
```

---

## ۵. مجوز و ارتباط

- این پروژه تحت مجوز **GNU General Public License v3.0** (`GPL-3.0`) منتشر شده است.
- **تماس در دیسکورد:** [lulushu (Discord ID: 852103749228036136)](https://discord.com/users/852103749228036136)
- **ثبت اشکال و باگ:** [GitHub Issues](https://github.com/lulushuz/Vane/issues)
