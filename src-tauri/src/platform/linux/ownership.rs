use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxRuleOwnership {
    pub installation_id: String,
    pub instance_id: String,
    pub generation: u64,
    pub config_revision: u64,
    pub config_fingerprint: String,
    pub queue_number: u16,
    pub table_name: String,
    pub chain_name: String,
    pub rule_ids: Vec<String>,
}

impl LinuxRuleOwnership {
    pub fn new(
        installation_id: &str,
        instance_id: &str,
        generation: u64,
        config_revision: u64,
        config_fingerprint: &str,
        queue_number: u16,
    ) -> Self {
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

        let table_name = format!("vane_tbl_{}", inst_prefix);
        let chain_name = format!("vane_chn_{}_g{}", instance_prefix, generation);

        Self {
            installation_id: installation_id.to_string(),
            instance_id: instance_id.to_string(),
            generation,
            config_revision,
            config_fingerprint: config_fingerprint.to_string(),
            queue_number,
            table_name: table_name.clone(),
            chain_name: chain_name.clone(),
            rule_ids: vec![format!("{}_tcp", chain_name), format!("{}_udp", chain_name)],
        }
    }
}
