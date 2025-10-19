use std::time::Duration;

use async_trait::async_trait;
use serde_json::from_value;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, warn};

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Default timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Tool that executes shell commands with an optional timeout.
pub struct BashTool;

impl BashTool {
    /// Execute the provided shell command with default timeout.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` when the command times out, fails to spawn, or when reading the output fails.
    async fn execute_command(&self, command: &str) -> ToolResult<ToolOutput> {
        debug!("Executing command: {}", command);

        // Always use bash with -c flag (works on Unix and Windows with Git Bash/WSL)
        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(command);

        let timeout_duration = Duration::from_secs(DEFAULT_TIMEOUT_SECS);

        let output = if let Ok(result) = timeout(timeout_duration, cmd.output()).await {
            result?
        } else {
            warn!("Command timed out after {DEFAULT_TIMEOUT_SECS} seconds");
            return Err(ToolError::ExecutionFailed(format!(
                "Command timed out after {DEFAULT_TIMEOUT_SECS} seconds"
            )));
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let success = output.status.success();
        let message = if success {
            format!("Command executed successfully (exit code: {exit_code})")
        } else {
            format!("Command failed with exit code: {exit_code}")
        };

        let data = serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        });

        if success {
            Ok(ToolOutput::success_with_data(message, data))
        } else {
            Ok(ToolOutput {
                success: false,
                message,
                data: Some(data),
            })
        }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command (bash on Unix, PowerShell on Windows). Takes a single string parameter containing the command to execute."
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let command: String = from_value(input.params)?;
        self.execute_command(&command).await
    }
}
