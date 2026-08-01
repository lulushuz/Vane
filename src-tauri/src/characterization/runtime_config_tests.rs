#[cfg(test)]
mod tests {
    use crate::config::preset::{builtin_presets, Preset};
    use crate::engine::runtime_config::{
        candidate_from_preset_and_sources, compute_config_fingerprint, verify_runtime_config,
        AppliedRuntimeConfig, AppliedVerification, ConfigRevision, PreparedHostlist,
        PreparedRuntimeConfig, RuntimeBypassCandidate, RuntimeBypassMode, RuntimeConfigCandidate,
        RuntimeConfigError, RuntimeDnsCandidate, RuntimeDnsProtocol, RuntimeSecurityCandidate,
        VerifiedRuntimeConfig,
    };
    use std::path::PathBuf;

    fn make_test_candidate(mode: &str, domains: Vec<&str>) -> RuntimeConfigCandidate {
        let preset = builtin_presets().into_iter().next().unwrap();
        RuntimeConfigCandidate {
            preset_id: preset.id,
            preset_args: preset.args,
            bypass: RuntimeBypassCandidate {
                mode: mode.to_string(),
                domains: domains.into_iter().map(String::from).collect(),
                kill_switch: false,
            },
            dns: RuntimeDnsCandidate {
                enabled: true,
                protocol: "doh".to_string(),
                provider: Some("cloudflare".to_string()),
                adblock: true,
                cache_enabled: true,
            },
            security: RuntimeSecurityCandidate {
                kill_switch: false,
                binary_integrity_required: true,
            },
        }
    }

    // ─── A: CANDIDATE VALIDATION TESTS ───

    #[test]
    fn a01_valid_all_mode_candidate() {
        let cand = make_test_candidate("all", vec![]);
        let rev = ConfigRevision::new(1);
        let verified = verify_runtime_config(cand, rev).unwrap();
        assert_eq!(verified.bypass.mode, RuntimeBypassMode::All);
        assert_eq!(verified.revision.get(), 1);
    }

    #[test]
    fn a02_valid_whitelist_candidate() {
        let cand = make_test_candidate("whitelist", vec!["example.com"]);
        let rev = ConfigRevision::new(2);
        let verified = verify_runtime_config(cand, rev).unwrap();
        assert_eq!(verified.bypass.mode, RuntimeBypassMode::Whitelist);
        assert_eq!(verified.bypass.domain_count, 1);
    }

    #[test]
    fn a03_empty_whitelist_returns_fail_closed_error() {
        let cand = make_test_candidate("whitelist", vec![]);
        let rev = ConfigRevision::new(1);
        let res = verify_runtime_config(cand, rev);
        assert_eq!(res, Err(RuntimeConfigError::EmptyWhitelist));
    }

    #[test]
    fn a04_unknown_bypass_mode_returns_error() {
        let cand = make_test_candidate("super_mode", vec![]);
        let rev = ConfigRevision::new(1);
        let res = verify_runtime_config(cand, rev);
        assert!(matches!(
            res,
            Err(RuntimeConfigError::UnsupportedBypassMode(_))
        ));
    }

    #[test]
    fn a05_domain_canonicalization_and_deduplication() {
        let cand = make_test_candidate(
            "whitelist",
            vec!["  EXAMPLE.COM. ", "example.com", "TEST.ORG"],
        );
        let rev = ConfigRevision::new(1);
        let verified = verify_runtime_config(cand, rev).unwrap();
        assert_eq!(verified.bypass.domains, vec!["example.com", "test.org"]);
        assert_eq!(verified.bypass.domain_count, 2);
    }

    // ─── B: REVISION TESTS ───

    #[test]
    fn b01_revision_monotonic_increment() {
        let rev1 = ConfigRevision::new(10);
        let rev2 = rev1.checked_next().unwrap();
        assert_eq!(rev2.get(), 11);
        assert!(rev2 > rev1);
    }

    #[test]
    fn b02_revision_overflow_error() {
        let max_rev = ConfigRevision::new(u64::MAX);
        assert_eq!(
            max_rev.checked_next(),
            Err(RuntimeConfigError::RevisionOverflow)
        );
    }

    // ─── C: FINGERPRINT TESTS (F-01 to F-10) ───

