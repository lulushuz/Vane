# Vane Configuration Contract

This document describes the configuration contract implemented by Vane 2.0.8. It distinguishes persisted UI intent from backend-validated runtime state. Items under **Remaining work** are not claimed as implemented.

## Ownership and sources of truth

| Data | Owner | Lifetime | Authority |
| --- | --- | --- | --- |
| User preferences | Zustand | UI session | Optimistic user intent only |
| `settings.json` and `settings.json.bak` | Rust settings repository | Persistent | Durable configuration source |
| Pattern runtime cache | Rust `BYPASS_CONFIG_CACHE` | Process | Canonical input for the next engine start |
| DNS runtime cache | Rust `DNS_SETTINGS_CACHE` | Process | Canonical input for the forwarder |
| Engine state | Rust `EngineManager` | Process | Actual process lifecycle state |
| Pattern/DNS status responses | Rust IPC commands | Per operation | Backend-validated applied/prepared snapshot |

The frontend must not treat a local Zustand mutation as applied runtime state. Pattern and DNS changes become verified only after the corresponding command returns a typed status response.

## Persistent envelope

Zustand stores one value under the only permitted repository key, `vane-settings`:

```json
{
  "vane-settings": "{\"state\":{...},\"version\":1}"
}
```

The outer object is owned by Rust. The inner serialized Zustand envelope has:

- `version`: currently `1`;
- `state`: the persisted field set;
- a maximum IPC payload size of 1 MiB.

Rust serializes writes with a process-wide mutex, writes a temporary file, flushes it, and atomically replaces the primary file. The previous valid primary is flushed to `settings.json.bak`. A damaged or missing primary is recovered from that backup rather than silently replaced with defaults.

For multiple windows, Rust compares each window's previous snapshot and merges only fields changed by that window. Session-only state is excluded by Zustand's `partialize` function.

## Persisted fields

| Field | Type | Default | Runtime consumer |
| --- | --- | --- | --- |
| `activePresetId` | string | `default` | Engine start/restart |
| `bypassMode` | `all \| whitelist \| blacklist` | `all` | Pattern preparation |
| `whitelistDomains` | string[] | `[]` | Whitelist hostlist |
| `blacklistDomains` | string[] | `[]` | Blacklist exclusion list |
| `domainList` | string | empty | Legacy/UI projection; not authoritative |
| `dnsProtocol` | `doh \| dot` | `doh` | DNS forwarder |
| `dnsAdBlock` | boolean | `false` | Local DNS filtering |
| `dnsCache` | boolean | `true` | In-memory bounded DNS cache |
| `proxySocks5` | string | empty | DNS upstream and engine proxy configuration |
| `killSwitch` | boolean | `false` | DNS leak protection |
| `watchdog` | boolean | `true` | DNS forwarder watchdog |
| `dnsForwarderEnabled` | boolean | `false` | Startup reconciliation |
| `healthCheckTargets` | string[] | `example.com` | DNS health checks |
| `selectedDnsId` | string | empty | System DNS restoration/startup |
| `dnsCustomPrimary` | string | empty | Custom system DNS |
| `dnsCustomSecondary` | string | empty | Custom system DNS |
| `advancedConfig` | `AdvancedConfig` | frontend defaults | Preset argument generation |
| `language` | `tr \| en` | `en` | UI/log presentation |

## Migration

Schema version 1 accepts historical domain lists stored as newline-delimited strings and converts them to arrays. Invalid non-array/non-string domain values become empty arrays. A missing `dnsForwarderEnabled` value becomes `false`.

Migration does not bypass backend validation. Before engine use, Rust canonicalizes domain rules and rejects an invalid mode or invalid rule. An empty whitelist is fail-closed and prevents engine startup.

## Pattern IPC contract

Command: `sync_bypass_config`

Input remains a flat Tauri payload for compatibility. Rust treats `whitelistDomains` and `blacklistDomains` as authoritative and only compares the legacy `list` projection for diagnostics.

Successful response:

```ts
interface BypassConfigStatus {
  mode: 'all' | 'whitelist' | 'blacklist';
  domainCount: number;
  configRevision: number;
  stage: 'prepared' | 'process_started';
  engineRestarted: boolean;
  engineRunning: boolean;
  whitelistDomains: string[];
  blacklistDomains: string[];
  activePresetId: string;
}
```

`prepared` means the canonical configuration is ready for the next start. `process_started` means a child process was started with the changed configuration; it does not claim that external traffic health has been proven.

## DNS IPC contract

Command: `sync_dns_settings`

Successful response:

```ts
interface DnsConfigStatus {
  protocol: 'doh' | 'dot';
  adblock: boolean;
  cache: boolean;
  socks5Proxy: string;
  forwarderActive: boolean;
  configRevision: number;
  stage: 'persisted' | 'applied';
}
```

`persisted` means the setting is prepared for a future forwarder start. `applied` means the running forwarder was restarted or already uses the verified settings. Disabling Smart DNS Cache clears existing memory entries before the response is returned.

## Structured errors

Pattern and DNS synchronization errors use this payload:

```ts
interface IpcErrorPayload {
  code: string;
  message: string;
  operation?: string;
  retryable?: boolean;
  configRevision?: number;
}
```

Validation errors are non-retryable until the input changes. Runtime/state failures are marked retryable. A cross-window event notification failure is logged but does not convert an already-applied configuration into a failed command response.

## Revision semantics

Pattern and DNS each have a separate monotonic backend revision counter. Revisions:

- increment after a successful backend operation;
- are returned in the canonical status;
- are checked by frontend listeners so an older event cannot overwrite a newer snapshot;
- are process-local and reset when Vane restarts.

Frontend debounce counters separately discard stale promise completions within the current window. They are not persistent configuration versions.

## Startup reconciliation

The UI waits for Zustand rehydration before initiating normal Pattern and DNS synchronization. Engine start explicitly flushes the current Pattern and DNS snapshots before starting the process. Autostart reads the Rust-owned persistent repository, validates Pattern semantics, restores runtime caches, and then starts the selected preset.

## Remaining work

- Replace the single migration callback with an explicit version-by-version migration chain before schema version 2.
- Add request IDs supplied by the caller and echo them in status/error payloads.
- Persist a configuration revision or content hash so reconciliation can span process restarts.
- Generate TypeScript DTOs from the Rust schema, or validate shared fixtures in both languages, to prevent future contract drift.
- Separate optimistic UI values from applied snapshots as distinct store fields.
- Convert remaining legacy `Result<_, String>` commands to the shared structured error contract where the UI needs machine-readable recovery behavior.
