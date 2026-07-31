#[cfg(test)]
mod tests {
    use crate::platform::linux::{
        build_linux_filter_plan, render_iptables_step_args, render_nftables_batch,
        render_nftables_cleanup, FakeLinuxFirewallExecutor, LinuxFilterIntent,
        LinuxFirewallBackend, LinuxFirewallExecutor, LinuxPlatformCapabilities, LinuxRuleOwnership,
        PersistedLinuxFilterMetadata, PortRange,
    };

    fn mock_capabilities(nftables: bool, iptables: bool) -> LinuxPlatformCapabilities {
        LinuxPlatformCapabilities {
            nftables_available: nftables,
            nft_atomic_batch: nftables,
            iptables_available: iptables,
            ip6tables_available: iptables,
            nfqueue_available: true,
            comment_match_available: true,
            ipv6_available: true,
            effective_uid: 0,
            has_required_privileges: true,
        }
    }

    #[test]
    fn group_a01_filter_intent_default_tcp_80_443() {
        let intent = LinuxFilterIntent::from_specs(None, None, "all");
        assert_eq!(
            intent.tcp_ports,
            vec![
                PortRange { start: 80, end: 80 },
                PortRange {
                    start: 443,
                    end: 443
                }
            ]
        );
        assert!(intent.udp_ports.is_empty());
        assert!(!intent.requires_quic);
    }

    #[test]
    fn group_a02_filter_intent_tcp_only_443() {
        let intent = LinuxFilterIntent::from_specs(Some("443"), None, "whitelist");
        assert_eq!(
            intent.tcp_ports,
            vec![PortRange {
                start: 443,
                end: 443
            }]
        );
        assert!(intent.udp_ports.is_empty());
    }

    #[test]
    fn group_a03_filter_intent_custom_tcp_range() {
        let intent = LinuxFilterIntent::from_specs(Some("8080-8090,443"), None, "all");
        assert_eq!(
            intent.tcp_ports,
            vec![
                PortRange {
                    start: 443,
                    end: 443
                },
                PortRange {
                    start: 8080,
                    end: 8090
                }
            ]
        );
    }

    #[test]
    fn group_a04_filter_intent_udp_quic_443() {
        let intent = LinuxFilterIntent::from_specs(Some("80,443"), Some("443"), "all");
        assert_eq!(
            intent.udp_ports,
            vec![PortRange {
                start: 443,
                end: 443
            }]
        );
        assert!(intent.requires_quic);
    }

    #[test]
    fn group_a05_filter_intent_discord_voip_udp_range() {
        let intent = LinuxFilterIntent::from_specs(Some("443"), Some("50000-65535"), "all");
        assert_eq!(
            intent.udp_ports,
            vec![PortRange {
                start: 50000,
                end: 65535
            }]
        );
        assert!(!intent.requires_quic);
    }

    #[test]
    fn group_c01_nftables_plan_generation_and_ownership() {
        let caps = mock_capabilities(true, true);
        let intent = LinuxFilterIntent::from_specs(Some("80,443"), Some("443"), "all");
        let ownership =
            LinuxRuleOwnership::new("inst-12345678", "instance-87654321", 1, 10, "fp123", 200);

        let plan = build_linux_filter_plan(ownership, &intent, &caps);

        assert_eq!(plan.backend, LinuxFirewallBackend::Nftables);
        assert_eq!(plan.queue_number, 200);
        assert!(plan.ownership.table_name.contains("vane_tbl_inst-123"));
        assert!(plan.ownership.chain_name.contains("vane_chn_instance_g1"));

        let batch = render_nftables_batch(&plan);
        assert!(batch.contains("add table ip vane_tbl_inst-123"));
        assert!(batch.contains("add chain ip vane_tbl_inst-123 vane_chn_instance_g1"));
        assert!(batch.contains("dport { 80, 443 } queue num 200"));
        assert!(batch.contains("dport { 443 } queue num 200"));

        let cleanup = render_nftables_cleanup(&plan);
        assert_eq!(
            cleanup,
            format!("delete table ip {}\n", plan.ownership.table_name)
        );
    }

    #[test]
    fn group_d01_iptables_fallback_plan_and_comment_ownership() {
        let caps = mock_capabilities(false, true);
        let intent = LinuxFilterIntent::from_specs(Some("443"), Some("50000-65535"), "whitelist");
        let ownership =
            LinuxRuleOwnership::new("inst-12345678", "instance-87654321", 2, 15, "fp456", 200);

        let plan = build_linux_filter_plan(ownership, &intent, &caps);

        assert_eq!(plan.backend, LinuxFirewallBackend::Iptables);
        assert_eq!(plan.apply_steps.len(), 3);

        let (cmd, args) = render_iptables_step_args(&plan.apply_steps[1]).unwrap();
        assert_eq!(cmd, "iptables");
        assert!(args.iter().any(|a: &String| a == "--comment"));
        assert!(args
            .iter()
            .any(|a: &String| a.contains("Vane DPI inst=inst-123")));
    }

    #[test]
    fn group_e01_partial_apply_rollback_simulation() {
        let caps = mock_capabilities(false, true);
        let intent = LinuxFilterIntent::from_specs(Some("80,443"), Some("443"), "all");
        let ownership = LinuxRuleOwnership::new("inst-123", "inst-876", 1, 1, "fp", 200);
        let plan = build_linux_filter_plan(ownership, &intent, &caps);

        let executor = FakeLinuxFirewallExecutor::with_fail_at(2);
        let res = executor.apply(&plan);

        assert!(res.is_err());
        let applied_guard = executor.applied_steps.lock().unwrap();
        assert!(applied_guard.is_empty());
    }

    #[test]
    fn group_g01_metadata_serialization_roundtrip() {
        let ownership =
            LinuxRuleOwnership::new("inst-12345678", "instance-87654321", 5, 42, "fp_hash", 200);
        let meta = PersistedLinuxFilterMetadata {
            schema_version: 1,
            installation_id: ownership.installation_id.clone(),
            instance_id: ownership.instance_id.clone(),
            generation: ownership.generation,
            config_revision: ownership.config_revision,
            config_fingerprint: ownership.config_fingerprint.clone(),
            backend: "nftables".into(),
            queue_number: ownership.queue_number,
            table_name: ownership.table_name.clone(),
            chain_name: ownership.chain_name.clone(),
            created_at: "1700000000".into(),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: PersistedLinuxFilterMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.installation_id, "inst-12345678");
        assert_eq!(deserialized.table_name, ownership.table_name);
        assert_eq!(deserialized.queue_number, 200);
    }
}
