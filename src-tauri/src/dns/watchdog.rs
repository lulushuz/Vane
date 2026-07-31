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
                if let Err(error) = crate::engine::manager::apply_kill_switch(false) {
                    tracing::error!(
                        "DNS watchdog could not remove the kill switch during recovery: {error}"
                    );
                }
                let handle = app_handle.try_state::<crate::AppState>().and_then(|state| {
                    state
                        .forwarder
                        .lock()
                        .ok()
                        .and_then(|mut guard| guard.take())
                });
                let previous_dns = handle.as_ref().map(|handle| handle.previous_dns.clone());
                if let Some(handle) = handle {
                    handle.stop().await;
                }
                let res = previous_dns
                    .as_deref()
                    .map(crate::dns::restore_dns_snapshot)
                    .unwrap_or_else(crate::dns::reset_dns_to_dhcp);
                if res.success {
                    if let Err(error) = crate::dns::clear_dns_restore_snapshot(&app_handle) {
                        tracing::error!("DNS watchdog recovered connectivity but could not clear the recovery snapshot: {error}");
                    }
                    tracing::info!(
                        "System DNS was restored successfully after the upstream failure."
                    );
                    let _ = app_handle.emit("dns_status_changed", ());
                    let _ = app_handle.emit(
                        "dns_auto_applied",
                        "DNS_WATCHDOG_PREVIOUS_CONFIGURATION_RESTORED",
                    );
                } else {
                    tracing::error!(
                        "Failed to restore system DNS after the upstream failure: {:?}",
                        res.error
                    );
                }

                // Exit watchdog since we reverted
                break;
            }
        }
        tracing::info!("DNS watchdog task stopped.");
    });
}
