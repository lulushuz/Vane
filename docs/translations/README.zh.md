<p align="center">
  <img src="src-tauri/icons/icon.png" width="160" alt="Vane Logo">
</p>

<h1 align="center">Vane DPI</h1>

<p align="center">
  <strong>基于 Zapret 引擎的高级 DPI 绕过与加密 DNS 控制中心</strong>
</p>

<p align="center">
  <a href="../../README.md"><img src="https://img.shields.io/badge/English-EN-blue?style=flat-square&labelColor=black" alt="English"></a>
  <a href="README.tr.md"><img src="https://img.shields.io/badge/Türkçe-TR-red?style=flat-square&labelColor=black" alt="Türkçe"></a>
  <a href="README.ru.md"><img src="https://img.shields.io/badge/Русский-RU-blue?style=flat-square&labelColor=black" alt="Русский"></a>
  <a href="README.fa.md"><img src="https://img.shields.io/badge/فارسی-FA-green?style=flat-square&labelColor=black" alt="فارسی"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/lulushuz/Vane/releases.yml?style=flat-square&label=构建状态&labelColor=black" alt="Build Status">
  <img src="https://img.shields.io/github/license/lulushuz/Vane?style=flat-square&color=blue&labelColor=black" alt="License">
  <img src="https://img.shields.io/github/v/release/lulushuz/Vane?style=flat-square&color=blue&label=版本&labelColor=black" alt="Release Version">
  <a href="https://discord.com/users/852103749228036136"><img src="https://img.shields.io/badge/Discord-lulushu-5865F2?style=flat-square&logo=discord&logoColor=white&labelColor=black" alt="Discord"></a>
</p>

<p align="center">
  <img src="../assets/VanePre.png" width="580" alt="Vane 桌面客户端预览">
</p>

---

## 目录