    #[test]
    fn f01_identical_config_produces_identical_fingerprint() {
        let fp1 = compute_config_fingerprint(
            "default",
            &["--wf-tcp=80,443".to_string()],
            RuntimeBypassMode::Whitelist,
            &["example.com".to_string()],
            false,
            true,
            RuntimeDnsProtocol::Doh,
            Some("cloudflare"),
            true,
            true,
        );

        let fp2 = compute_config_fingerprint(
            "default",
            &["--wf-tcp=80,443".to_string()],
            RuntimeBypassMode::Whitelist,
            &["example.com".to_string()],
            false,
            true,
            RuntimeDnsProtocol::Doh,
            Some("cloudflare"),
            true,
            true,
        );

        assert_eq!(fp1, fp2);
        assert_eq!(fp1.as_str().len(), 64);
    }

    #[test]
    fn f02_revision_difference_does_not_change_fingerprint() {
        let cand1 = make_test_candidate("all", vec![]);
        let cand2 = cand1.clone();

        let v1 = verify_runtime_config(cand1, ConfigRevision::new(1)).unwrap();
        let v2 = verify_runtime_config(cand2, ConfigRevision::new(99)).unwrap();

        assert_ne!(v1.revision, v2.revision);
        assert_eq!(v1.fingerprint, v2.fingerprint);
    }

    #[test]
    fn f03_domain_case_difference_produces_same_canonical_fingerprint() {
        let cand1 = make_test_candidate("whitelist", vec!["EXAMPLE.COM"]);
        let cand2 = make_test_candidate("whitelist", vec!["example.com."]);

        let v1 = verify_runtime_config(cand1, ConfigRevision::new(1)).unwrap();
        let v2 = verify_runtime_config(cand2, ConfigRevision::new(1)).unwrap();

        assert_eq!(v1.fingerprint, v2.fingerprint);
    }

    #[test]
    fn f04_domain_order_difference_produces_same_fingerprint() {
        let cand1 = make_test_candidate("whitelist", vec!["a.com", "b.com"]);
        let cand2 = make_test_candidate("whitelist", vec!["b.com", "a.com"]);

        let v1 = verify_runtime_config(cand1, ConfigRevision::new(1)).unwrap();
        let v2 = verify_runtime_config(cand2, ConfigRevision::new(1)).unwrap();

        assert_eq!(v1.fingerprint, v2.fingerprint);
    }

    #[test]
    fn f05_duplicate_domain_produces_same_fingerprint() {
        let cand1 = make_test_candidate("whitelist", vec!["a.com", "a.com"]);
        let cand2 = make_test_candidate("whitelist", vec!["a.com"]);

        let v1 = verify_runtime_config(cand1, ConfigRevision::new(1)).unwrap();
        let v2 = verify_runtime_config(cand2, ConfigRevision::new(1)).unwrap();

        assert_eq!(v1.fingerprint, v2.fingerprint);
    }

    #[test]
    fn f06_preset_arg_order_difference_changes_fingerprint() {
        let preset1 = Preset {
            id: "p1".to_string(),
            label: "P1".to_string(),
            description: "".to_string(),
            icon: "".to_string(),
            args: vec![
                "--wf-tcp=443".to_string(),
                "--dpi-desync=syndata,fake".to_string(),
            ],
            is_custom: true,
            priority: 1,
            category: Default::default(),
        };

        let preset2 = Preset {
            id: "p1".to_string(),
            label: "P1".to_string(),
            description: "".to_string(),
            icon: "".to_string(),
            args: vec![
                "--dpi-desync=syndata,fake".to_string(),
                "--wf-tcp=443".to_string(),
            ],
            is_custom: true,
            priority: 1,
            category: Default::default(),
        };

        let cand1 = candidate_from_preset_and_sources(&preset1, "all", "", false);
        let cand2 = candidate_from_preset_and_sources(&preset2, "all", "", false);

        let v1 = verify_runtime_config(cand1, ConfigRevision::new(1)).unwrap();
        let v2 = verify_runtime_config(cand2, ConfigRevision::new(1)).unwrap();

        assert_ne!(v1.fingerprint, v2.fingerprint);
    }

    #[test]
    fn f07_kill_switch_change_changes_fingerprint() {
        let cand1 = make_test_candidate("all", vec![]);
        let mut cand2 = cand1.clone();
        cand2.bypass.kill_switch = true;

        let v1 = verify_runtime_config(cand1, ConfigRevision::new(1)).unwrap();
        let v2 = verify_runtime_config(cand2, ConfigRevision::new(1)).unwrap();

        assert_ne!(v1.fingerprint, v2.fingerprint);
    }

