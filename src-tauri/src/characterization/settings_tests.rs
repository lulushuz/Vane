#[cfg(test)]
mod tests {
    use crate::characterization::TempTestDir;
    use crate::settings::{
        atomic_replace_bytes, atomic_write, load_with_recovery, parse_store, RuntimeSettings,
        SETTINGS_KEY,
    };
    use serde_json::{json, Map, Value};
    use std::fs;

    #[test]
    fn e01_empty_settings_file_returns_empty_map() {
        let temp = TempTestDir::new("e01");
        let non_existent = temp.path().join("non_existent.json");
        let store = parse_store(&non_existent).unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn e02_valid_settings_deserializes_all_runtime_fields() {
        let val = json!({
            "activePresetId": "default",
            "bypassMode": "all",
            "whitelistDomains": ["allowed.org"],
            "blacklistDomains": ["blocked.org"],
            "dnsProtocol": "doh",
            "dnsAdBlock": true,
            "dnsCache": true,
            "proxySocks5": "",
            "killSwitch": false,
            "watchdog": true,
            "dnsForwarderEnabled": false,
            "healthCheckTargets": ["example.com"],
            "selectedDnsId": "cloudflare",
            "dnsCustomPrimary": "",
            "dnsCustomSecondary": ""
        });
        let settings: RuntimeSettings = serde_json::from_value(val).unwrap();
        assert_eq!(settings.active_preset_id, "default");
        assert_eq!(settings.bypass_mode, "all");
        assert_eq!(settings.whitelist_domains, vec!["allowed.org"]);
        assert!(settings.dns_ad_block);
    }

    #[test]
    fn e03_atomic_replace_writes_complete_file() {
        let temp = TempTestDir::new("e03");
        let target = temp.path().join("atomic_target.json");
        let content = b"{\"key\":\"value\"}";

        atomic_replace_bytes(&target, content).unwrap();

        assert!(target.exists());
        let read = fs::read(&target).unwrap();
        assert_eq!(read, content);
    }

    #[test]
    fn e04_atomic_write_creates_backup_before_overwriting() {
        let temp = TempTestDir::new("e04");
        let primary = temp.path().join("settings.json");
        let backup = temp.path().join("settings.json.bak");

        let mut v1 = Map::new();
        v1.insert(SETTINGS_KEY.to_string(), Value::String("rev1".to_string()));
        atomic_write(&primary, &backup, &v1).unwrap();

        let mut v2 = Map::new();
        v2.insert(SETTINGS_KEY.to_string(), Value::String("rev2".to_string()));
        atomic_write(&primary, &backup, &v2).unwrap();

        assert_eq!(
            parse_store(&primary).unwrap().get(SETTINGS_KEY),
            Some(&json!("rev2"))
        );
        assert_eq!(
            parse_store(&backup).unwrap().get(SETTINGS_KEY),
            Some(&json!("rev1"))
        );
    }

    #[test]
    fn e05_damaged_primary_recovers_from_valid_backup() {
        let temp = TempTestDir::new("e05");
        let primary = temp.path().join("settings.json");
        let backup = temp.path().join("settings.json.bak");

        fs::write(&primary, b"{ damaged json ...").unwrap();
        let backup_data = json!({ SETTINGS_KEY: "backup-val" });
        fs::write(&backup, serde_json::to_vec(&backup_data).unwrap()).unwrap();

        let recovered = load_with_recovery(&primary, &backup).unwrap();
        assert_eq!(recovered.get(SETTINGS_KEY), Some(&json!("backup-val")));
    }

    #[test]
    fn e06_both_damaged_returns_error_without_panic() {
        let temp = TempTestDir::new("e06");
        let primary = temp.path().join("settings.json");
        let backup = temp.path().join("settings.json.bak");

        fs::write(&primary, b"{ damaged 1").unwrap();
        fs::write(&backup, b"{ damaged 2").unwrap();

        let res = load_with_recovery(&primary, &backup);
        assert!(res.is_err());
    }

    #[test]
    fn e07_truncated_json_returns_error() {
        let temp = TempTestDir::new("e07");
        let primary = temp.path().join("truncated.json");
        fs::write(&primary, b"{\"vane-settings\": {\"state\": ").unwrap();

        let res = parse_store(&primary);
        assert!(res.is_err());
    }

    #[test]
    fn e08_large_settings_payload_simulation() {
        let temp = TempTestDir::new("e08");
        let primary = temp.path().join("large_settings.json");
        let backup = temp.path().join("large_settings.json.bak");

        let mut map = Map::new();
        let large_domains: Vec<Value> = (0..5000)
            .map(|i| Value::String(format!("domain{i}.example.com")))
            .collect();
        map.insert("domains".to_string(), Value::Array(large_domains));

        assert!(atomic_write(&primary, &backup, &map).is_ok());
    }

    #[test]
    fn e09_runtime_settings_defaults_missing_fields() {
        let partial_json = json!({
            "activePresetId": "custom-1"
        });
        let settings: RuntimeSettings = serde_json::from_value(partial_json).unwrap();
        assert_eq!(settings.active_preset_id, "custom-1");
        assert_eq!(settings.bypass_mode, "all"); // Default
        assert_eq!(settings.dns_protocol, "doh"); // Default
    }
}
