use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::from_value;
use tokio::{process::Command, time::timeout};
use tracing::{debug, warn};

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Parameters describing a shell command to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BashParams {
    /// Command line that will be executed by the shell.
    command: String,
    #[serde(default)]
    /// Optional working directory to run the command inside.
    working_dir: Option<String>,
    #[serde(default = "default_timeout")]
    /// Maximum number of seconds the command is allowed to run before timing out.
    timeout_secs: u64,
}

/// Default timeout applied when none is specified in [`BashParams`].
fn default_timeout() -> u64 {
    30
}

/// Tool that executes shell commands with an optional timeout.
pub struct BashTool;

impl BashTool {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Execute the provided shell command, enforcing the configured timeout.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` when the command times out, fails to spawn, or when reading the output fails.
    async fn execute_command(&self, params: BashParams) -> ToolResult<ToolOutput> {
        debug!("Executing command: {}", params.command);

        let shell = if cfg!(target_os = "windows") {
            "powershell"
        } else {
            "bash"
        };

        let shell_flag = if cfg!(target_os = "windows") {
            "-Command"
        } else {
            "-c"
        };

        let mut command = Command::new(shell);
        command.arg(shell_flag).arg(&params.command);

        if let Some(working_dir) = &params.working_dir {
            command.current_dir(working_dir);
        }

        let timeout = Duration::from_secs(params.timeout_secs);

        let output = match timeout(timeout, command.output()).await {
            Ok(result) => result?,
            Err(_) => {
                warn!("Command timed out after {} seconds", params.timeout_secs);
                return Err(ToolError::ExecutionFailed(format!(
                    "Command timed out after {} seconds",
                    params.timeout_secs
                )));
            }
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
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command (bash on Unix, PowerShell on Windows). \
         Parameters: command (string), working_dir (string, optional), \
         timeout_secs (number, optional, default: 30)"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let params: BashParams = from_value(input.params)?;
        self.execute_command(params).await
    }
}
