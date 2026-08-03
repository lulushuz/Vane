#![allow(dead_code)]

use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

static ENGINE_OPERATION_SEQUENCE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

use crate::engine::runtime_config::{ConfigFingerprint, ConfigRevision};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EngineGeneration(pub u64);

impl EngineGeneration {
    pub(crate) fn new(val: u64) -> Self {
        Self(val)
    }

    pub(crate) fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, String> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or_else(|| "Engine generation overflow".into())
    }
}

impl fmt::Display for EngineGeneration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gen({})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct EngineOperationId(pub String);

impl EngineOperationId {
    pub(crate) fn generate(prefix: &str) -> Self {
        let id = format!(
            "{}-{}-{}",
            prefix,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            ENGINE_OPERATION_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        Self(id)
    }
}

impl fmt::Display for EngineOperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Op({})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineProcessIdentity {
    pub installation_id: String,
    pub instance_id: String,
    pub generation: EngineGeneration,
    pub expected_executable: PathBuf,
    pub config_revision: ConfigRevision,
    pub config_fingerprint: ConfigFingerprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminationMode {
    Graceful { timeout_ms: u64 },
    Forced,
}

#[derive(Debug)]
pub(crate) struct ProcessExit {
    pub pid: u32,
    pub exit_code: Option<i32>,
    pub terminated_by_us: bool,
}

pub(crate) enum OwnedChildHandle {
    #[cfg(target_os = "windows")]
    Windows {
        child: Box<tokio::process::Child>,
        job_guard: Option<crate::engine::job::JobObjectGuard>,
        pid: u32,
    },
    #[cfg(target_os = "linux")]
    Linux {
        child: Box<tokio::process::Child>,
        pid: u32,
    },
    Fake {
        pid: u32,
        exited: bool,
    },
}

impl fmt::Debug for OwnedChildHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows { pid, .. } => write!(f, "OwnedChildHandle::Windows(PID {})", pid),
            #[cfg(target_os = "linux")]
            Self::Linux { pid, .. } => write!(f, "OwnedChildHandle::Linux(PID {})", pid),
            Self::Fake { pid, exited } => {
                write!(f, "OwnedChildHandle::Fake(PID {}, exited: {})", pid, exited)
            }
        }
    }
}

pub(crate) struct OwnedEngineProcess {
    identity: EngineProcessIdentity,
    child_handle: OwnedChildHandle,
    #[allow(dead_code)]
    started_at: SystemTime,
}

impl OwnedEngineProcess {
    pub(crate) fn new(identity: EngineProcessIdentity, child_handle: OwnedChildHandle) -> Self {
        Self {
            identity,
            child_handle,
            started_at: SystemTime::now(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn identity(&self) -> &EngineProcessIdentity {
        &self.identity
    }

    pub(crate) fn pid(&self) -> u32 {
        match &self.child_handle {
            #[cfg(target_os = "windows")]
            OwnedChildHandle::Windows { pid, .. } => *pid,
            #[cfg(target_os = "linux")]
            OwnedChildHandle::Linux { pid, .. } => *pid,
            OwnedChildHandle::Fake { pid, .. } => *pid,
        }
    }

    pub(crate) fn generation(&self) -> EngineGeneration {
        self.identity.generation
    }

    pub(crate) fn config_revision(&self) -> ConfigRevision {
        self.identity.config_revision
    }

    pub(crate) fn config_fingerprint(&self) -> &ConfigFingerprint {
        &self.identity.config_fingerprint
    }

    pub(crate) fn is_alive(&mut self) -> bool {
        match &mut self.child_handle {
            #[cfg(target_os = "windows")]
            OwnedChildHandle::Windows { child, pid, .. } => {
                if *pid == 0 {
                    return false;
                }
                match child.try_wait() {
                    Ok(Some(_)) => false,
                    Ok(None) => true,
                    Err(_) => false,
                }
            }
            #[cfg(target_os = "linux")]
            OwnedChildHandle::Linux { child, pid, .. } => {
                if *pid == 0 {
                    return false;
                }
                match child.try_wait() {
                    Ok(Some(_)) => false,
                    Ok(None) => true,
                    Err(_) => false,
                }
            }
            OwnedChildHandle::Fake { exited, .. } => !*exited,
        }
    }

    pub(crate) async fn terminate(&mut self, mode: TerminationMode) -> Result<ProcessExit, String> {
        let pid = self.pid();
        tracing::info!(
            "Terminating owned engine process PID {} with mode {:?}",
            pid,
            mode
        );

        match &mut self.child_handle {
            #[cfg(target_os = "windows")]
            OwnedChildHandle::Windows { child, .. } => {
                let kill_res = child.start_kill();
                if let Err(e) = kill_res {
                    tracing::warn!("Failed to signal kill on child PID {}: {}", pid, e);
                }
                let status = child.wait().await.map_err(|e| e.to_string())?;
                Ok(ProcessExit {
                    pid,
                    exit_code: status.code(),
                    terminated_by_us: true,
                })
            }
            #[cfg(target_os = "linux")]
            OwnedChildHandle::Linux { child, .. } => {
                let kill_res = child.start_kill();
                if let Err(e) = kill_res {
                    tracing::warn!("Failed to signal kill on child PID {}: {}", pid, e);
                }
                let status = child.wait().await.map_err(|e| e.to_string())?;
                Ok(ProcessExit {
                    pid,
                    exit_code: status.code(),
                    terminated_by_us: true,
                })
            }
            OwnedChildHandle::Fake { exited, .. } => {
                *exited = true;
                Ok(ProcessExit {
                    pid,
                    exit_code: Some(0),
                    terminated_by_us: true,
                })
            }
        }
    }
}

impl fmt::Debug for OwnedEngineProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedEngineProcess")
            .field("pid", &self.pid())
            .field("generation", &self.identity.generation)
            .field("config_revision", &self.identity.config_revision)
            .field("child_handle", &self.child_handle)
            .finish()
    }
}
