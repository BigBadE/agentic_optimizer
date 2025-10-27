//! Command execution utility for running exit commands in task lists.

use merlin_core::{Result, RoutingError};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Result of running a command
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Exit code from the command
    pub exit_code: i32,
    /// Standard output from the command
    pub stdout: String,
    /// Standard error from the command
    pub stderr: String,
    /// Duration the command took to execute
    pub duration: Duration,
    /// Whether the command succeeded (exit code 0)
    pub success: bool,
}

impl CommandResult {
    /// Get a combined error message from stderr and stdout
    #[must_use]
    pub fn error_message(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else {
            format!("{}\n{}", self.stderr, self.stdout)
        }
    }
}

/// Command runner for executing shell commands
#[derive(Debug, Clone)]
pub struct CommandRunner {
    /// Working directory for command execution
    working_dir: PathBuf,
}

impl CommandRunner {
    /// Create a new command runner with the given working directory
    #[must_use]
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    /// Run a command and return the result
    ///
    /// # Errors
    /// Returns an error if the command cannot be spawned
    pub fn run(&self, command_str: &str) -> Result<CommandResult> {
        let start = Instant::now();

        if command_str.trim().is_empty() {
            return Err(RoutingError::Other("Empty command string".to_owned()));
        }

        merlin_deps::tracing::debug!(
            "Running command: {} in directory: {:?}",
            command_str,
            self.working_dir
        );

        // On Windows, try bash first (for Git Bash), fall back to cmd.exe
        // On other platforms, use sh for compatibility
        #[cfg(target_os = "windows")]
        let shell_commands = [
            ("bash.exe", vec!["-c", command_str]),
            ("cmd.exe", vec!["/c", command_str]),
        ];

        #[cfg(not(target_os = "windows"))]
        let shell_commands = [("sh", vec!["-c", command_str])];

        // Try each shell until one works
        let mut last_error = String::new();
        for (shell, args) in &shell_commands {
            match Command::new(shell)
                .args(args)
                .current_dir(&self.working_dir)
                .stdin(Stdio::null())
                .output()
            {
                Ok(output_result) => {
                    let duration = start.elapsed();
                    let exit_code = output_result.status.code().unwrap_or(-1);
                    let success = output_result.status.success();

                    let stdout = String::from_utf8_lossy(&output_result.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output_result.stderr).to_string();

                    merlin_deps::tracing::debug!(
                        "Command completed with exit code {} in {:?}",
                        exit_code,
                        duration
                    );

                    if !success {
                        merlin_deps::tracing::debug!("Command stderr: {}", stderr);
                    }

                    return Ok(CommandResult {
                        exit_code,
                        stdout,
                        stderr,
                        duration,
                        success,
                    });
                }
                Err(err) => {
                    last_error = format!("Failed with {shell}: {err}");
                }
            }
        }

        // All shells failed
        Err(RoutingError::Other(format!(
            "Failed to execute command '{command_str}': {last_error}"
        )))
    }
}

#[cfg(test)]
mod tests {}
