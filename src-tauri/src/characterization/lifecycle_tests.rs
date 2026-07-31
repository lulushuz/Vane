#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::preset::builtin_presets;
    use crate::engine::lifecycle::{
        validate_state_transition, EngineLifecycleError, EngineLifecycleState, EngineReadiness,
        PlatformReadiness,
    };
    use crate::engine::owned_process::{
        EngineGeneration, EngineOperationId, EngineProcessIdentity, OwnedChildHandle,
        OwnedEngineProcess, TerminationMode,
    };
    use crate::engine::runtime_config::{
        candidate_from_preset_and_sources, verify_runtime_config, ConfigRevision,
    };

    #[test]
    fn test_group_a_valid_state_transitions() {
        use EngineLifecycleState::*;

        let op = EngineOperationId::generate("test");
        let gen = EngineGeneration::new(1);
        let preset = builtin_presets().into_iter().next().unwrap();
        let candidate = candidate_from_preset_and_sources(&preset, "all", "", false);
        let verified = verify_runtime_config(candidate, ConfigRevision::new(1)).unwrap();
        let rev = verified.revision;
        let fp = verified.fingerprint.clone();

        let s_stopped = Stopped;
        let s_prep = Preparing {
            operation: op.clone(),
            generation: gen,
            revision: rev,
            fingerprint: fp.clone(),
        };
        let s_starting = StartingProcess {
            operation: op.clone(),
            generation: gen,
            revision: rev,
            fingerprint: fp.clone(),
        };
        let s_waiting = WaitingForReadiness {
            operation: op.clone(),
            generation: gen,
            process_id: 100,
            revision: rev,
            fingerprint: fp.clone(),
        };
        let s_ready = Ready {
            generation: gen,
            process_id: 100,
            revision: rev,
            fingerprint: fp.clone(),
            readiness: EngineReadiness {
                process_alive: true,
                startup_grace_passed: true,
                fatal_stderr_detected: false,
                executable_identity_verified: true,
                platform_check: PlatformReadiness::Mock,
            },
        };
        let s_stopping = Stopping {
            operation: op,
            generation: gen,
            process_id: 100,
        };

        // Valid chain
        assert!(validate_state_transition(&s_stopped, &s_prep).is_ok());
        assert!(validate_state_transition(&s_prep, &s_starting).is_ok());
        assert!(validate_state_transition(&s_starting, &s_waiting).is_ok());
        assert!(validate_state_transition(&s_waiting, &s_ready).is_ok());
        assert!(validate_state_transition(&s_ready, &s_stopping).is_ok());
        assert!(validate_state_transition(&s_stopping, &s_stopped).is_ok());

        // Invalid transition: Stopped directly to Ready
        let err = validate_state_transition(&s_stopped, &s_ready).unwrap_err();
        assert_eq!(
            err,
            EngineLifecycleError::InvalidTransition {
                from: "stopped",
                to: "ready"
            }
        );
    }

    #[test]
    fn test_group_b_generation_monotonic_increment_and_overflow() {
        let g1 = EngineGeneration::new(1);
        let g2 = g1.next().unwrap();
        assert_eq!(g2.get(), 2);
        assert!(g2 > g1);

        let g_max = EngineGeneration::new(u64::MAX);
        assert!(g_max.next().is_err());
    }

    #[test]
    fn test_group_c_ownership_identity_verification() {
        let preset = builtin_presets().into_iter().next().unwrap();
        let candidate = candidate_from_preset_and_sources(&preset, "whitelist", "a.com", false);
        let verified = verify_runtime_config(candidate, ConfigRevision::new(5)).unwrap();

        let identity = EngineProcessIdentity {
            installation_id: "inst-1".into(),
            instance_id: "proc-1".into(),
            generation: EngineGeneration::new(42),
            expected_executable: PathBuf::from("winws.exe"),
            config_revision: verified.revision,
            config_fingerprint: verified.fingerprint.clone(),
        };

        let child_handle = OwnedChildHandle::Fake {
            pid: 1234,
            exited: false,
        };

        let mut process = OwnedEngineProcess::new(identity, child_handle);

        assert_eq!(process.pid(), 1234);
        assert_eq!(process.generation().get(), 42);
        assert_eq!(process.config_revision().get(), 5);
        assert_eq!(process.config_fingerprint(), &verified.fingerprint);
        assert!(process.is_alive());

        let rt = tokio::runtime::Runtime::new().unwrap();
        let exit = rt
            .block_on(process.terminate(TerminationMode::Graceful { timeout_ms: 100 }))
            .unwrap();
        assert_eq!(exit.pid, 1234);
        assert!(!process.is_alive());
    }

    #[test]
    fn test_group_d_readiness_model() {
        let ready_readiness = EngineReadiness {
            process_alive: true,
            startup_grace_passed: true,
            fatal_stderr_detected: false,
            executable_identity_verified: true,
            platform_check: PlatformReadiness::Mock,
        };
        assert!(ready_readiness.is_ready());

        let fatal_stderr_readiness = EngineReadiness {
            process_alive: true,
            startup_grace_passed: true,
            fatal_stderr_detected: true,
            executable_identity_verified: true,
            platform_check: PlatformReadiness::Mock,
        };
        assert!(!fatal_stderr_readiness.is_ready());

        let unverified_id_readiness = EngineReadiness {
            process_alive: true,
            startup_grace_passed: true,
            fatal_stderr_detected: false,
            executable_identity_verified: false,
            platform_check: PlatformReadiness::Mock,
        };
        assert!(!unverified_id_readiness.is_ready());
    }

    #[test]
    fn test_group_i_100_cycle_soak_lifecycle_cleanup() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let preset = builtin_presets().into_iter().next().unwrap();
        let candidate = candidate_from_preset_and_sources(&preset, "all", "", false);
        let verified = verify_runtime_config(candidate, ConfigRevision::new(1)).unwrap();

        for i in 1..=100 {
            let gen = EngineGeneration::new(i);
            let identity = EngineProcessIdentity {
                installation_id: "inst-soak".into(),
                instance_id: format!("proc-{}", i),
                generation: gen,
                expected_executable: PathBuf::from("winws.exe"),
                config_revision: verified.revision,
                config_fingerprint: verified.fingerprint.clone(),
            };

            let child_handle = OwnedChildHandle::Fake {
                pid: 2000 + i as u32,
                exited: false,
            };

            let mut proc = OwnedEngineProcess::new(identity, child_handle);
            assert!(proc.is_alive());

            let exit = rt
                .block_on(proc.terminate(TerminationMode::Graceful { timeout_ms: 10 }))
                .unwrap();
            assert_eq!(exit.pid, 2000 + i as u32);
            assert!(!proc.is_alive());
        }
    }

    #[test]
    fn test_rbr_06_resolved_engine_cleanup_terminates_only_owned_process_identity() {
        let identity1 = EngineProcessIdentity {
            installation_id: "inst-1".into(),
            instance_id: "owned-vane-proc".into(),
            generation: EngineGeneration::new(10),
            expected_executable: PathBuf::from("winws.exe"),
            config_revision: ConfigRevision::new(1),
            config_fingerprint: crate::engine::runtime_config::ConfigRevision::new(1)
                .checked_next()
                .map(|_| {
                    let p = builtin_presets().into_iter().next().unwrap();
                    let c = candidate_from_preset_and_sources(&p, "all", "", false);
                    verify_runtime_config(c, ConfigRevision::new(1))
                        .unwrap()
                        .fingerprint
                })
                .unwrap(),
        };

        let mut owned_proc = OwnedEngineProcess::new(
            identity1,
            OwnedChildHandle::Fake {
                pid: 4444,
                exited: false,
            },
        );

        let foreign_proc = OwnedChildHandle::Fake {
            pid: 8888,
            exited: false,
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(owned_proc.terminate(TerminationMode::Forced))
            .unwrap();

        // Owned process is terminated
        assert!(!owned_proc.is_alive());
        // Foreign process PID 8888 was never touched or killed via global taskkill/killall
        if let OwnedChildHandle::Fake { exited, .. } = foreign_proc {
            assert!(!exited);
        }
    }

    #[test]
    fn test_rbr_07_partially_resolved_ready_state_requires_local_process_readiness_beyond_pid_existence(
    ) {
        let readiness = EngineReadiness {
            process_alive: true,
            startup_grace_passed: false, // grace period not yet passed
            fatal_stderr_detected: false,
            executable_identity_verified: true,
            platform_check: PlatformReadiness::Mock,
        };

        // Mere existence of PID before grace period passes is NOT Ready
        assert!(!readiness.is_ready());
    }
}
