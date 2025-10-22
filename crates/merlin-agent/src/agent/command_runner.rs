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

        tracing::debug!(
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

                    tracing::debug!(
                        "Command completed with exit code {} in {:?}",
                        exit_code,
                        duration
                    );

                    if !success {
                        tracing::debug!("Command stderr: {}", stderr);
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
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_command_runner_success() {
        let runner = CommandRunner::new(PathBuf::from("."));
        let result = runner.run("echo hello").unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
    }

    #[test]
    fn test_command_runner_failure() {
        let runner = CommandRunner::new(PathBuf::from("."));
        // Use a command that should fail on all platforms
        let result = runner.run("cargo this-command-does-not-exist");

        // The command should either fail to spawn or return non-zero exit code
        if let Ok(cmd_result) = result {
            assert!(!cmd_result.success);
            assert_ne!(cmd_result.exit_code, 0);
        }
    }

    #[test]
    fn test_command_runner_with_args() {
        let runner = CommandRunner::new(PathBuf::from("."));
        let result = runner.run("echo hello world").unwrap();

        assert!(result.success);
        assert!(result.stdout.contains("hello"));
        assert!(result.stdout.contains("world"));
    }

    #[test]
    fn test_command_result_error_message() {
        let result = CommandResult {
            exit_code: 1,
            stdout: "some output".to_owned(),
            stderr: "error occurred".to_owned(),
            duration: Duration::from_secs(1),
            success: false,
        };

        let error_msg = result.error_message();
        assert!(error_msg.contains("error occurred"));
        assert!(error_msg.contains("some output"));
    }

    #[test]
    fn test_command_result_error_message_no_stderr() {
        let result = CommandResult {
            exit_code: 1,
            stdout: "some output".to_owned(),
            stderr: String::new(),
            duration: Duration::from_secs(1),
            success: false,
        };

        let error_msg = result.error_message();
        assert_eq!(error_msg, "some output");
    }

    #[test]
    fn test_command_runner_with_pipes() {
        let runner = CommandRunner::new(PathBuf::from("."));
        // Test command with pipes (requires shell)
        let result = runner.run("echo test | grep test").unwrap();

        assert!(result.success);
        assert!(result.stdout.contains("test"));
    }

    #[test]
    fn test_command_runner_with_redirects() {
        let runner = CommandRunner::new(PathBuf::from("."));
        // Test command with redirects (requires shell)
        let result = runner.run("echo stdout >&2");

        // Should complete even if output goes to stderr
        result.unwrap();
    }

    #[test]
    fn test_command_runner_with_env_vars() {
        let runner = CommandRunner::new(PathBuf::from("."));
        // Test command that uses environment variables
        let result = runner.run("echo $HOME").unwrap();

        assert!(result.success);
        // Should have some output (either expanded var or literal)
        assert!(!result.stdout.trim().is_empty());
    }

    #[test]
    fn test_command_runner_multiline_command() {
        let runner = CommandRunner::new(PathBuf::from("."));
        // Test multiline command with semicolon
        let result = runner.run("echo first ; echo second").unwrap();

        assert!(result.success);
        // Should contain both outputs
        assert!(result.stdout.contains("first"));
        assert!(result.stdout.contains("second"));
    }

    #[test]
    fn test_command_runner_with_special_chars() {
        let runner = CommandRunner::new(PathBuf::from("."));
        // Test command with special characters that need shell handling
        let result = runner.run("echo 'hello world'").unwrap();

        assert!(result.success);
        assert!(result.stdout.contains("hello world"));
    }

    #[test]
    fn test_command_runner_empty_string() {
        let runner = CommandRunner::new(PathBuf::from("."));
        let result = runner.run("");

        // Should error on empty command
        result.unwrap_err();
    }

    #[test]
    fn test_command_runner_whitespace_only() {
        let runner = CommandRunner::new(PathBuf::from("."));
        let result = runner.run("   \t\n   ");

        // Should error on whitespace-only command
        result.unwrap_err();
    }

    #[test]
    fn test_command_runner_working_directory() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().expect("Failed to create temp dir");
        let test_file = temp.path().join("test.txt");
        fs::write(&test_file, "content").expect("Failed to write test file");

        let runner = CommandRunner::new(temp.path().to_path_buf());

        // List files in the working directory
        #[cfg(target_os = "windows")]
        let result = runner.run("dir").unwrap();

        #[cfg(not(target_os = "windows"))]
        let result = runner.run("ls").unwrap();

        assert!(result.success);
        // Should list the test file
        assert!(result.stdout.contains("test.txt"));
    }

    #[test]
    fn test_command_runner_exit_codes() {
        let runner = CommandRunner::new(PathBuf::from("."));

        // Success case
        let result = runner.run("echo success").unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.success);

        // Failure case - try to cd to nonexistent directory
        let fail_result = runner
            .run("cd /nonexistent/path/that/does/not/exist")
            .unwrap();
        assert_ne!(fail_result.exit_code, 0);
        assert!(!fail_result.success);
    }

    #[test]
    fn test_command_runner_duration_tracking() {
        let runner = CommandRunner::new(PathBuf::from("."));
        let result = runner.run("echo test").unwrap();

        // Duration should be set and reasonable
        assert!(result.duration.as_millis() < 5000); // Should complete in under 5 seconds
    }

    #[test]
    fn test_command_result_clone() {
        let result = CommandResult {
            exit_code: 0,
            stdout: "output".to_owned(),
            stderr: "error".to_owned(),
            duration: Duration::from_millis(100),
            success: true,
        };

        let cloned = result.clone();
        assert_eq!(cloned.exit_code, result.exit_code);
        assert_eq!(cloned.stdout, result.stdout);
        assert_eq!(cloned.stderr, result.stderr);
        assert_eq!(cloned.success, result.success);
    }

    #[test]
    fn test_command_result_debug() {
        let result = CommandResult {
            exit_code: 1,
            stdout: "test output".to_owned(),
            stderr: "test error".to_owned(),
            duration: Duration::from_secs(1),
            success: false,
        };

        let debug_str = format!("{result:?}");
        assert!(debug_str.contains("exit_code"));
        assert!(debug_str.contains("stdout"));
    }
}
