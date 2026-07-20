# Vane Security, Architecture, and Reliability Remediation Plan

Date: 2026-07-20  
Source audit: `docs/AUDIT_2.0.8.md`

## Objective

Close every confirmed finding in the 2.0.8 audit, remove misleading or dead functionality, and
make security-sensitive behavior observable and testable. A finding is not considered closed when
the UI merely reports success; the resulting process, adapter, firewall, DNS, file, or scheduled-task
state must be verified.

## Branch and integration policy

New branches will not use a `codex/` prefix. Use purpose-based names:

- `security/<scope>` for vulnerability and trust-boundary work.
- `fix/<scope>` for isolated correctness defects.
- `refactor/<scope>` for behavior-preserving architectural work.
- `test/<scope>` for verification infrastructure.
- `docs/<scope>` for documentation-only changes.

Each branch should close one coherent risk group, include its tests and migration behavior, and be
merged only after its GitHub Actions checks pass. Avoid a single all-or-nothing hardening branch.
Security-sensitive changes require a second review of the exact diff before merge.

## Delivery sequence

### Phase 0 - Baseline and regression harness

Branch: `test/security-baseline`

Work:

1. Add frontend unit/component tests and Rust integration test structure.
2. Add deterministic fake adapters for process, DNS, firewall, filesystem, and scheduled-task code.
3. Add Windows-only elevated integration jobs for adapter apply/restore and process cleanup.
4. Record end-to-end fixtures for Pattern whitelist/blacklist, persisted settings, autostart,
   Smart DNS Cache, proxy, filters, Kill Switch, and watchdog behavior.
5. Add fault injection for partial DNS failure, corrupted settings, interrupted writes, forwarder
   cancellation, unavailable upstream DNS, and stale processes.

Exit criteria:

- Current expected behavior has repeatable tests.
- Tests fail when a known audit defect is deliberately reintroduced.
- CI preserves diagnostic artifacts without committing local error logs.

### Phase 1 - Dependency and release-chain security

Branch: `security/dependencies-release`

Closes: dependency advisories and `SUPPLY-H01`.

Work:

1. Remove the legacy `trust-dns-proto` line and consolidate DNS libraries on a maintained Hickory
   version that fixes the reported advisories.
2. Upgrade or remove affected `hickory-proto`, `idna`, `quick-xml`, `quinn-proto`,
   `rustls-webpki`, and affected transitive packages; document any target-inactive exception.
3. Make `cargo audit` and `npm audit` release gates with reviewed exception files and expiry dates.
4. Pin GitHub Actions to reviewed full commit SHAs, minimize job token permissions, and move signing
   to a protected final release job/environment.
5. Use pinned Node/Rust toolchains and `npm ci`.
6. Generate SBOM and build provenance; add a machine-readable inventory for Zapret, WinDivert,
   Cygwin, their versions, sources, licenses, and hashes.
7. Verify updater signatures and Windows Authenticode signatures in the workflow before publishing.

Exit criteria:

- No reachable RustSec or npm advisory remains without a documented, time-limited exception.
- Release jobs use immutable action revisions and least-privilege permissions.
- Unsigned, incorrectly signed, or provenance-mismatched artifacts cannot be released.

### Phase 2 - Narrow IPC and renderer attack surface

Branch: `security/ipc-boundaries`

Closes: `SEC-M01`, `SEC-M02`, and `SEC-M03`.

Work:

1. Split main/settings capabilities and create explicit permissions for sensitive custom commands.
2. Verify the caller window label and required application state inside every sensitive Rust command.
3. Restrict URL health checks to canonical public HTTPS destinations; reject credentials, IP-literal
   private/loopback/link-local/metadata targets, unsafe redirects, and DNS rebinding outcomes.
4. Make preset export a backend-owned save-dialog transaction or require a short-lived scoped path
   authorization tied to a user gesture.
5. Review CSP and plugin permissions after the command split; retain a self-only script policy.
6. Add negative authorization tests for both windows and malformed IPC payloads.

Exit criteria:

- A Settings renderer cannot call engine/DNS/update operations it does not require.
- Private-network health requests and unauthorized file writes are rejected in backend tests.
- Sensitive commands do not rely only on frontend visibility or validation.

### Phase 3 - Persistent-state correctness and migrations

Branch: `fix/settings-persistence`

Closes: the reported next-day reset defect and duplicated/dead persisted state.

Work:

