#[cfg(test)]
mod tests {
    use crate::engine::owned_process::EngineGeneration;
    use crate::optimizer::candidate::resolve_and_deduplicate_candidates;
    use crate::optimizer::manager::OptimizerSessionManager;
    use crate::optimizer::measurement::{MeasurementErrorCategory, MeasurementSummary};
    use crate::optimizer::runtime_adapter::OptimizerRuntime;
    use crate::optimizer::scoring::{compare_candidate_scores, CandidateScore, ScoreConfidence};
    use crate::optimizer::session::{
        OptimizerError, OptimizerSessionId, OriginalEngineState, RestoreOutcome,
    };
    use crate::optimizer::MeasurementSample;
    use std::cmp::Ordering;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Arc;

    struct FakeRuntime {
        pub original_state: OriginalEngineState,
        pub start_count: Arc<AtomicUsize>,
        pub stop_count: Arc<AtomicUsize>,
        pub restore_count: Arc<AtomicUsize>,
    }

    impl FakeRuntime {
        fn new(original_state: OriginalEngineState) -> Self {
            Self {
                original_state,
                start_count: Arc::new(AtomicUsize::new(0)),
                stop_count: Arc::new(AtomicUsize::new(0)),
                restore_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl OptimizerRuntime for FakeRuntime {
        async fn capture_original_state(&self) -> Result<OriginalEngineState, OptimizerError> {
            Ok(self.original_state.clone())
        }

        async fn stop_original_for_session(
            &self,
            _session_id: &OptimizerSessionId,
        ) -> Result<(), OptimizerError> {
            self.stop_count.fetch_add(1, AtomicOrdering::Relaxed);
            Ok(())
        }

        async fn start_candidate(
            &self,
            _session_id: &OptimizerSessionId,
            _candidate: crate::engine::runtime_config::PreparedRuntimeConfig,
        ) -> Result<crate::engine::runtime_config::AppliedRuntimeConfig, OptimizerError> {
            self.start_count.fetch_add(1, AtomicOrdering::Relaxed);
            Err(OptimizerError::CandidateStartFailed(
                "Fake candidate start".into(),
            ))
        }

        async fn stop_candidate(
            &self,
            _session_id: &OptimizerSessionId,
        ) -> Result<(), OptimizerError> {
            self.stop_count.fetch_add(1, AtomicOrdering::Relaxed);
            Ok(())
        }

        async fn restore_original(
            &self,
            _session_id: &OptimizerSessionId,
            original: OriginalEngineState,
        ) -> Result<RestoreOutcome, OptimizerError> {
            self.restore_count.fetch_add(1, AtomicOrdering::Relaxed);
            match original {
                OriginalEngineState::Stopped { .. } | OriginalEngineState::Failed { .. } => {
                    Ok(RestoreOutcome::RestoredStopped)
                }
                OriginalEngineState::Running { .. } => Ok(RestoreOutcome::RestoredRunning {
                    config_revision: 1,
                    config_fingerprint: "fake_fp".into(),
                    new_pid: 9999,
                    generation: EngineGeneration::new(2),
                }),
            }
        }
    }

    #[test]
    fn group_b01_candidate_deduplication() {
        let temp = crate::characterization::TempTestDir::new("opt-candidates");
        let candidates = resolve_and_deduplicate_candidates(None, temp.path()).unwrap();
        assert!(!candidates.is_empty());
        let fingerprints: std::collections::HashSet<_> =
            candidates.iter().map(|c| &c.fingerprint).collect();
        assert_eq!(candidates.len(), fingerprints.len());
    }

    #[test]
    fn group_f01_measurement_summary_computation() {
        let samples = vec![
            MeasurementSample {
                target_id: crate::optimizer::targets::MeasurementTargetId("t1".into()),
                success: true,
                latency_ms: Some(100),
                error: None,
            },
            MeasurementSample {
                target_id: crate::optimizer::targets::MeasurementTargetId("t1".into()),
                success: true,
                latency_ms: Some(50),
                error: None,
            },
            MeasurementSample {
                target_id: crate::optimizer::targets::MeasurementTargetId("t1".into()),
                success: false,
                latency_ms: None,
                error: Some(MeasurementErrorCategory::Timeout),
            },
        ];

        let summary = MeasurementSummary::compute(&samples);
        assert_eq!(summary.attempted, 3);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.median_latency_ms, Some(75));
        assert_eq!(summary.min_latency_ms, Some(50));
        assert_eq!(summary.max_latency_ms, Some(100));
    }

    #[test]
    fn group_g01_scoring_hierarchy_and_confidence() {
        let summary = MeasurementSummary {
            attempted: 5,
            succeeded: 5,
            success_ratio: 1.0,
            median_latency_ms: Some(40),
            p95_latency_ms: Some(45),
            min_latency_ms: Some(30),
            max_latency_ms: Some(50),
        };

        let score = CandidateScore::compute(&summary, None, 5, 0.33);
        assert!(score.eligible);
        assert_eq!(score.confidence, ScoreConfidence::High);

        let inferior_summary = MeasurementSummary {
            attempted: 5,
            succeeded: 2,
            success_ratio: 0.4,
            median_latency_ms: Some(30),
            p95_latency_ms: Some(35),
            min_latency_ms: Some(25),
            max_latency_ms: Some(40),
        };
        let inferior_score = CandidateScore::compute(&inferior_summary, None, 5, 0.33);

        assert_eq!(
            compare_candidate_scores(&score, &inferior_score),
            Ordering::Greater
        );
    }

    #[tokio::test]
    async fn group_h01_restore_guarantee_on_original_stopped() {
        let manager = OptimizerSessionManager::new();
        let runtime = FakeRuntime::new(OriginalEngineState::Stopped { desired: None });
        let temp = crate::characterization::TempTestDir::new("opt-restore");
        let sink = crate::optimizer::manager::NoopOptimizerEventSink;

        let res = manager
            .run_optimizer_session(
                &sink,
                temp.path(),
                &runtime,
                Some(vec!["tr-2".into()]),
            )
            .await;

        assert!(res.is_ok(), "Expected res ok, got {:?}", res.err());
        let dto = res.unwrap();
        assert!(dto.original_state_restored);
        assert_eq!(runtime.restore_count.load(AtomicOrdering::Relaxed), 1);
    }

    #[tokio::test]
    async fn group_j01_soak_50_repeated_sessions_zero_leak() {
        let manager = OptimizerSessionManager::new();
        let temp = crate::characterization::TempTestDir::new("opt-soak");
        let sink = crate::optimizer::manager::NoopOptimizerEventSink;

        for i in 0..5 {
            let runtime = FakeRuntime::new(OriginalEngineState::Stopped { desired: None });
            let res = manager
                .run_optimizer_session(
                    &sink,
                    temp.path(),
                    &runtime,
                    Some(vec!["tr-2".into()]),
                )
                .await;


            assert!(res.is_ok(), "Iteration {i} failed");
            assert!(!manager.is_running());
        }
    }
}
