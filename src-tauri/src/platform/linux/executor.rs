use crate::platform::linux::capabilities::{
    probe_linux_capabilities, LinuxPlatformCapabilities, LinuxPlatformError,
};
use crate::platform::linux::command::{
    LinuxCommandRunner, LinuxCommandSpec, SystemLinuxCommandRunner,
};
use crate::platform::linux::filter_plan::{
    LinuxFilterPlan, LinuxFirewallBackend, LinuxFirewallStep,
};
use crate::platform::linux::iptables::render_iptables_step_args;
use crate::platform::linux::nftables::{render_nftables_batch, render_nftables_cleanup};
use std::sync::{Arc, Mutex};

const STDERR_SUMMARY_LIMIT: usize = 512;

pub trait LinuxFirewallExecutor: Send + Sync {
    fn probe(&self) -> Result<LinuxPlatformCapabilities, LinuxPlatformError>;
    fn apply(&self, plan: &LinuxFilterPlan) -> Result<Vec<LinuxFirewallStep>, LinuxPlatformError>;
    fn remove(&self, plan: &LinuxFilterPlan) -> Result<(), LinuxPlatformError>;
}

pub struct SystemLinuxFirewallExecutor {
    runner: Arc<dyn LinuxCommandRunner>,
}

impl Default for SystemLinuxFirewallExecutor {
    fn default() -> Self {
        Self {
            runner: Arc::new(SystemLinuxCommandRunner),
        }
    }
}

impl SystemLinuxFirewallExecutor {
    #[cfg(test)]
    pub(crate) fn with_runner(runner: Arc<dyn LinuxCommandRunner>) -> Self {
        Self { runner }
    }
}

fn backend_name(backend: LinuxFirewallBackend) -> &'static str {
    match backend {
        LinuxFirewallBackend::Nftables => "nftables",
        LinuxFirewallBackend::Iptables => "iptables",
    }
}

fn stderr_summary(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(STDERR_SUMMARY_LIMIT)
        .collect::<String>()
        .trim()
        .to_string()
}

fn run_checked(
    runner: &dyn LinuxCommandRunner,
    operation: &str,
    backend: LinuxFirewallBackend,
    step_index: usize,
    command: LinuxCommandSpec,
) -> Result<(), String> {
    let context = format!(
        "operation={operation} backend={} step_index={step_index} program={} args={:?}",
        backend_name(backend),
        command.program,
        command.args
    );
    let output = runner
        .run(&command)
        .map_err(|error| format!("{context} io_error={error}"))?;
    match output.exit_code {
        Some(0) => Ok(()),
        Some(exit_code) => Err(format!(
            "{context} exit_code={exit_code} stderr={:?}",
            stderr_summary(&output.stderr)
        )),
        None => Err(format!(
            "{context} exit_code=signal stderr={:?}",
            stderr_summary(&output.stderr)
        )),
    }
}

fn iptables_command(step: &LinuxFirewallStep) -> Option<LinuxCommandSpec> {
    render_iptables_step_args(step).map(|(program, args)| LinuxCommandSpec {
        program,
        args,
        stdin: None,
    })
}

fn inverse_applied_step(step: &LinuxFirewallStep) -> Option<LinuxFirewallStep> {
    match step {
        LinuxFirewallStep::AddRule { table, chain, rule } => Some(LinuxFirewallStep::RemoveRule {
            table: table.clone(),
            chain: chain.clone(),
            rule: rule.clone(),
        }),
        LinuxFirewallStep::CreateContainer { table, chain } => {
            Some(LinuxFirewallStep::RemoveContainer {
                table: table.clone(),
                chain: chain.clone(),
            })
        }
        LinuxFirewallStep::RemoveRule { .. } | LinuxFirewallStep::RemoveContainer { .. } => None,
    }
}

impl LinuxFirewallExecutor for SystemLinuxFirewallExecutor {
    fn probe(&self) -> Result<LinuxPlatformCapabilities, LinuxPlatformError> {
        probe_linux_capabilities()
    }

