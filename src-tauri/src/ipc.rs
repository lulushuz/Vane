#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
    pub operation: &'static str,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_revision: Option<u64>,
}

impl IpcError {
    pub fn validation(
        operation: &'static str,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            operation,
            retryable: false,
            config_revision: None,
        }
    }

    pub fn runtime(
        operation: &'static str,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            operation,
            retryable: true,
            config_revision: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_serializes_as_stable_ipc_contract() {
        let error = IpcError::validation(
            "sync_bypass_config",
            "WHITELIST_EMPTY",
            "Whitelist requires a domain.",
        );

        assert_eq!(
            serde_json::to_value(error).expect("serialize IPC error"),
            serde_json::json!({
                "code": "WHITELIST_EMPTY",
                "message": "Whitelist requires a domain.",
                "operation": "sync_bypass_config",
                "retryable": false
            })
        );
    }
}
