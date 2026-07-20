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

## Follow-up architecture and security review (2026-07-20)

### Review scope and limits

This follow-up reviewed the complete registered Tauri command surface, capabilities and CSP,
administrator/autostart boundary, process and driver lifecycle, DNS forwarder concurrency,
settings and recovery files, updater and remote preset trust, release workflows, bundled binary
provenance, unsafe Rust blocks, secret-like material, and current JavaScript/Rust dependency
advisories.

The review included:

- Manual static tracing of all 40 registered Tauri commands and their frontend callers.
- Review of every OS command execution site, network listener, external URL, filesystem write,
  and `unsafe` block.
- `npm audit --json`: zero known JavaScript vulnerabilities on 2026-07-20.
- `cargo audit --json`: eight vulnerability advisories in the lockfile plus unmaintained/unsound
  transitive dependency warnings on 2026-07-20.
- Current-tree and targeted Git-history secret pattern checks; no private key or token was found.
- Review of release artifacts and updater signature assets. The Windows installer Authenticode
  check could not be completed because GitHub returned HTTP 503 twice for the asset download.

This was not a penetration test, Windows kernel-driver audit, elevated dynamic acceptance test,
network packet capture, WebView fuzzing campaign, or audit of the upstream Zapret/WinDivert source.
No review can prove the absence of all vulnerabilities. The findings below distinguish confirmed
problems from defense-in-depth recommendations.

### Executive verdict

No confirmed unauthenticated remote-code-execution path or embedded secret was found. The code has
useful defenses: strict CSP script policy, local-only DNS listeners, signed updater metadata,
signed remote presets, argument allowlisting, binary hashes, bounded downloads, and fail-closed
Pattern startup.

The application is nevertheless **not ready to be described as fully security-audited or free of
known vulnerabilities**. Three architectural/supply-chain issues deserve first priority, followed
by several correctness and availability gaps.

### High-priority findings

#### ARCH-H01: the complete WebView application runs as Administrator

`build.rs` requests `requireAdministrator` for the entire Tauri process. Consequently the React
renderer, updater, dialog plugin, both windows, and every registered custom command share the same
elevated process. A WebView/XSS/IPC compromise therefore has an administrator-sized blast radius,
even though the current CSP materially reduces the chance of XSS.

The professional target architecture is an unelevated UI plus a small privileged broker/service.
The broker should expose typed, authenticated, narrowly scoped operations for engine start/stop,
adapter DNS changes, firewall changes, and autostart registration. It should never expose general
shell, arbitrary file, or arbitrary URL operations.

#### ARCH-H02: low-privilege settings are consumed by elevated autostart

The scheduled task runs Vane with highest privileges, while `settings.json`, custom presets, and
the DNS recovery snapshot live in the user's writable application-data directory. Another process
running as that user can modify persisted state before the elevated task starts. Validation blocks
shell injection, but attacker-controlled saved DNS/proxy/runtime choices can still cause the
elevated process to change machine network configuration without a new UAC decision.

Privileged desired state must be stored behind an administrator-only ACL and committed by the
broker after validation. The unelevated UI store should contain presentation preferences only.
The recovery snapshot also needs strict schema validation, adapter identity validation, and a
privileged storage location.

#### SUPPLY-H01: release actions are mutable while holding signing authority

The release workflow references `tauri-apps/tauri-action@v0` and other actions by mutable tags.
That job receives the repository write token and Tauri signing private key. Compromise or retagging
of an action can therefore affect release artifacts or signing material. Pin every action to a
reviewed full commit SHA, use a protected GitHub Environment with manual approval, minimize job
permissions, and keep signing in a separate final job. Generate provenance/SBOM attestations.

### Dependency advisories

`cargo audit` reported the following eight lockfile advisories:

| Advisory | Package | Locked version | Assessment |
| --- | --- | ---: | --- |
| RUSTSEC-2026-0119 | `hickory-proto` | 0.24.4 | Direct DNS path; malicious DNS structures may amplify CPU usage. Upgrade to Hickory 0.26.1 or newer. |
| RUSTSEC-2024-0421 | `idna` | 0.4.0 | Reached through direct `trust-dns-proto`; remove the legacy DNS stack and use a current Hickory stack. |
| RUSTSEC-2026-0194 | `quick-xml` | 0.39.2 | Lockfile advisory; confirm target reachability after dependency updates. |
| RUSTSEC-2026-0195 | `quick-xml` | 0.39.2 | Lockfile advisory; untrusted XML DoS if the affected parser path is used. |
| RUSTSEC-2026-0185 | `quinn-proto` | 0.11.14 | Lockfile advisory; likely optional/target-specific, but must be removed or patched. |
| RUSTSEC-2026-0098 | `rustls-webpki` | 0.101.7 | Reached through Hickory's Rustls 0.21 stack; certificate name-constraint issue. |
| RUSTSEC-2026-0099 | `rustls-webpki` | 0.101.7 | Reached through Hickory's Rustls 0.21 stack; wildcard name-constraint issue. |
| RUSTSEC-2026-0104 | `rustls-webpki` | 0.101.7 | Reached through Hickory's Rustls 0.21 stack; reachable panic in affected CRL parsing paths. |