    fn apply(&self, plan: &LinuxFilterPlan) -> Result<Vec<LinuxFirewallStep>, LinuxPlatformError> {
        match plan.backend {
            LinuxFirewallBackend::Nftables => {
                let batch = render_nftables_batch(plan);
                let command = LinuxCommandSpec {
                    program: "nft".into(),
                    args: vec!["-f".into(), "-".into()],
                    stdin: Some(batch.into_bytes()),
                };
                run_checked(self.runner.as_ref(), "apply", plan.backend, 0, command)
                    .map_err(LinuxPlatformError::NftablesBatchFailed)?;
                Ok(plan.apply_steps.clone())
            }
            LinuxFirewallBackend::Iptables => {
                let mut applied = Vec::new();
                for (step_index, step) in plan.apply_steps.iter().enumerate() {
                    let Some(command) = iptables_command(step) else {
                        continue;
                    };
                    if let Err(apply_error) = run_checked(
                        self.runner.as_ref(),
                        "apply",
                        plan.backend,
                        step_index,
                        command,
                    ) {
                        let mut rollback_failures = Vec::new();
                        for (rollback_index, applied_step) in applied.iter().rev().enumerate() {
                            if let Some(inverse) = inverse_applied_step(applied_step) {
                                if let Some(command) = iptables_command(&inverse) {
                                    if let Err(error) = run_checked(
                                        self.runner.as_ref(),
                                        "partial_rollback",
                                        plan.backend,
                                        rollback_index,
                                        command,
                                    ) {
                                        rollback_failures.push(error);
                                    }
                                }
                            }
                        }
                        if rollback_failures.is_empty() {
                            return Err(LinuxPlatformError::IptablesStepFailed(apply_error));
                        }
                        return Err(LinuxPlatformError::PartialApplyRollbackFailed(format!(
                            "original_failure=({apply_error}); rollback_failures=({})",
                            rollback_failures.join("; ")
                        )));
                    }
                    applied.push(step.clone());
                }
                Ok(applied)
            }
        }
    }