- [1. 什么是 Vane？](#1-什么是-vane)
  - [Vane 实现的自动化](#vane-实现的自动化)
- [2. 核心功能](#2-核心功能)
  - [2.1. DNS Guard: 本地加密 DNS 解析器](#21-dns-guard-本地加密-dns-解析器)
  - [2.2. AdBlock DNS 广告与遥测拦截](#22-adblock-dns-广告与遥测拦截)
  - [2.3. 双栈 (Dual-Stack) DNS Kill Switch](#23-双栈-dual-stack-dns-kill-switch)
  - [2.4. 自动故障恢复看门狗 (Watchdog)](#24-自动故障恢复看门狗-watchdog)
  - [2.5. 预设优化器 (Preset Optimizer)](#25-预设优化器-preset-optimizer)
  - [2.6. SOCKS5 上游代理](#26-socks5-上游代理)
  - [2.7. 远程预设配置同步](#27-远程预设配置同步)
  - [2.8. 分类日志控制台](#28-分类日志控制台)
  - [2.9. 平台原生自启集成](#29-平台原生自启集成)
  - [2.10. 网络环境变动检测](#210-网络环境变动检测)
- [3. 安全机制与架构设计](#3-安全机制与架构设计)
- [4. 内置预设与快速绕过指南](#4-内置预设与快速绕过指南)
- [5. 防火墙管道架构 (Linux 与 Windows)](#5-防火墙管道架构-linux-与-windows)
- [6. 安装与源码编译](#6-安装与源码编译)
- [7. 常见问题排查与开源许可](#7-常见问题排查与开源许可)

---

## 1. 什么是 Vane？

Vane 是一款用于管理 [zapret](https://github.com/bol-van/zapret) 深度数据包检测 (DPI) 规避引擎的开源桌面图形应用。它将底层的 `winws` (Windows) 和 `nfqws` (Linux) 守护进程包装为基于 Tauri v2、Rust、React 19 和 Zustand 构建的高性能、轻量级控制面板。

**Vane 并不是 VPN。** 它绝不会将您的网络流量转发到任何第三方远程服务器或商业代理节点。相反，它直接运行在您的本地计算机上，在网络底层捕获并发起对握手数据包（TCP/TLS 与 UDP）的微小混淆操作。这些修改能有效干扰运营商部署的 DPI 审查设备，在完全保留您原生带宽和 Ping 延迟的前提下，解锁受限的网络服务。

### Vane 实现的自动化

| 任务 | 手动配置 Zapret | 使用 Vane |
|---|---|---|
| **二进制完整性** | 未校验的本地文件 | Minisign 签名 + SHA-256 + 进程启动前 TOCTOU 校验 |
| **引擎执行** | 包含 20 多个参数的复杂终端指令 | 一键图形化启停与实时状态管理 |
| **DNS 加密** | 手动修改系统 stub 解析器 | 内置 DoH/DoT 本地解析器与事务性回滚 |
| **DNS 泄漏防护** | 依赖外部工具 / 频繁泄漏 | 双栈 (IPv4 + IPv6) 硬件级 Kill Switch |
| **防火墙配置** | 手动安装驱动或复杂 iptables 规则 | 全自动 nftables / iptables / WinDivert 规则集成 |
| **崩溃状态清理** | 残留失效规则导致断网 | Windows Job Object 自动终止 + Linux 孤立规则恢复 |
| **预设参数管理** | 手动修改 shell/bat 脚本 | 可视化编辑器、`.vane` 打包导入导出、签名远程同步 |
| **网络连通性诊断** | 手动 curl/ping 测试 | 一键 HTTP 连通性测试与隐私脱敏诊断包导出 |

---

## 2. 核心功能

### 2.1. DNS Guard: 本地加密 DNS 解析器

DNS Guard 在本地 `127.0.0.127:5353` 上运行嵌入式解析器，捕获来自系统和浏览器的明文 UDP/53 DNS 查询，并通过高安全加密信道转发：
- **Cloudflare** (DoH / DoT): `https://cloudflare-dns.com/dns-query`
- **Google** (DoH / DoT): `https://dns.google/dns-query`
- 支持基于 TTL 的高速内存缓存、本地保留域名解析 (`.local`, `.lan`) 以及失败自动回滚机制 (`DnsTransactionManager`)。

### 2.2. AdBlock DNS 广告与遥测拦截

集成 [StevenBlack hosts](https://github.com/StevenBlack/hosts) 规则库，收录超过 100,000 个广告跟踪与恶意域名。命中黑名单时立即本地返回 `0.0.0.0`，杜绝任何外部网络泄露。

### 2.3. 双栈 (Dual-Stack) DNS Kill Switch

- **Windows:** 通过 `netsh advfirewall` 阻断针对除本地环回接口 (`127.0.0.1` 和 `::1`) 之外所有外部 IPv4/IPv6 地址的明文 53 端口通信。
- **Linux:** 在 `OUTPUT` 链中通过高权限执行器注入 `iptables` / `ip6tables` 阻断规则。

### 2.4. 自动故障恢复看门狗 (Watchdog)

后台服务持续探测核心网络目标（默认：`discord.com`）。检测到引擎崩溃或网络阻断失效时，自动重新初始化服务或提示运行优化预设。

### 2.5. 预设优化器 (Preset Optimizer)

自动轮询预设库中的各个配置项，针对真实 HTTP 目标进行并发连通性测试，自动筛选出当前网络环境下延迟最低、稳定性最高的预设。

### 2.6. SOCKS5 上游代理

针对 DoH 官方端点本身遭到 IP 阻断的严苛网络环境，支持将 DNS Guard 查询通过经过认证的 SOCKS5 代理安全中继。

---

## 3. 安全机制与架构设计

```
+--------------------------------------------------------------------------+
|                                 Vane 架构                                |
+--------------------------------------------------------------------------+
|  [ 前端界面: React 19 + Zustand ]                                        |
|         |                                                                |
|  [ Tauri IPC 安全桥 ] -- (严格 CSP、文件路径白名单、SSRF 拦截)            |
|         |                                                                |
|  [ Rust 核心: EngineManager, DnsTransactionManager, DiagnosticsStore ]   |
|         |                                                                |
|   +-----+--------------------------------+                               |
|   | (Windows)                            | (Linux)                       |
|   v                                      v                               |
| [ WinDivert64.sys 内核驱动 ]       [ nftables / iptables 引擎 ]          |
| [ winws.exe 守护进程 ]             [ nfqws 守护进程 (独立 setcap) ]      |
| [ Windows Job Object (自动销毁) ]  [ 孤立规则恢复模块 ]                  |
+--------------------------------------------------------------------------+
```

- **Linux 特权完全隔离:** `setcap cap_net_admin,cap_net_raw+ep` 权限严格限制在独立的 `nfqws` 二进制文件上，主 Tauri 进程保持完全无特权，确保 WebKitGTK 沙盒安全。
- **TOCTOU 防篡改校验:** 执行文件启动前瞬间读取并计算 SHA-256，防止时间差替换攻击。

---

## 4. 内置预设与快速指南

| 预设 ID | 预设名称 | 适用协议 | 规避策略 | 推荐使用场景 |
|---|---|---|---|---|
| `tr-1` | **TR Standard** | TCP 80, 443 | `fake,multidisorder` + `autottl` + `badseq` | 网页浏览、常规媒体站点 |
| `tr-2` | **TR Aggressive** | TCP 80, 443 | `fake,multidisorder` + `md5sig` + 固定 TTL | 具有强状态 TCP 重组的运营商 |
| `tr-3` | **TR Fragment** | TCP 80, 443 | `multisplit` + `fakedsplit` | 深度 TLS 特征检测网络 |
| `tr-4` | **TR Desync-HTTPS**| TCP 443 | SNI 记录拆分 | 仅拦截 HTTPS 流量的网络 |
| `tr-5` | **TR QUIC** | UDP 443 | `fakeknown` + `udplen` | YouTube 4K (QUIC/HTTP3) 流媒体 |
| `discord-voip` | **Discord & VoIP** | UDP 50000-65535 | UDP 伪造包注入 | Discord 语音频道及游戏语音通话 |

---

## 5. 防火墙管道架构

- **Windows:** 采用通过数字签名的 `WinDivert64.sys` 驱动，并绑定 Windows Job Object (`KILL_ON_JOB_CLOSE`)，客户端关闭时内核自动注销过滤规则。
- **Linux:** 优先使用现代 Linux 内核原生支持的 `nftables` 事务化引擎，豁免本地环回 (`oifname "lo" return`)，并在旧版系统中优雅降级至 `iptables`。

---

## 6. 安装与源码编译

### 6.1. 系统要求

| 操作系统 | 最低系统版本 | 系统依赖 | 运行权限 |
|---|---|---|---|
| **Windows** | Windows 10 (1809+) 或 Windows 11 | 无 (内嵌 WinDivert 驱动) | 管理员权限 (UAC) |
| **Linux** | Ubuntu 22.04+, Debian 12+, Arch, Fedora | `libwebkit2gtk-4.1`, `nftables` / `iptables` | 普通用户 (`pkexec` 授权 nfqws) |

### 6.2. 安装包下载

最新编译成品可在 [GitHub Releases](https://github.com/lulushuz/Vane/releases) 页面获取：
- **Windows:** 下载并运行 `Vane_0.2.0_x64-setup.exe` 或 `.msi`。
- **Linux:** 下载 `vane_0.2.0_amd64.deb` 或免安装的 `.AppImage`。

```bash
# Debian / Ubuntu 安装:
sudo dpkg -i vane_0.2.0_amd64.deb

# AppImage 运行:
chmod +x Vane_0.2.0_amd64.AppImage
./Vane_0.2.0_amd64.AppImage
```

### 6.3. 源码构建

```bash
git clone https://github.com/lulushuz/Vane.git
cd Vane
npm install
npm run tauri dev
```

---

## 7. 常见问题排查与开源许可

- **Windows 引擎启动失败:** 请检查并完全退出 GoodbyeDPI 或其它正在占用 WinDivert 驱动的软件。
- **DNS 无法解析:** 若上游 DoH IP 遭到运营商阻断，请在设置中开启 SOCKS5 代理中继。
- **开源许可:** 本项目基于 **GNU General Public License v3.0** (`GPL-3.0`) 授权。详情参见 [LICENSE](LICENSE)。
- **Discord 联络:** [lulushu (Discord ID: 852103749228036136)](https://discord.com/users/852103749228036136)
