use crate::engine::manager::{EngineManager, EngineStatus};
use crate::engine::runtime_config::{AppliedRuntimeConfig, PreparedRuntimeConfig};
use crate::optimizer::session::{
    OptimizerError, OptimizerSessionId, OriginalEngineState, RestoreOutcome,
};
use tauri::{AppHandle, Manager};

#[allow(async_fn_in_trait)]
pub(crate) trait OptimizerRuntime: Send + Sync {
    async fn capture_original_state(&self) -> Result<OriginalEngineState, OptimizerError>;

    async fn stop_original_for_session(
        &self,
        session_id: &OptimizerSessionId,
    ) -> Result<(), OptimizerError>;

    async fn start_candidate(
        &self,
        session_id: &OptimizerSessionId,
        candidate: PreparedRuntimeConfig,
    ) -> Result<AppliedRuntimeConfig, OptimizerError>;

    async fn stop_candidate(&self, session_id: &OptimizerSessionId) -> Result<(), OptimizerError>;

    async fn restore_original(
        &self,
        session_id: &OptimizerSessionId,
        original: OriginalEngineState,
    ) -> Result<RestoreOutcome, OptimizerError>;
}

pub(crate) struct ProductionOptimizerRuntime {
    app: AppHandle,
    engine_manager: EngineManager,
    active_session: std::sync::Mutex<Option<OptimizerSessionId>>,
}

impl ProductionOptimizerRuntime {
    pub fn new(app: AppHandle, engine_manager: EngineManager) -> Self {
        Self {
            app,
            engine_manager,
            active_session: std::sync::Mutex::new(None),
        }
    }

    fn require_session(&self, session_id: &OptimizerSessionId) -> Result<(), OptimizerError> {
        let guard = self.active_session.lock().map_err(|_| {
            OptimizerError::CandidateStartFailed("Optimizer session lock poisoned".into())
        })?;
        if guard.as_ref() != Some(session_id) {
            return Err(OptimizerError::CandidateStartFailed(
                "Optimizer session ownership mismatch".into(),
            ));
        }
        Ok(())
    }
}

impl OptimizerRuntime for ProductionOptimizerRuntime {
    async fn capture_original_state(&self) -> Result<OriginalEngineState, OptimizerError> {
        let status = self.engine_manager.current_status();
        let config_state = self.engine_manager.runtime_config_state();
        let runtime_guard = config_state.lock().map_err(|_| {
            OptimizerError::OriginalStateCaptureFailed("Runtime config state lock poisoned.".into())
        })?;

        let desired = runtime_guard.desired().cloned();

        match status {
            EngineStatus::Ready { .. } => {
                let applied = runtime_guard.applied().cloned().ok_or_else(|| {
                    OptimizerError::OriginalStateCaptureFailed(
                        "Running engine missing applied runtime config.".into(),
                    )
                })?;

                let app_dir = self
                    .app
                    .path()
                    .app_data_dir()
                    .map_err(|e| OptimizerError::OriginalStateCaptureFailed(e.to_string()))?;

                let (prepared, _) =
                    crate::engine::pattern_transaction::prepare_runtime_config_for_transaction(
                        applied.verified.clone(),
                        &app_dir,
                    )
                    .map_err(|e| {
                        OptimizerError::OriginalStateCaptureFailed(format!(
                            "Failed to prepare original config: {e}"
                        ))
                    })?;

                Ok(OriginalEngineState::Running {
                    desired: applied.verified.clone(),
                    applied,
                    prepared,
                })
            }
            EngineStatus::Stopped => Ok(OriginalEngineState::Stopped { desired }),
            EngineStatus::Error { .. } => Ok(OriginalEngineState::Failed { desired }),
            _ => Ok(OriginalEngineState::Stopped { desired }),
        }
    }

    async fn stop_original_for_session(
        &self,
        session_id: &OptimizerSessionId,
    ) -> Result<(), OptimizerError> {
        {
            let mut guard = self.active_session.lock().map_err(|_| {
                OptimizerError::OriginalStopFailed("Optimizer session lock poisoned".into())
            })?;
            if guard.is_some() && guard.as_ref() != Some(session_id) {
                return Err(OptimizerError::OriginalStopFailed(
                    "Another optimizer session owns the runtime".into(),
                ));
            }
            *guard = Some(session_id.clone());
        }
        self.engine_manager
            .stop(&self.app)
            .await
            .map_err(|e| OptimizerError::OriginalStopFailed(e.to_string()))
    }

    async fn start_candidate(
        &self,
        session_id: &OptimizerSessionId,
        candidate: PreparedRuntimeConfig,
    ) -> Result<AppliedRuntimeConfig, OptimizerError> {
        self.require_session(session_id)?;
        self.engine_manager
            .start_prepared_config(candidate, &self.app)
            .await
            .map_err(|e| OptimizerError::CandidateStartFailed(e.to_string()))
    }

    async fn stop_candidate(&self, session_id: &OptimizerSessionId) -> Result<(), OptimizerError> {
        self.require_session(session_id)
            .map_err(|error| OptimizerError::CandidateCleanupFailed(error.to_string()))?;
        self.engine_manager
            .stop(&self.app)
            .await
            .map_err(|e| OptimizerError::CandidateCleanupFailed(e.to_string()))
    }

    async fn restore_original(
        &self,
        session_id: &OptimizerSessionId,
        original: OriginalEngineState,
    ) -> Result<RestoreOutcome, OptimizerError> {
        self.require_session(session_id)
            .map_err(|error| OptimizerError::RestoreFailed(error.to_string()))?;
        let result = match original {
            OriginalEngineState::Stopped { .. } | OriginalEngineState::Failed { .. } => {
                let _ = self.engine_manager.stop(&self.app).await;
                Ok(RestoreOutcome::RestoredStopped)
            }
            OriginalEngineState::Running { prepared, .. } => {
                let _ = self.engine_manager.stop(&self.app).await;
                match self
                    .engine_manager
                    .start_prepared_config(prepared, &self.app)
                    .await
                {
                    Ok(restored_applied) => {
                        let _ =
                            self.engine_manager
                                .runtime_config_state()
                                .lock()
                                .map(|mut guard| {
                                    guard.restore_applied(restored_applied.clone());
                                });

                        Ok(RestoreOutcome::RestoredRunning {
                            config_revision: restored_applied.verified.revision.get(),
                            config_fingerprint: restored_applied.verified.fingerprint.to_string(),
                            new_pid: restored_applied.process_id,
                            generation: self.engine_manager.current_generation(),
                        })
                    }
                    Err(e) => Ok(RestoreOutcome::Failed {
                        error: format!("Failed to restore running engine config: {e}"),
                    }),
                }
            }
        };
        if !matches!(result, Ok(RestoreOutcome::Failed { .. })) {
            if let Ok(mut guard) = self.active_session.lock() {
                *guard = None;
            }
        }
        result
    }
}
