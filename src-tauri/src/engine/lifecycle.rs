#![allow(dead_code)]

use std::fmt;
use std::time::Duration;

use crate::engine::owned_process::{EngineGeneration, EngineOperationId};
use crate::engine::runtime_config::{ConfigFingerprint, ConfigRevision};

pub(crate) const ENGINE_STARTUP_GRACE_PERIOD: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum PlatformReadiness {
    Windows {
        job_assigned: bool,
        windivert_observed: Option<bool>,
    },
    Linux {
        process_group_verified: bool,
    },
    Mock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct EngineReadiness {
    pub process_alive: bool,
    pub startup_grace_passed: bool,
    pub fatal_stderr_detected: bool,
    pub executable_identity_verified: bool,
    pub platform_check: PlatformReadiness,
}

impl EngineReadiness {
    pub(crate) fn is_ready(&self) -> bool {
        self.process_alive
            && self.startup_grace_passed
            && !self.fatal_stderr_detected
            && self.executable_identity_verified
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct LifecycleErrorSummary {
    pub message: String,
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum EngineLifecycleState {
    Stopped,
    Preparing {
        operation: EngineOperationId,
        generation: EngineGeneration,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    StartingProcess {
        operation: EngineOperationId,
        generation: EngineGeneration,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    WaitingForReadiness {
        operation: EngineOperationId,
        generation: EngineGeneration,
        process_id: u32,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    RunningUnverified {
        generation: EngineGeneration,
        process_id: u32,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    Ready {
        generation: EngineGeneration,
        process_id: u32,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
        readiness: EngineReadiness,
    },
    Restarting {
        operation: EngineOperationId,
        previous_generation: EngineGeneration,
        next_generation: EngineGeneration,
    },
    Stopping {
        operation: EngineOperationId,
        generation: EngineGeneration,
        process_id: u32,
    },
    Failed {
        generation: Option<EngineGeneration>,
        error: LifecycleErrorSummary,
    },
}

impl EngineLifecycleState {
    pub(crate) fn stage_name(&self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Preparing { .. } => "preparing",
            Self::StartingProcess { .. } => "starting_process",
            Self::WaitingForReadiness { .. } => "waiting_for_readiness",
            Self::RunningUnverified { .. } => "running_unverified",
            Self::Ready { .. } => "ready",
            Self::Restarting { .. } => "restarting",
            Self::Stopping { .. } => "stopping",
            Self::Failed { .. } => "failed",
        }
    }

    pub(crate) fn is_running_or_ready(&self) -> bool {
        matches!(self, Self::RunningUnverified { .. } | Self::Ready { .. })
    }

    pub(crate) fn current_pid(&self) -> Option<u32> {
        match self {
            Self::WaitingForReadiness { process_id, .. }
            | Self::RunningUnverified { process_id, .. }
            | Self::Ready { process_id, .. }
            | Self::Stopping { process_id, .. } => Some(*process_id),
            _ => None,
        }
    }

    pub(crate) fn current_generation(&self) -> Option<EngineGeneration> {
        match self {
            Self::Stopped => None,
            Self::Preparing { generation, .. }
            | Self::StartingProcess { generation, .. }
            | Self::WaitingForReadiness { generation, .. }
            | Self::RunningUnverified { generation, .. }
            | Self::Ready { generation, .. }
            | Self::Stopping { generation, .. } => Some(*generation),
            Self::Restarting {
                next_generation, ..
            } => Some(*next_generation),
            Self::Failed { generation, .. } => *generation,
        }
    }
}

impl fmt::Display for EngineLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.stage_name())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum EngineLifecycleError {
    #[error("invalid lifecycle state transition from '{from}' to '{to}'")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("engine generation overflow")]
    GenerationOverflow,
    #[error("start operation was cancelled")]
    StartCancelled,
    #[error("process spawn failed: {0}")]
    SpawnFailed(String),
    #[error("readiness check failed: {0}")]
    ReadinessFailed(String),
    #[error("process exited unexpectedly during startup grace period")]
    ProcessExitedDuringStartup,
    #[error("fatal stderr output detected during startup: {0}")]
    FatalStartupError(String),
    #[error("process termination failed: {0}")]
    TerminationFailed(String),
    #[error("ownership mismatch for PID {pid}")]
    OwnershipMismatch { pid: u32 },
}

pub(crate) fn validate_state_transition(
    from: &EngineLifecycleState,
    to: &EngineLifecycleState,
) -> Result<(), EngineLifecycleError> {
    use EngineLifecycleState::*;

    let valid = match (from, to) {
        (Stopped, Preparing { .. }) => true,
        (Preparing { .. }, StartingProcess { .. }) => true,
        (StartingProcess { .. }, WaitingForReadiness { .. }) => true,
        (WaitingForReadiness { .. }, Ready { .. }) => true,
        (WaitingForReadiness { .. }, RunningUnverified { .. }) => true,
        (WaitingForReadiness { .. }, Failed { .. }) => true,
        (WaitingForReadiness { .. }, Stopping { .. }) => true,
        (RunningUnverified { .. }, Ready { .. }) => true,
        (RunningUnverified { .. }, Stopping { .. }) => true,
        (RunningUnverified { .. }, Failed { .. }) => true,
        (Ready { .. }, Stopping { .. }) => true,
        (Ready { .. }, Restarting { .. }) => true,
        (Ready { .. }, Failed { .. }) => true,
        (Stopping { .. }, Stopped) => true,
        (Stopping { .. }, Failed { .. }) => true,
        (Restarting { .. }, Preparing { .. }) => true,
        (Restarting { .. }, StartingProcess { .. }) => true,
        (Restarting { .. }, Failed { .. }) => true,
        (Failed { .. }, Preparing { .. }) => true,
        (Failed { .. }, Stopped) => true,
        // Self transitions (idempotent / status refresh)
        (Stopped, Stopped) => true,
        (Ready { .. }, Ready { .. }) => true,
        (Failed { .. }, Failed { .. }) => true,
        _ => false,
    };

    if valid {
        Ok(())
    } else {
        Err(EngineLifecycleError::InvalidTransition {
            from: from.stage_name(),
            to: to.stage_name(),
        })
    }
}
