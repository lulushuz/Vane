use crate::config::preset::Preset;
use crate::optimizer::candidate::{resolve_and_deduplicate_candidates, OptimizerCandidate};
use crate::optimizer::measurement::{
    MeasurementErrorCategory, MeasurementPolicy, MeasurementSample, MeasurementSummary,
};
use crate::optimizer::runtime_adapter::OptimizerRuntime;
use crate::optimizer::scoring::{compare_candidate_scores, CandidateScore};
use crate::optimizer::session::{OptimizerError, OptimizerSessionId, OriginalEngineState};
use crate::optimizer::targets::{default_measurement_targets, MeasurementTarget};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizerProgressEvent {
    pub session_id: String,
    pub step: String,
    pub preset_name: String,
    pub progress_pct: u8,
    pub candidate_index: usize,
    pub total_candidates: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizerResultDto {
    pub session_id: String,
    pub best_preset: Option<Preset>,
    pub recommended_candidate_id: Option<String>,
    pub confidence: String,
    pub original_state_restored: bool,
}

pub(crate) trait OptimizerEventSink: Send + Sync {
    fn emit_progress(&self, event: OptimizerProgressEvent);
}

impl<R: tauri::Runtime> OptimizerEventSink for tauri::AppHandle<R> {
    fn emit_progress(&self, event: OptimizerProgressEvent) {
        let _ = self.emit("optimize_progress", event);
    }
}

#[allow(dead_code)]
pub(crate) struct NoopOptimizerEventSink;

impl OptimizerEventSink for NoopOptimizerEventSink {
    fn emit_progress(&self, _event: OptimizerProgressEvent) {}
}

pub struct OptimizerSessionManager {
    active_session: Arc<Mutex<Option<OptimizerSessionId>>>,
    cancel_flag: Arc<AtomicBool>,
    last_optimized_preset: Arc<Mutex<Option<Preset>>>,
}

impl Default for OptimizerSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimizerSessionManager {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            active_session: Arc::new(Mutex::new(None)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            last_optimized_preset: Arc::new(Mutex::new(None)),
        }
    }

    pub fn cancel_active(&self) -> bool {
        self.cancel_flag.store(true, Ordering::SeqCst);
        true
    }

    pub fn is_running(&self) -> bool {
        let guard = self.active_session.try_lock();
        match guard {
            Ok(g) => g.is_some(),
            Err(_) => true,
        }
    }

    pub fn last_optimized_preset(&self) -> Option<Preset> {
        let guard = self.last_optimized_preset.try_lock();
        match guard {
            Ok(g) => g.clone(),
            Err(_) => None,
        }
    }

    pub(crate) async fn run_optimizer_session<R: OptimizerRuntime, S: OptimizerEventSink>(
        &self,
        event_sink: &S,
        app_data_dir: &std::path::Path,
        runtime: &R,
        candidate_ids: Option<Vec<String>>,
    ) -> Result<OptimizerResultDto, OptimizerError> {
        let mut session_guard = self
            .active_session
            .try_lock()
            .map_err(|_| OptimizerError::SessionAlreadyRunning)?;

        if session_guard.is_some() {
            return Err(OptimizerError::SessionAlreadyRunning);
        }

        let session_id = OptimizerSessionId::new();
        *session_guard = Some(session_id.clone());
        self.cancel_flag.store(false, Ordering::SeqCst);

        let candidates = resolve_and_deduplicate_candidates(candidate_ids, app_data_dir)?;
        let targets = default_measurement_targets();
        let policy = MeasurementPolicy::default();

        let original_state = match runtime.capture_original_state().await {
            Ok(state) => state,
            Err(e) => {
                *session_guard = None;
                return Err(e);
            }
        };

        let result = self
            .execute_session_loop(
                event_sink,
                runtime,
                &session_id,
                original_state.clone(),
                candidates,
                targets,
                policy,
            )
            .await;

        let restore_res = runtime.restore_original(&session_id, original_state).await;
        *session_guard = None;

        let restored_ok = matches!(
            restore_res,
            Ok(crate::optimizer::session::RestoreOutcome::RestoredRunning { .. })
                | Ok(crate::optimizer::session::RestoreOutcome::RestoredStopped)
        );

        match result {
            Ok((best_candidate, score)) => {
                let best_p = best_candidate.map(|c| c.preset);
                if let Ok(mut g) = self.last_optimized_preset.try_lock() {
                    *g = best_p.clone();
                }
                Ok(OptimizerResultDto {
                    session_id: session_id.0,
                    best_preset: best_p,
                    recommended_candidate_id: score.as_ref().map(|_| "best".into()),
                    confidence: format!("{:?}", score.as_ref().map(|s| &s.confidence)),
                    original_state_restored: restored_ok,
                })
            }
            Err(e) => Err(e),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_session_loop<R: OptimizerRuntime, S: OptimizerEventSink>(
        &self,
        event_sink: &S,
        runtime: &R,
        session_id: &OptimizerSessionId,
        original_state: OriginalEngineState,
        candidates: Vec<OptimizerCandidate>,
        targets: Vec<MeasurementTarget>,
        policy: MeasurementPolicy,
    ) -> Result<(Option<OptimizerCandidate>, Option<CandidateScore>), OptimizerError> {
        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(OptimizerError::Cancelled);
        }

        if matches!(original_state, OriginalEngineState::Running { .. }) {
            runtime.stop_original_for_session(session_id).await?;
        }

        event_sink.emit_progress(OptimizerProgressEvent {
            session_id: session_id.0.clone(),
            step: "Measuring Baseline...".into(),
            preset_name: "Baseline".into(),
            progress_pct: 5,
            candidate_index: 0,
            total_candidates: candidates.len(),
        });

        let baseline_summary = self.measure_targets(&targets, &policy).await;

        let total = candidates.len();
        let mut best_candidate: Option<OptimizerCandidate> = None;
        let mut best_score: Option<CandidateScore> = None;

        for (idx, candidate) in candidates.into_iter().enumerate() {
            if self.cancel_flag.load(Ordering::SeqCst) {
                let _ = runtime.stop_candidate(session_id).await;
                return Err(OptimizerError::Cancelled);
            }

            let pct = (((idx as f32) / (total as f32)) * 90.0) as u8 + 10;
            event_sink.emit_progress(OptimizerProgressEvent {
                session_id: session_id.0.clone(),
                step: format!("Testing Candidate ({}/{})", idx + 1, total),
                preset_name: candidate.preset.label.clone(),
                progress_pct: pct,
                candidate_index: idx + 1,
                total_candidates: total,
            });

            let start_res = runtime
                .start_candidate(session_id, candidate.prepared_config.clone())
                .await;

            if let Err(e) = start_res {
                tracing::warn!("Candidate {} failed to start: {}", candidate.preset.id, e);
                let _ = runtime.stop_candidate(session_id).await;
                continue;
            }

            tokio::time::sleep(Duration::from_millis(2000)).await;

            let summary = self.measure_targets(&targets, &policy).await;
            if let Some(ref summary_val) = summary {
                let score = CandidateScore::compute(
                    summary_val,
                    baseline_summary.as_ref(),
                    policy.sample_count,
                    policy.minimum_success_ratio,
                );

                if score.eligible {
                    let is_better = match &best_score {
                        None => true,
                        Some(current_best) => {
                            compare_candidate_scores(&score, current_best)
                                == std::cmp::Ordering::Greater
                        }
                    };

                    if is_better {
                        best_score = Some(score);
                        best_candidate = Some(candidate);
                    }
                }
            }

            let _ = runtime.stop_candidate(session_id).await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        event_sink.emit_progress(OptimizerProgressEvent {
            session_id: session_id.0.clone(),
            step: "Completed".into(),
            preset_name: "Completed".into(),
            progress_pct: 100,
            candidate_index: total,
            total_candidates: total,
        });

        Ok((best_candidate, best_score))
    }

    async fn measure_targets(
        &self,
        targets: &[MeasurementTarget],
        policy: &MeasurementPolicy,
    ) -> Option<MeasurementSummary> {
        let client = reqwest::Client::builder()
            .timeout(policy.request_timeout)
            .tcp_keepalive(None)
            .pool_max_idle_per_host(0)
            .user_agent(concat!("VaneDPI/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok()?;

        let mut samples = Vec::new();

        for _ in 0..policy.warmup_count {
            for target in targets {
                let _ = self.single_request(&client, target).await;
            }
        }

        for _ in 0..policy.sample_count {
            if self.cancel_flag.load(Ordering::SeqCst) {
                break;
            }
            for target in targets {
                let sample = self.single_request(&client, target).await;
                samples.push(sample);
                tokio::time::sleep(policy.inter_sample_delay).await;
            }
        }

        if samples.is_empty() {
            None
        } else {
            Some(MeasurementSummary::compute(&samples))
        }
    }

    async fn single_request(
        &self,
        client: &reqwest::Client,
        target: &MeasurementTarget,
    ) -> MeasurementSample {
        let url = format!(
            "https://{}{}",
            target.host,
            target.path.as_deref().unwrap_or("/")
        );
        let start = Instant::now();
        let res = client.head(&url).send().await;
        let elapsed = start.elapsed().as_millis() as u64;

        match res {
            Ok(r) => {
                let ok = r.status().is_success() || r.status().as_u16() < 400;
                MeasurementSample {
                    target_id: target.id.clone(),
                    success: ok,
                    latency_ms: if ok { Some(elapsed) } else { None },
                    error: if ok {
                        None
                    } else {
                        Some(MeasurementErrorCategory::HttpStatus)
                    },
                }
            }
            Err(e) => {
                let err_cat = if e.is_timeout() {
                    MeasurementErrorCategory::Timeout
                } else if e.is_connect() {
                    MeasurementErrorCategory::ConnectionRefused
                } else {
                    MeasurementErrorCategory::NetworkUnavailable
                };

                MeasurementSample {
                    target_id: target.id.clone(),
                    success: false,
                    latency_ms: None,
                    error: Some(err_cat),
                }
            }
        }
    }
}
