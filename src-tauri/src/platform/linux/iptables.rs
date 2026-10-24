use crate::platform::linux::filter_plan::LinuxFirewallStep;

fn resolve_table(table: &str) -> &str {
    if table.starts_with("vane_tbl_") {
        "mangle"
    } else {
        table
    }
}

pub fn render_iptables_step_args(step: &LinuxFirewallStep) -> Option<(String, Vec<String>)> {
    match step {
        LinuxFirewallStep::CreateContainer { table, chain } => Some((
            "iptables".into(),
            vec![
                "-t".into(),
                resolve_table(table).into(),
                "-N".into(),
                chain.clone(),
            ],
        )),
        LinuxFirewallStep::RemoveContainer { table, chain } => Some((
            "iptables".into(),
            vec![
                "-t".into(),
                resolve_table(table).into(),
                "-F".into(),
                chain.clone(),
            ],
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
                    resolve_table(table).into(),
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
                    "--queue-bypass".into(),
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
                    resolve_table(table).into(),
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
                    "--queue-bypass".into(),
                    "-m".into(),
                    "comment".into(),
                    "--comment".into(),
                    rule.comment.clone(),
                ],
            ))
        }
    }
}

pub fn render_iptables_mark_bypass(table: &str, chain: &str) -> (String, Vec<String>) {
    (
        "iptables".into(),
        vec![
            "-t".into(),
            resolve_table(table).into(),
            "-A".into(),
            chain.to_string(),
            "-m".into(),
            "mark".into(),
            "--mark".into(),
            "0x40000000/0x40000000".into(),
            "-j".into(),
            "ACCEPT".into(),
        ],
    )
}

pub fn render_iptables_link_output(table: &str, chain: &str) -> (String, Vec<String>) {
    (
        "iptables".into(),
        vec![
            "-t".into(),
            resolve_table(table).into(),
            "-I".into(),
            "OUTPUT".into(),
            "1".into(),
            "-j".into(),
            chain.to_string(),
        ],
    )
}

pub fn render_iptables_unlink_output(table: &str, chain: &str) -> (String, Vec<String>) {
    (
        "iptables".into(),
        vec![
            "-t".into(),
            resolve_table(table).into(),
            "-D".into(),
            "OUTPUT".into(),
            "-j".into(),
            chain.to_string(),
        ],
    )
}

pub fn render_iptables_delete_chain(table: &str, chain: &str) -> (String, Vec<String>) {
    (
        "iptables".into(),
        vec![
            "-t".into(),
            resolve_table(table).into(),
            "-X".into(),
            chain.to_string(),
        ],
    )
}
