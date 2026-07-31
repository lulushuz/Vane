pub mod candidate;
pub mod manager;
pub mod measurement;
pub mod runtime_adapter;
pub mod scoring;
pub mod session;
pub mod targets;

#[allow(unused_imports)]
pub(crate) use candidate::{resolve_and_deduplicate_candidates, OptimizerCandidate};
pub use manager::{OptimizerProgressEvent, OptimizerResultDto, OptimizerSessionManager};
pub use measurement::{MeasurementPolicy, MeasurementSample, MeasurementSummary};
#[allow(unused_imports)]
pub(crate) use runtime_adapter::{OptimizerRuntime, ProductionOptimizerRuntime};
pub use scoring::{compare_candidate_scores, CandidateScore, ScoreConfidence};
pub use session::{OptimizerError, OptimizerSessionId, OptimizerSessionState};
#[allow(unused_imports)]
pub(crate) use session::{OriginalEngineState, RestoreOutcome};
pub use targets::{default_measurement_targets, MeasurementProtocol, MeasurementTarget};
