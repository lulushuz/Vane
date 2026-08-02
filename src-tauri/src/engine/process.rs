use crate::engine::error::EngineError;
use tokio::process::Child as AsyncChild;

#[cfg(target_os = "windows")]
use crate::engine::job::JobObjectGuard;

/*
   Structure holding ownership of the running winws process.
   RAII semantics: when `ProcessHandle` is dropped, the process is
   automatically terminated. On Windows, the `JobObjectGuard` additionally
   ensures kernel-level cleanup even if Vane itself is force-killed.
*/
pub struct ProcessHandle {
    child: Option<AsyncChild>,
    pid: u32,
    #[cfg(target_os = "windows")]
    _job_guard: Option<JobObjectGuard>,
    #[cfg(target_os = "linux")]
    _route_guard: Option<crate::network::router::NetworkRouteGuard>,
    #[cfg(target_os = "linux")]
    _filter_guard: Option<crate::platform::linux::LinuxFilterGuard>,
}

impl std::fmt::Debug for ProcessHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessHandle")
            .field("pid", &self.pid)
            .finish()
    }
}

impl ProcessHandle {
    pub fn new(
        child: AsyncChild,
        pid: u32,
        #[cfg(target_os = "windows")] job_guard: Option<JobObjectGuard>,
        #[cfg(target_os = "linux")] route_guard: Option<crate::network::router::NetworkRouteGuard>,
        #[cfg(target_os = "linux")] filter_guard: Option<crate::platform::linux::LinuxFilterGuard>,
    ) -> Self {
        Self {
            child: Some(child),
            pid,
            #[cfg(target_os = "windows")]
            _job_guard: job_guard,
            #[cfg(target_os = "linux")]
            _route_guard: route_guard,
            #[cfg(target_os = "linux")]
            _filter_guard: filter_guard,
        }
    }

    /// Requests a clean process exit, waits to a deadline, then escalates to a
    /// force kill. Tokio owns the waiting so the application runtime is never
    /// blocked by polling sleeps.
    pub async fn terminate(&mut self) -> Result<(), EngineError> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };

        // Attempt graceful termination via CTRL_BREAK on Windows.
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Console::GenerateConsoleCtrlEvent;
            use windows::Win32::System::Console::CTRL_BREAK_EVENT;

            /*
               CTRL_BREAK_EVENT to the process group of winws.
               This allows winws to catch the signal and flush WinDivert handles.
            */
            let _ = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, self.pid) };
        }

        // Closing stdin asks the Linux wrapper to run its EXIT cleanup trap.
        #[cfg(not(target_os = "windows"))]
        {
            drop(child.stdin.take());
        }

        let graceful_timeout = if cfg!(target_os = "windows") {
            std::time::Duration::from_millis(500)
        } else {
            std::time::Duration::from_secs(2)
        };
        match tokio::time::timeout(graceful_timeout, child.wait()).await {
            Ok(Ok(status)) => {
                tracing::info!(pid = self.pid, ?status, "Engine process exited cleanly.");
                return Ok(());
            }
            Ok(Err(error)) => {
                tracing::warn!(pid = self.pid, %error, "Could not wait for the engine process; escalating termination.")
            }
            Err(_) => tracing::warn!(
                pid = self.pid,
                "Engine process missed its graceful shutdown deadline; escalating termination."
            ),
        }

        child.start_kill().map_err(|error| {
            EngineError::IoError(format!("Engine process force-kill request failed: {error}"))
        })?;
        match tokio::time::timeout(std::time::Duration::from_secs(2), child.wait()).await {
            Ok(Ok(status)) => {
                tracing::info!(
                    pid = self.pid,
                    ?status,
                    "Engine process force-kill completed."
                );
                Ok(())
            }
            Ok(Err(error)) => Err(EngineError::IoError(format!(
                "Engine process exit could not be observed after force-kill: {error}"
            ))),
            Err(_) => Err(EngineError::IoError(
                "Engine process did not exit before the force-kill deadline.".into(),
            )),
        }
    }

    // Immediate forceful termination. Call this only in Drop or panic paths.
    pub fn kill(&mut self) -> Result<(), EngineError> {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
        Ok(())
    }

    // Returns the PID of the running process.
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

/*
   RAII Drop — forceful kill on scope exit.
   Graceful kill is handled by `EngineManager::stop()` which awaits
   `terminate()` before dropping the handle.
*/
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
            tracing::debug!("ProcessHandle::drop — engine terminated.");
        }
    }
}
