use std::process::Command;

use async_trait::async_trait;
use merlin_deps::serde_json::from_value;
use tokio::task::spawn_blocking;

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Tool that executes shell commands asynchronously using `sh`.
///
/// Uses `tokio::task::spawn_blocking` with `std::process::Command` to avoid
/// blocking the Tokio runtime. Commands are executed via `sh -c` for POSIX
/// compliance and optimal performance across all platforms.
///
/// ## Performance Note
/// On Windows (MINGW64/Git Bash), `bash` has ~6 second startup overhead when
/// spawned from Rust's `std::process::Command`, while `sh` has only ~55ms.
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
        merlin_deps::tracing::debug!("Executing shell command: {}", command_str);

        let command = command.to_owned();

        // Use spawn_blocking to run blocking I/O on Tokio's global thread pool
        // This works even when called from a current_thread runtime
        merlin_deps::tracing::debug!("About to call spawn_blocking for command: {}", command);
        let output = spawn_blocking(move || {
            merlin_deps::tracing::debug!("Inside spawn_blocking, about to run command");
            // Use sh for better performance on all platforms
            // On Windows (MINGW64/Git Bash), bash has ~6s startup overhead when spawned
            // from Rust's std::process::Command, while sh has only ~55ms overhead
            // sh is POSIX compliant and sufficient for all our use cases
            let bash_cmd = "sh";

            let result = Command::new(bash_cmd)
                .arg("-c")
                .arg(&command)
                .env("LANG", "C.UTF-8") // Ensure consistent locale
                .output();

            merlin_deps::tracing::debug!(
                "Command finished in spawn_blocking with result: {:?}",
                result.as_ref().map(|output| output.status)
            );
            result
        })
        .await
        .map_err(|err| ToolError::ExecutionFailed(format!("Task join failed: {err}")))?
        .map_err(|err| {
            ToolError::ExecutionFailed(format!(
                "Command execution failed (is sh available in PATH?): {err}"
            ))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let success = output.status.success();
        let message = if success {
            format!("Command executed successfully (exit code: {exit_code})")
        } else {
            merlin_deps::tracing::warn!(
                "Command failed: {} | Exit code: {} | Stdout: {} | Stderr: {}",
                command_str,
                exit_code,
                stdout,
                stderr
            );
            format!("Command failed with exit code: {exit_code}")
        };

        let data = merlin_deps::serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        });

        merlin_deps::tracing::debug!(
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

    fn typescript_signature(&self) -> &'static str {
        "/**\n\
         * Execute a shell command using bash. \n\
         * Usage: bash(\"command string\")\n\
         * Example: bash(\"ls -la\") or bash(\"grep -r TODO . --exclude-dir={.git,target,node_modules}\")\n\
         * \n\
         * IMPORTANT for grep/search commands:\n\
         * - Always exclude build artifacts: --exclude-dir={.git,target,node_modules,dist,build}\n\
         * - Exclude binary files: --binary-files=without-match or -I\n\
         * - Filter by file type using multiple --include flags (one per extension)\n\
         * - Example: bash(\"grep -r -I 'pattern' . --include='*.rs' --exclude-dir={.git,target}\")\n\
         * - Example: bash(\"grep -r 'TODO' . --include='*.rs' --include='*.toml' --exclude-dir={.git,target}\")\n\
         */\n\
         declare function bash(command: string): Promise<{ stdout: string; stderr: string; exit_code: number }>;"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        // Support both direct string parameter (from TypeScript runtime)
        // and object parameter (from agent/routing system)
        let command: String = if input.params.is_string() {
            // Direct string from TypeScript: bash("command")
            from_value(input.params)?
        } else {
            // Object from agent: { "command": "..." }
            from_value(
                input
                    .params
                    .get("command")
                    .ok_or_else(|| {
                        ToolError::InvalidInput("Missing 'command' parameter".to_owned())
                    })?
                    .clone(),
            )?
        };
        self.execute_command(&command).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_deps::anyhow::Result;

    /// Tests basic bash command execution with successful output.
    ///
    /// # Errors
    /// Returns an error if command execution fails or output parsing fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_bash_tool_simple_command() -> Result<()> {
        let tool = BashTool;
        let input = ToolInput {
            params: merlin_deps::serde_json::json!("echo 'hello'"),
        };

        let output = tool.execute(input).await?;
        assert!(output.success);
        assert!(output.message.contains("successfully"));
        assert!(output.data.is_some());

        let data = output
            .data
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected data"))?;
        let stdout = data["stdout"]
            .as_str()
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected stdout as string"))?;
        assert!(stdout.contains("hello"));
        Ok(())
    }

    /// Tests bash command execution with non-zero exit code.
    ///
    /// # Errors
    /// Returns an error if command execution fails unexpectedly.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_bash_tool_command_failure() -> Result<()> {
        let tool = BashTool;
        let input = ToolInput {
            params: merlin_deps::serde_json::json!("exit 1"),
        };

        let output = tool.execute(input).await?;
        assert!(!output.success);
        assert!(output.message.contains("failed"));
        let data = output
            .data
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected data"))?;
        assert_eq!(data["exit_code"], 1);
        Ok(())
    }

    /// Tests bash command execution with object-style parameters.
    ///
    /// # Errors
    /// Returns an error if command execution or output parsing fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_bash_tool_with_object_params() -> Result<()> {
        let tool = BashTool;
        let input = ToolInput {
            params: merlin_deps::serde_json::json!({"command": "echo test"}),
        };

        let output = tool.execute(input).await?;
        assert!(output.success);
        let data = output
            .data
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected data"))?;
        let stdout = data["stdout"]
            .as_str()
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected stdout as string"))?;
        assert!(stdout.contains("test"));
        Ok(())
    }

    /// Tests bash tool error handling when required command parameter is missing.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_bash_tool_missing_command_param() {
        let tool = BashTool;
        let input = ToolInput {
            params: merlin_deps::serde_json::json!({"wrong": "param"}),
        };

        let result = tool.execute(input).await;
        assert!(result.is_err(), "Should fail with missing command param");
    }

    /// Tests bash tool name and TypeScript signature generation.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_bash_tool_name_and_signature() {
        let tool = BashTool;
        assert_eq!(tool.name(), "bash");
        assert!(!tool.typescript_signature().is_empty());
    }

    // Test removed: test_git_bash_env_var was causing race conditions when run in parallel
    // with other tests by mutating global environment state. The GIT_BASH env var behavior
    // is already indirectly tested by the other bash tests when bash is available.
}
