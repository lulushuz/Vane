# Windows Acceptance Test Specification — Vane DPI v1.0.0-rc.1

## Overview
This document specifies the privileged manual acceptance test matrix for Windows 10/11 environments using the automated harness `scripts/windows/Invoke-VaneAcceptance.ps1`.

## Acceptance Matrix

| Test ID | Test Category | Description | Execution Result |
| :--- | :--- | :--- | :---: |
| **WIN-01** | Installation | NSIS Installer signature & per-machine install | NOT EXECUTED (VM Pending) |
| **WIN-02** | Engine Lifecycle | Start, stop, preset change, winws generation | NOT EXECUTED (VM Pending) |
| **WIN-03** | Pattern Transaction | Whitelist, blacklist, all modes, revision commit | NOT EXECUTED (VM Pending) |
| **WIN-04** | DNS & Kill Switch | DoH/DoT transition, WinDivert kill switch rules | NOT EXECUTED (VM Pending) |
| **WIN-05** | Optimizer | Candidate measurement, original state restore | NOT EXECUTED (VM Pending) |
| **WIN-06** | Diagnostics | Local consistency checks, manual HTTPS probe | NOT EXECUTED (VM Pending) |
| **WIN-07** | Artifact Tamper | Modified winws/driver fails closed on launch | NOT EXECUTED (VM Pending) |
| **WIN-08** | Uninstallation | Clean removal, zero foreign process/rule impact | NOT EXECUTED (VM Pending) |
