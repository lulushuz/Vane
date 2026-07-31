use crate::platform::linux::filter_plan::LinuxFirewallStep;

pub fn render_iptables_step_args(step: &LinuxFirewallStep) -> Option<(String, Vec<String>)> {
    match step {
        LinuxFirewallStep::CreateContainer { table, chain } => Some((
            "iptables".into(),
            vec!["-t".into(), table.clone(), "-N".into(), chain.clone()],
        )),
        LinuxFirewallStep::RemoveContainer { table, chain } => Some((
            "iptables".into(),
            vec!["-t".into(), table.clone(), "-F".into(), chain.clone()],
        )),
        LinuxFirewallStep::AddRule { table, chain, rule } => {
            let mut ports = Vec::new();
            for r in &rule.port_ranges {
                if r.start == r.end {
                    ports.push(format!("{}", r.start));
                } else {
                    ports.push(format!("{}:{}", r.start, r.end));
                }
            }
            Some((
                "iptables".into(),
                vec![
                    "-t".into(),
                    table.clone(),
                    "-A".into(),
                    chain.clone(),
                    "-p".into(),
                    rule.protocol.clone(),
                    "-m".into(),
                    "multiport".into(),
                    "--dports".into(),
                    ports.join(","),
                    "-j".into(),
                    "NFQUEUE".into(),
                    "--queue-num".into(),
                    rule.queue_number.to_string(),
                    "-m".into(),
                    "comment".into(),
                    "--comment".into(),
                    rule.comment.clone(),
                ],
            ))
        }
        LinuxFirewallStep::RemoveRule { table, chain, rule } => {
            let mut ports = Vec::new();
            for r in &rule.port_ranges {
                if r.start == r.end {
                    ports.push(format!("{}", r.start));
                } else {
                    ports.push(format!("{}:{}", r.start, r.end));
                }
            }
            Some((
                "iptables".into(),
                vec![
                    "-t".into(),
                    table.clone(),
                    "-D".into(),
                    chain.clone(),
                    "-p".into(),
                    rule.protocol.clone(),
                    "-m".into(),
                    "multiport".into(),
                    "--dports".into(),
                    ports.join(","),
                    "-j".into(),
                    "NFQUEUE".into(),
                    "--queue-num".into(),
                    rule.queue_number.to_string(),
                    "-m".into(),
                    "comment".into(),
                    "--comment".into(),
                    rule.comment.clone(),
                ],
            ))
        }
    }
}
