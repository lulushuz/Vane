use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Emitter;

pub fn spawn_dns_watchdog(
    client: reqwest::Client,
    endpoint: crate::dns::forwarder::DoHEndpoint,
    protocol: String,
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
                crate::dns::forwarder::probe_dot_upstream(endpoint).await
            } else {
                crate::dns::doh::resolve_doh(&client, endpoint.url(), "example.com")
                    .await
                    .success
            };

            match reachable {
                true => {
                    fail_count = 0;
                }
                false => {
                    fail_count += 1;
                    tracing::warn!(
                        "DNS watchdog: {} upstream could not resolve the health-check domain (failure {}/3).",
                        protocol.to_uppercase(),
                        fail_count
                    );
                }
            }

            if fail_count >= 3 {
                tracing::error!("CRITICAL: DNS upstream failed three real resolution checks. Reverting system DNS to DHCP!");
                
                // Revert system DNS to DHCP
                let res = crate::dns::reset_dns_to_dhcp();
                if res.success {
                    tracing::info!("System DNS reverted to DHCP successfully.");
                    let _ = app_handle.emit("dns_status_changed", ());
                    let _ = app_handle.emit("dns_auto_applied", "DoH bağlantısı koptu. İnternet erişimini kurtarmak için sistem DNS'i otomatik olarak DHCP'ye sıfırlandı.");
                } else {
                    tracing::error!("Failed to revert system DNS to DHCP: {:?}", res.error);
                }
                
                // Exit watchdog since we reverted
                break;
            }
        }
        tracing::info!("DNS watchdog task stopped.");
    });
}
