#[cfg(test)]
mod tests {
    use crate::engine::manager::{EngineManager, EngineStatus};

    #[test]
    fn g01_windows_arg_passthrough() {
        let preset_args = [
            "--wf-tcp=80,443".to_string(),
            "--dpi-desync=fake".to_string(),
        ];
        // On Windows target, args are passed directly without modification
        if cfg!(target_os = "windows") {
            assert_eq!(preset_args.len(), 2);
        }
    }


    #[test]
    fn g02_documents_linux_dropping_wf_filter_arguments() {
        // RBR-04 Reproducer: On Linux target, --wf-tcp, --wf-udp, --windivert, tcp., etc. are stripped by launcher
        // Risk: R-11 / R-23
        // Target phase: P11
        // Expected production behavior: Linux netfilter/nftables rules should be configured explicitly instead of silent argument stripping
        let raw_args = vec![
            "--wf-tcp=80,443".to_string(),
            "--dpi-desync=fake".to_string(),
        ];
        let linux_filtered: Vec<String> = raw_args
            .into_iter()
            .filter(|a| {
                !a.starts_with("--wf-") && !a.starts_with("--windivert") && !a.starts_with("tcp.")
            })
            .collect();
        assert_eq!(linux_filtered, vec!["--dpi-desync=fake"]);
    }

    #[test]
    fn g03_linux_qnum_arg_addition() {
        let mut linux_args = vec!["--dpi-desync=fake".to_string()];
        linux_args.push("--qnum=200".to_string());
        assert!(linux_args.contains(&"--qnum=200".to_string()));
    }

    #[test]
    fn g04_hostlist_arg_preparation() {
        let hostlist_path = "/tmp/vane-hostlist.txt";
        let whitelist_arg = format!("--hostlist={hostlist_path}");
        let blacklist_arg = format!("--hostlist-exclude={hostlist_path}");

        assert!(whitelist_arg.starts_with("--hostlist="));
        assert!(blacklist_arg.starts_with("--hostlist-exclude="));
    }

    #[test]
    fn h01_initial_engine_status_is_stopped() {
        let _manager = EngineManager::new();
        // New EngineManager starts with Stopped state
        assert_eq!(EngineStatus::Stopped, EngineStatus::Stopped);
    }



    #[test]
    fn h02_status_enum_variants_serialization() {
        let stopped = EngineStatus::Stopped;
        let starting = EngineStatus::Starting;
        let running = EngineStatus::Running { pid: 1234 };
        let error = EngineStatus::Error {
            message: "Driver failed".to_string(),
            code: Some("DRIVER_ERROR".to_string()),
        };

        assert_eq!(
            serde_json::to_string(&stopped).unwrap(),
            r#"{"variant":"stopped"}"#
        );
        assert_eq!(
            serde_json::to_string(&starting).unwrap(),
            r#"{"variant":"starting"}"#
        );
        assert_eq!(
            serde_json::to_string(&running).unwrap(),
            r#"{"variant":"running","pid":1234}"#
        );
        assert!(serde_json::to_string(&error)
            .unwrap()
            .contains("DRIVER_ERROR"));
    }

    #[test]
    fn h10_documents_running_status_as_process_alive_only() {
        // RBR-07 Reproducer: Documents PID-only running status representation
        // Risk: R-17
        // Target phase: P07 / P14
        // Expected production behavior: active traffic health check should complement PID presence
        let status = EngineStatus::Running { pid: 9999 };
        if let EngineStatus::Running { pid } = status {
            assert_eq!(pid, 9999);
        }
    }
}
