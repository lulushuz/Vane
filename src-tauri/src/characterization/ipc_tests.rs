#[cfg(test)]
mod tests {
    use crate::ipc::IpcError;
    use serde_json::Value;

    #[test]
    fn l01_serde_camel_case_rename() {
        let err = IpcError::runtime("start_engine", "START_FAILED", "Could not start engine");
        let json_val = serde_json::to_value(&err).unwrap();
        assert!(json_val.get("code").is_some());
        assert!(json_val.get("message").is_some());
        assert!(json_val.get("operation").is_some());
        assert!(json_val.get("retryable").is_some());
    }

    #[test]
    fn l03_optional_fields_omitted_when_none() {
        let err = IpcError::validation("sync_dns_settings", "INVALID_DNS", "Invalid DNS setting");
        let json_val = serde_json::to_value(&err).unwrap();
        assert!(json_val.get("configRevision").is_none());
    }

    #[test]
    fn l05_matches_shared_ipc_error_fixture() {
        let fixture_str = include_str!("../../fixtures/ipc/ipc-error.json");
        let fixture_val: Value = serde_json::from_str(fixture_str).unwrap();

        let err = IpcError::validation(
            "sync_bypass_config",
            "WHITELIST_EMPTY",
            "Whitelist requires a domain.",
        );
        let code_val = serde_json::to_value(&err).unwrap();

        assert_eq!(code_val, fixture_val);
    }
}
