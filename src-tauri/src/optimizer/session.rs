use crate::engine::owned_process::EngineGeneration;
use crate::engine::runtime_config::{
    AppliedRuntimeConfig, PreparedRuntimeConfig, VerifiedRuntimeConfig,
};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OptimizerSessionId(pub String);

impl OptimizerSessionId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        Self(format!("opt_sess_{timestamp}"))
    }
}

impl Default for OptimizerSessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub(crate) enum OriginalEngineState {
    Stopped {
        desired: Option<VerifiedRuntimeConfig>,
    },
    Running {
        desired: VerifiedRuntimeConfig,
        applied: AppliedRuntimeConfig,
        prepared: PreparedRuntimeConfig,
    },
    Failed {
        desired: Option<VerifiedRuntimeConfig>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizerSessionState {
    Preparing,
    MeasuringBaseline,
    TestingCandidate {
        index: usize,
        total: usize,
        candidate_id: String,
    },
    RestoringOriginal,
    Completed,
    Cancelled,
    Failed,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum RestoreOutcome {
    RestoredRunning {
        config_revision: u64,
        config_fingerprint: String,
        new_pid: u32,
        generation: EngineGeneration,
    },
    RestoredStopped,
    Failed {
        error: String,
    },
}

#[derive(Debug, thiserror::Error, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizerError {
    #[error("Another optimizer session is already running.")]
    SessionAlreadyRunning,
    #[error("Candidate set is empty or invalid.")]
    InvalidCandidateSet,
    #[error("Candidate preset {0} was not found.")]
    CandidateNotFound(String),
    #[error("Candidate preset {0} is unsupported on platform {1}.")]
    CandidateUnsupported(String, String),
    #[error("Candidate validation failed: {0}")]
    CandidateValidationFailed(String),
    #[error("Runtime is busy with another operation.")]
    RuntimeBusy,
    #[error("Failed to capture original engine state: {0}")]
    OriginalStateCaptureFailed(String),
    #[error("Failed to stop original engine: {0}")]
    OriginalStopFailed(String),
    #[error("Baseline measurement failed: {0}")]
    BaselineFailed(String),
    #[error("Failed to start candidate engine: {0}")]
    CandidateStartFailed(String),
    #[error("Candidate engine failed readiness check: {0}")]
    CandidateReadinessFailed(String),
    #[error("Measurement failed: {0}")]
    MeasurementFailed(String),
    #[error("Network environment changed during session: {0}")]
    NetworkEnvironmentChanged(String),
    #[error("Candidate engine cleanup failed: {0}")]
    CandidateCleanupFailed(String),
    #[error("Restore of original engine state failed: {0}")]
    RestoreFailed(String),
    #[error("Optimizer session was cancelled.")]
    Cancelled,
    #[error("Internal optimizer state invariant violated: {0}")]
    StateInvariant(String),
}
