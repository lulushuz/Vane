#[cfg(test)]
mod tests {
    use crate::config::preset::builtin_presets;
    use crate::engine::sanitizer::validate_preset_args;
    use std::collections::HashSet;

    #[test]
    fn c01_parse_all_builtin_presets() {
        let presets = builtin_presets();
        assert!(
            !presets.is_empty(),
            "Built-in presets list must not be empty"
        );
    }

    #[test]
    fn c02_unique_builtin_preset_ids() {
        let presets = builtin_presets();
        let mut seen = HashSet::new();
        for p in &presets {
            assert!(seen.insert(&p.id), "Duplicate preset ID found: {}", p.id);
        }
    }

    #[test]
    fn c03_valid_preset_id_format() {
        let presets = builtin_presets();
        for p in &presets {
            assert!(!p.id.is_empty(), "Preset ID must not be empty");
            assert!(
                p.id.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "Invalid characters in preset ID: {}",
                p.id
            );
        }
    }

    #[test]
    fn c04_all_builtin_presets_pass_sanitizer() {
        let presets = builtin_presets();
        for p in &presets {
            let res = validate_preset_args(&p.args);
            assert!(
                res.is_ok(),
                "Built-in preset '{}' failed sanitizer validation: {:?}",
                p.id,
                res.err()
            );
        }
    }

    #[test]
    fn c05_test_only_preset_phase_semantic_report() {
        let presets = builtin_presets();
        for p in &presets {
            let mut phase0 = Vec::new();
            let mut phase1 = Vec::new();
            for arg in &p.args {
                if let Some(strat) = arg.strip_prefix("--dpi-desync=") {
                    for method in strat.split(',') {
                        match method {
                            "syndata" => phase0.push("syndata"),
                            "fake" | "fakedsplit" | "multisplit" | "multidisorder" | "split"
                            | "split2" | "disorder" => phase1.push(method),
                            _ => {}
                        }
                    }
                }
            }
            // Documenting phase summary for each built-in preset
            assert!(phase0.len() + phase1.len() <= 10);
        }
    }

    #[test]
    fn c06_documents_https_sni_ghost_phase_order_behavior() {
        // Reproducer test for https-sni-ghost or complex phase order preset
        // Target: P08
        // Risk: R-08
        let presets = builtin_presets();
        if let Some(ghost) = presets.iter().find(|p| p.id == "https-sni-ghost") {
            let res = validate_preset_args(&ghost.args);
            assert!(res.is_ok(), "https-sni-ghost passes current sanitizer");
        }
    }

    #[test]
    fn c07_metadata_audit_for_key_presets() {
        let presets = builtin_presets();
        let target_ids = [
            "tr-1",
            "tr-2",
            "youtube-quic",
            "discord-voip",
            "deep-fragmentation",
            "lightweight-gaming",
        ];
        for id in target_ids {
            let found = presets.iter().find(|p| p.id == id);
            assert!(
                found.is_some(),
                "Key preset '{}' must exist in built-in list",
                id
            );
            if let Some(p) = found {
                assert!(!p.label.is_empty());
                assert!(!p.description.is_empty());
            }
        }
    }

    #[test]
    fn c08_platform_capability_audit() {
        let presets = builtin_presets();
        for p in &presets {
            let contains_udp = p.args.iter().any(|a| a.starts_with("--wf-udp="));
            let contains_tcp = p.args.iter().any(|a| a.starts_with("--wf-tcp="));
            // Document current preset platform flag distribution
            let _ = (contains_udp, contains_tcp);
        }
    }
}