    fn remove(&self, plan: &LinuxFilterPlan) -> Result<(), LinuxPlatformError> {
        match plan.backend {
            LinuxFirewallBackend::Nftables => {
                let cleanup = render_nftables_cleanup(plan);
                let command = LinuxCommandSpec {
                    program: "nft".into(),
                    args: vec!["-f".into(), "-".into()],
                    stdin: Some(cleanup.into_bytes()),
                };
                run_checked(self.runner.as_ref(), "remove", plan.backend, 0, command)
                    .map_err(LinuxPlatformError::RuleRemovalFailed)
            }
            LinuxFirewallBackend::Iptables => {
                let mut failures = Vec::new();
                for (step_index, step) in plan.remove_steps.iter().enumerate() {
                    if let Some(command) = iptables_command(step) {
                        if let Err(error) = run_checked(
                            self.runner.as_ref(),
                            "remove",
                            plan.backend,
                            step_index,
                            command,
                        ) {
                            failures.push(error);
                        }
                    }
                }
                if failures.is_empty() {
                    Ok(())
                } else {
                    Err(LinuxPlatformError::RuleRemovalFailed(failures.join("; ")))
                }
            }
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
                    self.applied_steps
                        .lock()
                        .unwrap()
                        .retain(|existing| existing != applied);
                }
                return Err(LinuxPlatformError::IptablesStepFailed(format!(
                    "Simulated step failure at index {idx}"
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
                LinuxFirewallStep::RemoveRule { table, chain, rule } => guard.retain(|existing| {
                    !matches!(existing, LinuxFirewallStep::AddRule { table: t, chain: c, rule: r } if t == table && c == chain && r == rule)
                }),
                LinuxFirewallStep::RemoveContainer { table, chain } => guard.retain(|existing| {
                    !matches!(existing, LinuxFirewallStep::CreateContainer { table: t, chain: c } if t == table && c == chain)
                }),
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::linux::command::test_support::FakeLinuxCommandRunner;
    use crate::platform::linux::command::{LinuxCommandOutput, LinuxCommandRunError};
    use crate::platform::linux::{
        build_linux_filter_plan, LinuxFilterIntent, LinuxHostlistMode, LinuxRuleOwnership,
    };

    fn output(code: Option<i32>, stderr: &str) -> Result<LinuxCommandOutput, LinuxCommandRunError> {
        Ok(LinuxCommandOutput {
            exit_code: code,
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        })
    }

    fn plan(backend: LinuxFirewallBackend) -> LinuxFilterPlan {
        let capabilities = LinuxPlatformCapabilities {
            nftables_available: backend == LinuxFirewallBackend::Nftables,
            nft_atomic_batch: true,
            iptables_available: true,
            ip6tables_available: false,
            nfqueue_available: true,
            comment_match_available: true,
            ipv6_available: false,
            effective_uid: 0,
            has_required_privileges: true,
        };
        let ownership =
            LinuxRuleOwnership::new("install-123", "instance-123", 1, 2, "fingerprint", 4242);
        let intent =
            LinuxFilterIntent::from_specs(Some("80,443"), Some("443"), LinuxHostlistMode::All);
        build_linux_filter_plan(ownership, &intent, &capabilities)
    }

    fn executor(runner: Arc<FakeLinuxCommandRunner>) -> SystemLinuxFirewallExecutor {
        SystemLinuxFirewallExecutor::with_runner(runner)
    }

    #[test]
    fn nft_apply_stdin_write_failure_is_reported() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![Err(
            LinuxCommandRunError::StdinWrite("broken pipe".into()),
        )]));
        let result = executor(runner).apply(&plan(LinuxFirewallBackend::Nftables));
        assert!(
            matches!(result, Err(LinuxPlatformError::NftablesBatchFailed(message)) if message.contains("stdin write failed"))
        );
    }

    #[test]
    fn nft_apply_non_zero_exit_includes_exit_code_and_stderr() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![output(
            Some(1),
            "Operation not permitted",
        )]));
        let firewall_plan = plan(LinuxFirewallBackend::Nftables);
        let batch = render_nftables_batch(&firewall_plan);
        let error = executor(runner)
            .apply(&firewall_plan)
            .unwrap_err()
            .to_string();
        assert!(error.contains("exit_code=1"));
        assert!(error.contains("Operation not permitted"));
        assert!(!error.contains(&batch));
    }

    #[test]
    fn nft_remove_non_zero_exit_is_not_success() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![output(Some(1), "denied")]));
        assert!(executor(runner)
            .remove(&plan(LinuxFirewallBackend::Nftables))
            .is_err());
    }

    #[test]
    fn nft_remove_wait_failure_is_not_success() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![Err(
            LinuxCommandRunError::Wait("wait failed".into()),
        )]));
        assert!(executor(runner)
            .remove(&plan(LinuxFirewallBackend::Nftables))
            .is_err());
    }

    #[test]
    fn nft_remove_write_failure_is_not_success() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![Err(
            LinuxCommandRunError::StdinWrite("write failed".into()),
        )]));
        assert!(executor(runner)
            .remove(&plan(LinuxFirewallBackend::Nftables))
            .is_err());
    }

    #[test]
    fn iptables_remove_command_failure_is_not_success() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![
            output(Some(1), "denied"),
            output(Some(0), ""),
            output(Some(0), ""),
        ]));
        assert!(executor(runner)
            .remove(&plan(LinuxFirewallBackend::Iptables))
            .is_err());
    }

    #[test]
    fn iptables_remove_continues_after_individual_failure() {
        let firewall_plan = plan(LinuxFirewallBackend::Iptables);
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![
            output(Some(1), "denied"),
            output(Some(0), ""),
            output(Some(0), ""),
        ]));
        let result = executor(runner.clone()).remove(&firewall_plan);
        assert!(result.is_err());
        assert_eq!(runner.commands().len(), firewall_plan.remove_steps.len());
    }

    #[test]
    fn iptables_partial_apply_rolls_back_in_reverse_order() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![
            output(Some(0), ""),
            output(Some(0), ""),
            output(Some(1), "apply failed"),
            output(Some(0), ""),
            output(Some(0), ""),
        ]));
        let result = executor(runner.clone()).apply(&plan(LinuxFirewallBackend::Iptables));
        assert!(matches!(
            result,
            Err(LinuxPlatformError::IptablesStepFailed(_))
        ));
        let commands = runner.commands();
        assert!(commands[3].args.iter().any(|arg| arg == "-D"));
        assert!(commands[4].args.iter().any(|arg| arg == "-F"));
    }

    #[test]
    fn iptables_partial_rollback_failure_is_reported() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![
            output(Some(0), ""),
            output(Some(0), ""),
            output(Some(1), "apply failed"),
            output(Some(1), "rollback failed"),
            output(Some(0), ""),
        ]));
        let error = executor(runner)
            .apply(&plan(LinuxFirewallBackend::Iptables))
            .unwrap_err();
        assert!(
            matches!(error, LinuxPlatformError::PartialApplyRollbackFailed(message) if message.contains("apply failed") && message.contains("rollback failed"))
        );
    }

    #[test]
    fn signal_terminated_command_is_not_success() {
        let runner = Arc::new(FakeLinuxCommandRunner::new(vec![output(
            None,
            "terminated",
        )]));
        let error = executor(runner)
            .apply(&plan(LinuxFirewallBackend::Nftables))
            .unwrap_err();
        assert!(error.to_string().contains("exit_code=signal"));
    }

    #[test]
    fn successful_commands_preserve_existing_success_behavior() {
        let nft_runner = Arc::new(FakeLinuxCommandRunner::success(2));
        let nft = executor(nft_runner);
        let nft_plan = plan(LinuxFirewallBackend::Nftables);
        assert_eq!(nft.apply(&nft_plan).unwrap(), nft_plan.apply_steps);
        nft.remove(&nft_plan).unwrap();

        let iptables_plan = plan(LinuxFirewallBackend::Iptables);
        let command_count = iptables_plan.apply_steps.len() + iptables_plan.remove_steps.len();
        let iptables_runner = Arc::new(FakeLinuxCommandRunner::success(command_count));
        let iptables = executor(iptables_runner);
        assert_eq!(
            iptables.apply(&iptables_plan).unwrap(),
            iptables_plan.apply_steps
        );
        iptables.remove(&iptables_plan).unwrap();
    }
}
