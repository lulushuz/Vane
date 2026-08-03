use crate::platform::linux::{
    build_linux_filter_plan, LinuxFilterIntent, LinuxFilterPlan, LinuxFirewallBackend,
    LinuxFirewallExecutor, LinuxPlatformError, LinuxRuleOwnership, SystemLinuxFirewallExecutor,
};
use std::sync::Arc;
use tauri::AppHandle;

pub struct LinuxFilterGuard {
    app: AppHandle,
    plan: LinuxFilterPlan,
    executor: Arc<dyn LinuxFirewallExecutor>,
    committed: bool,
}

fn compensate_metadata_failure(
    executor: &dyn LinuxFirewallExecutor,
    plan: &LinuxFilterPlan,
    metadata_error: String,
) -> LinuxPlatformError {
    match executor.remove(plan) {
        Ok(()) => LinuxPlatformError::MetadataFailure(metadata_error),
        Err(cleanup_error) => LinuxPlatformError::PartialApplyRollbackFailed(format!(
            "metadata_failure=({metadata_error}); firewall_cleanup_failure=({cleanup_error})"
        )),
    }
}

fn remove_committed(
    executor: &dyn LinuxFirewallExecutor,
    plan: &LinuxFilterPlan,
    committed: &mut bool,
    clear_metadata: impl FnOnce() -> Result<(), String>,
) -> Result<(), LinuxPlatformError> {
    if !*committed {
        return Ok(());
    }
    executor.remove(plan)?;
    clear_metadata().map_err(LinuxPlatformError::MetadataFailure)?;
    *committed = false;
    Ok(())
}

impl LinuxFilterGuard {
    pub fn apply(
        app: &AppHandle,
        intent: LinuxFilterIntent,
        installation_id: &str,
        instance_id: &str,
        generation: u64,
        revision: u64,
        fingerprint: &str,
    ) -> Result<Self, LinuxPlatformError> {
        let executor: Arc<dyn LinuxFirewallExecutor> =
            Arc::new(SystemLinuxFirewallExecutor::default());
        let capabilities = executor.probe()?;
        if !capabilities.has_required_privileges {
            return Err(LinuxPlatformError::InsufficientPrivileges);
        }
        if !capabilities.nfqueue_available {
            return Err(LinuxPlatformError::MissingNfqueue);
        }
        if !capabilities.nftables_available && !capabilities.iptables_available {
            return Err(LinuxPlatformError::MissingFirewallBackend);
        }
        let queue = deterministic_queue(installation_id, instance_id, generation);
        if queue_in_use(queue) {
            return Err(LinuxPlatformError::QueueCollision(queue));
        }
        let ownership = LinuxRuleOwnership::new(
            installation_id,
            instance_id,
            generation,
            revision,
            fingerprint,
            queue,
        );
        let plan = build_linux_filter_plan(ownership, &intent, &capabilities);
        executor.apply(&plan)?;
        let backend = match plan.backend {
            LinuxFirewallBackend::Nftables => "nftables",
            LinuxFirewallBackend::Iptables => "iptables",
        };
        if let Err(error) =
            crate::platform::linux::save_linux_filter_metadata(app, &plan.ownership, backend)
        {
            return Err(compensate_metadata_failure(executor.as_ref(), &plan, error));
        }
        Ok(Self {
            app: app.clone(),
            plan,
            executor,
            committed: true,
        })
    }

    pub fn queue_number(&self) -> u16 {
        self.plan.queue_number
    }

    pub fn verify_owned(&self) -> bool {
        self.committed && self.plan.ownership.queue_number == self.plan.queue_number
    }

    pub fn remove(&mut self) -> Result<(), LinuxPlatformError> {
        remove_committed(
            self.executor.as_ref(),
            &self.plan,
            &mut self.committed,
            || crate::platform::linux::clear_linux_filter_metadata(&self.app),
        )
    }
}

