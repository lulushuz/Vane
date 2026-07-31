use crate::platform::linux::capabilities::LinuxPlatformCapabilities;
use crate::platform::linux::filter_intent::{LinuxFilterIntent, PortRange};
use crate::platform::linux::ownership::LinuxRuleOwnership;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinuxFirewallBackend {
    Nftables,
    Iptables,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxFirewallRule {
    pub protocol: String,
    pub port_ranges: Vec<PortRange>,
    pub queue_number: u16,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinuxFirewallStep {
    CreateContainer {
        table: String,
        chain: String,
    },
    AddRule {
        table: String,
        chain: String,
        rule: LinuxFirewallRule,
    },
    RemoveRule {
        table: String,
        chain: String,
        rule: LinuxFirewallRule,
    },
    RemoveContainer {
        table: String,
        chain: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxFilterPlan {
    pub ownership: LinuxRuleOwnership,
    pub backend: LinuxFirewallBackend,
    pub queue_number: u16,
    pub ipv4_rules: Vec<LinuxFirewallRule>,
    pub ipv6_rules: Vec<LinuxFirewallRule>,
    pub apply_steps: Vec<LinuxFirewallStep>,
    pub rollback_steps: Vec<LinuxFirewallStep>,
    pub remove_steps: Vec<LinuxFirewallStep>,
}

pub fn build_linux_filter_plan(
    ownership: LinuxRuleOwnership,
    intent: &LinuxFilterIntent,
    capabilities: &LinuxPlatformCapabilities,
) -> LinuxFilterPlan {
    let backend = if capabilities.nftables_available {
        LinuxFirewallBackend::Nftables
    } else {
        LinuxFirewallBackend::Iptables
    };

    let inst_pref = if ownership.installation_id.len() >= 8 {
        &ownership.installation_id[..8]
    } else {
        &ownership.installation_id
    };

    let comment = format!(
        "Vane DPI inst={} rev={}",
        inst_pref, ownership.config_revision
    );

    let mut ipv4_rules = Vec::new();

    if !intent.tcp_ports.is_empty() {
        ipv4_rules.push(LinuxFirewallRule {
            protocol: "tcp".into(),
            port_ranges: intent.tcp_ports.clone(),
            queue_number: ownership.queue_number,
            comment: comment.clone(),
        });
    }

    if !intent.udp_ports.is_empty() {
        ipv4_rules.push(LinuxFirewallRule {
            protocol: "udp".into(),
            port_ranges: intent.udp_ports.clone(),
            queue_number: ownership.queue_number,
            comment: comment.clone(),
        });
    }

    let ipv6_rules = if capabilities.ipv6_available {
        ipv4_rules.clone()
    } else {
        vec![]
    };

    let mut apply_steps = vec![LinuxFirewallStep::CreateContainer {
        table: ownership.table_name.clone(),
        chain: ownership.chain_name.clone(),
    }];

    for rule in &ipv4_rules {
        apply_steps.push(LinuxFirewallStep::AddRule {
            table: ownership.table_name.clone(),
            chain: ownership.chain_name.clone(),
            rule: rule.clone(),
        });
    }

    let mut rollback_steps = Vec::new();
    for step in apply_steps.iter().rev() {
        match step {
            LinuxFirewallStep::AddRule { table, chain, rule } => {
                rollback_steps.push(LinuxFirewallStep::RemoveRule {
                    table: table.clone(),
                    chain: chain.clone(),
                    rule: rule.clone(),
                });
            }
            LinuxFirewallStep::CreateContainer { table, chain } => {
                rollback_steps.push(LinuxFirewallStep::RemoveContainer {
                    table: table.clone(),
                    chain: chain.clone(),
                });
            }
            _ => {}
        }
    }

    let remove_steps = rollback_steps.clone();

    LinuxFilterPlan {
        ownership,
        backend,
        queue_number: 200,
        ipv4_rules,
        ipv6_rules,
        apply_steps,
        rollback_steps,
        remove_steps,
    }
}
