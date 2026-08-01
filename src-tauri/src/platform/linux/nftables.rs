use crate::platform::linux::filter_plan::LinuxFilterPlan;

pub fn render_nftables_batch(plan: &LinuxFilterPlan) -> String {
    let mut batch = String::new();
    batch.push_str(&format!("add table ip {}\n", plan.ownership.table_name));
    batch.push_str(&format!(
        "add chain ip {} {} {{ type filter hook output priority mangle; policy accept; }}\n",
        plan.ownership.table_name, plan.ownership.chain_name
    ));

    for rule in &plan.ipv4_rules {
        let ports_str = rule
            .port_ranges
            .iter()
            .map(|r| {
                if r.start == r.end {
                    format!("{}", r.start)
                } else {
                    format!("{}-{}", r.start, r.end)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        batch.push_str(&format!(
            "add rule ip {} {} {} dport {{ {} }} queue num {}\n",
            plan.ownership.table_name,
            plan.ownership.chain_name,
            rule.protocol,
            ports_str,
            rule.queue_number
        ));
    }

    batch
}

pub fn render_nftables_cleanup(plan: &LinuxFilterPlan) -> String {
    format!("delete table ip {}\n", plan.ownership.table_name)
}
