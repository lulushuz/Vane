#[allow(unused_imports)]
use crate::platform::linux::capabilities::{
    probe_linux_capabilities, LinuxPlatformCapabilities, LinuxPlatformError,
};
#[allow(unused_imports)]
use crate::platform::linux::filter_plan::{
    LinuxFilterPlan, LinuxFirewallBackend, LinuxFirewallStep,
};
#[allow(unused_imports)]
use crate::platform::linux::iptables::render_iptables_step_args;
#[allow(unused_imports)]
use crate::platform::linux::nftables::{render_nftables_batch, render_nftables_cleanup};
use std::sync::Mutex;

pub trait LinuxFirewallExecutor: Send + Sync {
    fn probe(&self) -> Result<LinuxPlatformCapabilities, LinuxPlatformError>;
    fn apply(&self, plan: &LinuxFilterPlan) -> Result<Vec<LinuxFirewallStep>, LinuxPlatformError>;
    fn remove(&self, plan: &LinuxFilterPlan) -> Result<(), LinuxPlatformError>;
}

pub struct SystemLinuxFirewallExecutor;

impl LinuxFirewallExecutor for SystemLinuxFirewallExecutor {
    fn probe(&self) -> Result<LinuxPlatformCapabilities, LinuxPlatformError> {
        probe_linux_capabilities()
    }

    fn apply(&self, plan: &LinuxFilterPlan) -> Result<Vec<LinuxFirewallStep>, LinuxPlatformError> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            match plan.backend {
                LinuxFirewallBackend::Nftables => {
                    let batch = render_nftables_batch(plan);
                    let mut child = Command::new("nft")
                        .arg("-f")
                        .arg("-")
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .map_err(|e| LinuxPlatformError::NftablesBatchFailed(e.to_string()))?;

                    if let Some(stdin) = child.stdin.as_mut() {
                        use std::io::Write;
                        let _ = stdin.write_all(batch.as_bytes());
                    }
                    let status = child
                        .wait()
                        .map_err(|e| LinuxPlatformError::NftablesBatchFailed(e.to_string()))?;
                    if !status.success() {
                        return Err(LinuxPlatformError::NftablesBatchFailed(
                            "nft execution returned non-zero exit status".into(),
                        ));
                    }
                    Ok(plan.apply_steps.clone())
                }
                LinuxFirewallBackend::Iptables => {
                    let mut applied = Vec::new();
                    for (idx, step) in plan.apply_steps.iter().enumerate() {
                        if let Some((cmd_name, args)) = render_iptables_step_args(step) {
                            let res = Command::new(&cmd_name).args(&args).output();
                            match res {
                                Ok(out) if out.status.success() => {
                                    applied.push(step.clone());
                                }
                                Ok(out) => {
                                    let err_msg = String::from_utf8_lossy(&out.stderr).to_string();
                                    tracing::warn!(
                                        "Iptables step {} failed: {}. Executing reverse partial rollback...",
                                        idx, err_msg
                                    );
                                    for applied_step in applied.iter().rev() {
                                        if let LinuxFirewallStep::AddRule { table, chain, rule } =
                                            applied_step
                                        {
                                            let undo = LinuxFirewallStep::RemoveRule {
                                                table: table.clone(),
                                                chain: chain.clone(),
                                                rule: rule.clone(),
                                            };
                                            if let Some((c, a)) = render_iptables_step_args(&undo) {
                                                let _ = Command::new(&c).args(&a).output();
                                            }
                                        }
                                    }
                                    return Err(LinuxPlatformError::IptablesStepFailed(err_msg));
                                }
                                Err(e) => {
                                    return Err(LinuxPlatformError::IptablesStepFailed(
                                        e.to_string(),
                                    ));
                                }
                            }
                        }
                    }
                    Ok(applied)
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(plan.apply_steps.clone())
        }
    }

    fn remove(&self, plan: &LinuxFilterPlan) -> Result<(), LinuxPlatformError> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            match plan.backend {
                LinuxFirewallBackend::Nftables => {
                    let cleanup = render_nftables_cleanup(plan);
                    let mut child = Command::new("nft")
                        .arg("-f")
                        .arg("-")
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .map_err(|e| LinuxPlatformError::RuleRemovalFailed(e.to_string()))?;
                    if let Some(stdin) = child.stdin.as_mut() {
                        use std::io::Write;
                        let _ = stdin.write_all(cleanup.as_bytes());
                    }
                    let _ = child.wait();
                }
                LinuxFirewallBackend::Iptables => {
                    for step in &plan.remove_steps {
                        if let Some((cmd_name, args)) = render_iptables_step_args(step) {
                            let _ = Command::new(&cmd_name).args(&args).output();
                        }
                    }
                }
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = plan;
            Ok(())
        }
    }
}

pub struct FakeLinuxFirewallExecutor {
    pub applied_steps: Mutex<Vec<LinuxFirewallStep>>,
    pub fail_at_step: Option<usize>,
}

impl Default for FakeLinuxFirewallExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeLinuxFirewallExecutor {
    pub fn new() -> Self {
        Self {
            applied_steps: Mutex::new(Vec::new()),
            fail_at_step: None,
        }
    }

    pub fn with_fail_at(step_index: usize) -> Self {
        Self {
            applied_steps: Mutex::new(Vec::new()),
            fail_at_step: Some(step_index),
        }
    }
}

impl LinuxFirewallExecutor for FakeLinuxFirewallExecutor {
    fn probe(&self) -> Result<LinuxPlatformCapabilities, LinuxPlatformError> {
        Ok(LinuxPlatformCapabilities {
            nftables_available: true,
            nft_atomic_batch: true,
            iptables_available: true,
            ip6tables_available: true,
            nfqueue_available: true,
            comment_match_available: true,
            ipv6_available: true,
            effective_uid: 0,
            has_required_privileges: true,
        })
    }

    fn apply(&self, plan: &LinuxFilterPlan) -> Result<Vec<LinuxFirewallStep>, LinuxPlatformError> {
        let mut executed = Vec::new();
        for (idx, step) in plan.apply_steps.iter().enumerate() {
            if Some(idx) == self.fail_at_step {
                for applied in executed.iter().rev() {
                    let mut guard = self.applied_steps.lock().unwrap();
                    guard.retain(|s| s != applied);
                }
                return Err(LinuxPlatformError::IptablesStepFailed(format!(
                    "Simulated step failure at index {}",
                    idx
                )));
            }
            executed.push(step.clone());
            self.applied_steps.lock().unwrap().push(step.clone());
        }
        Ok(executed)
    }

    fn remove(&self, plan: &LinuxFilterPlan) -> Result<(), LinuxPlatformError> {
        let mut guard = self.applied_steps.lock().unwrap();
        for step in &plan.remove_steps {
            match step {
                LinuxFirewallStep::RemoveRule { table, chain, rule } => {
                    guard.retain(|s| {
                        !matches!(s, LinuxFirewallStep::AddRule { table: t, chain: c, rule: r } if t == table && c == chain && r == rule)
                    });
                }
                LinuxFirewallStep::RemoveContainer { table, chain } => {
                    guard.retain(|s| {
                        !matches!(s, LinuxFirewallStep::CreateContainer { table: t, chain: c } if t == table && c == chain)
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }
}
