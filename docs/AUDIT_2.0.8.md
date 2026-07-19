# Vane 2.0.8 Architecture and Stability Audit

## Scope

The audit compared `v2.0.0` (`2d37c42`) with the pre-audit `main` head (`41c5f3b`): 14 commits and 29 changed files. It traced Advanced, Pattern, DNS, persistence, logging, and process lifecycle behavior from React through Zustand/Tauri IPC into Rust and the Zapret process.

## Runtime ownership map

| Feature | UI/store source | IPC boundary | Rust/runtime source |
| --- | --- | --- | --- |
| Advanced | `AdvancedConfig`, selected `Preset.args` | preset save/start commands | validated `Preset.args` passed to `winws`/`nfqws` |
| Pattern | bypass mode plus whitelist/blacklist arrays | `sync_bypass_config` | canonical Rust domain list, `domains.txt`, `--hostlist` or `--hostlist-exclude` |
| DNS | protocol, cache, ad filter, proxy, watchdog | DNS sync and forwarder commands | `DnsSettings` cache, local forwarder, OS DNS adapter, optional watchdog |
| Persistence | Zustand partial state | `settings_get/set/remove` | Rust-owned atomic `settings.json` plus last-known-good backup |

## Confirmed root causes and resolutions

### P0: whitelist could fail open

`read_bypass_config` silently converted unreadable or malformed settings into `all`. The engine also accepted an empty whitelist. Persisted Pattern settings are now parsed as a fallible operation; malformed settings stop startup, and an empty whitelist is rejected. A missing settings file remains the explicit first-run `all` default.

### P0: Pattern scope was broader than the UI showed

The frontend silently expanded several domains into service aliases. Rust now owns canonicalization, performs boundary-safe hostname validation, and builds the active list from the verified whitelist/blacklist arrays instead of trusting the legacy `domainList` string. No aliases are added implicitly.

### P0: settings and restart races

Multiple webviews could persist complete stale Zustand snapshots, while runtime readers observed partially replaced files. Rust now owns all settings I/O, serializes writes, atomically replaces the file, retains a last-known-good backup, migrates the schema, and merges only fields changed by each webview. Startup refuses to overwrite unreadable settings with defaults.

### P0: Kill Switch could survive a failed engine start

Firewall command results were ignored and the rule was enabled before several fallible startup operations. Firewall additions now require successful command status, application happens immediately before spawn, and an RAII guard removes the rule if spawn or process-guard setup fails.

### P1: Watchdog toggle and health check were not truthful

The forwarder always started Watchdog regardless of the saved toggle. Its probe used HTTP `HEAD`, which is not a valid DNS-over-HTTPS health check and can receive a healthy endpoint's 4xx response. The toggle is now passed into Rust, returned in verified status, and logged. Health checks perform real DoH wire-format or DoT resolver queries before recovery is triggered.

### P1: Advanced preset round trips lost data

Values were read with `split('=')[1]`, numeric values could become `NaN`, and unmodeled arguments disappeared. Values now retain later `=` characters, integers must be safe and finite, invalid known numeric arguments are omitted with localized warnings, and unknown arguments are preserved for the serializer and revalidated by Rust.

### P1: advertised features did not exist

The package has `winws` and `nfqws`, but no TPWS binary. IPSet had no safe file-import path and normal Windows paths were rejected. These controls are now visibly unavailable and their arguments are rejected by Rust until complete implementations exist.

### P0: DNS state could survive a crash

The previous adapter configuration existed only inside the forwarder handle. A process crash could therefore leave Windows pointing at `127.0.0.1` with no resolver. Vane now writes a versioned atomic restore snapshot before changing adapter DNS, restores and verifies it at the next launch after an interrupted session, and removes it only after a confirmed clean restore.

### P0: DNS controls could report intent instead of runtime truth

Forwarder configuration was cached without consistently restarting the active service, Watchdog state could differ from the toggle, and provider selection could persist before Windows accepted it. Runtime-affecting changes are now serialized, validated, applied/restarted when necessary, reread, emitted, and logged. Repeated identical multi-window hydration no longer restarts the service.

### P1: DNS proxy, cache, and filter gaps

SOCKS5 created clients per query and could resolve upstream names outside the proxy; DoT+proxy did not have a supported fail-closed path. Cache keys omitted query options and expiration did not age returned TTLs. AdBlock downloads were unbounded. SOCKS5H client reuse, protocol incompatibility checks, full-query cache keys, TTL aging, bounded LRU eviction, and streamed/validated filter downloads close these gaps.

### P1: Advanced input and preset authority gaps

Several numeric, port, and cutoff fields could reach Rust with invalid semantics. More importantly, multiple controls emitted flags absent from the bundled winws, including protocol-specific desync overrides, `--dpi-desync2`, `--mss`, and `--bind-addr`. Those controls are now explicitly unavailable, TCP Receiver Window uses the real `--wssize` flag, built-in presets use supported modes, and Rust rejects unsupported modes/flags. Presets also cannot inject hostlists and override the Pattern screen. Unsupported binary payload editing remains disabled rather than pretending arbitrary text is a safe packet payload.

## Verification

The GitHub `Verify` workflow is the build authority for this change. It runs the frontend production build and Rust unit/property tests plus warning-free Clippy on Windows and Linux. No local release build was produced for this patch; the pushed commit is intentionally verified by the workflow.

## Required real-Windows acceptance matrix

These checks require an elevated Windows session and must be completed before declaring a release candidate:

1. Start/stop all built-in presets and verify the child command line, Job Object ownership, and cleanup.
2. Verify whitelist apex/subdomain behavior and confirm an unrelated domain is not bypassed.
3. Verify blacklist exclusion and all-sites behavior with captured traffic.
4. Toggle cache, ad filter, protocol, proxy, Watchdog, and Kill Switch while stopped and running; restart the app and confirm persistence.
5. Force a `winws` spawn failure after Kill Switch enablement and confirm the firewall rule is removed.
6. Simulate three upstream DNS failures and confirm the exact previous static/DHCP adapter state is restored; confirm one or two failures do not alter system DNS.
7. Rapidly change Pattern and DNS controls and confirm only the final revision is applied and logged.
8. Force-kill Vane while the forwarder owns system DNS, relaunch it, and verify the persisted recovery snapshot restores every adapter before auto-start.
9. Verify UDP and TCP DNS, NXDOMAIN/negative responses, large EDNS responses, cache TTL aging, SOCKS5H fail-closed behavior, and filter download rejection.

## Remaining engineering work

- Add elevated Windows integration fixtures for firewall, adapter DNS, and child argv verification.
- Resolve the repository-wide pre-existing Rust formatting debt, then add `cargo fmt --check` to CI.
