/* 
   Windows Job Object RAII wrapper.
   When this guard is dropped (application exit, panic, or forced kill),
   the Windows kernel automatically terminates all processes assigned
   to this job — including winws.exe.
   This is a kernel-level guarantee that supersedes all software-level
   cleanup (on_window_event hooks, Drop impls, etc.). 
*/
#[cfg(target_os = "windows")]
pub struct JobObjectGuard {
    handle: windows::Win32::Foundation::HANDLE,
}

// SAFETY: this guard has exclusive ownership of a process-wide Windows kernel
// handle. Moving that ownership to another thread does not invalidate the
// handle, and Drop closes it exactly once.
#[cfg(target_os = "windows")]
unsafe impl Send for JobObjectGuard {}

#[cfg(target_os = "windows")]
impl JobObjectGuard {
    // Creates a new anonymous Job Object with `KILL_ON_JOB_CLOSE` semantics.
    pub fn new() -> Result<Self, crate::engine::error::EngineError> {
        use windows::Win32::System::JobObjects::{
            CreateJobObjectW, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        let handle = unsafe {
            CreateJobObjectW(None, None)
                .map_err(|e| crate::engine::error::EngineError::IoError(
                    format!("CreateJobObject failed: {}", e)
                ))?
        };
        let guard = Self { handle };

        /* 
           Any process assigned to this job will be killed when the last
           handle to the job object is closed. 
        */
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        unsafe {
            SetInformationJobObject(
                guard.handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ).map_err(|e| crate::engine::error::EngineError::IoError(
                format!("SetInformationJobObject failed: {}", e)
            ))?;
        }

        tracing::debug!("Job Object created successfully.");
        Ok(guard)
    }

    // Assigns a process (by PID) to this job object.
    /* 
       Errors: Returns `EngineError::IoError` if the process cannot be opened or assigned. 
    */
    pub fn assign(&self, pid: u32) -> Result<(), crate::engine::error::EngineError> {
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
        };
        use windows::Win32::System::JobObjects::AssignProcessToJobObject;

        let process_handle = unsafe {
            OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
                .map_err(|e| crate::engine::error::EngineError::IoError(
                    format!("OpenProcess({}) failed: {}", pid, e)
                ))?
        };

        let result = unsafe {
            AssignProcessToJobObject(self.handle, process_handle)
                .map_err(|e| crate::engine::error::EngineError::IoError(
                    format!("AssignProcessToJobObject failed: {}", e)
                ))
        };

        // Always close the process handle — we only needed it for assignment.
        unsafe { let _ = windows::Win32::Foundation::CloseHandle(process_handle); }

        result
    }
}

// Closing the job handle signals Windows to kill all assigned processes.
#[cfg(target_os = "windows")]
impl Drop for JobObjectGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.handle);
        }
        tracing::debug!("Job Object dropped — child processes will be terminated by kernel.");
    }
}
