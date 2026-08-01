use tauri::AppHandle;

pub use crate::optimizer::manager::{
    OptimizerProgressEvent as OptimizePayload, OptimizerResultDto,
};
pub use crate::optimizer::session::OptimizerError as OptimizeError;

#[allow(dead_code)]
pub struct Optimizer {
    app: AppHandle,
}

impl Optimizer {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    pub fn save_state(
        _app: &AppHandle,
        _preset: &crate::config::preset::Preset,
    ) -> std::io::Result<()> {
        // Obsolete in P12: Optimizer candidates are ephemeral and never auto-persisted.
        Ok(())
    }

    pub fn load_state(_app: &AppHandle) -> Option<crate::config::preset::Preset> {
        // Obsolete in P12
        None
    }
}