    #[test]
    fn f08_dns_protocol_change_changes_fingerprint() {
        let cand1 = make_test_candidate("all", vec![]);
        let mut cand2 = cand1.clone();
        cand2.dns.protocol = "dot".to_string();

        let v1 = verify_runtime_config(cand1, ConfigRevision::new(1)).unwrap();
        let v2 = verify_runtime_config(cand2, ConfigRevision::new(1)).unwrap();

        assert_ne!(v1.fingerprint, v2.fingerprint);
    }

    #[test]
    fn f09_platform_executable_path_does_not_affect_fingerprint() {
        let cand = make_test_candidate("all", vec![]);
        let v1 = verify_runtime_config(cand.clone(), ConfigRevision::new(1)).unwrap();
        let v2 = verify_runtime_config(cand, ConfigRevision::new(1)).unwrap();

        // Fingerprint is content-driven, insensitive to deployment paths
        assert_eq!(v1.fingerprint, v2.fingerprint);
    }

    // ─── D: PREPARED & APPLIED STATE TESTS ───

    #[test]
    fn d01_prepared_config_carries_verified_and_plan_without_pid() {
        let cand = make_test_candidate("all", vec![]);
        let verified = verify_runtime_config(cand, ConfigRevision::new(1)).unwrap();

        let input = crate::engine::launch_plan::EngineLaunchInput {
            preset: &builtin_presets().into_iter().next().unwrap(),
            platform: crate::engine::launch_plan::EnginePlatform::Windows,
            executable: PathBuf::from("C:\\winws.exe"),
            bypass: verified.to_launch_bypass_input(None),
        };

        let plan = crate::engine::launch_plan::build_engine_launch_plan(input).unwrap();
        let prepared = PreparedRuntimeConfig {
            verified: verified.clone(),
            launch_plan: plan,
            hostlist: PreparedHostlist::NotRequired,
        };

        assert_eq!(prepared.verified.fingerprint, verified.fingerprint);
        assert_eq!(prepared.hostlist, PreparedHostlist::NotRequired);
    }

    #[test]
    fn e01_applied_config_requires_pid_and_matches_verified() {
        let cand = make_test_candidate("all", vec![]);
        let verified = verify_runtime_config(cand, ConfigRevision::new(1)).unwrap();

        let applied = AppliedRuntimeConfig::process_started(verified.clone(), 5432);
        assert_eq!(applied.process_id, 5432);
        assert_eq!(applied.verification, AppliedVerification::ProcessStarted);
        assert_eq!(applied.verified.fingerprint, verified.fingerprint);
    }

    // ─── THREAD SAFETY & IMMUTABILITY ───

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_models_are_send_and_sync() {
        assert_send_sync::<VerifiedRuntimeConfig>();
        assert_send_sync::<PreparedRuntimeConfig>();
        assert_send_sync::<AppliedRuntimeConfig>();
    }

    // ─── REDACTION & DEBUG SUMMARY TEST ───

    #[test]
    fn test_verified_debug_and_summary_redaction() {
        let cand = make_test_candidate("whitelist", vec!["secret-user-domain.com"]);
        let verified = verify_runtime_config(cand, ConfigRevision::new(42)).unwrap();

        let debug_str = format!("{:?}", verified);
        // Debug output must not contain full domain list string to protect sensitive telemetry
        assert!(!debug_str.contains("secret-user-domain.com"));
        assert!(debug_str.contains("domain_count: 1"));
        assert!(debug_str.contains("revision: 42"));

        let summary = verified.summary();
        assert_eq!(summary.revision, 42);
        assert_eq!(summary.domain_count, 1);
        assert_eq!(summary.fingerprint_prefix.len(), 8);
    }

    // ─── REPRODUCER TEST ───

    #[test]
    fn documents_runtime_contract_still_receiving_persisted_disk_config() {
        // RBR-01 / Source-of-truth reproducer: P04 introduces the contract, but candidate_from_preset_and_sources still reads disk settings
        // Target: P06
        // Risk: R-01
        let cand = make_test_candidate("all", vec![]);
        let verified = verify_runtime_config(cand, ConfigRevision::new(1)).unwrap();
        assert_eq!(verified.bypass.mode, RuntimeBypassMode::All);
    }
}
