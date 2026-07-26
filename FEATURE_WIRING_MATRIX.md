# Vane Feature Wiring Matrix

Status meanings:

- **Wired**: represented in UI/state, serialized, accepted by Rust validation, and passed to the child argument list.
- **Disabled**: visible for compatibility/context but disabled, not serialized, and rejected if imported as an argument.
- **Internal only**: retained in the persisted model for compatibility but not exposed as an active control.
- **Runtime API**: applied through a typed Rust command rather than preset arguments.

Passing an argument to `Command::args` proves process configuration, not successful external traffic bypass. Real traffic verification remains a Windows integration/release test.

## Advanced configuration

| UI/state field | Generated argument | Rust validation | Child argv | Test evidence | Status |
| --- | --- | --- | --- | --- | --- |
| `desyncMethod` | `--dpi-desync=<mode>` | Strategy allowlist | Yes | serializer/parser + sanitizer | Wired |
| `customDesyncMethod` | `--dpi-desync=<list>` | Every comma-delimited strategy must be allowed | Yes | sanitizer rejects unknown/injected values | Wired |
| `splitPosition` | `--dpi-desync-split-pos=<n>` | marker/numeric validation; numeric zero rejected | Yes | invalid finite-number and sanitizer tests | Wired |
| `desyncRepeats` | `--dpi-desync-repeats=<n>` when `n >= 2` | `1..100` | Yes | serializer omission + sanitizer range tests | Wired; `1` uses engine default |
| `desyncFooling` | `--dpi-desync-fooling=<list>` | Fooling allowlist | Yes | parser + sanitizer tests | Wired |
| `anyProtocol` | `--dpi-desync-any-protocol` | Exact flag only | Yes | default serializer + exact-flag rejection tests | Wired |
| `autoTtl` | `--dpi-desync-autottl` | Exact flag only | Yes | serializer + sanitizer tests | Wired |
| `fakeTtl` | `--dpi-desync-ttl=<n>` when auto TTL is off | `1..255` | Yes | manual TTL + sanitizer tests | Wired |
| `mssFix` | None | `--mss` rejected | No | unsupported argument tests | Disabled |
| `quicUdpHandling` | `--wf-udp=443` | Port validation | Yes | default serializer + port tests | Wired |
| `httpPorts` | `--wf-tcp=<ports>` | Port/range validation | Yes | parser + sanitizer port tests | Wired |
| `desyncHttp` | None | `--dpi-desync-http` rejected | No | unsupported fields quarantined | Disabled |
| `desyncHttps` | None | `--dpi-desync-https` rejected | No | sanitizer unknown-argument policy | Disabled |
| `desyncQuic` | None | `--dpi-desync-quic` rejected | No | sanitizer unknown-argument policy | Disabled |
| `desyncCutoff` | `--dpi-desync-cutoff=<kind+n>` | Prefix `n`, `d`, or `s`; bounded number | Yes | parser + cutoff validator | Wired |
| `splitHttpReq` | `--dpi-desync-split-http-req=method|host` | Explicit selector allowlist | Yes | parser + selector validator | Wired |
| `splitPosHttpReq` | None | Separate position flag rejected | No | parser quarantines unsupported inputs | Disabled |
| `splitTls` | `--dpi-desync-split-tls=sni|sniext` | Explicit selector allowlist | Yes | parser + selector validator | Wired |
| `splitPosTls` | None | Separate position flag rejected | No | parser quarantines unsupported inputs | Disabled |
| `fakeTtlExt` | None | TTL extension flag rejected | No | sanitizer unknown-argument policy | Disabled |
| `fakeTlsSni` | None | Fake-SNI flag rejected | No | parser quarantines unsupported inputs | Disabled |
| `fakeHttpPayload` | None | Payload/path flag rejected | No | UI disabled; unknown/path arguments rejected | Disabled |
| `fakeTlsPayload` | None | Payload/path flag rejected | No | UI disabled; unknown/path arguments rejected | Disabled |
| `fakeQuicPayload` | None | Payload/path flag rejected | No | UI disabled; unknown/path arguments rejected | Disabled |
| `desync2` | None | `--dpi-desync2` rejected | No | parser quarantines unsupported inputs | Disabled |
| `tcpWindowSize` | `--wssize=<n>` when positive | `1..16777216` | Yes | parser + negative omission + sanitizer tests | Wired |
| `ipsetPath` | None | `--ipset` and traversal rejected | No | parser quarantine + sanitizer tests | Internal only |
| `tpwsMode` | None | tpws-only flags rejected | No | explicit unsupported tpws test | Internal only |
| `bindInterface` | None | `--bind-addr` rejected | No | parser quarantine test | Disabled |
| `passthroughArgs` | Preserved known imported flags | Full Rust allowlist remains authoritative | Yes, only after validation | passthrough deduplication + unknown flag rejection | Wired with backend gate |
| `invalidArgs` | Never serialized | N/A | No | malformed/import quarantine tests | Diagnostic only |

