# Vane 1.0.0 Release Checklist

GitHub Actions owns release builds. Local verification is limited to focused tests, static checks, and source-level evidence.

Windows acceptance installers are produced by `windows-acceptance-build.yml` as short-lived workflow
artifacts. This workflow never creates a tag or public GitHub Release.

## Automated quality gates

- [x] Frontend unit tests pass.
- [x] TypeScript compilation passes.
- [x] Frontend production build passes in GitHub Actions.
- [x] Rust library tests pass on Windows and Linux runners.
- [x] Clippy passes with warnings denied.
- [x] RustSec audit passes.
- [x] npm high-severity audit passes.
- [x] GitHub Actions dependencies are commit-pinned.
- [x] Release workflow verifies version/tag consistency.
- [x] Release workflow validates bundled Zapret hashes before packaging.
- [ ] Add a repository-wide Rustfmt gate after existing historical formatting debt is normalized in a dedicated change.
- [ ] Add ESLint after adopting a repository configuration without mixing it into reliability fixes.

## Configuration and persistence

- [x] Rust is the only writer of `settings.json`.
- [x] Writes use temporary file, flush, atomic replacement, and last-known-good backup.
- [x] Multiple windows merge only their changed fields.
- [x] Legacy string domain lists migrate to arrays.
- [x] Pattern/DNS changes return canonical backend status.
- [x] Stale Pattern/DNS events are revision-gated.
- [x] Start sends the current Pattern and DNS state before launching the engine.
- [x] Autostart restores persisted preset, Pattern, DNS, proxy, kill switch, and watchdog settings.
- [ ] Add explicit version-to-version migrations before schema version 2.
- [ ] Add persistent config hash/revision for cross-process reconciliation.

## Engine and Pattern

- [x] Start/stop transitions are serialized.
- [x] Stale async start results cannot replace newer state.
- [x] Stop waits for real process exit with bounded escalation.
- [x] Tray/application exit has one shutdown owner.
- [x] Crash recovery performs five cancellable backoff attempts.
- [x] Empty whitelist fails closed.
- [x] Invalid/corrupt Pattern state does not silently fall back to all-sites mode.
- [x] Hidden domain alias expansion is removed.
- [x] Unsupported wildcard semantics are rejected.
- [x] Supported Advanced controls are validated before child spawn.
- [x] Unsupported Advanced controls are disabled and their arguments rejected.
- [ ] Inspect final child command line in the packaged Windows build.
- [ ] Capture positive/negative Pattern traffic over TCP and QUIC.

## DNS and safety

- [x] Smart DNS Cache off clears in-memory entries.
- [x] DNS settings restart an active forwarder only when values changed.
- [x] DoT with SOCKS5 is rejected to prevent proxy bypass.
- [x] SOCKS5 uses remote hostname resolution through `socks5h`.
- [x] Watchdog requires three failed upstream resolution probes before recovery.
- [x] Watchdog uses a neutral `example.com` default rather than a commonly blocked service.
- [x] DNS recovery removes the kill switch and restores the prior adapter snapshot.
- [x] Successful config application is not reported as failed only because an event notification failed.
- [ ] Verify Windows firewall rules and adapter DNS before/after normal exit.
- [ ] Verify cleanup after forced application crash and machine reboot.
- [ ] Verify IPv4/IPv6 behavior independently.

## Documentation and release decision

- [x] Canonical configuration ownership and IPC stages documented in `CONFIG_SCHEMA.md`.
- [x] Advanced/Pattern/DNS wiring documented in `FEATURE_WIRING_MATRIX.md`.
- [x] Automated versus manual reproduction evidence documented in `REPRODUCTION_MATRIX.md`.
- [ ] Complete packaged Windows smoke tests.
- [ ] Complete Roblox positive and government-site negative traffic controls.
- [x] Add a redacted, snapshot-first Windows acceptance evidence collector.
- [ ] Update final changelog/release notes after all integration evidence is collected.
- [ ] Mark the release ready only when no critical/high finding remains and every mandatory manual check above has evidence.

## Current decision

**Not yet ready for a stable public release.** Source-level reliability and automated gates are substantially hardened, but administrator-level Windows cleanup and real TCP/QUIC traffic evidence remain mandatory.
