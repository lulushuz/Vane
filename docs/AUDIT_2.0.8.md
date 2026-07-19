# Vane 2.0.8 Architecture and Stability Audit

## Scope

The audit compared `v2.0.0` (`2d37c42`) with the pre-audit `main` head (`41c5f3b`): 14 commits and 29 changed files. It traced Advanced, Pattern, DNS, persistence, logging, and process lifecycle behavior from React through Zustand/Tauri IPC into Rust and the Zapret process.

## Runtime ownership map

| Feature | UI/store source | IPC boundary | Rust/runtime source |
| --- | --- | --- | --- |
| Advanced | `AdvancedConfig`, selected `Preset.args` | preset save/start commands | validated `Preset.args` passed to `winws`/`nfqws` |
| Pattern | bypass mode plus whitelist/blacklist arrays | `sync_bypass_config` | canonical Rust domain list, `domains.txt`, `--hostlist` or `--hostlist-exclude` |
| DNS | protocol, cache, ad filter, proxy, watchdog | DNS sync and forwarder commands | `DnsSettings` cache, local forwarder, OS DNS adapter, optional watchdog |
| Persistence | Zustand partial state | Tauri Store | `settings.json` (single writer after this audit) |

## Confirmed root causes and resolutions

### P0: whitelist could fail open

`read_bypass_config` silently converted unreadable or malformed settings into `all`. The engine also accepted an empty whitelist. Persisted Pattern settings are now parsed as a fallible operation; malformed settings stop startup, and an empty whitelist is rejected. A missing settings file remains the explicit first-run `all` default.

### P0: Pattern scope was broader than the UI showed

The frontend silently expanded several domains into service aliases. Rust now owns canonicalization, performs boundary-safe hostname validation, and builds the active list from the verified whitelist/blacklist arrays instead of trusting the legacy `domainList` string. No aliases are added implicitly.

### P0: settings and restart races

Zustand Store and Rust independently rewrote the same JSON file. Debounced Pattern/DNS IPC calls could overlap, and old responses could update the UI after newer choices. Rust file writes were removed, Store writes are serialized, UI responses carry an in-memory revision guard, and backend Pattern/DNS synchronization is mutex-serialized.

### P0: Kill Switch could survive a failed engine start

Firewall command results were ignored and the rule was enabled before several fallible startup operations. Firewall additions now require successful command status, application happens immediately before spawn, and an RAII guard removes the rule if spawn or process-guard setup fails.

### P1: Watchdog toggle and health check were not truthful

The forwarder always started Watchdog regardless of the saved toggle. Its probe used HTTP `HEAD`, which is not a valid DNS-over-HTTPS health check and can receive a healthy endpoint's 4xx response. The toggle is now passed into Rust, returned in verified status, and logged. Health checks perform real DoH wire-format or DoT resolver queries before recovery is triggered.

### P1: Advanced preset round trips lost data

Values were read with `split('=')[1]`, numeric values could become `NaN`, and unmodeled arguments disappeared. Values now retain later `=` characters, integers must be safe and finite, invalid known numeric arguments are omitted with localized warnings, and unknown arguments are preserved for the serializer and revalidated by Rust.

### P1: advertised features did not exist

The package has `winws` and `nfqws`, but no TPWS binary. IPSet had no safe file-import path and normal Windows paths were rejected. These controls are now visibly unavailable and their arguments are rejected by Rust until complete implementations exist.

## Automated evidence

- `cargo test --lib`: 22 unit/property tests pass.
- `cargo clippy --lib -- -D warnings`: passes.
- `npm run build`: TypeScript strict compilation and Vite production build pass.
- `.github/workflows/ci.yml`: repeats frontend validation and Rust validation on Windows and Linux.

The administrator-manifest application binary itself is intentionally excluded from `cargo test`; Windows returns elevation error 740 when Cargo tries to execute that test harness. Library tests cover the deterministic logic without requiring elevation.

## Required real-Windows acceptance matrix

These checks require an elevated Windows session and must be completed before declaring a release candidate:

1. Start/stop all built-in presets and verify the child command line, Job Object ownership, and cleanup.
2. Verify whitelist apex/subdomain behavior and confirm an unrelated domain is not bypassed.
3. Verify blacklist exclusion and all-sites behavior with captured traffic.
4. Toggle cache, ad filter, protocol, proxy, Watchdog, and Kill Switch while stopped and running; restart the app and confirm persistence.
5. Force a `winws` spawn failure after Kill Switch enablement and confirm the firewall rule is removed.
6. Simulate three upstream DNS failures and confirm DHCP recovery; confirm one or two failures do not alter system DNS.
7. Rapidly change Pattern and DNS controls and confirm only the final revision is applied and logged.

## Remaining engineering work

- Return structured configuration revisions from IPC instead of relying only on frontend revision guards.
- Add elevated Windows integration fixtures for firewall, adapter DNS, and child argv verification.
- Resolve the repository-wide pre-existing Rust formatting debt, then add `cargo fmt --check` to CI.
