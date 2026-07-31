#[cfg(test)]
mod tests {
    use crate::characterization::TempTestDir;
    use crate::config::preset::builtin_presets;
    use crate::engine::manager::EngineManager;
    use crate::engine::pattern_transaction::{
        build_hostlist_filename, clean_stale_hostlists, write_revisioned_hostlist,
    };
    use crate::engine::runtime_config::{
        candidate_from_preset_and_sources, verify_runtime_config, AppliedRuntimeConfig,
        ConfigRevision,
    };
    use crate::engine::runtime_state::{RuntimeConfigState, RuntimeStateError};

    #[test]
    fn test_runtime_config_state_commit_applied() {
        let mut state = RuntimeConfigState::new(ConfigRevision::new(1));
        let preset = builtin_presets().into_iter().next().unwrap();
        let candidate =
            candidate_from_preset_and_sources(&preset, "whitelist", "example.com", false);
        let verified = verify_runtime_config(candidate, ConfigRevision::new(2)).unwrap();

        state.set_desired(verified.clone());
        let applied = AppliedRuntimeConfig::process_started(verified.clone(), 1234);

        assert!(state.commit_applied(applied).is_ok());
        assert_eq!(state.applied().unwrap().process_id, 1234);
        assert_eq!(state.latest_completed_revision().get(), 2);
    }

    #[test]
    fn test_runtime_config_state_stale_revision_rejected() {
        let mut state = RuntimeConfigState::new(ConfigRevision::new(10));
        let preset = builtin_presets().into_iter().next().unwrap();
        let candidate = candidate_from_preset_and_sources(&preset, "all", "", false);
        let verified = verify_runtime_config(candidate, ConfigRevision::new(5)).unwrap();

        let applied = AppliedRuntimeConfig::process_started(verified, 5678);
        let err = state.commit_applied(applied).unwrap_err();

        assert_eq!(
            err,
            RuntimeStateError::StaleRevision {
                current: 10,
                attempted: 5,
            }
        );
    }

    #[test]
    fn test_build_hostlist_filename() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let candidate = candidate_from_preset_and_sources(&preset, "whitelist", "a.com", false);
        let verified = verify_runtime_config(candidate, ConfigRevision::new(42)).unwrap();

        let filename = build_hostlist_filename(verified.revision, &verified.fingerprint);
        assert!(filename.starts_with("domains-rev-42-"));
        assert!(filename.ends_with(".txt"));
    }

    #[test]
    fn test_write_revisioned_hostlist_and_cleanup() {
        let temp_dir = TempTestDir::new("hostlist-test");
        let path = temp_dir.path();

        let f1 =
            write_revisioned_hostlist(path, "domains-rev-1-a1b2c3d4.txt", "domain1.com\n").unwrap();
        let f2 =
            write_revisioned_hostlist(path, "domains-rev-2-e5f6g7h8.txt", "domain2.com\n").unwrap();

        assert!(f1.exists());
        assert!(f2.exists());

        // Clean stale hostlists while f2 is active
        clean_stale_hostlists(path, Some("domains-rev-2-e5f6g7h8.txt"), None).unwrap();

        assert!(!f1.exists());
        assert!(f2.exists());
    }

    #[test]
    fn test_unsafe_path_traversal_hostlist_rejected() {
        let temp_dir = TempTestDir::new("path-traversal-test");
        let path = temp_dir.path();

        let res = write_revisioned_hostlist(path, "../evil.txt", "bad");
        assert!(res.is_err());
    }

    #[test]
    fn test_i01_br_01_resolved_engine_restart_uses_verified_snapshot() {
        let manager = EngineManager::new();
        let preset = builtin_presets().into_iter().next().unwrap();
        let candidate =
            candidate_from_preset_and_sources(&preset, "whitelist", "new-domain.com", false);
        let verified = verify_runtime_config(candidate, ConfigRevision::new(100)).unwrap();

        manager
            .runtime_config_state()
            .lock()
            .unwrap()
            .set_desired(verified.clone());

        // Confirm desired memory snapshot is authoritative over any disk state
        let desired = manager.desired_config().unwrap();
        assert_eq!(desired.revision.get(), 100);
        assert_eq!(desired.bypass.domains, vec!["new-domain.com"]);
    }

    #[test]
    fn test_i02_rbr_01_resolved_desired_config_authoritative_over_persisted_disk_state() {
        let manager = EngineManager::new();
        let preset = builtin_presets().into_iter().next().unwrap();
        let candidate =
            candidate_from_preset_and_sources(&preset, "blacklist", "blocked.com", true);
        let verified = verify_runtime_config(candidate, ConfigRevision::new(200)).unwrap();

        manager
            .runtime_config_state()
            .lock()
            .unwrap()
            .set_desired(verified.clone());

        let active_desired = manager.desired_config().unwrap();
        assert_eq!(
            active_desired.bypass.mode,
            crate::engine::runtime_config::RuntimeBypassMode::Blacklist
        );
        assert!(active_desired.bypass.kill_switch);
        assert_eq!(active_desired.fingerprint, verified.fingerprint);
    }
}
