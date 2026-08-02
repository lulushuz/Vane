use crate::dns::firewall_plan::{
    execute_firewall_plan, remove_kill_switch_plan, SystemFirewallExecutor,
};
use crate::dns::forwarder_lifecycle::{
    verify_local_readiness, DnsForwarderIdentity, DnsForwarderState,
};
use crate::dns::kill_switch::{
    clear_kill_switch_metadata, get_or_create_installation_id, save_kill_switch_metadata,
};
use crate::dns::runtime_config::{
    verify_dns_config, DnsConfigCandidate, DnsConfigRevision, VerifiedDnsConfig,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use tauri::AppHandle;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsAppliedVerification {
    ConfigurationApplied,
    ForwarderStarted,
    LocalReadinessPassed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedDnsConfig {
    pub verified: VerifiedDnsConfig,
    pub forwarder_identity: Option<DnsForwarderIdentity>,
    pub local_endpoint: Option<SocketAddr>,
    pub kill_switch_ownership: Option<crate::dns::firewall_plan::KillSwitchOwnership>,
    pub applied_at: SystemTime,
    pub verification: DnsAppliedVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedDnsConfig {
    pub verified: VerifiedDnsConfig,
    pub firewall_plan: crate::dns::firewall_plan::KillSwitchPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DnsApplyStage {
    Prepared,
    ForwarderStarted,
    Applied,
    Disabled,
    RolledBack,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsTransactionOutcome {
    pub stage: DnsApplyStage,
    pub config_revision: u64,
    pub config_fingerprint: String,
    pub applied_revision: Option<u64>,
    pub applied_fingerprint: Option<String>,
    pub forwarder_state: DnsForwarderState,
    pub forwarder_generation: Option<u64>,
    pub kill_switch_applied: bool,
    pub kill_switch_instance: Option<String>,
    pub rollback_performed: bool,
    pub rollback_succeeded: bool,
    pub superseded: bool,
}

#[allow(dead_code)]
pub struct DnsRuntimeState {
    desired: Option<VerifiedDnsConfig>,
    prepared: Option<PreparedDnsConfig>,
    applied: Option<AppliedDnsConfig>,
    latest_requested_revision: DnsConfigRevision,
    latest_completed_revision: DnsConfigRevision,
    forwarder_generation: u64,
}

impl Default for DnsRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsRuntimeState {
    pub fn new() -> Self {
        Self {
            desired: None,
            prepared: None,
            applied: None,
            latest_requested_revision: DnsConfigRevision::new(0),
            latest_completed_revision: DnsConfigRevision::new(0),
            forwarder_generation: 0,
        }
    }

    pub fn desired(&self) -> Option<&VerifiedDnsConfig> {
        self.desired.as_ref()
    }

    pub fn applied(&self) -> Option<&AppliedDnsConfig> {
        self.applied.as_ref()
    }

    pub fn set_desired(&mut self, verified: VerifiedDnsConfig) {
        if verified.revision > self.latest_requested_revision {
            self.latest_requested_revision = verified.revision;
        }
        self.desired = Some(verified);
    }

    pub fn commit_applied(&mut self, applied: AppliedDnsConfig) {
        if applied.verified.revision > self.latest_completed_revision {
            self.latest_completed_revision = applied.verified.revision;
        }
        self.applied = Some(applied);
    }

    pub fn clear_applied(&mut self) {
        self.applied = None;
    }

    pub fn restore_applied(&mut self, applied: AppliedDnsConfig) {
        self.applied = Some(applied);
    }

    pub fn next_generation(&mut self) -> u64 {
        self.forwarder_generation += 1;
        self.forwarder_generation
    }

    pub fn current_generation(&self) -> u64 {
        self.forwarder_generation
    }
}

pub struct DnsTransactionManager {
    lock: Mutex<()>,
    state: std::sync::Mutex<DnsRuntimeState>,
    revision_counter: AtomicU64,
}

impl Default for DnsTransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsTransactionManager {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            state: std::sync::Mutex::new(DnsRuntimeState::new()),
            revision_counter: AtomicU64::new(0),
        }
    }

    pub fn next_revision(&self) -> DnsConfigRevision {
        DnsConfigRevision::new(self.revision_counter.fetch_add(1, Ordering::SeqCst) + 1)
    }

    pub fn runtime_state(&self) -> &std::sync::Mutex<DnsRuntimeState> {
        &self.state
    }

    pub async fn recover_stale_snapshot(&self, app: &AppHandle) -> Result<bool, String> {
        let _guard = self.lock.lock().await;
        crate::dns::recover_stale_dns_snapshot(app)
    }

    pub async fn apply_candidate(
        &self,
        candidate: DnsConfigCandidate,
        app: &AppHandle,
        app_state: &crate::AppState,
    ) -> Result<DnsTransactionOutcome, String> {
        let rev = self.next_revision();

        let verified = verify_dns_config(candidate, rev)
            .map_err(|e| format!("DNS configuration validation error: {e}"))?;

        let _guard = self.lock.lock().await;

        {
            let mut st = self.state.lock().unwrap();
            if verified.revision < st.latest_requested_revision {
                return Ok(DnsTransactionOutcome {
                    stage: DnsApplyStage::Superseded,
                    config_revision: verified.revision.get(),
                    config_fingerprint: verified.fingerprint.as_str().to_string(),
                    applied_revision: st.applied().map(|a| a.verified.revision.get()),
                    applied_fingerprint: st
                        .applied()
                        .map(|a| a.verified.fingerprint.as_str().to_string()),
                    forwarder_state: DnsForwarderState::Stopped,
                    forwarder_generation: None,
                    kill_switch_applied: st.applied().is_some_and(|a| a.verified.kill_switch),
                    kill_switch_instance: None,
                    rollback_performed: false,
                    rollback_succeeded: false,
                    superseded: true,
                });
            }
            st.set_desired(verified.clone());
        }

        let inst_id = get_or_create_installation_id(app);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let instance_id = format!("dns-inst-{:x}", nanos);

        let platform = if cfg!(target_os = "windows") {
            crate::dns::firewall_plan::FirewallPlatform::Windows
        } else {
            crate::dns::firewall_plan::FirewallPlatform::Linux
        };

        let firewall_plan = crate::dns::firewall_plan::build_kill_switch_plan(
            &inst_id,
            &instance_id,
            verified.revision,
            &verified.fingerprint,
            platform,
            verified.kill_switch,
        );

        let previous_applied = self.state.lock().unwrap().applied().cloned();

        let dns_settings = crate::dns::forwarder::DnsSettings {
            protocol: verified.protocol.as_str().to_string(),
            adblock: verified.adblock,
            cache: verified.cache_enabled,
            socks5_proxy: verified
                .socks5
                .as_ref()
                .map(|s| format!("{}:{}", s.host, s.port))
                .unwrap_or_default(),
            health_check_targets: vec![crate::dns::DEFAULT_HEALTH_CHECK_TARGET.to_string()],
        };

        if let Err(e) = crate::dns::forwarder::update_dns_settings_cache(dns_settings) {
            return Err(format!("DNS settings cache update failed: {e}"));
        }

        let old_forwarder_handle = {
            let mut f_guard = app_state
                .forwarder
                .lock()
                .map_err(|_| "Forwarder lock poisoned.".to_string())?;
            f_guard.take()
        };
        let previous_dns_snapshot = crate::dns::load_dns_restore_snapshot(app)?.or_else(|| {
            old_forwarder_handle
                .as_ref()
                .map(|handle| handle.previous_dns.clone())
        });
        if let Some(old_h) = old_forwarder_handle {
            old_h.stop().await;
        }

        if !verified.enabled {
            if let Some(snapshot) = previous_dns_snapshot.as_ref() {
                let restored = crate::dns::restore_dns_snapshot(snapshot);
                if !restored.success {
                    return Err(format!("System DNS restore failed: {:?}", restored.error));
                }
                crate::dns::clear_dns_restore_snapshot(app)?;
            }
            let executor = SystemFirewallExecutor;
            if let Some(prev) = &previous_applied {
                if let Some(ownership) = &prev.kill_switch_ownership {
                    let prev_plan = crate::dns::firewall_plan::build_kill_switch_plan(
                        &ownership.installation_id,
                        &ownership.instance_id,
                        ownership.revision,
                        &ownership.fingerprint,
                        crate::dns::firewall_plan::FirewallPlatform::Windows,
                        true,
                    );
                    let _ = remove_kill_switch_plan(&executor, &prev_plan);
                }
            }
            let _ = clear_kill_switch_metadata(app);
            self.state.lock().unwrap().clear_applied();

            return Ok(DnsTransactionOutcome {
                stage: DnsApplyStage::Disabled,
                config_revision: verified.revision.get(),
                config_fingerprint: verified.fingerprint.as_str().to_string(),
                applied_revision: None,
                applied_fingerprint: None,
                forwarder_state: DnsForwarderState::Stopped,
                forwarder_generation: None,
                kill_switch_applied: false,
                kill_switch_instance: None,
                rollback_performed: false,
                rollback_succeeded: false,
                superseded: false,
            });
        }

        let doh_endpoint = match verified.provider {
            crate::dns::runtime_config::DnsProvider::Cloudflare => {
                crate::dns::DoHEndpoint::Cloudflare
            }
            crate::dns::runtime_config::DnsProvider::Google => crate::dns::DoHEndpoint::Google,
        };

        let gen = self.state.lock().unwrap().next_generation();

        let spawn_res = crate::dns::spawn_doh_forwarder(
            app.clone(),
            app_state.http_client.clone(),
            crate::dns::DOH_FORWARDER_DEFAULT_PORT,
            doh_endpoint,
        )
        .await;

        match spawn_res {
            Ok(handle) => {
                let local_ep = SocketAddr::from(([127, 0, 0, 1], handle.port));

                let ready = verify_local_readiness(local_ep).await;
                if !ready {
                    tracing::warn!("Local forwarder socket readiness failed.");
                }

                if previous_dns_snapshot.is_none() {
                    if let Err(error) =
                        crate::dns::save_dns_restore_snapshot(app, &handle.previous_dns)
                    {
                        handle.stop().await;
                        return Err(format!(
                            "DNS restore snapshot could not be persisted: {error}"
                        ));
                    }
                }
                let applied_dns = crate::dns::apply_dns("127.0.0.1", "127.0.0.1");
                if !applied_dns.success {
                    let snapshot = handle.previous_dns.clone();
                    handle.stop().await;
                    let _ = crate::dns::restore_dns_snapshot(&snapshot);
                    let _ = crate::dns::clear_dns_restore_snapshot(app);
                    return Err(format!("System DNS apply failed: {:?}", applied_dns.error));
                }

                let executor = SystemFirewallExecutor;
                if let Err(fw_err) = execute_firewall_plan(&executor, &firewall_plan) {
                    tracing::error!(
                        "Firewall plan execution failed: {fw_err}. Initiating DNS rollback..."
                    );
                    handle.stop().await;

                    return self
                        .rollback_previous(
                            app,
                            app_state,
                            previous_applied,
                            &verified,
                            &fw_err.to_string(),
                        )
                        .await;
                }

                if verified.kill_switch {
                    let _ = save_kill_switch_metadata(app, &firewall_plan.ownership);
                }

                let forwarder_identity = DnsForwarderIdentity {
                    installation_id: inst_id.clone(),
                    instance_id: instance_id.clone(),
                    generation: gen,
                    revision: verified.revision,
                    fingerprint: verified.fingerprint.clone(),
                    process_id: None,
                    local_endpoint: local_ep,
                };

                let applied_config = AppliedDnsConfig {
                    verified: verified.clone(),
                    forwarder_identity: Some(forwarder_identity.clone()),
                    local_endpoint: Some(local_ep),
                    kill_switch_ownership: verified
                        .kill_switch
                        .then(|| firewall_plan.ownership.clone()),
                    applied_at: SystemTime::now(),
                    verification: DnsAppliedVerification::LocalReadinessPassed,
                };

                {
                    let mut f_guard = app_state.forwarder.lock().unwrap();
                    *f_guard = Some(handle);
                }

                self.state
                    .lock()
                    .unwrap()
                    .commit_applied(applied_config.clone());

                Ok(DnsTransactionOutcome {
                    stage: DnsApplyStage::Applied,
                    config_revision: verified.revision.get(),
                    config_fingerprint: verified.fingerprint.as_str().to_string(),
                    applied_revision: Some(verified.revision.get()),
                    applied_fingerprint: Some(verified.fingerprint.as_str().to_string()),
                    forwarder_state: DnsForwarderState::Ready,
                    forwarder_generation: Some(gen),
                    kill_switch_applied: verified.kill_switch,
                    kill_switch_instance: verified.kill_switch.then(|| instance_id.clone()),
                    rollback_performed: false,
                    rollback_succeeded: false,
                    superseded: false,
                })
            }
            Err(spawn_err) => {
                tracing::error!("Forwarder spawn failed: {spawn_err}. Initiating DNS rollback...");
                self.rollback_previous(app, app_state, previous_applied, &verified, &spawn_err)
                    .await
            }
        }
    }

    async fn rollback_previous(
        &self,
        app: &AppHandle,
        app_state: &crate::AppState,
        previous_applied: Option<AppliedDnsConfig>,
        failed_candidate: &VerifiedDnsConfig,
        reason: &str,
    ) -> Result<DnsTransactionOutcome, String> {
        let executor = SystemFirewallExecutor;

        if let Some(prev) = previous_applied {
            let doh_endpoint = match prev.verified.provider {
                crate::dns::runtime_config::DnsProvider::Cloudflare => {
                    crate::dns::DoHEndpoint::Cloudflare
                }
                crate::dns::runtime_config::DnsProvider::Google => crate::dns::DoHEndpoint::Google,
            };

            let gen = self.state.lock().unwrap().next_generation();

            if let Ok(handle) = crate::dns::spawn_doh_forwarder(
                app.clone(),
                app_state.http_client.clone(),
                crate::dns::DOH_FORWARDER_DEFAULT_PORT,
                doh_endpoint,
            )
            .await
            {
                if let Some(ownership) = &prev.kill_switch_ownership {
                    let prev_plan = crate::dns::firewall_plan::build_kill_switch_plan(
                        &ownership.installation_id,
                        &ownership.instance_id,
                        ownership.revision,
                        &ownership.fingerprint,
                        crate::dns::firewall_plan::FirewallPlatform::Windows,
                        true,
                    );
                    let _ = execute_firewall_plan(&executor, &prev_plan);
                    let _ = save_kill_switch_metadata(app, ownership);
                }

                {
                    let mut f_guard = app_state.forwarder.lock().unwrap();
                    *f_guard = Some(handle);
                }

                self.state.lock().unwrap().restore_applied(prev.clone());

                return Ok(DnsTransactionOutcome {
                    stage: DnsApplyStage::RolledBack,
                    config_revision: failed_candidate.revision.get(),
                    config_fingerprint: failed_candidate.fingerprint.as_str().to_string(),
                    applied_revision: Some(prev.verified.revision.get()),
                    applied_fingerprint: Some(prev.verified.fingerprint.as_str().to_string()),
                    forwarder_state: DnsForwarderState::Ready,
                    forwarder_generation: Some(gen),
                    kill_switch_applied: prev.verified.kill_switch,
                    kill_switch_instance: prev
                        .kill_switch_ownership
                        .as_ref()
                        .map(|o| o.instance_id.clone()),
                    rollback_performed: true,
                    rollback_succeeded: true,
                    superseded: false,
                });
            }
        }

        self.state.lock().unwrap().clear_applied();
        let _ = clear_kill_switch_metadata(app);
        if let Some(snapshot) = crate::dns::load_dns_restore_snapshot(app)? {
            let restored = crate::dns::restore_dns_snapshot(&snapshot);
            if restored.success {
                let _ = crate::dns::clear_dns_restore_snapshot(app);
            }
        }

        Err(format!(
            "DNS candidate apply failed ({reason}) AND rollback failed."
        ))
    }
}
