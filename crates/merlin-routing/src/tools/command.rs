use super::Tool;
use crate::{Result, RoutingError};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Tool for running shell commands
pub struct RunCommandTool {
    workspace_root: PathBuf,
    allowed_commands: Vec<String>,
}

impl RunCommandTool {
    /// Create a new run command tool
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            // Default whitelist of safe commands
            allowed_commands: vec![
                "cargo".to_owned(),
                "git".to_owned(),
                "rustc".to_owned(),
                "rustfmt".to_owned(),
                "clippy".to_owned(),
            ],
        }
    }

    /// Set allowed commands
    #[must_use]
    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands;
        self
    }
}

#[async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &'static str {
        "run_command"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command in the workspace directory. Only whitelisted commands are allowed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute (e.g., 'cargo check', 'git status')"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let command_str = args
            .get("command")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'command' argument".to_owned()))?;

        // Parse command into parts
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(RoutingError::Other("Empty command".to_owned()));
        }

        let program = parts[0];
        let args_list = &parts[1..];

        // Security check: only allow whitelisted commands
        if !self
            .allowed_commands
            .iter()
            .any(|allowed| allowed == program)
        {
            return Err(RoutingError::Other(format!(
                "Command '{}' is not in the whitelist. Allowed: {:?}",
                program, self.allowed_commands
            )));
        }

        // Execute command
        let output = Command::new(program)
            .args(args_list)
            .current_dir(&self.workspace_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|error| RoutingError::Other(format!("Failed to execute command: {error}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(json!({
            "command": command_str,
            "exit_code": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "success": output.status.success()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;
    use tempfile::TempDir;

    #[tokio::test]
    /// # Errors
    /// Returns an error if tool execution fails in the test harness.
    ///
    /// # Panics
    /// Panics if returned JSON is missing expected fields.
    async fn test_run_command_tool() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let tool = RunCommandTool::new(temp_dir.path().to_path_buf());

        // Test a safe command (cargo --version should work)
        let result = tool
            .execute(json!({ "command": "cargo --version" }))
            .await?;

        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["success"].as_bool(), Some(true));
        if let Some(stdout_str) = result["stdout"].as_str() {
            assert!(stdout_str.contains("cargo"));
        } else {
            panic!("stdout missing or not a string");
        }
        Ok(())
    }

    #[tokio::test]
    /// # Errors
    /// Returns an error if `TempDir` creation fails in the test harness.
    ///
    /// # Panics
    /// Panics if a non-whitelisted command unexpectedly succeeds.
    async fn test_command_whitelist() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let tool = RunCommandTool::new(temp_dir.path().to_path_buf());

        // Test a non-whitelisted command
        let result = tool.execute(json!({ "command": "rm -rf /" })).await;
        match result {
            Ok(_) => panic!("expected whitelist failure"),
            Err(error) => assert!(error.to_string().contains("not in the whitelist")),
        }
        Ok(())
    }

    #[tokio::test]
    /// # Errors
    /// Returns an error if tool execution fails in the test harness.
    ///
    /// # Panics
    /// Panics if returned JSON is missing expected fields.
    async fn test_custom_whitelist() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Use a command that exists on all platforms
        let tool = RunCommandTool::new(temp_dir.path().to_path_buf())
            .with_allowed_commands(vec!["cargo".to_owned()]);

        let result = tool
            .execute(json!({ "command": "cargo --version" }))
            .await?;

        assert_eq!(result["exit_code"], 0);
        if let Some(stdout_str) = result["stdout"].as_str() {
            assert!(stdout_str.contains("cargo"));
        } else {
            panic!("stdout missing or not a string");
        }
        Ok(())
    }
}
