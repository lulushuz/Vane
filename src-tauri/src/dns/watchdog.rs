use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};

pub fn spawn_dns_watchdog(
    client: reqwest::Client,
    endpoint: crate::dns::forwarder::DoHEndpoint,
    protocol: String,
    health_check_target: String,
    shutdown: Arc<AtomicBool>,
    app_handle: tauri::AppHandle,
) {
    tokio::spawn(async move {
        let mut fail_count = 0;
        tracing::info!("DNS watchdog task started.");

        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }

            tokio::time::sleep(Duration::from_secs(5)).await;

            if shutdown.load(Ordering::SeqCst) {
                break;
            }

            let reachable = if protocol == "dot" {
                crate::dns::forwarder::probe_dot_upstream(endpoint, &health_check_target).await
            } else {
                match crate::dns::forwarder::current_doh_client(&client) {
                    Some(probe_client) => {
                        crate::dns::doh::resolve_doh(
                            &probe_client,
                            endpoint.url(),
                            &health_check_target,
                        )
                        .await
                        .success
                    }
                    None => false,
                }
            };

            match reachable {
                true => {
                    fail_count = 0;
                }
                false => {
                    fail_count += 1;
                    tracing::warn!(
                        "DNS watchdog: {} upstream could not resolve '{}' (failure {}/3).",
                        protocol.to_uppercase(),
                        health_check_target,
                        fail_count
                    );
                }
            }

            if fail_count >= 3 {
                tracing::error!("CRITICAL: DNS upstream failed three real resolution checks. Reverting system DNS to DHCP!");
                shutdown.store(true, Ordering::SeqCst);
                tracing::error!(
                    "DNS watchdog recovery requires the authoritative DnsTransactionManager"
                );
                let candidate = crate::dns::DnsConfigCandidate {
                    enabled: false,
                    protocol: "doh".into(),
                    provider: Some("cloudflare".into()),
                    adblock: false,
                    cache_enabled: false,
                    socks5: None,
                    kill_switch: false,
                };
                let recovered = match app_handle.try_state::<crate::AppState>() {
                    Some(state) => state
                        .dns_transaction_manager
                        .apply_candidate(candidate, &app_handle, state.inner())
                        .await
                        .is_ok(),
                    None => false,
                };
                if recovered {
                    let _ = app_handle.emit("dns_status_changed", ());
                    let _ = app_handle.emit(
                        "dns_auto_applied",
                        "DNS_WATCHDOG_TRANSACTION_ROLLBACK_COMPLETED",
                    );
                } else {
                    tracing::error!("DnsTransactionManager could not roll back watchdog failure.");
                }
                break;
            }
        }
        tracing::info!("DNS watchdog task stopped.");
    });
}
