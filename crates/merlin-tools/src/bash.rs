use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::time;
use tracing::{debug, warn};

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BashParams {
    command: String,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
}

const fn default_timeout() -> u64 {
    30
}

pub struct BashTool;

impl BashTool {
    pub const fn new() -> Self {
        Self
    }

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

        let output = if let Ok(result) = time::timeout(timeout, command.output()).await {
            result?
        } else {
            warn!("Command timed out after {} seconds", params.timeout_secs);
            return Err(ToolError::ExecutionFailed(format!(
                "Command timed out after {} seconds",
                params.timeout_secs
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
        let params: BashParams = serde_json::from_value(input.params)?;
        self.execute_command(params).await
    }
}