1. Define one versioned settings schema with typed defaults and explicit migrations.
2. Keep Pattern mode, domain lists, preset, transport, DNS, cache, proxy, filters, watchdog, Kill
   Switch, autostart, and Advanced values in one canonical persisted model.
3. Remove or migrate legacy `domainList`, disabled `doq`, and unsupported Advanced fields.
4. Use atomic write/replace, flush, bounded backup rotation, strict read validation, and recovery from
   the most recent valid copy. Never silently replace the only copy with defaults.
5. Serialize writes and reject stale revisions so concurrent UI/backend saves cannot overwrite newer
   settings.
6. Make autostart consume the same committed configuration revision and log the loaded revision/hash.
7. Separate presentation preferences from privileged desired state in preparation for the broker.

Exit criteria:

- Restart, reboot, autostart, upgrade, corrupt-primary-file, interrupted-write, and concurrent-save
  tests preserve the last valid user configuration.
- Whitelist/blacklist and all Advanced text fields round-trip exactly after validation.
- Defaults are used only on a genuine first run or an explicit, confirmed reset.

### Phase 4 - Transactional DNS ownership

Branch: `fix/dns-transactions-ipv6`

Closes: `DNS-M01` and `DNS-M02`.

Work:

1. Discover, snapshot, configure, verify, and restore both IPv4 and IPv6 DNS state.
2. Introduce an adapter transaction: pre-snapshot, staged apply, full verification, rollback on any
   failure, and a durable recovery journal for interruption or crash.
3. Validate adapter identity and snapshot schema before privileged restoration.
4. Verify exact Kill Switch rule semantics, including protocol, direction, family, local/remote
   address, port, profile, ownership, and intended exclusions.
5. Report the limitation that application-managed DoH/DoQ over port 443 is not stopped by a
   port-53-only policy, or implement an explicitly designed stronger policy.
6. Make logs reflect observed OS state rather than button clicks or requested state.

Exit criteria:

- No partial adapter configuration remains after an injected failure.
- IPv4 and IPv6 leak tests pass with each supported mode and after rollback/recovery.
- Kill Switch status is derived from exact installed-rule verification.

### Phase 5 - DNS forwarder lifecycle and cache/filter correctness

Branch: `fix/dns-runtime-lifecycle`

Closes: `DNS-M03` and `DNS-M04`; verifies Smart DNS Cache, Local AdBlock, and Malware Filter.

Work:

1. Give listeners and every child query/connection task a shared cancellation token and tracked task
   set; await full shutdown before emitting `stopped`.
2. Add TCP idle/read timeouts, maximum queries per connection, bounded queues, and explicit overload
   behavior.
3. Make filter refresh single-flight, use unique temporary files, validate before atomic replacement,
   and cancel obsolete generations.
4. Define cache ownership, TTL bounds, negative-cache policy, size/eviction metrics, flush semantics,
   and an authoritative enabled/disabled state.
5. Add real outcome events for upstream protocol, proxy connection, cache hit/miss/eviction, blocklist
   activation, refresh result, and shutdown completion without logging sensitive queries by default.

Exit criteria:

- No listener, accepted connection, or query task survives a completed stop.
- Repeated cache/filter toggles cannot re-enable a disabled feature or race the cache file.
- Resource-exhaustion and upstream-failure tests remain bounded and recover automatically.

### Phase 6 - Single engine runtime owner

Branch: `refactor/engine-runtime`

Closes: `ENGINE-M01`, `ENGINE-M02`, and `SEC-L01`.

Work:

1. Introduce one `EngineRuntime` state machine with stopped, starting, running, testing, recovering,
   stopping, and failed states.
2. Replace competing start paths with one transactional start use case that includes Pattern and DNS
   guard preparation, verification, rollback, and structured result data.
3. Make optimizer sessions lease the same runtime and reuse binary hash checks, argument validation,
   job ownership, cancellation, and cleanup.
4. Replace image-name process termination with Vane-owned PID/job identity. Avoid deleting a generic
   WinDivert service based only on its name.
5. Treat Job Object creation/assignment failures as startup failures where containment is required.
6. Close token handles with RAII and replace `PROCESS_ALL_ACCESS` with minimum required access.
7. Make watchdog recovery revision-aware so it cannot restart an obsolete configuration.

Exit criteria:

- Only one engine/optimizer owner exists and concurrent starts are deterministically rejected.
- Unrelated same-name processes and shared driver users survive Vane cleanup.
- Handle/resource leak tests and repeated start/stop/recovery stress tests pass.

