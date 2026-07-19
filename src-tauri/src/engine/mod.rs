pub mod error;
pub mod job;
pub mod manager;
pub mod process;
pub mod optimizer;
pub mod sanitizer;
pub mod logger;

pub use manager::{EngineManager, EngineStatus};
pub use error::EngineError;
pub use optimizer::{Optimizer, OptimizePayload, OptimizeError};
pub use sanitizer::validate_preset_args;

#[cfg(target_os = "windows")]
pub use job::JobObjectGuard;
