# Vane Reproduction Matrix

Evidence states:

- **Automated**: covered by a repeatable repository test.
- **Code verified**: call chain and failure handling were inspected, but OS/network behavior still needs an integration environment.
- **Manual pending**: requires a packaged Windows application, administrator privileges, or controlled network capture.

| # | Scenario | Expected result | Current evidence | Release state |
| --- | --- | --- | --- | --- |
| 1 | Clean installation | Defaults persist; engine starts only after validated sync | Settings/default tests + startup review | Code verified |
| 2 | Upgrade from 2.0.0 | Historical state migrates without reset | String-to-array migration tests | Automated |
| 3 | Corrupt legacy domain string | Valid lines migrate; invalid schema does not become `all` silently | Persistence and manager tests | Automated |
| 4 | Pattern change while engine stopped | Canonical config is prepared for next start | Typed status `prepared` | Automated at contract level |
| 5 | Pattern change while engine running | Serialized stop/start uses the new hostlist | Lifecycle + Pattern tests | Code verified; traffic pending |
| 6 | 50 rapid domain changes | Last frontend revision wins; older events ignored | Revision-gate tests | Automated at state level |
| 7 | Rapid mode changes | Stale completions do not overwrite latest state | Revision-gate tests | Automated at state level |
| 8 | Close immediately after change | Atomic store protects completed writes | Atomic persistence tests | Manual pending for debounce-before-exit |
| 9 | Start immediately after change | Pending debounce is cancelled and current state is synchronously sent | Store start path review | Code verified |
| 10 | Stop during start | Generation/cancel prevents stale start result | Lifecycle tests | Automated |
| 11 | Start during stop | Start is rejected while stopping | Lifecycle state review | Automated at state-machine level |
| 12 | Child process crash | Five retries with 1/2/4/8/16 second backoff | Recovery state tests + supervisor review | Code verified; forced crash pending |
| 13 | Application crash | Windows Job Object owns child cleanup | RAII implementation | Manual pending |
| 14 | DNS forwarder crash | Watchdog restores prior DNS after three failed probes | Watchdog review | Manual pending |
| 15 | Network adapter change | DNS status is refreshed | event/listener review | Manual pending |
| 16 | Wi-Fi/Ethernet transition | Snapshot and adapter DNS reconciliation remain consistent | DNS manager review | Manual pending |
| 17 | Sleep/resume | No orphan process or stale DNS/firewall rule | No packaged test yet | Manual pending |
| 18 | IPv4-only | Pattern and DNS operate without IPv6 assumptions | platform code review | Manual pending |
| 19 | IPv6-preferred | Kill switch covers TCP/UDP 53 for IPv6 | ip6tables/Windows rule review | Manual pending |
| 20 | QUIC enabled | UDP 443 enters configured WinDivert filter | serializer/argv tests | Manual traffic capture pending |
| 21 | QUIC disabled | UDP 443 filter is omitted | serializer tests | Automated at argv level |
| 22 | Browser Secure DNS enabled | Browser bypass of local resolver is observable and documented | No browser automation yet | Manual pending |
| 23 | Browser Secure DNS disabled | Local forwarder receives resolver traffic | No packet capture yet | Manual pending |
| 24 | Whitelist only `roblox.com` | Only apex/boundary-safe subdomains enter hostlist | canonical domain tests | Automated at rule level; traffic pending |
| 25 | Empty whitelist | Engine refuses to start | manager regression test | Automated |
| 26 | Blacklist `turkiye.gov.tr` | Domain and subdomains are excluded; lookalikes are not | boundary-safe domain tests | Automated at rule level; traffic pending |
| 27 | Wildcard/apex input | Wildcard is rejected; apex has explicit subdomain semantics | domain tests | Automated |
| 28 | Preset import/export | Unsafe/unknown arguments cannot reach Rust engine | frontend validator + Rust sanitizer tests | Automated; file-dialog round trip pending |
| 29 | Every Advanced field | Supported controls map to allowed argv; unsupported controls remain disabled/rejected | `FEATURE_WIRING_MATRIX.md` + tests | Automated at contract level |
| 30 | Two windows update concurrently | Rust merges only fields changed by each window | settings merge regression test | Automated |

## Required manual evidence format

For every manual release run, record:

- Vane commit and package version;
- Windows version and architecture;
- administrator/elevation state;
- selected preset and redacted final child argv;
- Pattern mode, canonical domain count, and config revision;
- DNS protocol/cache/filter/proxy status and revision;
- WinDivert/process observation;
- IPv4/IPv6 and TCP/UDP protocol used;
- expected and actual result;
- cleanup result after stop, exit, crash, and reboot.
