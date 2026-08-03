use crate::dns::firewall_plan::{
    apply_plan_with_metadata, disable_plan_verified, execute_firewall_plan,
    rebuild_owned_kill_switch_plan, SystemFirewallExecutor,
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
use std::future::Future;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use tauri::AppHandle;
use tokio::sync::Mutex;

const FORWARDER_NOT_READY_ERROR: &str =
    "DNS forwarder did not become ready; system DNS and firewall were not changed.";

async fn require_forwarder_readiness<T, Stop, StopFuture>(
    ready: bool,
    candidate: T,
    stop: Stop,
) -> Result<T, String>
where
    Stop: FnOnce(T) -> StopFuture,
    StopFuture: Future<Output = ()>,
{
    if ready {
        Ok(candidate)
    } else {
        stop(candidate).await;
        Err(FORWARDER_NOT_READY_ERROR.to_string())
    }
}

fn restore_snapshot_verified<Restore, Clear>(restore: Restore, clear: Clear) -> Result<(), String>
where
    Restore: FnOnce() -> Result<(), String>,
    Clear: FnOnce() -> Result<(), String>,
{
    restore()?;
    clear().map_err(|error| format!("DNS snapshot was restored but could not be cleared: {error}"))
}

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
            if let Some(prev) = &previous_applied {
                if let Some(ownership) = &prev.kill_switch_ownership {
                    let prev_plan = rebuild_owned_kill_switch_plan(ownership);
                    let executor = SystemFirewallExecutor::new(ownership.platform);
                    disable_plan_verified(
                        &executor,
                        &prev_plan,
                        || clear_kill_switch_metadata(app),
                        || self.state.lock().unwrap().clear_applied(),
                    )
                    .map_err(|error| format!(
                        "DNS was restored and the forwarder was stopped, but Kill Switch disable failed; metadata and applied state were preserved: {error}"
                    ))?;
                } else {
                    clear_kill_switch_metadata(app)?;
                    self.state.lock().unwrap().clear_applied();
                }
            } else {
                clear_kill_switch_metadata(app)?;
                self.state.lock().unwrap().clear_applied();
            }

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
                let handle = match require_forwarder_readiness(ready, handle, |candidate| {
                    candidate.stop()
                })
                .await
                {
                    Ok(handle) => handle,
                    Err(readiness_error) => {
                        tracing::error!("{readiness_error}");
                        if previous_applied.is_some() {
                            let rollback = self
                                .rollback_previous(
                                    app,
                                    app_state,
                                    previous_applied,
                                    &verified,
                                    &readiness_error,
                                )
                                .await;
                            return match rollback {
                                Ok(outcome) if outcome.rollback_succeeded => Err(format!(
                                    "{readiness_error} Previous applied DNS configuration was restored."
                                )),
                                Ok(_) => Err(format!(
                                    "{readiness_error} Previous DNS rollback was not verified."
                                )),
                                Err(rollback_error) => Err(format!(
                                    "{readiness_error} Previous DNS rollback failed: {rollback_error}"
                                )),
                            };
                        }
                        return Err(readiness_error);
                    }
                };

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
                    restore_snapshot_verified(
                        || {
                            let restored = crate::dns::restore_dns_snapshot(&snapshot);
                            restored.success.then_some(()).ok_or_else(|| {
                                format!("snapshot restore failed: {:?}", restored.error)
                            })
                        },
                        || crate::dns::clear_dns_restore_snapshot(app),
                    )
                    .map_err(|error| format!(
                        "System DNS apply failed ({:?}); {error}; snapshot was preserved when restore failed.",
                        applied_dns.error
                    ))?;
                    return Err(format!("System DNS apply failed: {:?}", applied_dns.error));
                }

                let executor = SystemFirewallExecutor::new(firewall_plan.platform);
                let firewall_result = if verified.kill_switch {
                    apply_plan_with_metadata(&executor, &firewall_plan, |ownership| {
                        save_kill_switch_metadata(app, ownership)
                    })
                } else {
                    execute_firewall_plan(&executor, &firewall_plan)
                };
                if let Err(fw_err) = firewall_result {
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
        let mut rollback_failure_detail = "previous forwarder could not be restored".to_string();

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
                let local_ep = SocketAddr::from(([127, 0, 0, 1], handle.port));
                if !verify_local_readiness(local_ep).await {
                    handle.stop().await;
                    tracing::error!(
                        "Previous DNS forwarder rollback did not become locally ready."
                    );
                } else {
                    if prev.verified.kill_switch && prev.kill_switch_ownership.is_none() {
                        handle.stop().await;
                        return Err(format!(
                            "DNS candidate apply failed ({reason}) AND rollback failed: previous Kill Switch ownership/platform metadata is missing."
                        ));
                    }
                    if let Some(ownership) = &prev.kill_switch_ownership {
                        let prev_plan = rebuild_owned_kill_switch_plan(ownership);
                        let executor = SystemFirewallExecutor::new(ownership.platform);
                        if let Err(error) =
                            apply_plan_with_metadata(&executor, &prev_plan, |restored_ownership| {
                                save_kill_switch_metadata(app, restored_ownership)
                            })
                        {
                            rollback_failure_detail = format!(
                                "previous Kill Switch firewall/metadata restore failed: {error}"
                            );
                            handle.stop().await;
                            self.state.lock().unwrap().clear_applied();
                            if let Some(snapshot) = crate::dns::load_dns_restore_snapshot(app)? {
                                restore_snapshot_verified(
                                    || {
                                        let restored = crate::dns::restore_dns_snapshot(&snapshot);
                                        restored.success.then_some(()).ok_or_else(|| format!("snapshot restore failed: {:?}", restored.error))
                                    },
                                    || crate::dns::clear_dns_restore_snapshot(app),
                                ).map_err(|error| format!(
                                    "DNS candidate apply failed ({reason}); {rollback_failure_detail}; {error}"
                                ))?;
                            }
                            return Err(format!(
                                "DNS candidate apply failed ({reason}) AND rollback failed: {rollback_failure_detail}"
                            ));
                        }
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
        }

        self.state.lock().unwrap().clear_applied();
        if let Some(snapshot) = crate::dns::load_dns_restore_snapshot(app)? {
            restore_snapshot_verified(
                || {
                    let restored = crate::dns::restore_dns_snapshot(&snapshot);
                    restored
                        .success
                        .then_some(())
                        .ok_or_else(|| format!("snapshot restore failed: {:?}", restored.error))
                },
                || crate::dns::clear_dns_restore_snapshot(app),
            )
            .map_err(|error| {
                format!("DNS candidate apply failed ({reason}); {rollback_failure_detail}; {error}")
            })?;
        }

        Err(format!(
            "DNS candidate apply failed ({reason}) AND rollback failed: {rollback_failure_detail}."
        ))
    }
}

