use crate::dns::runtime_config::{DnsConfigFingerprint, DnsConfigRevision};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;

const STDERR_LIMIT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirewallPlatform {
    Windows,
    Linux,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillSwitchOwnership {
    pub installation_id: String,
    pub instance_id: String,
    pub revision: DnsConfigRevision,
    pub fingerprint: DnsConfigFingerprint,
    pub platform: FirewallPlatform,
    pub rule_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirewallRuleSpec {
    pub name: String,
    pub direction: String,
    pub action: String,
    pub protocol: String,
    pub port: u16,
    pub remote_ip: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirewallStep {
    AddRule(FirewallRuleSpec),
    RemoveRule(FirewallRuleSpec),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillSwitchPlan {
    pub ownership: KillSwitchOwnership,
    pub platform: FirewallPlatform,
    pub apply_steps: Vec<FirewallStep>,
    pub rollback_steps: Vec<FirewallStep>,
    pub remove_steps: Vec<FirewallStep>,
}

pub fn build_kill_switch_plan(
    installation_id: &str,
    instance_id: &str,
    revision: DnsConfigRevision,
    fingerprint: &DnsConfigFingerprint,
    platform: FirewallPlatform,
    enabled: bool,
) -> KillSwitchPlan {
    let inst_prefix = &installation_id[..installation_id.len().min(8)];
    let instance_prefix = &instance_id[..instance_id.len().min(8)];
    let rev_num = revision.get();
    let names = [
        format!("Vane-DNS-{inst_prefix}-{instance_prefix}-r{rev_num}-AllowUDP"),
        format!("Vane-DNS-{inst_prefix}-{instance_prefix}-r{rev_num}-AllowTCP"),
        format!("Vane-DNS-{inst_prefix}-{instance_prefix}-r{rev_num}-UDP53"),
        format!("Vane-DNS-{inst_prefix}-{instance_prefix}-r{rev_num}-TCP53"),
    ];
    let ownership = KillSwitchOwnership {
        installation_id: installation_id.into(),
        instance_id: instance_id.into(),
        revision,
        fingerprint: fingerprint.clone(),
        platform,
        rule_ids: names.to_vec(),
    };
    if !enabled {
        return KillSwitchPlan {
            ownership,
            platform,
            apply_steps: Vec::new(),
            rollback_steps: Vec::new(),
            remove_steps: Vec::new(),
        };
    }
    let comment = Some(format!(
        "Vane DNS KillSwitch inst={inst_prefix} rev={rev_num}"
    ));
    let specs = vec![
        FirewallRuleSpec {
            name: names[0].clone(),
            direction: "out".into(),
            action: "allow".into(),
            protocol: "UDP".into(),
            port: 53,
            remote_ip: Some("127.0.0.1".into()),
            comment: comment.clone(),
        },
        FirewallRuleSpec {
            name: names[1].clone(),
            direction: "out".into(),
            action: "allow".into(),
            protocol: "TCP".into(),
            port: 53,
            remote_ip: Some("127.0.0.1".into()),
            comment: comment.clone(),
        },
        FirewallRuleSpec {
            name: names[2].clone(),
            direction: "out".into(),
            action: "block".into(),
            protocol: "UDP".into(),
            port: 53,
            remote_ip: None,
            comment: comment.clone(),
        },
        FirewallRuleSpec {
            name: names[3].clone(),
            direction: "out".into(),
            action: "block".into(),
            protocol: "TCP".into(),
            port: 53,
            remote_ip: None,
            comment,
        },
    ];
    let apply_steps = specs.iter().cloned().map(FirewallStep::AddRule).collect();
    let rollback_steps: Vec<_> = specs
        .into_iter()
        .rev()
        .map(FirewallStep::RemoveRule)
        .collect();
    KillSwitchPlan {
        ownership,
        platform,
        apply_steps,
        remove_steps: rollback_steps.clone(),
        rollback_steps,
    }
}

pub fn rebuild_owned_kill_switch_plan(ownership: &KillSwitchOwnership) -> KillSwitchPlan {
    build_kill_switch_plan(
        &ownership.installation_id,
        &ownership.instance_id,
        ownership.revision,
        &ownership.fingerprint,
        ownership.platform,
        true,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirewallCommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirewallCommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FirewallCommandRunError {
    #[error("spawn/wait failed: {0}")]
    Io(String),
}

pub trait FirewallCommandRunner: Send + Sync {
    fn run(
        &self,
        command: &FirewallCommandSpec,
    ) -> Result<FirewallCommandOutput, FirewallCommandRunError>;
}

struct SystemFirewallCommandRunner;

impl FirewallCommandRunner for SystemFirewallCommandRunner {
    fn run(
        &self,
        command: &FirewallCommandSpec,
    ) -> Result<FirewallCommandOutput, FirewallCommandRunError> {
        let mut process = Command::new(&command.program);
        process.args(&command.args);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(0x08000000);
        }
        let output = process
            .output()
            .map_err(|error| FirewallCommandRunError::Io(error.to_string()))?;
        Ok(FirewallCommandOutput {
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FirewallExecutionError {
    #[error("Firewall command failed: {0}")]
    CommandFailed(String),
    #[error("Partial apply failed at step {step_index}: {reason}")]
    PartialApplyFailed { step_index: usize, reason: String },
    #[error("Partial apply failed at step {step_index}: {apply_reason}; rollback failures: {rollback_failures:?}")]
    PartialApplyRollbackFailed {
        step_index: usize,
        apply_reason: String,
        rollback_failures: Vec<String>,
    },
    #[error("Firewall removal failed: {failures:?}")]
    RemovalFailed { failures: Vec<String> },
    #[error("Firewall metadata transaction failed: {0}")]
    MetadataTransactionFailed(String),
}

pub trait FirewallExecutor {
    fn execute(&self, step: &FirewallStep) -> Result<(), FirewallExecutionError>;
}

pub struct SystemFirewallExecutor {
    platform: FirewallPlatform,
    runner: Arc<dyn FirewallCommandRunner>,
}

impl SystemFirewallExecutor {
    pub fn new(platform: FirewallPlatform) -> Self {
        Self {
            platform,
            runner: Arc::new(SystemFirewallCommandRunner),
        }
    }

    #[cfg(test)]
    fn with_runner(platform: FirewallPlatform, runner: Arc<dyn FirewallCommandRunner>) -> Self {
        Self { platform, runner }
    }
}

fn validate_spec(spec: &FirewallRuleSpec) -> Result<(), FirewallExecutionError> {
    if spec.direction != "out"
        || !matches!(spec.action.as_str(), "allow" | "block")
        || !matches!(spec.protocol.as_str(), "UDP" | "TCP")
        || spec.port == 0
    {
        return Err(FirewallExecutionError::CommandFailed(
            "Invalid firewall rule specification".into(),
        ));
    }
    Ok(())
}

fn command_for(
    platform: FirewallPlatform,
    step: &FirewallStep,
) -> Result<FirewallCommandSpec, FirewallExecutionError> {
    let spec = match step {
        FirewallStep::AddRule(spec) | FirewallStep::RemoveRule(spec) => spec,
    };
    validate_spec(spec)?;
    match (platform, step) {
        (FirewallPlatform::Windows, FirewallStep::AddRule(_)) => {
            let mut args = vec![
                "advfirewall".into(),
                "firewall".into(),
                "add".into(),
                "rule".into(),
                format!("name={}", spec.name),
            ];
            args.extend([
                format!("dir={}", spec.direction),
                format!("action={}", spec.action),
                format!("protocol={}", spec.protocol),
                format!("remoteport={}", spec.port),
            ]);
            if let Some(ip) = &spec.remote_ip {
                args.push(format!("remoteip={ip}"));
            }
            Ok(FirewallCommandSpec {
                program: "netsh".into(),
                args,
            })
        }
        (FirewallPlatform::Windows, FirewallStep::RemoveRule(_)) => Ok(FirewallCommandSpec {
            program: "netsh".into(),
            args: vec![
                "advfirewall".into(),
                "firewall".into(),
                "delete".into(),
                "rule".into(),
                format!("name={}", spec.name),
            ],
        }),
        (FirewallPlatform::Linux, linux_step) => {
            let mut args = vec![
                if matches!(linux_step, FirewallStep::AddRule(_)) {
                    "-A"
                } else {
                    "-D"
                }
                .into(),
                "OUTPUT".into(),
                "-p".into(),
                spec.protocol.clone(),
                "--dport".into(),
                spec.port.to_string(),
            ];
            if let Some(ip) = &spec.remote_ip {
                args.extend(["-d".into(), ip.clone()]);
            }
            args.extend([
                "-j".into(),
                if spec.action == "allow" {
                    "ACCEPT".into()
                } else {
                    "DROP".into()
                },
            ]);
            Ok(FirewallCommandSpec {
                program: "iptables".into(),
                args,
            })
        }
    }
}

fn stderr_summary(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(STDERR_LIMIT)
        .collect::<String>()
        .trim()
        .to_string()
}

impl FirewallExecutor for SystemFirewallExecutor {
    fn execute(&self, step: &FirewallStep) -> Result<(), FirewallExecutionError> {
        let command = command_for(self.platform, step)?;
        let operation = if matches!(step, FirewallStep::AddRule(_)) {
            "add"
        } else {
            "remove"
        };
        let rule_name = match step {
            FirewallStep::AddRule(s) | FirewallStep::RemoveRule(s) => &s.name,
        };
        let context = format!(
            "operation={operation} platform={:?} rule={rule_name:?} program={} args={:?}",
            self.platform, command.program, command.args
        );
        let output = self.runner.run(&command).map_err(|error| {
            FirewallExecutionError::CommandFailed(format!("{context} io_error={error}"))
        })?;
        match output.exit_code {
            Some(0) => Ok(()),
            Some(code) => Err(FirewallExecutionError::CommandFailed(format!(
                "{context} exit_code={code} stderr={:?}",
                stderr_summary(&output.stderr)
            ))),
            None => Err(FirewallExecutionError::CommandFailed(format!(
                "{context} exit_code=signal stderr={:?}",
                stderr_summary(&output.stderr)
            ))),
        }
    }
}

fn inverse(step: &FirewallStep) -> FirewallStep {
    match step {
        FirewallStep::AddRule(spec) => FirewallStep::RemoveRule(spec.clone()),
        FirewallStep::RemoveRule(spec) => FirewallStep::AddRule(spec.clone()),
    }
}

pub fn execute_firewall_plan<E: FirewallExecutor>(
    executor: &E,
    plan: &KillSwitchPlan,
) -> Result<Vec<FirewallStep>, FirewallExecutionError> {
    let mut applied = Vec::new();
    for (step_index, step) in plan.apply_steps.iter().enumerate() {
        if let Err(error) = executor.execute(step) {
            let apply_reason = error.to_string();
            let mut rollback_failures = Vec::new();
            for applied_step in applied.iter().rev() {
                if let Err(rollback_error) = executor.execute(&inverse(applied_step)) {
                    rollback_failures.push(rollback_error.to_string());
                }
            }
            return if rollback_failures.is_empty() {
                Err(FirewallExecutionError::PartialApplyFailed {
                    step_index,
                    reason: apply_reason,
                })
            } else {
                Err(FirewallExecutionError::PartialApplyRollbackFailed {
                    step_index,
                    apply_reason,
                    rollback_failures,
                })
            };
        }
        applied.push(step.clone());
    }
    Ok(applied)
}

pub fn remove_kill_switch_plan<E: FirewallExecutor>(
    executor: &E,
    plan: &KillSwitchPlan,
) -> Result<(), FirewallExecutionError> {
    let failures: Vec<_> = plan
        .remove_steps
        .iter()
        .filter_map(|step| executor.execute(step).err().map(|error| error.to_string()))
        .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(FirewallExecutionError::RemovalFailed { failures })
    }
}

pub fn apply_plan_with_metadata<E, S>(
    executor: &E,
    plan: &KillSwitchPlan,
    save_metadata: S,
) -> Result<Vec<FirewallStep>, FirewallExecutionError>
where
    E: FirewallExecutor,
    S: FnOnce(&KillSwitchOwnership) -> Result<(), String>,
{
    let applied = execute_firewall_plan(executor, plan)?;
    if let Err(metadata_error) = save_metadata(&plan.ownership) {
        return match remove_kill_switch_plan(executor, plan) {
            Ok(()) => Err(FirewallExecutionError::MetadataTransactionFailed(
                metadata_error,
            )),
            Err(cleanup_error) => Err(FirewallExecutionError::MetadataTransactionFailed(format!(
                "metadata_failure=({metadata_error}); cleanup_failure=({cleanup_error})"
            ))),
        };
    }
    Ok(applied)
}

pub fn disable_plan_verified<E, C, A>(
    executor: &E,
    plan: &KillSwitchPlan,
    clear_metadata: C,
    clear_applied: A,
) -> Result<(), FirewallExecutionError>
where
    E: FirewallExecutor,
    C: FnOnce() -> Result<(), String>,
    A: FnOnce(),
{
    remove_kill_switch_plan(executor, plan)?;
    clear_metadata().map_err(FirewallExecutionError::MetadataTransactionFailed)?;
    clear_applied();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    fn plan(platform: FirewallPlatform) -> KillSwitchPlan {
        build_kill_switch_plan(
            "installation",
            "instance",
            DnsConfigRevision(7),
            &DnsConfigFingerprint("fingerprint".into()),
            platform,
            true,
        )
    }

    fn output(
        code: Option<i32>,
        stderr: &str,
    ) -> Result<FirewallCommandOutput, FirewallCommandRunError> {
        Ok(FirewallCommandOutput {
            exit_code: code,
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        })
    }

    struct FakeRunner {
        outcomes: Mutex<VecDeque<Result<FirewallCommandOutput, FirewallCommandRunError>>>,
        commands: Mutex<Vec<FirewallCommandSpec>>,
    }
    impl FakeRunner {
        fn new(outcomes: Vec<Result<FirewallCommandOutput, FirewallCommandRunError>>) -> Self {
            Self {
                outcomes: Mutex::new(outcomes.into()),
                commands: Mutex::new(Vec::new()),
            }
        }
        fn commands(&self) -> Vec<FirewallCommandSpec> {
            self.commands.lock().unwrap().clone()
        }
    }
    impl FirewallCommandRunner for FakeRunner {
        fn run(
            &self,
            command: &FirewallCommandSpec,
        ) -> Result<FirewallCommandOutput, FirewallCommandRunError> {
            self.commands.lock().unwrap().push(command.clone());
            self.outcomes
                .lock()
                .unwrap()
                .pop_front()
                .expect("fake outcome")
        }
    }

    struct SequenceExecutor {
        outcomes: Mutex<VecDeque<Result<(), FirewallExecutionError>>>,
        steps: Mutex<Vec<FirewallStep>>,
    }
    impl SequenceExecutor {
        fn new(outcomes: Vec<Result<(), FirewallExecutionError>>) -> Self {
            Self {
                outcomes: Mutex::new(outcomes.into()),
                steps: Mutex::new(Vec::new()),
            }
        }
    }
    impl FirewallExecutor for SequenceExecutor {
        fn execute(&self, step: &FirewallStep) -> Result<(), FirewallExecutionError> {
            self.steps.lock().unwrap().push(step.clone());
            self.outcomes.lock().unwrap().pop_front().unwrap_or(Ok(()))
        }
    }
    fn failed(label: &str) -> Result<(), FirewallExecutionError> {
        Err(FirewallExecutionError::CommandFailed(label.into()))
    }

    #[test]
    fn rollback_rebuilds_windows_plan_from_windows_ownership() {
        let original = plan(FirewallPlatform::Windows);
        assert_eq!(
            rebuild_owned_kill_switch_plan(&original.ownership).platform,
            FirewallPlatform::Windows
        );
    }
    #[test]
    fn rollback_rebuilds_linux_plan_from_linux_ownership() {
        let original = plan(FirewallPlatform::Linux);
        assert_eq!(
            rebuild_owned_kill_switch_plan(&original.ownership).platform,
            FirewallPlatform::Linux
        );
    }
    #[test]
    fn rollback_never_hardcodes_windows_platform() {
        let original = plan(FirewallPlatform::Linux);
        assert_eq!(
            rebuild_owned_kill_switch_plan(&original.ownership)
                .ownership
                .platform,
            FirewallPlatform::Linux
        );
    }
    #[test]
    fn windows_add_uses_netsh_add_rule() {
        let command = command_for(
            FirewallPlatform::Windows,
            &plan(FirewallPlatform::Windows).apply_steps[0],
        )
        .unwrap();
        assert_eq!(command.program, "netsh");
        assert_eq!(
            &command.args[..4],
            ["advfirewall", "firewall", "add", "rule"]
        );
    }
    #[test]
    fn windows_remove_uses_netsh_delete_rule() {
        let command = command_for(
            FirewallPlatform::Windows,
            &plan(FirewallPlatform::Windows).remove_steps[0],
        )
        .unwrap();
        assert_eq!(command.program, "netsh");
        assert_eq!(
            &command.args[..4],
            ["advfirewall", "firewall", "delete", "rule"]
        );
    }
    #[test]
    fn windows_remove_never_uses_remove_verb() {
        let command = command_for(
            FirewallPlatform::Windows,
            &plan(FirewallPlatform::Windows).remove_steps[0],
        )
        .unwrap();
        assert!(!command.args.iter().any(|argument| argument == "remove"));
    }
    #[test]
    fn linux_add_uses_iptables_append() {
        let command = command_for(
            FirewallPlatform::Linux,
            &plan(FirewallPlatform::Linux).apply_steps[0],
        )
        .unwrap();
        assert_eq!(command.program, "iptables");
        assert_eq!(&command.args[..2], ["-A", "OUTPUT"]);
    }
    #[test]
    fn linux_remove_uses_iptables_delete() {
        let command = command_for(
            FirewallPlatform::Linux,
            &plan(FirewallPlatform::Linux).remove_steps[0],
        )
        .unwrap();
        assert_eq!(command.program, "iptables");
        assert_eq!(&command.args[..2], ["-D", "OUTPUT"]);
    }
    #[test]
    fn windows_remove_non_zero_exit_is_error() {
        let runner = Arc::new(FakeRunner::new(vec![output(Some(1), "denied")]));
        let executor = SystemFirewallExecutor::with_runner(FirewallPlatform::Windows, runner);
        assert!(executor
            .execute(&plan(FirewallPlatform::Windows).remove_steps[0])
            .is_err());
    }
    #[test]
    fn linux_add_non_zero_exit_is_error() {
        let runner = Arc::new(FakeRunner::new(vec![output(Some(1), "denied")]));
        let executor = SystemFirewallExecutor::with_runner(FirewallPlatform::Linux, runner);
        assert!(executor
            .execute(&plan(FirewallPlatform::Linux).apply_steps[0])
            .is_err());
    }
    #[test]
    fn linux_remove_non_zero_exit_is_error() {
        let runner = Arc::new(FakeRunner::new(vec![output(Some(1), "denied")]));
        let executor = SystemFirewallExecutor::with_runner(FirewallPlatform::Linux, runner);
        assert!(executor
            .execute(&plan(FirewallPlatform::Linux).remove_steps[0])
            .is_err());
    }
    #[test]
    fn command_spawn_failure_is_error() {
        let runner = Arc::new(FakeRunner::new(vec![Err(FirewallCommandRunError::Io(
            "spawn".into(),
        ))]));
        let executor = SystemFirewallExecutor::with_runner(FirewallPlatform::Windows, runner);
        assert!(executor
            .execute(&plan(FirewallPlatform::Windows).apply_steps[0])
            .is_err());
    }
    #[test]
    fn signal_terminated_command_is_error() {
        let runner = Arc::new(FakeRunner::new(vec![output(None, "signal")]));
        let executor = SystemFirewallExecutor::with_runner(FirewallPlatform::Linux, runner);
        assert!(executor
            .execute(&plan(FirewallPlatform::Linux).apply_steps[0])
            .is_err());
    }
    #[test]
    fn successful_command_preserves_success_behavior() {
        let runner = Arc::new(FakeRunner::new(vec![output(Some(0), "warning")]));
        let executor = SystemFirewallExecutor::with_runner(FirewallPlatform::Linux, runner);
        assert!(executor
            .execute(&plan(FirewallPlatform::Linux).apply_steps[0])
            .is_ok());
    }
    #[test]
    fn linux_command_uses_planned_port() {
        let runner = Arc::new(FakeRunner::new(vec![output(Some(0), "")]));
        let executor = SystemFirewallExecutor::with_runner(FirewallPlatform::Linux, runner.clone());
        let mut step = plan(FirewallPlatform::Linux).apply_steps[0].clone();
        if let FirewallStep::AddRule(spec) = &mut step {
            spec.port = 5353;
        }
        executor.execute(&step).unwrap();
        assert!(runner.commands()[0].args.iter().any(|arg| arg == "5353"));
    }
    #[test]
    fn partial_apply_rolls_back_in_reverse_order() {
        let executor = SequenceExecutor::new(vec![Ok(()), Ok(()), failed("apply"), Ok(()), Ok(())]);
        assert!(execute_firewall_plan(&executor, &plan(FirewallPlatform::Windows)).is_err());
        let steps = executor.steps.lock().unwrap();
        assert_eq!(steps[3], inverse(&steps[1]));
        assert_eq!(steps[4], inverse(&steps[0]));
    }
    #[test]
    fn partial_apply_attempts_all_cleanup_steps() {
        let executor = SequenceExecutor::new(vec![
            Ok(()),
            Ok(()),
            failed("apply"),
            failed("undo1"),
            failed("undo2"),
        ]);
        assert!(execute_firewall_plan(&executor, &plan(FirewallPlatform::Windows)).is_err());
        assert_eq!(executor.steps.lock().unwrap().len(), 5);
    }
    #[test]
    fn partial_rollback_failure_is_reported_with_original_error() {
        let executor = SequenceExecutor::new(vec![Ok(()), failed("apply"), failed("rollback")]);
        let error = execute_firewall_plan(&executor, &plan(FirewallPlatform::Windows)).unwrap_err();
        assert!(
            matches!(error, FirewallExecutionError::PartialApplyRollbackFailed { apply_reason, rollback_failures, .. } if apply_reason.contains("apply") && rollback_failures[0].contains("rollback"))
        );
    }
    #[test]
    fn remove_plan_returns_error_when_any_step_fails() {
        let executor = SequenceExecutor::new(vec![failed("remove"), Ok(()), Ok(()), Ok(())]);
        assert!(remove_kill_switch_plan(&executor, &plan(FirewallPlatform::Windows)).is_err());
    }
    #[test]
    fn remove_plan_attempts_remaining_steps_after_failure() {
        let firewall_plan = plan(FirewallPlatform::Windows);
        let executor = SequenceExecutor::new(vec![failed("remove"), Ok(()), Ok(()), Ok(())]);
        let _ = remove_kill_switch_plan(&executor, &firewall_plan);
        assert_eq!(
            executor.steps.lock().unwrap().len(),
            firewall_plan.remove_steps.len()
        );
    }
    #[test]
    fn remove_plan_success_requires_all_steps_to_succeed() {
        assert!(remove_kill_switch_plan(
            &SequenceExecutor::new(vec![]),
            &plan(FirewallPlatform::Windows)
        )
        .is_ok());
    }
    #[test]
    fn metadata_save_failure_triggers_firewall_cleanup() {
        let firewall_plan = plan(FirewallPlatform::Windows);
        let executor = SequenceExecutor::new((0..8).map(|_| Ok(())).collect());
        assert!(
            apply_plan_with_metadata(&executor, &firewall_plan, |_| Err("metadata".into()))
                .is_err()
        );
        assert_eq!(executor.steps.lock().unwrap().len(), 8);
    }
    #[test]
    fn metadata_save_failure_does_not_commit_applied_state() {
        let committed = AtomicBool::new(false);
        let result = apply_plan_with_metadata(
            &SequenceExecutor::new((0..8).map(|_| Ok(())).collect()),
            &plan(FirewallPlatform::Windows),
            |_| Err("metadata".into()),
        );
        if result.is_ok() {
            committed.store(true, Ordering::SeqCst);
        }
        assert!(!committed.load(Ordering::SeqCst));
    }
    #[test]
    fn metadata_and_cleanup_failure_are_both_reported() {
        let executor = SequenceExecutor::new(vec![
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            failed("cleanup"),
            Ok(()),
            Ok(()),
            Ok(()),
        ]);
        let error = apply_plan_with_metadata(&executor, &plan(FirewallPlatform::Windows), |_| {
            Err("metadata".into())
        })
        .unwrap_err()
        .to_string();
        assert!(error.contains("metadata") && error.contains("cleanup"));
    }
    #[test]
    fn disable_remove_failure_preserves_metadata() {
        let clears = AtomicUsize::new(0);
        let executor = SequenceExecutor::new(vec![failed("remove"), Ok(()), Ok(()), Ok(())]);
        let _ = disable_plan_verified(
            &executor,
            &plan(FirewallPlatform::Windows),
            || {
                clears.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            || {},
        );
        assert_eq!(clears.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn disable_remove_failure_does_not_clear_applied_state() {
        let cleared = AtomicBool::new(false);
        let executor = SequenceExecutor::new(vec![failed("remove"), Ok(()), Ok(()), Ok(())]);
        let _ = disable_plan_verified(
            &executor,
            &plan(FirewallPlatform::Windows),
            || Ok(()),
            || cleared.store(true, Ordering::SeqCst),
        );
        assert!(!cleared.load(Ordering::SeqCst));
    }
    #[test]
    fn disable_metadata_clear_failure_is_not_reported_as_disabled() {
        let cleared = AtomicBool::new(false);
        let result = disable_plan_verified(
            &SequenceExecutor::new(vec![]),
            &plan(FirewallPlatform::Windows),
            || Err("clear".into()),
            || cleared.store(true, Ordering::SeqCst),
        );
        assert!(result.is_err() && !cleared.load(Ordering::SeqCst));
    }
    #[test]
    fn successful_disable_clears_metadata_and_applied_state() {
        let metadata = AtomicBool::new(false);
        let applied = AtomicBool::new(false);
        disable_plan_verified(
            &SequenceExecutor::new(vec![]),
            &plan(FirewallPlatform::Windows),
            || {
                metadata.store(true, Ordering::SeqCst);
                Ok(())
            },
            || applied.store(true, Ordering::SeqCst),
        )
        .unwrap();
        assert!(metadata.load(Ordering::SeqCst) && applied.load(Ordering::SeqCst));
    }
    #[test]
    fn rollback_without_kill_switch_does_not_touch_firewall() {
        let executor = SequenceExecutor::new(vec![]);
        let disabled = build_kill_switch_plan(
            "i",
            "x",
            DnsConfigRevision(1),
            &DnsConfigFingerprint("f".into()),
            FirewallPlatform::Linux,
            false,
        );
        assert!(execute_firewall_plan(&executor, &disabled).is_ok());
        assert!(executor.steps.lock().unwrap().is_empty());
    }

    #[test]
    fn rollback_firewall_apply_failure_is_not_success() {
        let executor = SequenceExecutor::new(vec![failed("firewall apply")]);
        assert!(
            apply_plan_with_metadata(&executor, &plan(FirewallPlatform::Linux), |_| Ok(()))
                .is_err()
        );
    }

    #[test]
    fn rollback_metadata_save_failure_is_not_success() {
        let executor = SequenceExecutor::new((0..8).map(|_| Ok(())).collect());
        assert!(
            apply_plan_with_metadata(&executor, &plan(FirewallPlatform::Linux), |_| Err(
                "metadata save".into()
            ))
            .is_err()
        );
    }

    #[test]
    fn rollback_success_requires_forwarder_firewall_and_metadata() {
        let forwarder_ready = true;
        let executor = SequenceExecutor::new((0..4).map(|_| Ok(())).collect());
        let firewall_and_metadata =
            apply_plan_with_metadata(&executor, &plan(FirewallPlatform::Linux), |_| Ok(()));
        assert!(forwarder_ready && firewall_and_metadata.is_ok());
    }
}
