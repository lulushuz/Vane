use crate::dns::runtime_config::{DnsConfigFingerprint, DnsConfigRevision};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillSwitchOwnership {
    pub installation_id: String,
    pub instance_id: String,
    pub revision: DnsConfigRevision,
    pub fingerprint: DnsConfigFingerprint,
    pub rule_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirewallPlatform {
    Windows,
    Linux,
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
    let inst_prefix = if installation_id.len() >= 8 {
        &installation_id[..8]
    } else {
        installation_id
    };
    let instance_prefix = if instance_id.len() >= 8 {
        &instance_id[..8]
    } else {
        instance_id
    };
    let rev_num = revision.get();

    let rule_udp_name = format!(
        "Vane-DNS-{}-{}-r{}-UDP53",
        inst_prefix, instance_prefix, rev_num
    );
    let rule_tcp_name = format!(
        "Vane-DNS-{}-{}-r{}-TCP53",
        inst_prefix, instance_prefix, rev_num
    );
    let rule_allow_udp = format!(
        "Vane-DNS-{}-{}-r{}-AllowUDP",
        inst_prefix, instance_prefix, rev_num
    );
    let rule_allow_tcp = format!(
        "Vane-DNS-{}-{}-r{}-AllowTCP",
        inst_prefix, instance_prefix, rev_num
    );

    let ownership = KillSwitchOwnership {
        installation_id: installation_id.to_string(),
        instance_id: instance_id.to_string(),
        revision,
        fingerprint: fingerprint.clone(),
        rule_ids: vec![
            rule_allow_udp.clone(),
            rule_allow_tcp.clone(),
            rule_udp_name.clone(),
            rule_tcp_name.clone(),
        ],
    };

    if !enabled {
        return KillSwitchPlan {
            ownership,
            platform,
            apply_steps: vec![],
            rollback_steps: vec![],
            remove_steps: vec![],
        };
    }

    let comment_text = format!("Vane DNS KillSwitch inst={} rev={}", inst_prefix, rev_num);

    let allow_udp_spec = FirewallRuleSpec {
        name: rule_allow_udp,
        direction: "out".into(),
        action: "allow".into(),
        protocol: "UDP".into(),
        port: 53,
        remote_ip: Some("127.0.0.1".into()),
        comment: Some(comment_text.clone()),
    };
    let allow_tcp_spec = FirewallRuleSpec {
        name: rule_allow_tcp,
        direction: "out".into(),
        action: "allow".into(),
        protocol: "TCP".into(),
        port: 53,
        remote_ip: Some("127.0.0.1".into()),
        comment: Some(comment_text.clone()),
    };
    let block_udp_spec = FirewallRuleSpec {
        name: rule_udp_name,
        direction: "out".into(),
        action: "block".into(),
        protocol: "UDP".into(),
        port: 53,
        remote_ip: None,
        comment: Some(comment_text.clone()),
    };
    let block_tcp_spec = FirewallRuleSpec {
        name: rule_tcp_name,
        direction: "out".into(),
        action: "block".into(),
        protocol: "TCP".into(),
        port: 53,
        remote_ip: None,
        comment: Some(comment_text.clone()),
    };

    let apply_steps = vec![
        FirewallStep::AddRule(allow_udp_spec.clone()),
        FirewallStep::AddRule(allow_tcp_spec.clone()),
        FirewallStep::AddRule(block_udp_spec.clone()),
        FirewallStep::AddRule(block_tcp_spec.clone()),
    ];

    let rollback_steps = vec![
        FirewallStep::RemoveRule(block_tcp_spec.clone()),
        FirewallStep::RemoveRule(block_udp_spec.clone()),
        FirewallStep::RemoveRule(allow_tcp_spec.clone()),
        FirewallStep::RemoveRule(allow_udp_spec),
    ];

    let remove_steps = rollback_steps.clone();

    KillSwitchPlan {
        ownership,
        platform,
        apply_steps,
        rollback_steps,
        remove_steps,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FirewallExecutionError {
    #[error("Firewall command failed: {0}")]
    CommandFailed(String),
    #[error("Partial apply failed at step {step_index}: {reason}")]
    PartialApplyFailed { step_index: usize, reason: String },
}

pub trait FirewallExecutor {
    fn execute(&self, step: &FirewallStep) -> Result<(), FirewallExecutionError>;
}

pub struct SystemFirewallExecutor;

impl FirewallExecutor for SystemFirewallExecutor {
    fn execute(&self, step: &FirewallStep) -> Result<(), FirewallExecutionError> {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            match step {
                FirewallStep::AddRule(spec) => {
                    let mut args = vec![
                        "advfirewall".to_string(),
                        "firewall".to_string(),
                        "add".to_string(),
                        "rule".to_string(),
                        format!("name={}", spec.name),
                        format!("dir={}", spec.direction),
                        format!("action={}", spec.action),
                        format!("protocol={}", spec.protocol),
                        format!("remoteport={}", spec.port),
                    ];
                    if let Some(ip) = &spec.remote_ip {
                        args.push(format!("remoteip={}", ip));
                    }
                    let output = Command::new("netsh")
                        .args(&args)
                        .creation_flags(CREATE_NO_WINDOW)
                        .output()
                        .map_err(|e| FirewallExecutionError::CommandFailed(e.to_string()))?;
                    if !output.status.success() {
                        return Err(FirewallExecutionError::CommandFailed(
                            String::from_utf8_lossy(&output.stderr).to_string(),
                        ));
                    }
                }
                FirewallStep::RemoveRule(spec) => {
                    let args = vec![
                        "advfirewall".to_string(),
                        "firewall".to_string(),
                        "delete".to_string(),
                        "rule".to_string(),
                        format!("name={}", spec.name),
                    ];
                    let output = Command::new("netsh")
                        .args(&args)
                        .creation_flags(CREATE_NO_WINDOW)
                        .output()
                        .map_err(|e| FirewallExecutionError::CommandFailed(e.to_string()))?;
                    if !output.status.success() {
                        tracing::warn!(
                            "Netsh rule deletion returned status failure: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                }
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            match step {
                FirewallStep::AddRule(spec) => {
                    let mut args = vec!["-A", "OUTPUT", "-p", &spec.protocol, "--dport", "53"];
                    if let Some(ip) = &spec.remote_ip {
                        args.extend(["-d", ip]);
                    }
                    args.extend([
                        "-j",
                        if spec.action == "allow" {
                            "ACCEPT"
                        } else {
                            "DROP"
                        },
                    ]);
                    let output = std::process::Command::new("iptables").args(&args).output();
                    if let Ok(out) = output {
                        if !out.status.success() {
                            tracing::warn!(
                                "iptables add rule warning: {}",
                                String::from_utf8_lossy(&out.stderr)
                            );
                        }
                    }
                }
                FirewallStep::RemoveRule(spec) => {
                    let mut args = vec!["-D", "OUTPUT", "-p", &spec.protocol, "--dport", "53"];
                    if let Some(ip) = &spec.remote_ip {
                        args.extend(["-d", ip]);
                    }
                    args.extend([
                        "-j",
                        if spec.action == "allow" {
                            "ACCEPT"
                        } else {
                            "DROP"
                        },
                    ]);
                    let output = std::process::Command::new("iptables").args(&args).output();
                    if let Ok(out) = output {
                        if !out.status.success() {
                            tracing::warn!(
                                "iptables remove rule warning: {}",
                                String::from_utf8_lossy(&out.stderr)
                            );
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

pub fn execute_firewall_plan<E: FirewallExecutor>(
    executor: &E,
    plan: &KillSwitchPlan,
) -> Result<Vec<FirewallStep>, FirewallExecutionError> {
    let mut applied_steps = Vec::new();

    for (idx, step) in plan.apply_steps.iter().enumerate() {
        if let Err(err) = executor.execute(step) {
            tracing::warn!(
                "Firewall step {} failed: {err}. Executing partial apply rollback in reverse order...",
                idx
            );
            for applied in applied_steps.iter().rev() {
                let rollback_step = match applied {
                    FirewallStep::AddRule(spec) => FirewallStep::RemoveRule(spec.clone()),
                    FirewallStep::RemoveRule(spec) => FirewallStep::AddRule(spec.clone()),
                };
                let _ = executor.execute(&rollback_step);
            }
            return Err(FirewallExecutionError::PartialApplyFailed {
                step_index: idx,
                reason: err.to_string(),
            });
        }
        applied_steps.push(step.clone());
    }

    Ok(applied_steps)
}

pub fn remove_kill_switch_plan<E: FirewallExecutor>(
    executor: &E,
    plan: &KillSwitchPlan,
) -> Result<(), FirewallExecutionError> {
    for step in &plan.remove_steps {
        let _ = executor.execute(step);
    }
    Ok(())
}