## Advanced call chain

1. Cards update `AdvancedConfig` in `AdvancedView`.
2. `serializeConfigToArgs` produces the canonical supported argument list.
3. Saving creates a `Preset.args` array.
4. `save_custom_preset` persists the preset through Rust `ConfigLoader`.
5. `EngineManager::start` calls `validate_preset_args` before changing lifecycle state.
6. `spawn_and_run` calls `prepare_args`, adds Pattern-owned arguments, and passes the final vector directly to `Command::args` without a shell.

Unknown fields are never silently forwarded. Unsupported imported arguments remain in `invalidArgs` for UI diagnostics and are blocked again by Rust if another caller bypasses the frontend.

## Pattern

| Setting | Backend authority | Runtime result | Evidence | Status |
| --- | --- | --- | --- | --- |
| `all` | Rust mode validation | No hostlist restriction | manager tests | Wired |
| `whitelist` | Canonical `whitelistDomains` array | `--hostlist=<generated file>` | fail-closed and canonical array tests | Wired |
| `blacklist` | Canonical `blacklistDomains` array | `--hostlist-exclude=<generated file>` | domain canonicalization tests | Wired |
| Empty whitelist | Rust validation | Engine start refused | regression test | Fail-closed |
| Domain normalization | Rust canonicalizer | lowercase, trim, final-dot removal, boundary-safe rules | domain unit tests | Wired |
| Wildcard | Rust canonicalizer | Rejected because bundled Zapret semantics do not match advertised behavior | unit test | Explicitly unsupported |
| Hidden aliases | None | No automatic scope expansion | unit test | Removed |
| Runtime change | `sync_bypass_config` | Current engine stops and restarts through serialized lifecycle state | IPC status + lifecycle tests | Wired |
| Persistence | Rust settings repository | Atomic primary/backup | settings tests | Wired |

## DNS, proxy, and safety

| Setting | Application path | Verification returned | Status |
| --- | --- | --- | --- |
| Transport protocol | `sync_dns_settings` → `DnsSettings` | `doh` or `dot` canonical status | Wired |
| Smart DNS Cache | Runtime cache update | `applied` only when forwarder is active; disabling clears RAM cache | Wired |
| Local AdBlock/Malware Filter | DNS settings → adblock initialization | Canonical status reports configured value | Wired at configuration level; live blocklist traffic remains integration-tested |
| SOCKS5 upstream | Normalized `host:port`; DoH uses `socks5h` | Invalid credentials/host/port rejected; DoT+proxy rejected | Wired |
| DNS Kill Switch | Pattern config → engine start/stop firewall operation | Engine start fails if firewall verification fails | Wired; administrator/OS integration required |
| DNS watchdog | Forwarder runtime → three upstream resolution probes | Restores prior DNS snapshot and removes kill switch after repeated failure | Wired |
| Health-check target | Canonical domain array; neutral default `example.com` | Invalid/empty targets rejected | Wired |
| Config revisions | Separate Pattern/DNS atomic counters | Monotonic status/event gating during process lifetime | Wired |
| Structured failures | Shared `IpcError` | code, message, operation, retryability | Wired for Pattern/DNS sync |

## Known verification boundary

The repository tests prove serialization, validation, persistence, state transitions, process argument construction, and rollback decisions. They do not replace these release checks:

- packaged Windows process command-line inspection;
- real WinDivert capture;
- IPv4/IPv6 and TCP/QUIC traffic tests;
- OS firewall and adapter DNS behavior under administrator privileges;
- sleep/resume and machine reboot recovery.
