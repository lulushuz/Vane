# Vane Windows acceptance harness

This harness collects the administrator-level evidence that repository unit tests cannot prove:

- the packaged Vane and `winws` process command line;
- WinDivert driver state;
- the exact `VaneDNSKillSwitch` rules and port filters;
- IPv4/IPv6 DNS server state for every adapter;
- TCP/UDP port 53 listeners;
- persistence metadata without exporting domains, proxy endpoints, usernames, or the computer name.

It does not start, stop, or reconfigure Vane, DNS, adapters, or firewall rules. The operator owns
those actions through the application. Packet capture is opt-in and is stopped in a `finally` block.

Use the installer from the `Windows Acceptance Build` workflow artifact. Do not run release evidence
against an older installed version or publish a stable tag before this evidence passes. The artifact
contains `manifest.json`; confirm its version and commit match the report produced by this harness.

## Guided run

Open PowerShell as administrator from the repository root:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\windows\Invoke-VaneAcceptance.ps1
```

The script takes a baseline, pauses while the operator starts and configures Vane, records the
running state, and pauses again for a clean stop. It writes `evidence.json` and `REPORT.md` under
`%TEMP%\VaneAcceptance\<session-id>`.

To include a packet trace while performing the Pattern TCP/QUIC checks:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\windows\Invoke-VaneAcceptance.ps1 -CaptureTraffic
```

Packet capture stores the first 128 bytes per packet so network/transport headers remain useful
without intentionally collecting application payloads. It may still contain destination metadata;
review `traffic.etl` and `traffic.txt` before sharing them. The JSON and Markdown reports redact user
paths and proxy values and store only a hash of the computer name.

## Automation and diagnostics

A single non-interactive snapshot is suitable for support diagnostics:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\windows\Invoke-VaneAcceptance.ps1 -Mode Snapshot -NonInteractive
```

Guided mode intentionally requires operator checkpoints and cannot be combined with
`-NonInteractive`. The script never claims a traffic result without an operator verdict; missing
verdicts remain `not-recorded`.

## Evidence interpretation

- `before`: Vane should be stopped and no Vane-owned DNS/firewall residue should remain.
- `running`: `winws` should have the expected hostlist argument, the forwarder should own port 53
  when enabled, and Kill Switch rules should exist only when selected.
- `after-stop`: `winws`, Vane-owned firewall rules, and local port 53 listeners should be gone; DNS
  adapter values should match the baseline.

The tool records observations, not a false pass. A reviewer must compare snapshots, inspect the
optional packet trace, and confirm the operator outcomes before closing manual release gates.

The Windows CI job parses the collector and runs `Test-VaneAcceptanceHarness.ps1` against positive
and negative redaction/Pattern evaluation fixtures. It never performs privileged network changes.
