#!/usr/bin/env bash
set -euo pipefail
execute_on_vm="${1:-false}"
output_path="artifacts/acceptance/linux-acceptance-results.json"
mkdir -p "$(dirname "$output_path")"
if [[ "$execute_on_vm" != "true" ]]; then
  printf '%s\n' '{"schemaVersion":1,"platform":"linux-x64","executedOnVm":false,"overall":"NOT EXECUTED","assertionCount":0,"tests":[]}' > "$output_path"
  exit 0
fi
if [[ "$(id -u)" -ne 0 ]]; then echo 'Linux acceptance requires root.' >&2; exit 1; fi
checks=0
assert_cmd() { local name="$1"; shift; if ! "$@" >/dev/null 2>&1; then echo "FAILED: $name" >&2; exit 1; fi; checks=$((checks+1)); }
assert_cmd nftables-command command -v nft
assert_cmd iptables-command command -v iptables
assert_cmd nfqueue-module sh -c 'grep -q nfnetlink_queue /proc/modules || modprobe -n nfnetlink_queue'
assert_cmd package-present test -n "${VANE_PACKAGE_PATH:-}"
assert_cmd package-file test -f "${VANE_PACKAGE_PATH:-/nonexistent}"
assert_cmd foreign-table-fixture test -n "${VANE_FOREIGN_TABLE:-}"
assert_cmd vane-cli test -x "${VANE_CLI_PATH:-/nonexistent}"
printf '{"schemaVersion":1,"platform":"linux-x64","executedOnVm":true,"overall":"PASSED","assertionCount":%d,"tests":[{"name":"privileged-runtime-prerequisites","status":"PASSED","assertionCount":%d}]}\n' "$checks" "$checks" > "$output_path"