The audit also reported unsound/unmaintained transitive packages, including `anyhow 1.0.102`,
`glib 0.18.5`, `rand 0.7.3`, the GTK 0.18 family, and `trust-dns-proto 0.23.2`. Some are
platform-specific or may not be reachable at runtime, but a clean release policy should fail CI on
reachable vulnerabilities and explicitly document temporary exceptions.

Add `cargo audit` and `npm audit` to CI, add Dependabot/Renovate, and consolidate the two DNS
protocol families (`trust-dns-proto` and `hickory-resolver`) into one maintained version line.

### Other confirmed design and correctness findings

#### SEC-M01: custom command authority is not separated by window

The `main` and `settings` windows share one capability. Registered application commands are not
split into least-privilege permission groups, so a compromise in either renderer can reach DNS,
firewall-adjacent, engine, updater, preset, autostart, and export operations. Create distinct
capabilities/permissions and also enforce the caller window label in sensitive Rust commands.

#### SEC-M02: arbitrary health-check URLs create an SSRF primitive

`check_url_health` accepts user-controlled `http://` or `https://` targets and does not reject
loopback, private, link-local, or cloud metadata addresses. The feature is user initiated, but a
compromised renderer could scan local services through the elevated process. Accept canonical
public hostnames only, reject redirects to non-public addresses, and disable plain HTTP.

#### SEC-M03: preset export is an arbitrary `.vane` file write

`export_preset` trusts a renderer-provided absolute path and only checks the extension and JSON
shape. A compromised renderer can overwrite any administrator-writable `.vane` path. Move the
save-dialog selection and write authorization into one backend transaction, or issue a one-time
scoped path token after a user gesture.

#### DNS-M01: IPv6 DNS adapter state is not owned by the forwarder

Adapter discovery, apply, snapshot, and restore cover IPv4 DNS only. Windows may retain/use an IPv6
resolver. The Kill Switch blocks TCP/UDP port 53 over IPv6 when enabled, but the encrypted forwarder
without Kill Switch cannot claim complete leak protection. Snapshot and configure both address
families, then verify both. Documentation must also disclose that application-managed DoH/DoQ on
port 443 is outside a port-53 Kill Switch.

#### DNS-M02: adapter changes are not transactional

`apply_dns`, DHCP reset, and snapshot restore iterate adapters. If a later adapter fails, earlier
adapters remain modified and no automatic rollback occurs. Capture a pre-operation snapshot,
apply all changes, verify all adapters, and roll back every changed adapter on any failure.

#### DNS-M03: detached DNS tasks can outlive the forwarder handle

Stopping the forwarder aborts the listener task, but per-query UDP tasks and accepted TCP connection
tasks are detached. A TCP client can keep a connection alive indefinitely; 100 idle local clients
can also consume all permits. Use a shared cancellation token, a `JoinSet`, per-read idle timeouts,
maximum queries per connection, and await child-task shutdown before reporting `stopped`.

#### DNS-M04: repeated filter initialization can race

Repeated AdBlock enable/sync operations can spawn concurrent downloads that use the same temporary
cache path. Add a single-flight refresh lock and generation/cancellation tracking.

#### ENGINE-M01: optimizer bypasses the normal engine authority path

The optimizer spawns bundled binaries directly instead of using `EngineManager`. It does not use
the manager's binary hash check, canonical argument preparation, engine state serialization, or
normal rollback path; Job Object creation/assignment errors are silently discarded. It can also
conflict with an already running engine. Introduce one `EngineRuntime` owner and make optimization
lease it for controlled test sessions.

#### ENGINE-M02: name-based startup cleanup can terminate unrelated software

