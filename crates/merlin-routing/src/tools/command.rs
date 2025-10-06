use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use crate::{Result, RoutingError};
use super::Tool;

/// Tool for running shell commands
pub struct RunCommandTool {
    workspace_root: PathBuf,
    allowed_commands: Vec<String>,
}

impl RunCommandTool {
    #[must_use] 
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
        let command_str = args.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'command' argument".to_owned()))?;
        
        // Parse command into parts
        let parts: Vec<&str> = command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(RoutingError::Other("Empty command".to_owned()));
        }
        
        let program = parts[0];
        let args_list = &parts[1..];
        
        // Security check: only allow whitelisted commands
        if !self.allowed_commands.iter().any(|allowed| allowed == program) {
            return Err(RoutingError::Other(
                format!("Command '{}' is not in the whitelist. Allowed: {:?}", 
                    program, self.allowed_commands)
            ));
        }
        
        // Execute command
        let output = Command::new(program)
            .args(args_list)
            .current_dir(&self.workspace_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| RoutingError::Other(format!("Failed to execute command: {e}")))?;
        
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
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_run_command_tool() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = RunCommandTool::new(temp_dir.path().to_path_buf());
        
        // Test a safe command (cargo --version should work)
        let result = tool.execute(json!({ "command": "cargo --version" })).await.unwrap();
        
        assert_eq!(result["exit_code"], 0);
        assert!(result["success"].as_bool().unwrap());
        assert!(result["stdout"].as_str().unwrap().contains("cargo"));
    }

    #[tokio::test]
    async fn test_command_whitelist() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = RunCommandTool::new(temp_dir.path().to_path_buf());
        
        // Test a non-whitelisted command
        let result = tool.execute(json!({ "command": "rm -rf /" })).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in the whitelist"));
    }

    #[tokio::test]
    async fn test_custom_whitelist() {
        let temp_dir = TempDir::new().unwrap();
        
        // Use a command that exists on all platforms
        let tool = RunCommandTool::new(temp_dir.path().to_path_buf())
            .with_allowed_commands(vec!["cargo".to_owned()]);
        
        let result = tool.execute(json!({ "command": "cargo --version" })).await.unwrap();
        
        assert_eq!(result["exit_code"], 0);
        assert!(result["stdout"].as_str().unwrap().contains("cargo"));
    }
}
