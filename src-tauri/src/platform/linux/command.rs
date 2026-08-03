use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxCommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxCommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LinuxCommandRunError {
    #[error("spawn failed: {0}")]
    Spawn(String),
    #[error("stdin write failed: {0}")]
    StdinWrite(String),
    #[error("wait failed: {0}")]
    Wait(String),
}

pub trait LinuxCommandRunner: Send + Sync {
    fn run(&self, command: &LinuxCommandSpec) -> Result<LinuxCommandOutput, LinuxCommandRunError>;
}

#[derive(Default)]
pub struct SystemLinuxCommandRunner;

impl LinuxCommandRunner for SystemLinuxCommandRunner {
    fn run(&self, command: &LinuxCommandSpec) -> Result<LinuxCommandOutput, LinuxCommandRunError> {
        let mut process = Command::new(&command.program);
        process
            .args(&command.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if command.stdin.is_some() {
            process.stdin(Stdio::piped());
        } else {
            process.stdin(Stdio::null());
        }

        let mut child = process
            .spawn()
            .map_err(|error| LinuxCommandRunError::Spawn(error.to_string()))?;
        if let Some(payload) = command.stdin.as_ref() {
            let write_result = child
                .stdin
                .take()
                .ok_or_else(|| {
                    LinuxCommandRunError::StdinWrite("stdin pipe was unavailable".into())
                })
                .and_then(|mut stdin| {
                    stdin
                        .write_all(payload)
                        .map_err(|error| LinuxCommandRunError::StdinWrite(error.to_string()))
                });
            if let Err(error) = write_result {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }

        let output = child
            .wait_with_output()
            .map_err(|error| LinuxCommandRunError::Wait(error.to_string()))?;
        Ok(LinuxCommandOutput {
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    pub struct FakeLinuxCommandRunner {
        outcomes: Mutex<VecDeque<Result<LinuxCommandOutput, LinuxCommandRunError>>>,
        commands: Mutex<Vec<LinuxCommandSpec>>,
    }

    impl FakeLinuxCommandRunner {
        pub fn new(outcomes: Vec<Result<LinuxCommandOutput, LinuxCommandRunError>>) -> Self {
            Self {
                outcomes: Mutex::new(outcomes.into()),
                commands: Mutex::new(Vec::new()),
            }
        }

        pub fn success(count: usize) -> Self {
            Self::new(
                (0..count)
                    .map(|_| {
                        Ok(LinuxCommandOutput {
                            exit_code: Some(0),
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                        })
                    })
                    .collect(),
            )
        }

        pub fn commands(&self) -> Vec<LinuxCommandSpec> {
            self.commands.lock().unwrap().clone()
        }
    }

    impl LinuxCommandRunner for FakeLinuxCommandRunner {
        fn run(
            &self,
            command: &LinuxCommandSpec,
        ) -> Result<LinuxCommandOutput, LinuxCommandRunError> {
            self.commands.lock().unwrap().push(command.clone());
            self.outcomes
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| panic!("No fake outcome for command: {command:?}"))
        }
    }
}
