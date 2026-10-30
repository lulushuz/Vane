# Security Policy

We take the security of Vane seriously. This document outlines how to report security vulnerabilities and our policy regarding security updates.

## Supported Versions

Only the latest stable release of Vane is supported with security updates. We recommend always running the latest version.

| Version | Supported |
| ------- | --------- |
| v0.2.0+ | ✅ |
| < v0.2.0| ❌ |

## Reporting a Vulnerability

**Please do not report security vulnerabilities via public GitHub issues.**

If you identify a security vulnerability in Vane, please report it privately through one of the following channels:

| Channel | Contact |
|---------|---------|
| 📧 Email | **alp@archey.com.tr** |
| 💬 Discord | **[lulushu](https://discord.com/users/852103749228036136)** (ID: 852103749228036136) |
| 🔒 GitHub | Security → Advisories → New private advisory |

### Reporting Guidelines

1. Include a **detailed description** of the vulnerability.
2. Provide clear **steps to reproduce** the issue.
3. Attach a **Proof of Concept (PoC)** if available.
4. Specify the **affected version(s)** and operating system.

We will acknowledge your report within **48 hours**, validate the vulnerability, and coordinate a release patch. We follow responsible disclosure principles and ask that you give us reasonable time to patch the issue before making it public.

## Security Disclosure Process

1. **Report received** — You receive an acknowledgment within 48 hours.
2. **Validation** — We reproduce and verify the issue.
3. **Patch development** — A fix is developed and tested.
4. **Release** — A new version is published with the patch.
5. **Public disclosure** — After the fix is distributed, the vulnerability may be publicly disclosed.

## Presets & Binary Verification

Vane uses cryptographic signatures to verify presets and binary integrity. The official public key is:

### Minisign Public Key
```text
untrusted comment: minisign public key: 2A7CBD213C2CD2E8
RWTo0iw8Ib18KoSGwlXjG4Hlz+oMjaFhN6077H5nNlTH6KuJogHeUra1
```

You can verify download artifacts and remote preset payloads using this key with the `minisign` tool:

```bash
minisign -Vm <file> -P RWTo0iw8Ib18KoSGwlXjG4Hlz+oMjaFhN6077H5nNlTH6KuJogHeUra1
```
