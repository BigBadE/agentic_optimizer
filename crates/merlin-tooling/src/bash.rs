use std::process::Command;

use async_trait::async_trait;
use serde_json::from_value;
use tokio::task::spawn_blocking;

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Tool that executes shell commands asynchronously.
///
/// Uses `tokio::process::Command` for true async execution that integrates
/// seamlessly with Deno Core's event loop and Tokio's async runtime.
#[derive(Debug, Clone, Copy)]
pub struct BashTool;

impl BashTool {
    /// Execute the provided shell command using blocking I/O in `spawn_blocking`.
    ///
    /// Uses `tokio::task::spawn_blocking` with `std::process::Command` to avoid blocking
    /// the `current_thread` runtime. The `spawn_blocking` uses Tokio's global thread pool.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` when the command fails to spawn or when reading the output fails.
    async fn execute_command(&self, command: &str) -> ToolResult<ToolOutput> {
        let command_str = command;
        tracing::debug!("Executing shell command: {}", command_str);

        let command = command.to_owned();

        // Use spawn_blocking to run blocking I/O on Tokio's global thread pool
        // This works even when called from a current_thread runtime
        tracing::debug!("About to call spawn_blocking");
        let output = spawn_blocking(move || {
            tracing::debug!("Inside spawn_blocking, about to run command");
            // Use bash on all platforms for consistency
            // On Windows, this requires Git Bash or similar to be installed
            let result = Command::new("bash").arg("-c").arg(&command).output();

            tracing::debug!("Command finished in spawn_blocking");
            result
        })
        .await
        .map_err(|err| ToolError::ExecutionFailed(format!("Task join failed: {err}")))?
        .map_err(|err| ToolError::ExecutionFailed(format!("Command execution failed: {err}")))?;

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

        tracing::debug!(
            "Bash command completed with exit code {}: {}",
            exit_code,
            command_str
        );

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
        "Execute a shell command using bash. Takes a single string parameter containing the command to execute. On Windows, requires Git Bash or similar to be installed."
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let command: String = from_value(input.params)?;
        self.execute_command(&command).await
    }
}
