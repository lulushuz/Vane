use crate::engine::runtime_config::{
    AppliedRuntimeConfig, ConfigRevision, PreparedRuntimeConfig, VerifiedRuntimeConfig,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum RuntimeStateError {
    #[error("fingerprint mismatch between prepared ({prepared}) and applied ({applied}) config")]
    FingerprintMismatch { prepared: String, applied: String },
    #[error("revision decreased from {current} to {attempted}")]
    StaleRevision { current: u64, attempted: u64 },
    #[error(transparent)]
    ConfigError(#[from] crate::engine::runtime_config::RuntimeConfigError),
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfigState {
    desired: Option<VerifiedRuntimeConfig>,
    prepared: Option<PreparedRuntimeConfig>,
    applied: Option<AppliedRuntimeConfig>,
    latest_requested_revision: ConfigRevision,
    latest_completed_revision: ConfigRevision,
}

impl RuntimeConfigState {
    pub(crate) fn new(initial_revision: ConfigRevision) -> Self {
        Self {
            desired: None,
            prepared: None,
            applied: None,
            latest_requested_revision: initial_revision,
            latest_completed_revision: initial_revision,
        }
    }

    pub(crate) fn desired(&self) -> Option<&VerifiedRuntimeConfig> {
        self.desired.as_ref()
    }

    #[allow(dead_code)]
    pub(crate) fn prepared(&self) -> Option<&PreparedRuntimeConfig> {
        self.prepared.as_ref()
    }

    pub(crate) fn applied(&self) -> Option<&AppliedRuntimeConfig> {
        self.applied.as_ref()
    }

    pub(crate) fn latest_requested_revision(&self) -> ConfigRevision {
        self.latest_requested_revision
    }

    #[allow(dead_code)]
    pub(crate) fn latest_completed_revision(&self) -> ConfigRevision {
        self.latest_completed_revision
    }

    pub(crate) fn advance_requested_revision(
        &mut self,
    ) -> Result<ConfigRevision, RuntimeStateError> {
        let next = self.latest_requested_revision.next()?;
        self.latest_requested_revision = next;
        Ok(next)
    }

    pub(crate) fn set_desired(&mut self, config: VerifiedRuntimeConfig) {
        if config.revision.get() > self.latest_requested_revision.get() {
            self.latest_requested_revision = config.revision;
        }
        self.desired = Some(config);
    }

    pub(crate) fn set_prepared(&mut self, config: PreparedRuntimeConfig) {
        self.prepared = Some(config);
    }

    pub(crate) fn commit_applied(
        &mut self,
        config: AppliedRuntimeConfig,
    ) -> Result<(), RuntimeStateError> {
        if let Some(prepared) = &self.prepared {
            if prepared.verified.fingerprint != config.verified.fingerprint {
                return Err(RuntimeStateError::FingerprintMismatch {
                    prepared: prepared.verified.fingerprint.to_string(),
                    applied: config.verified.fingerprint.to_string(),
                });
            }
        }

        if config.verified.revision.get() < self.latest_completed_revision.get() {
            return Err(RuntimeStateError::StaleRevision {
                current: self.latest_completed_revision.get(),
                attempted: config.verified.revision.get(),
            });
        }

        self.latest_completed_revision = config.verified.revision;
        self.applied = Some(config);
        Ok(())
    }

    pub(crate) fn restore_applied(&mut self, config: AppliedRuntimeConfig) {
        self.applied = Some(config);
    }

    pub(crate) fn clear_applied(&mut self) {
        self.applied = None;
    }
}
