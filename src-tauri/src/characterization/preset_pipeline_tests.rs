#[cfg(test)]
mod tests {
    use crate::config::preset::{builtin_presets, Preset, PresetCategory};
    use crate::config::validator::{
        validate_preset, DesyncMethod, DesyncPhase, PlatformSupport, PresetSource,
        PresetValidationError,
    };

    #[test]
    fn test_every_builtin_preset_is_structurally_semantically_and_platform_valid() {
        let builtins = builtin_presets();
        assert!(!builtins.is_empty(), "Built-in preset list cannot be empty");

        for preset in &builtins {
            let verified = validate_preset(preset, PresetSource::BuiltIn).unwrap_or_else(|e| {
                panic!(
                    "Built-in preset '{}' failed unified validation: {}",
                    preset.id, e
                )
            });

            assert_eq!(verified.id, preset.id);
            assert_eq!(verified.arguments, preset.args);
            assert_eq!(
                verified.supported_platforms.windows,
                PlatformSupport::Supported,
                "Built-in preset '{}' must be supported on Windows",
                preset.id
            );
        }
    }

    #[test]
    fn test_phase_ordering_rejects_descending_phases() {
        let invalid_preset = Preset {
            id: "test-descending-phase".into(),
            label: "Test".into(),
            description: "Test".into(),
            icon: "⚡".into(),
            args: vec!["--wf-tcp=443".into(), "--dpi-desync=fake,syndata".into()],
            is_custom: false,
            priority: 1,
            category: PresetCategory::Standard,
        };

        let err = validate_preset(&invalid_preset, PresetSource::Custom).unwrap_err();
        match err {
            PresetValidationError::InvalidPhaseOrder {
                prev_method,
                prev_phase,
                next_method,
                next_phase,
                ..
            } => {
                assert_eq!(prev_method, "fake");
                assert_eq!(prev_phase, DesyncPhase::Phase1);
                assert_eq!(next_method, "syndata");
                assert_eq!(next_phase, DesyncPhase::Phase0);
            }
            other => panic!("Expected InvalidPhaseOrder error, got: {:?}", other),
        }
    }

    #[test]
    fn test_rbr_02_resolved_semantic_validator_rejects_descending_desync_phase_order() {
        let raw = Preset {
            id: "rbr-02-test".into(),
            label: "RBR-02".into(),
            description: "Desc".into(),
            icon: "⚙️".into(),
            args: vec!["--dpi-desync=disorder,syndata".into()],
            is_custom: true,
            priority: 1,
            category: PresetCategory::Custom,
        };

        assert!(validate_preset(&raw, PresetSource::Custom).is_err());
    }

    #[test]
    fn test_rbr_03_resolved_https_sni_ghost_uses_semantically_valid_phase_sequence() {
        let builtins = builtin_presets();
        let ghost = builtins
            .iter()
            .find(|p| p.id == "https-sni-ghost")
            .expect("https-sni-ghost preset must be present");

        let verified = validate_preset(ghost, PresetSource::BuiltIn).unwrap();
        assert_eq!(
            verified.parsed_desync_methods,
            vec![DesyncMethod::Syndata, DesyncMethod::Fake]
        );
    }

    #[test]
    fn test_cross_arg_conflicting_ttl_rejected() {
        let preset = Preset {
            id: "conflict-ttl".into(),
            label: "Conflict TTL".into(),
            description: "Desc".into(),
            icon: "❌".into(),
            args: vec![
                "--wf-tcp=80,443".into(),
                "--dpi-desync=fake".into(),
                "--dpi-desync-autottl".into(),
                "--dpi-desync-ttl=5".into(),
            ],
            is_custom: true,
            priority: 1,
            category: PresetCategory::Custom,
        };

        let err = validate_preset(&preset, PresetSource::Custom).unwrap_err();
        assert_eq!(err, PresetValidationError::ConflictingTtl);
    }

    #[test]
    fn test_dangling_split_position_without_split_method_rejected() {
        let preset = Preset {
            id: "dangling-split".into(),
            label: "Dangling Split".into(),
            description: "Desc".into(),
            icon: "❌".into(),
            args: vec![
                "--wf-tcp=80,443".into(),
                "--dpi-desync=fake".into(),
                "--dpi-desync-split-pos=2".into(),
            ],
            is_custom: true,
            priority: 1,
            category: PresetCategory::Custom,
        };

        let err = validate_preset(&preset, PresetSource::Custom).unwrap_err();
        assert_eq!(err, PresetValidationError::DanglingSplitPosition);
    }

    #[test]
    fn test_duplicate_single_value_args_rejected() {
        let preset = Preset {
            id: "duplicate-arg".into(),
            label: "Dup".into(),
            description: "Desc".into(),
            icon: "❌".into(),
            args: vec![
                "--wf-tcp=80".into(),
                "--wf-tcp=443".into(),
                "--dpi-desync=fake".into(),
            ],
            is_custom: true,
            priority: 1,
            category: PresetCategory::Custom,
        };

        let err = validate_preset(&preset, PresetSource::Custom).unwrap_err();
        match err {
            PresetValidationError::DuplicateArgument { arg } => {
                assert_eq!(arg, "--wf-tcp=443");
            }
            other => panic!("Expected DuplicateArgument, got: {:?}", other),
        }
    }

    #[test]
    fn test_linux_platform_support_for_udp_quic_only_preset() {
        let builtins = builtin_presets();
        let quic_preset = builtins
            .iter()
            .find(|p| p.id == "youtube-quic")
            .expect("youtube-quic preset must be present");

        let verified = validate_preset(quic_preset, PresetSource::BuiltIn).unwrap();
        assert_eq!(
            verified.supported_platforms.windows,
            PlatformSupport::Supported
        );
        assert!(matches!(
            verified.supported_platforms.linux,
            PlatformSupport::Unsupported { .. }
        ));
    }
}
