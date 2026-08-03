#![allow(dead_code)]

use tauri::AppHandle;

use crate::engine::launch_plan::EngineLaunchPlan;
use crate::engine::owned_process::{
    EngineProcessIdentity, OwnedChildHandle, OwnedEngineProcess, ProcessExit, TerminationMode,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum EngineLaunchError {
    #[error("binary not found: {0}")]
    BinaryNotFound(String),
    #[error("process spawn failed: {0}")]
    SpawnFailed(String),
    #[error("job object assignment failed: {0}")]
    JobAssignmentFailed(String),
    #[error("authorization denied: {0}")]
    AuthorizationFailed(String),
}

#[allow(dead_code)]
pub(crate) trait PlatformEngineLauncher: Send + Sync {
    fn spawn(
        &self,
        plan: &EngineLaunchPlan,
        identity: EngineProcessIdentity,
        app: &AppHandle,
    ) -> Result<OwnedEngineProcess, EngineLaunchError>;

    fn terminate(
        &self,
        process: &mut OwnedEngineProcess,
        mode: TerminationMode,
    ) -> Result<ProcessExit, EngineLaunchError>;
}

#[allow(dead_code)]
pub(crate) struct RealEngineLauncher;

impl PlatformEngineLauncher for RealEngineLauncher {
    fn spawn(
        &self,
        plan: &EngineLaunchPlan,
        identity: EngineProcessIdentity,
        app: &AppHandle,
    ) -> Result<OwnedEngineProcess, EngineLaunchError> {
        let executable = &plan.binary.executable;
        if !executable.exists() && !cfg!(target_os = "windows") {
            return Err(EngineLaunchError::BinaryNotFound(format!(
                "Executable path is invalid: {:?}",
                executable
            )));
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Stdio;

            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let working_dir = executable.parent().unwrap_or(executable);
            let mut command = std::process::Command::new(executable);
            command
                .args(&plan.final_arguments)
                .current_dir(working_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW);

            let mut child = tokio::process::Command::from(command)
                .spawn()
                .map_err(|e| EngineLaunchError::SpawnFailed(e.to_string()))?;

            let pid = child.id().unwrap_or(0);
            let job_guard = match crate::engine::job::JobObjectGuard::new()
                .and_then(|j| j.assign(pid).map(|_| j))
            {
                Ok(j) => Some(j),
                Err(e) => {
                    tracing::warn!("Job Object assignment failed for PID {}: {}", pid, e);
                    None
                }
            };

            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            if let Some(out) = stdout {
                crate::engine::logger::spawn_log_reader(out, app.clone(), None);
            }
            if let Some(err) = stderr {
                crate::engine::logger::spawn_log_reader(err, app.clone(), Some("HATA: "));
            }

            let child_handle = OwnedChildHandle::Windows {
                child: Box::new(child),
                job_guard,
                pid,
            };

            Ok(OwnedEngineProcess::new(identity, child_handle))
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Stdio;

            let mut command = tokio::process::Command::new(executable);
            command
                .args(&plan.final_arguments)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command
                .spawn()
                .map_err(|e| EngineLaunchError::SpawnFailed(e.to_string()))?;

            let pid = child.id().unwrap_or(0);
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            if let Some(out) = stdout {
                crate::engine::logger::spawn_log_reader(out, app.clone(), None);
            }
            if let Some(err) = stderr {
                crate::engine::logger::spawn_log_reader(err, app.clone(), Some("HATA: "));
            }

            let child_handle = OwnedChildHandle::Linux {
                child: Box::new(child),
                pid,
            };
            Ok(OwnedEngineProcess::new(identity, child_handle))
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(EngineLaunchError::SpawnFailed("Unsupported OS".into()))
        }
    }

    fn terminate(
        &self,
        process: &mut OwnedEngineProcess,
        mode: TerminationMode,
    ) -> Result<ProcessExit, EngineLaunchError> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async { process.terminate(mode).await })
            .map_err(EngineLaunchError::SpawnFailed)
    }
}

#[allow(dead_code)]
pub(crate) struct FakeEngineLauncher {
    pub should_fail_spawn: bool,
    pub next_pid: u32,
    pub immediate_crash: bool,
}

#[allow(dead_code)]
impl FakeEngineLauncher {
    pub(crate) fn new() -> Self {
        Self {
            should_fail_spawn: false,
            next_pid: 9000,
            immediate_crash: false,
        }
    }
}

impl PlatformEngineLauncher for FakeEngineLauncher {
    fn spawn(
        &self,
        _plan: &EngineLaunchPlan,
        identity: EngineProcessIdentity,
        _app: &AppHandle,
    ) -> Result<OwnedEngineProcess, EngineLaunchError> {
        if self.should_fail_spawn {
            return Err(EngineLaunchError::SpawnFailed(
                "Simulated spawn failure".into(),
            ));
        }

        let child_handle = OwnedChildHandle::Fake {
            pid: self.next_pid,
            exited: self.immediate_crash,
        };

        Ok(OwnedEngineProcess::new(identity, child_handle))
    }

    fn terminate(
        &self,
        process: &mut OwnedEngineProcess,
        mode: TerminationMode,
    ) -> Result<ProcessExit, EngineLaunchError> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async { process.terminate(mode).await })
            .map_err(EngineLaunchError::SpawnFailed)
    }
}
