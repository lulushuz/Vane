use tokio::process::Child as AsyncChild;
use crate::engine::error::EngineError;

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
        #[cfg(target_os = "windows")]
        job_guard: Option<JobObjectGuard>,
        #[cfg(target_os = "linux")]
        route_guard: Option<crate::network::router::NetworkRouteGuard>,
    ) -> Self {
        Self {
            child: Some(child),
            pid,
            #[cfg(target_os = "windows")]
            _job_guard: job_guard,
            #[cfg(target_os = "linux")]
            _route_guard: route_guard,
        }
    }

    /* 
       Graceful shutdown with 500ms timeout.
       Strategy:
       1. Send `CTRL_BREAK_EVENT` — gives winws time to flush WinDivert state.
       2. Poll for exit up to 500ms in 10ms increments.
       3. Force kill if still running after timeout.
       This prevents corrupt WinDivert kernel state that can occur when
       the process is killed mid-packet-processing. 
    */
    pub fn kill_graceful(&mut self) {
        let child = match self.child.take() {
            Some(c) => c,
            None => return,
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
            let _ = unsafe {
                GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, self.pid)
            };

            // Poll for clean exit (max 500ms, 10ms steps = 50 iterations).
            let mut child = child;
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
            while std::time::Instant::now() < deadline {
                // try_wait is non-blocking
                match child.try_wait() {
                    Ok(Some(_)) => {
                        tracing::info!("winws (PID {}) graceful shutdown completed successfully.", self.pid);
                        return;
                    }
                    Ok(None) => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        tracing::warn!("try_wait error: {}", e);
                        break;
                    }
                }
            }

            // Graceful period elapsed — force kill.
            tracing::warn!("winws (PID {}) 500ms içinde bitmedi, force kill uygulanıyor.", self.pid);
            let _ = child.start_kill();
        }

        // On non-Windows targets (Linux Root Wrapper case), close stdin to trigger cleanup.
        #[cfg(not(target_os = "windows"))]
        {
            let mut child = child;
            // Stdin'i kapatmak, script içindeki `cat > /dev/null` komutunu sonlandırır 
            // ve iptables temizlik komutlarının çalışmasını sağlar.
            drop(child.stdin.take());

            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            let mut exited = false;
            while std::time::Instant::now() < deadline {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        exited = true;
                        break;
                    }
                    Ok(None) => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
            
            if !exited {
                tracing::warn!("Linux Root Wrapper (PID {}) did not finish in 2 seconds, applying force kill.", self.pid);
                let _ = child.start_kill();
            } else {
                tracing::info!("Linux Root Wrapper (PID {}) closed successfully after cleanup.", self.pid);
            }
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
   Graceful kill is handled by `EngineManager::stop()` which calls
   `kill_graceful()` before dropping the handle. 
*/
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(target_os = "linux")]
            {
                // Stdin'i kapat, scriptin temizlik yapmasına izin ver (max 500ms bekle)
                drop(child.stdin.take());
                let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
                while std::time::Instant::now() < deadline {
                    if let Ok(Some(_)) = child.try_wait() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            let _ = child.start_kill();
            tracing::debug!("ProcessHandle::drop — engine terminated.");
        }
    }
}
