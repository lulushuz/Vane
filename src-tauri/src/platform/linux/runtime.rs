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
        let executor: Arc<dyn LinuxFirewallExecutor> = Arc::new(SystemLinuxFirewallExecutor);
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
            let _ = executor.remove(&plan);
            return Err(LinuxPlatformError::MetadataFailure(error));
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
        if !self.committed {
            return Ok(());
        }
        self.executor.remove(&self.plan)?;
        crate::platform::linux::clear_linux_filter_metadata(&self.app)
            .map_err(LinuxPlatformError::MetadataFailure)?;
        self.committed = false;
        Ok(())
    }
}

impl Drop for LinuxFilterGuard {
    fn drop(&mut self) {
        let _ = self.remove();
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
    use super::deterministic_queue;

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
}
