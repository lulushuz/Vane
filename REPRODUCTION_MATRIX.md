# Vane 2.0.8 Reproduction & Acceptance Test Matrix

This matrix categorizes all verification requirements, test types, execution requirements, and safety controls across the codebase.

---

## Test Execution Matrix

| Verification Target | Test Method | Environment Needed | Packet Capture Needed | Destruction Risk | Safety & Cleanup Requirements |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Whitelist Positive Bypass** | E2E Real Traffic | Windows 11 Elevated | Yes (Wireshark/pktmon) | None | Restores original DNS on exit |
| **Whitelist Negative Isolation** | E2E Real Traffic | Windows 11 Elevated | Yes (Wireshark/pktmon) | None | Verify non-target traffic is unbypassed |
| **Blacklist Exclusion Boundary**| E2E Real Traffic | Windows 11 Elevated | Yes (Wireshark/pktmon) | None | Verify targeted domain is excluded |
| **DNS Leak / Firewall Rules** | Automated Harness | Windows 11 Elevated | Yes (pktmon port 53) | Low | Delete test WFP rules on teardown |
| **Adapter DNS Snapshot/Restore**| Automated Harness | Windows 11 Elevated | No | Medium | Restore original network adapter IPs |
| **Unset Auto-Recovery Recovery**| Fault Injection | Windows 11 Elevated | No | None | Restart engine process up to 5 times |
| **Multi-Window Atomic Save** | Integration Test | Node.js + Tauri Test Bed| No | None | Isolates temporary test directory |
| **Malformed Settings Recovery**| Unit Test | Cargo Rust Test | No | None | Uses mock AppData directory |
| **Invalid WinWS Argument** | Unit Test | Cargo Rust Test | No | None | No system state modified |
| **SSRF Health Check Allowlist**| Unit Test | Cargo Rust Test | No | None | Mock HTTP server handles requests |

---

## Automated vs Manual Verification Breakdown

### 1. Fully Automated (CI Verified)
- `sanitizer.rs` argument allowlisting (Rust unit tests)
- `remote.rs` SSRF IP/host filtering (Rust unit tests)
- `engineStore.ts` domain list array sanitization (Vite unit tests)
- Clippy warnings-free compilation (`cargo check`)

### 2. Manual & Windows Acceptance Required
- Real WinDivert packet splitting verification on HTTP/TLS/QUIC protocols
- Windows Firewall netsh rule creation and verification under unexpected process kill
- Network adapter primary/secondary DNS restoration after system reboot
- Windows Job Object cleanup when Vane is forcibly terminated via Task Manager
