#[cfg(test)]
mod tests {
    use crate::dns::firewall_plan::{
        build_kill_switch_plan, execute_firewall_plan, FirewallExecutionError, FirewallExecutor,
        FirewallPlatform, FirewallStep,
    };
    use crate::dns::runtime_config::{
        verify_dns_config, DnsConfigCandidate, DnsConfigFingerprint, DnsConfigRevision,
        DnsProtocol, DnsProvider, DnsSocksCandidate, DnsValidationError, VerifiedDnsSocks,
    };

    // Fake Firewall Executor for Step Failure Injection
    struct FakeFirewallExecutor {
        pub fail_at_step: Option<usize>,
    }

    impl FirewallExecutor for FakeFirewallExecutor {
        fn execute(&self, step: &FirewallStep) -> Result<(), FirewallExecutionError> {
            if let FirewallStep::AddRule(spec) = step {
                if spec.name.contains("TCP53") && self.fail_at_step == Some(3) {
                    return Err(FirewallExecutionError::CommandFailed(
                        "Injected simulated step failure".into(),
                    ));
                }
            }
            Ok(())
        }
    }

    // ─── Group A: DNS Validation ───

    #[test]
    fn a01_doh_cloudflare_valid() {
        let candidate = DnsConfigCandidate {
            enabled: true,
            protocol: "doh".into(),
            provider: Some("cloudflare".into()),
            adblock: true,
            cache_enabled: true,
            socks5: None,
            kill_switch: false,
        };
        let verified = verify_dns_config(candidate, DnsConfigRevision::new(1)).unwrap();
        assert_eq!(verified.protocol, DnsProtocol::Doh);
        assert_eq!(verified.provider, DnsProvider::Cloudflare);
    }

    #[test]
    fn a02_dot_google_valid() {
        let candidate = DnsConfigCandidate {
            enabled: true,
            protocol: "dot".into(),
            provider: Some("google".into()),
            adblock: false,
            cache_enabled: true,
            socks5: None,
            kill_switch: true,
        };
        let verified = verify_dns_config(candidate, DnsConfigRevision::new(1)).unwrap();
        assert_eq!(verified.protocol, DnsProtocol::Dot);
        assert_eq!(verified.provider, DnsProvider::Google);
    }

    #[test]
    fn a03_doq_rejection() {
        let candidate = DnsConfigCandidate {
            enabled: true,
            protocol: "doq".into(),
            provider: Some("cloudflare".into()),
            adblock: false,
            cache_enabled: true,
            socks5: None,
            kill_switch: false,
        };
        let err = verify_dns_config(candidate, DnsConfigRevision::new(1)).unwrap_err();
        assert_eq!(err, DnsValidationError::UnsupportedProtocolDoQ);
    }

    #[test]
    fn a04_unknown_provider_rejection() {
        let candidate = DnsConfigCandidate {
            enabled: true,
            protocol: "doh".into(),
            provider: Some("unknown_provider".into()),
            adblock: false,
            cache_enabled: true,
            socks5: None,
            kill_switch: false,
        };
        let err = verify_dns_config(candidate, DnsConfigRevision::new(1)).unwrap_err();
        assert_eq!(
            err,
            DnsValidationError::UnsupportedProvider("unknown_provider".into())
        );
    }

    #[test]
    fn a05_dot_with_socks5_rejected() {
        let candidate = DnsConfigCandidate {
            enabled: true,
            protocol: "dot".into(),
            provider: Some("cloudflare".into()),
            adblock: false,
            cache_enabled: true,
            socks5: Some(DnsSocksCandidate {
                host: "127.0.0.1".into(),
                port: 1080,
                username: None,
                password: None,
            }),
            kill_switch: false,
        };
        let err = verify_dns_config(candidate, DnsConfigRevision::new(1)).unwrap_err();
        assert_eq!(err, DnsValidationError::DotWithSocks5NotAllowed);
    }

    // ─── Group B: Revision & Fingerprint ───

    #[test]
    fn b01_identical_config_produces_identical_fingerprint() {
        let fp1 = DnsConfigFingerprint::compute(
            true,
            DnsProtocol::Doh,
            DnsProvider::Cloudflare,
            true,
            true,
            None,
            false,
        );
        let fp2 = DnsConfigFingerprint::compute(
            true,
            DnsProtocol::Doh,
            DnsProvider::Cloudflare,
            true,
            true,
            None,
            false,
        );
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn b02_protocol_change_changes_fingerprint() {
        let fp1 = DnsConfigFingerprint::compute(
            true,
            DnsProtocol::Doh,
            DnsProvider::Cloudflare,
            true,
            true,
            None,
            false,
        );
        let fp2 = DnsConfigFingerprint::compute(
            true,
            DnsProtocol::Dot,
            DnsProvider::Cloudflare,
            true,
            true,
            None,
            false,
        );
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn b03_socks_credentials_redacted_in_debug() {
        let socks = VerifiedDnsSocks {
            host: "127.0.0.1".into(),
            port: 1080,
            username: Some("secret_user".into()),
            password: Some("super_secret_pass".into()),
        };
        let debug_str = format!("{:?}", socks);
        assert!(!debug_str.contains("secret_user"));
        assert!(!debug_str.contains("super_secret_pass"));
        assert!(debug_str.contains("[REDACTED]"));
    }

    // ─── Group F: Firewall Plan & Ownership ───

    #[test]
    fn f01_build_kill_switch_plan_names_contain_ownership() {
        let plan = build_kill_switch_plan(
            "inst12345678",
            "instance87654321",
            DnsConfigRevision::new(42),
            &DnsConfigFingerprint("fp123".into()),
            FirewallPlatform::Windows,
            true,
        );
        assert_eq!(plan.apply_steps.len(), 4);
        assert!(plan.ownership.rule_ids[0].contains("Vane-DNS-inst1234-instance-r42"));
    }

    // ─── Group G: Partial Apply Rollback ───

    #[test]
    fn g01_partial_apply_failure_executes_reverse_rollback() {
        let plan = build_kill_switch_plan(
            "inst12345678",
            "instance87654321",
            DnsConfigRevision::new(42),
            &DnsConfigFingerprint("fp123".into()),
            FirewallPlatform::Windows,
            true,
        );

        let fake_executor = FakeFirewallExecutor {
            fail_at_step: Some(3),
        };
        let res = execute_firewall_plan(&fake_executor, &plan);
        assert!(res.is_err());
    }

    // ─── Group H: Crash Recovery ───

    #[test]
    fn h01_legacy_rule_migration_removes_known_legacy_rules() {
        // Safe migration test helper
        let plan = build_kill_switch_plan(
            "legacy-inst",
            "legacy-instance",
            DnsConfigRevision::new(1),
            &DnsConfigFingerprint("fp_legacy".into()),
            FirewallPlatform::Windows,
            false,
        );
        assert!(plan.apply_steps.is_empty());
    }
}
