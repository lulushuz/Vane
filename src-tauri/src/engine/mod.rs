pub mod error;
pub mod job;
pub(crate) mod launch_plan;
pub(crate) mod launcher;
pub(crate) mod lifecycle;
pub mod logger;
pub(crate) mod owned_process;
pub(crate) mod pattern_transaction;
pub(crate) mod runtime_config;
pub(crate) mod runtime_state;

pub mod manager;
pub mod optimizer;
pub mod process;
pub mod sanitizer;

pub use error::EngineError;
pub use manager::{EngineManager, EngineStatus};
pub use optimizer::{OptimizeError, OptimizePayload, Optimizer};
pub use sanitizer::validate_preset_args;

#[cfg(target_os = "windows")]
pub use job::JobObjectGuard;