impl Drop for LinuxFilterGuard {
    fn drop(&mut self) {
        if let Err(error) = self.remove() {
            tracing::error!(
                "Linux filter guard cleanup failed; owned metadata was preserved: {error}"
            );
        }
    }
}

pub fn deterministic_queue(installation_id: &str, instance_id: &str, generation: u64) -> u16 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in installation_id
        .bytes()
        .chain(instance_id.bytes())
        .chain(generation.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    1024 + (hash % 60000) as u16
}

fn queue_in_use(_queue: u16) -> bool {
    #[cfg(target_os = "linux")]
    {
        let needles = [
            format!("queue num {_queue}"),
            format!("--queue-num {_queue}"),
        ];
        for (command, args) in [("nft", vec!["list", "ruleset"]), ("iptables-save", vec![])] {
            if let Ok(output) = std::process::Command::new(command).args(args).output() {
                if output.status.success()
                    && needles
                        .iter()
                        .any(|needle| String::from_utf8_lossy(&output.stdout).contains(needle))
                {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{compensate_metadata_failure, deterministic_queue, remove_committed};
    use crate::platform::linux::{
        build_linux_filter_plan, LinuxFilterIntent, LinuxFirewallExecutor, LinuxHostlistMode,
        LinuxPlatformCapabilities, LinuxPlatformError, LinuxRuleOwnership,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct RemoveFailingExecutor;

    impl LinuxFirewallExecutor for RemoveFailingExecutor {
        fn probe(&self) -> Result<LinuxPlatformCapabilities, LinuxPlatformError> {
            unreachable!()
        }
        fn apply(
            &self,
            _plan: &crate::platform::linux::LinuxFilterPlan,
        ) -> Result<Vec<crate::platform::linux::LinuxFirewallStep>, LinuxPlatformError> {
            unreachable!()
        }
        fn remove(
            &self,
            _plan: &crate::platform::linux::LinuxFilterPlan,
        ) -> Result<(), LinuxPlatformError> {
            Err(LinuxPlatformError::RuleRemovalFailed(
                "cleanup denied".into(),
            ))
        }
    }

    fn plan() -> crate::platform::linux::LinuxFilterPlan {
        let capabilities = LinuxPlatformCapabilities {
            nftables_available: true,
            nft_atomic_batch: true,
            iptables_available: true,
            ip6tables_available: false,
            nfqueue_available: true,
            comment_match_available: true,
            ipv6_available: false,
            effective_uid: 0,
            has_required_privileges: true,
        };
        build_linux_filter_plan(
            LinuxRuleOwnership::new("installation", "instance", 1, 1, "fingerprint", 4242),
            &LinuxFilterIntent::from_specs(Some("443"), None, LinuxHostlistMode::All),
            &capabilities,
        )
    }

    #[test]
    fn queue_is_deterministic_non_global_and_generation_scoped() {
        let first = deterministic_queue("installation-a", "instance-a", 7);
        assert_eq!(
            first,
            deterministic_queue("installation-a", "instance-a", 7)
        );
        assert_ne!(first, 200);
        assert_ne!(
            first,
            deterministic_queue("installation-a", "instance-a", 8)
        );
        assert!((1024..=61023).contains(&first));
    }

    #[test]
    fn guard_remove_failure_preserves_metadata_and_committed_state() {
        let clear_count = AtomicUsize::new(0);
        let mut committed = true;
        let result = remove_committed(&RemoveFailingExecutor, &plan(), &mut committed, || {
            clear_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(clear_count.load(Ordering::SeqCst), 0);
        assert!(committed);
    }

    #[test]
    fn metadata_persistence_failure_and_cleanup_failure_are_both_reported() {
        let error = compensate_metadata_failure(
            &RemoveFailingExecutor,
            &plan(),
            "metadata disk full".into(),
        );
        assert!(
            matches!(error, LinuxPlatformError::PartialApplyRollbackFailed(message) if message.contains("metadata disk full") && message.contains("cleanup denied"))
        );
    }
}