#[cfg(test)]
mod readiness_gate_tests {
    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};

    #[derive(Default)]
    struct Effects {
        dns_apply_count: usize,
        firewall_apply_count: usize,
        metadata_write_count: usize,
        applied_state_committed: bool,
        candidate_stopped: bool,
        previous_config_preserved: bool,
        verification: Option<DnsAppliedVerification>,
    }

    struct Candidate(Arc<StdMutex<Effects>>);

    async fn run_readiness_flow(
        ready: bool,
        previous_applied: bool,
    ) -> (Result<(), String>, Arc<StdMutex<Effects>>) {
        let effects = Arc::new(StdMutex::new(Effects::default()));
        let candidate = Candidate(effects.clone());
        let gated = require_forwarder_readiness(ready, candidate, |candidate| async move {
            candidate.0.lock().unwrap().candidate_stopped = true;
        })
        .await;

        let result = match gated {
            Ok(_candidate) => {
                let mut effects = effects.lock().unwrap();
                effects.dns_apply_count += 1;
                effects.firewall_apply_count += 1;
                effects.metadata_write_count += 1;
                effects.applied_state_committed = true;
                effects.verification = Some(DnsAppliedVerification::LocalReadinessPassed);
                Ok(())
            }
            Err(error) => {
                if previous_applied {
                    effects.lock().unwrap().previous_config_preserved = true;
                }
                Err(error)
            }
        };

        (result, effects)
    }

    #[tokio::test]
    async fn readiness_failure_does_not_apply_system_dns() {
        let (result, effects) = run_readiness_flow(false, false).await;
        assert!(result.is_err());
        assert_eq!(effects.lock().unwrap().dns_apply_count, 0);
    }

    #[tokio::test]
    async fn readiness_failure_does_not_apply_firewall() {
        let (result, effects) = run_readiness_flow(false, false).await;
        assert!(result.is_err());
        let effects = effects.lock().unwrap();
        assert_eq!(effects.firewall_apply_count, 0);
        assert_eq!(effects.metadata_write_count, 0);
    }

    #[tokio::test]
    async fn readiness_failure_does_not_commit_applied_state() {
        let (result, effects) = run_readiness_flow(false, false).await;
        assert!(result.is_err());
        let effects = effects.lock().unwrap();
        assert!(!effects.applied_state_committed);
        assert_ne!(
            effects.verification,
            Some(DnsAppliedVerification::LocalReadinessPassed)
        );
    }

    #[tokio::test]
    async fn readiness_success_allows_transaction_to_continue() {
        let (result, effects) = run_readiness_flow(true, false).await;
        assert!(result.is_ok());
        let effects = effects.lock().unwrap();
        assert_eq!(effects.dns_apply_count, 1);
        assert_eq!(effects.firewall_apply_count, 1);
        assert_eq!(effects.metadata_write_count, 1);
        assert!(effects.applied_state_committed);
        assert_eq!(
            effects.verification,
            Some(DnsAppliedVerification::LocalReadinessPassed)
        );
    }

    #[tokio::test]
    async fn readiness_failure_stops_candidate_forwarder() {
        let (result, effects) = run_readiness_flow(false, false).await;
        assert!(result.is_err());
        assert!(effects.lock().unwrap().candidate_stopped);
    }

    #[tokio::test]
    async fn readiness_failure_preserves_or_restores_previous_applied_config() {
        let (result, effects) = run_readiness_flow(false, true).await;
        assert!(result.is_err());
        let effects = effects.lock().unwrap();
        assert!(effects.previous_config_preserved);
        assert!(!effects.applied_state_committed);
    }

    #[test]
    fn snapshot_restore_failure_preserves_snapshot() {
        let clear_called = StdMutex::new(false);
        let result = restore_snapshot_verified(
            || Err("restore failed".into()),
            || {
                *clear_called.lock().unwrap() = true;
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!*clear_called.lock().unwrap());
    }

    #[test]
    fn snapshot_clear_failure_is_reported() {
        let result = restore_snapshot_verified(|| Ok(()), || Err("clear denied".into()));
        assert!(matches!(result, Err(error) if error.contains("clear denied")));
    }
}
