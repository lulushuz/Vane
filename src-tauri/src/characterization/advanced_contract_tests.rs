#[cfg(test)]
mod tests {
    use crate::config::preset::Preset;
    use crate::config::validator::{validate_preset, AdvancedCapabilities, PresetSource};
    use crate::engine::launch_plan::{
        build_engine_launch_plan, EngineLaunchInput, EnginePlatform, LaunchBypassInput,
        LaunchBypassMode,
    };
    use serde::Deserialize;
    use std::fs;
    use std::path::PathBuf;

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureSpec {
        name: String,
        platform: String,
        input_args: Vec<String>,
        valid: bool,
        expected_canonical_args: Vec<String>,
    }

    #[test]
    fn test_advanced_capabilities_response_structure() {
        let caps = AdvancedCapabilities::for_current_platform();
        assert!(!caps.platform.is_empty());
        assert!(!caps.methods.is_empty());
        assert_eq!(caps.options.mss.state, "unsupported");
        assert_eq!(caps.options.ipset.state, "unsupported");
        assert_eq!(caps.options.tpws.state, "unsupported");
    }

    #[test]
    fn test_cross_language_advanced_fixtures_parity() {
        let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("advanced");

        assert!(
            fixture_dir.exists(),
            "Fixtures directory {:?} must exist",
            fixture_dir
        );

        let entries = fs::read_dir(&fixture_dir).expect("Failed to read fixtures dir");
        let mut tested_count = 0;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let content = fs::read_to_string(&path).expect("Failed to read fixture file");
            let spec: FixtureSpec =
                serde_json::from_str(&content).expect("Failed to parse fixture JSON");

            let preset = Preset {
                id: spec.name.clone(),
                label: spec.name.clone(),
                description: String::new(),
                icon: String::new(),
                args: spec.input_args.clone(),
                is_custom: true,
                priority: 0,
                category: Default::default(),
            };

            let res = validate_preset(&preset, PresetSource::Custom);
            if spec.valid {
                assert!(
                    res.is_ok(),
                    "Fixture '{}' was expected to be valid, got: {:?}",
                    spec.name,
                    res.err()
                );
            } else {
                assert!(
                    res.is_err(),
                    "Fixture '{}' was expected to fail validation",
                    spec.name
                );
            }
            tested_count += 1;
        }

        assert!(
            tested_count >= 8,
            "Must test at least 8 fixtures (tested {})",
            tested_count
        );
    }

    #[test]
    fn test_launch_plan_carries_verified_advanced_arguments() {
        let preset = Preset {
            id: "advanced-test".into(),
            label: "Advanced Test".into(),
            description: String::new(),
            icon: String::new(),
            args: vec![
                "--wf-tcp=80,443".into(),
                "--dpi-desync=syndata,fake".into(),
                "--dpi-desync-repeats=2".into(),
                "--dpi-desync-fooling=md5sig".into(),
            ],
            is_custom: true,
            priority: 0,
            category: Default::default(),
        };

        let input = EngineLaunchInput {
            preset: &preset,
            platform: EnginePlatform::Windows,
            executable: PathBuf::from("C:\\bin\\winws.exe"),
            bypass: LaunchBypassInput {
                mode: LaunchBypassMode::All,
                domain_list: String::new(),
                hostlist_path: None,
                kill_switch: false,
            },
        };

        let plan = build_engine_launch_plan(input).unwrap();
        assert!(plan
            .final_arguments
            .contains(&"--wf-tcp=80,443".to_string()));
        assert!(plan
            .final_arguments
            .contains(&"--dpi-desync=syndata,fake".to_string()));
        assert!(plan
            .final_arguments
            .contains(&"--dpi-desync-repeats=2".to_string()));
        assert!(plan
            .final_arguments
            .contains(&"--dpi-desync-fooling=md5sig".to_string()));
    }
}
