#[cfg(test)]
mod tests {
    use crate::characterization::TempTestDir;
    use crate::config::preset::{builtin_presets, Preset};
    use crate::engine::launch_plan::{
        build_engine_launch_plan, EngineBinaryKind, EngineLaunchInput, EnginePlatform,
        HostlistPlan, KillSwitchRequirement, LaunchBypassInput, LaunchBypassMode,
    };
    use std::path::PathBuf;

    // Helper constructor for tests
    fn make_test_input<'a>(
        preset: &'a Preset,
        platform: EnginePlatform,
        executable: PathBuf,
        mode: LaunchBypassMode,
        domain_list: &str,
        hostlist_path: Option<PathBuf>,
        kill_switch: bool,
    ) -> EngineLaunchInput<'a> {
        EngineLaunchInput {
            preset,
            platform,
            executable,
            bypass: LaunchBypassInput {
                mode,
                domain_list: domain_list.to_string(),
                hostlist_path,
                kill_switch,
            },
        }
    }

    // ─── WINDOWS PLANNING TESTS ───

    #[test]
    fn w01_default_preset_windows_plan() {
        let preset = builtin_presets()
            .into_iter()
            .find(|p| p.id == "default")
            .unwrap();
        let exe = PathBuf::from("C:\\Program Files\\Vane\\winws.exe");
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            exe.clone(),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.preset_id, "default");
        assert_eq!(plan.binary.kind, EngineBinaryKind::Winws);
        assert_eq!(plan.binary.executable, exe);
        assert_eq!(
            plan.binary.working_directory,
            PathBuf::from("C:\\Program Files\\Vane")
        );
        assert_eq!(plan.hostlist, HostlistPlan::None);
        assert_eq!(plan.kill_switch, KillSwitchRequirement::Disabled);
        assert_eq!(plan.final_arguments, preset.args);
    }

    #[test]
    fn w02_whitelist_hostlist_include_windows() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let hostlist_path = PathBuf::from("C:\\Users\\Test\\AppData\\domains.txt");
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            PathBuf::from("C:\\Vane\\winws.exe"),
            LaunchBypassMode::Whitelist,
            "example.com\ntest.org\n",
            Some(hostlist_path.clone()),
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        match plan.hostlist {
            HostlistPlan::Include { path, domain_count } => {
                assert_eq!(path, hostlist_path);
                assert_eq!(domain_count, 2);
            }
            _ => panic!("Expected HostlistPlan::Include"),
        }
        assert!(plan
            .final_arguments
            .last()
            .unwrap()
            .starts_with("--hostlist="));
    }

    #[test]
    fn w03_blacklist_hostlist_exclude_windows() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let hostlist_path = PathBuf::from("C:\\Vane\\domains.txt");
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            PathBuf::from("C:\\Vane\\winws.exe"),
            LaunchBypassMode::Blacklist,
            "bad.com",
            Some(hostlist_path.clone()),
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        match plan.hostlist {
            HostlistPlan::Exclude { path, domain_count } => {
                assert_eq!(path, hostlist_path);
                assert_eq!(domain_count, 1);
            }
            _ => panic!("Expected HostlistPlan::Exclude"),
        }
        assert!(plan
            .final_arguments
            .last()
            .unwrap()
            .starts_with("--hostlist-exclude="));
    }

    #[test]
    fn w04_all_mode_has_no_hostlist_arg() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            PathBuf::from("C:\\Vane\\winws.exe"),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.hostlist, HostlistPlan::None);
        assert!(!plan
            .final_arguments
            .iter()
            .any(|a| a.starts_with("--hostlist")));
    }

    #[test]
    fn w05_empty_whitelist_returns_fail_closed_error() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            PathBuf::from("C:\\Vane\\winws.exe"),
            LaunchBypassMode::Whitelist,
            "   \n  ",
            Some(PathBuf::from("C:\\domains.txt")),
            false,
        );

        let res = build_engine_launch_plan(input);
        assert!(res.is_err());
    }

    #[test]
    fn w06_path_with_spaces_retained_as_pathbuf() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let space_exe = PathBuf::from("C:\\Program Files (x86)\\Vane DPI Engine\\winws.exe");
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            space_exe.clone(),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.binary.executable, space_exe);
    }

    #[test]
    fn w09_kill_switch_required_in_plan() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            PathBuf::from("C:\\Vane\\winws.exe"),
            LaunchBypassMode::All,
            "",
            None,
            true,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.kill_switch, KillSwitchRequirement::Required);
    }

    // ─── LINUX PLANNING TESTS ───

    #[test]
    fn l01_linux_binary_kind_is_nfqws() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let input = make_test_input(
            &preset,
            EnginePlatform::Linux,
            PathBuf::from("/usr/bin/nfqws"),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.binary.kind, EngineBinaryKind::Nfqws);
    }

    #[test]
    fn l02_linux_includes_qnum_200_as_first_argument() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let input = make_test_input(
            &preset,
            EnginePlatform::Linux,
            PathBuf::from("/usr/bin/nfqws"),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.final_arguments.first().unwrap(), "--qnum=200");
    }

    #[test]
    fn l03_documents_linux_wf_stripping_behavior() {
        let preset = Preset {
            id: "test-wf".to_string(),
            label: "Test WF".to_string(),
            description: "".to_string(),
            icon: "zap".to_string(),
            args: vec![
                "--wf-tcp=80,443".to_string(),
                "--dpi-desync=fake".to_string(),
            ],
            is_custom: true,
            priority: 1,
            category: Default::default(),
        };
        let input = make_test_input(
            &preset,
            EnginePlatform::Linux,
            PathBuf::from("/usr/bin/nfqws"),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert!(!plan.final_arguments.iter().any(|a| a.starts_with("--wf-")));
        assert_eq!(
            plan.traffic_filter.declared_tcp_spec,
            Some("80,443".to_string())
        );
    }

    #[test]
    fn l04_documents_current_linux_udp_filter_not_being_applied() {
        // Characterization / Reproducer: On Linux target, declared UDP filters are stripped from final nfqws CLI args
        let preset = Preset {
            id: "udp-preset".to_string(),
            label: "UDP".to_string(),
            description: "".to_string(),
            icon: "zap".to_string(),
            args: vec!["--wf-udp=443".to_string(), "--dpi-desync=fake".to_string()],
            is_custom: true,
            priority: 1,
            category: Default::default(),
        };
        let input = make_test_input(
            &preset,
            EnginePlatform::Linux,
            PathBuf::from("/usr/bin/nfqws"),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(
            plan.traffic_filter.declared_udp_spec,
            Some("443".to_string())
        );
        assert_eq!(
            plan.traffic_filter.effective_linux_udp_spec,
            Some("443".to_string())
        );
    }

    // ─── PURITY TESTS ───

    #[test]
    fn s01_deterministic_planning_100_runs() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let exe = PathBuf::from("C:\\Vane\\winws.exe");

        let first_plan = build_engine_launch_plan(make_test_input(
            &preset,
            EnginePlatform::Windows,
            exe.clone(),
            LaunchBypassMode::All,
            "",
            None,
            false,
        ))
        .unwrap();

        for _ in 0..100 {
            let plan = build_engine_launch_plan(make_test_input(
                &preset,
                EnginePlatform::Windows,
                exe.clone(),
                LaunchBypassMode::All,
                "",
                None,
                false,
            ))
            .unwrap();
            assert_eq!(first_plan, plan);
        }
    }

    #[test]
    fn s03_no_filesystem_side_effects_on_temp_dir() {
        let temp = TempTestDir::new("s03");
        let preset = builtin_presets().into_iter().next().unwrap();
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            PathBuf::from("C:\\Vane\\winws.exe"),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let _plan = build_engine_launch_plan(input).unwrap();
        // Verify temp directory remains completely empty (no side effect files created during planning)
        let entries: Vec<_> = std::fs::read_dir(temp.path()).unwrap().collect();
        assert!(entries.is_empty());
    }

    // ─── PARITY TESTS ───

    #[test]
    fn p01_windows_default_preset_parity() {
        let preset = builtin_presets()
            .into_iter()
            .find(|p| p.id == "default")
            .unwrap();
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            PathBuf::from("C:\\Vane\\winws.exe"),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.final_arguments, preset.args);
    }

    #[test]
    fn p07_tr_1_preset_parity() {
        let preset = builtin_presets()
            .into_iter()
            .find(|p| p.id == "tr-1")
            .unwrap();
        let input = make_test_input(
            &preset,
            EnginePlatform::Windows,
            PathBuf::from("C:\\Vane\\winws.exe"),
            LaunchBypassMode::All,
            "",
            None,
            false,
        );

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.final_arguments, preset.args);
    }
}