### Phase 7 - Privilege separation

Branch: `security/privileged-broker`

Closes: `ARCH-H01` and `ARCH-H02`.

Work:

1. Run the Tauri/WebView UI without administrator privileges.
2. Move DNS adapter changes, firewall operations, engine/driver lifecycle, recovery, and autostart
   registration into a small privileged broker without a WebView.
3. Define versioned, typed, allowlisted broker operations; authenticate the expected local user/client
   and reject replay, downgrade, malformed, oversized, and out-of-sequence requests.
4. Store privileged desired state and recovery journals under administrator-only ACLs. Keep UI-only
   preferences in user storage.
5. Validate every value again at the privileged boundary; never expose arbitrary command, file, URL,
   registry, service, or firewall primitives.
6. Design broker installation, upgrade, rollback, crash recovery, and uninstall cleanup.
7. Threat-model the IPC transport and test calls from an unrelated local process and a compromised
   renderer fixture.

Exit criteria:

- The renderer and normal UI process have no administrator token.
- Modifying user-writable settings cannot cause unauthorized privileged state changes.
- Only authenticated, schema-valid, allowlisted operations reach privileged Windows APIs.

This phase is an architectural change and should target a minor release rather than being hidden in
an ordinary patch release.

### Phase 8 - Structured observability and codebase cleanup

Branch: `refactor/structured-events-cleanup`

Closes the professionalism and unnecessary-complexity findings.

Work:

1. Replace regex translation of English backend strings with stable event codes, typed parameters,
   severity, component, operation ID, requested state, observed state, and EN/TR message catalogs.
2. Do not log secrets, full proxy credentials, sensitive query history, or updater signing material.
3. Split oversized command, engine, DNS, and store files by use case and ownership boundary.
4. Remove fake `CVE-2`/`CVE-5` labels, obsolete error artifacts, dead fields, and duplicate state.
5. Correct README claims for DoQ, conntrack, Kill Switch scope, DNS leak guarantees, and upstream
   Zapret capabilities.
6. Add architecture decision records for privilege separation, persistence, DNS transactions,
   engine ownership, and release trust.

Exit criteria:

- Every setting-changing operation logs a verified outcome in EN/TR through structured events.
- Logs distinguish request, success, partial failure, rollback, recovery, and observed state.
- Documentation describes only implemented and tested behavior.

### Phase 9 - Final adversarial validation and release gate

Branch: `test/hardening-acceptance`

Work:

1. Run static analysis, dependency/secret scans, malformed IPC/property tests, and WebView/CSP checks.
2. Perform elevated Windows acceptance testing across supported Windows versions and common adapter
   types, including IPv6, VPN, Hyper-V/virtual adapters, sleep/resume, network changes, reboot, crash,
   and uninstall.
3. Capture packets to verify Pattern scope and DNS leakage claims across direct, proxy, DoH, DoT,
   cache, filters, Kill Switch, and failure/recovery scenarios.
4. Stress forwarder and engine lifecycles, simulate local hostile processes, and verify broker ACL/IPC
   enforcement.
5. Verify reproducible release inputs, SBOM/provenance, updater signature, Authenticode signature,
   clean install, upgrade from 2.0.8, rollback, and uninstall.

Exit criteria:

- Every audit finding has a linked fix, regression test, and evidence artifact.
- No open high/critical vulnerability or unexplained privilege-boundary failure remains.
- Remaining accepted risks are documented with owner, rationale, mitigation, and expiry date.

## Cross-cutting definition of done

A work item is complete only when all of the following are true:

1. Root cause is removed rather than masked in the UI.
2. Backend validates input and verifies the resulting external state.
3. Success, failure, rollback, and recovery produce structured EN/TR events.
4. Automated regression tests cover positive, negative, restart, and interruption behavior.
5. Migration and rollback paths preserve user data.
6. Security-sensitive code has a focused diff review.
7. GitHub Actions passes; release artifacts are produced by the workflow, not by an undocumented
   local build.
8. The audit finding is marked closed with links to the implementing PR and test evidence.

## Release strategy

- Ship dependency/release-chain and safe isolated correctness fixes as a patch release only after
  their acceptance gates pass.
- Ship the privileged broker and state-ownership redesign as a minor release because installation,
  IPC, recovery, and privilege behavior change materially.
- Do not mark the whole audit closed after an intermediate release; track closure per finding.