Startup uses `taskkill /IM winws-x86_64-pc-windows-msvc.exe` and may delete the generic `WinDivert`
service when it appears stale. Another application can legitimately use the same executable name
or service. Track Vane-owned process IDs/job identity and use a Vane-specific driver/service name
where upstream permits; never delete a resource based only on a shared name.

#### SEC-L01: Windows token handle leak

`is_elevated` opens a process token and does not close the returned handle. Wrap it in RAII or call
`CloseHandle` on every path. `JobObjectGuard::assign` also requests `PROCESS_ALL_ACCESS`; replace it
with only the rights required for job assignment.

### Professionalism and unnecessary complexity findings

- `commands.rs` (about 1,000 lines), `engine/manager.rs` (about 1,180),
  `dns/forwarder.rs` (about 926), and `engineStore.ts` (about 856) are oversized authority hubs.
- `start_engine` and `start_engine_with_dns_guard` expose competing orchestration paths. Keep one
  transactional start use case.
- Runtime truth exists in Zustand, Rust caches, files, process handles, and Windows state. Replace
  ad-hoc synchronization with explicit state machines and revisioned command results.
- `domainList` duplicates whitelist/blacklist arrays. Unsupported Advanced fields and the disabled
  `doq` union remain in the persisted schema. Remove legacy/dead state through a schema migration.
- Frontend log localization is implemented by regex-rewriting English Rust strings. Emit stable
  event codes plus structured parameters and translate only at the UI boundary.
- Source comments such as `CVE-2` and `CVE-5` are internal feature labels, not assigned CVE IDs.
  They are misleading and should be replaced with descriptive threat names.
- README claims complete DoH/DoT/DoQ support although DoQ is disabled and mapped to DoH in legacy
  state handling. It also attributes Zapret engine internals such as conntrack to Vane. Separate
  implemented Vane features from upstream engine capabilities.
- `src-tauri/err.txt` and `src-tauri/errors.txt` are obsolete build-error artifacts committed to
  the repository and should be removed.
- The release workflow uses `npm install` and floating Node LTS instead of a pinned toolchain and
  `npm ci`. Release inputs are therefore less reproducible than CI inputs.
- Bundled Zapret, WinDivert, and Cygwin artifacts lack a machine-readable provenance manifest,
  source version, download URL, license inventory, and SBOM. Hardcoded runtime hashes are useful
  tamper checks but do not replace provenance or code review.
- No frontend unit/component/E2E tests exist. Rust has useful unit/property coverage, but there are
  no elevated Windows integration fixtures or fault-injection tests.
- CI does not run a full Tauri bundle smoke test, `cargo fmt --check`, dependency audit, secret scan,
  CodeQL/SAST, SBOM generation, or installer signature verification.

### Recommended target architecture

1. **Unelevated presentation process** — React/Tauri UI, translations, non-sensitive preferences.
2. **Privileged broker** — narrow authenticated operations; administrator-only desired state and
   recovery journal; no WebView and no arbitrary shell/file primitives.
3. **Application use cases** — `StartBypass`, `StopBypass`, `ApplyDnsTransaction`,
   `ChangePatternTransaction`, `RunOptimizationSession`, and `RecoverInterruptedSession`.
4. **Domain types/invariants** — typed `DomainRule`, `DnsAddress`, `ProxyEndpoint`, `PresetArgs`,
   `AdapterId`, and versioned IPC DTOs instead of flat strings.
5. **Platform adapters** — Windows DNS/firewall/process implementations behind traits, with fake
   adapters for deterministic tests.
6. **Single runtime state machine** — explicit stopped/starting/running/recovering/stopping/failed
   states, revision numbers, cancellation, and observable verified state.
7. **Transactional persistence** — administrator-protected journal for privileged mutations and
   schema-migrated user preference storage for UI state.
8. **Hardened release chain** — immutable action SHAs, protected environment, deterministic installs,
   dependency policy, SBOM/provenance, updater signature and Windows Authenticode verification.

### Prioritized remediation order

1. Patch/remove RustSec-affected DNS/TLS dependencies and add audit gates.
2. Pin release actions and protect signing/release credentials.
3. Design the unelevated UI + privileged broker boundary and protected privileged state.
4. Fix IPv6 DNS ownership and make adapter operations transactional.
5. Make DNS child tasks cancellable and optimizer sessions use the single engine runtime owner.
6. Replace name-based process/service cleanup and close the token handle leak.
7. Split IPC permissions by window; constrain health-check and export commands.
8. Remove dead/duplicate state, correct documentation claims, and introduce structured event codes.
9. Add elevated Windows integration, frontend, release smoke, and fault-injection tests.
