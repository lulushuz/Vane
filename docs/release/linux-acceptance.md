# Linux Acceptance Test Specification — Vane DPI v1.0.0-rc.1

## Overview
This document specifies the privileged manual acceptance test matrix for Linux environments (Ubuntu 22.04 / 24.04 LTS).

## Acceptance Matrix

| Test ID | Test Category | Description | Execution Result |
| :--- | :--- | :--- | :---: |
| **LNX-01** | Packaging | AppImage / `.deb` installation & nfqws permissions | NOT EXECUTED (VM Pending) |
| **LNX-02** | Filter Planner | nftables / iptables queue rule planning & apply | NOT EXECUTED (VM Pending) |
| **LNX-03** | Rule Ownership | Table/chain comment ownership & foreign rule safety | NOT EXECUTED (VM Pending) |
| **LNX-04** | Engine Lifecycle | Start, pattern restart, stop, cleanup | NOT EXECUTED (VM Pending) |
| **LNX-05** | Diagnostics | Local checks & redacted diagnostics bundle export | NOT EXECUTED (VM Pending) |
| **LNX-06** | Uninstallation | Clean removal with zero orphan nftables chains | NOT EXECUTED (VM Pending) |
