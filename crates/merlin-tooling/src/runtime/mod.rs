//! TypeScript/JavaScript runtime using Boa engine.

mod conversion;
mod promise;
mod tool_registration;
mod typescript;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

use boa_engine::{Context, Source};
use serde_json::Value;
use tokio::task::spawn_blocking;
use tokio::time;

use crate::{Tool, ToolError, ToolResult};

// Re-export for internal use
pub use conversion::js_value_to_json;
use promise::extract_promise_if_needed;
use tool_registration::register_tool_functions;
use typescript::wrap_code;

/// Maximum execution time for JavaScript code
const MAX_EXECUTION_TIME: Duration = Duration::from_secs(30);

/// JavaScript runtime for executing tool calls (extracted from TypeScript code blocks)
///
/// Note: Currently executes JavaScript code. TypeScript type annotations should be
/// stripped before passing code to this runtime. Future enhancement will add automatic
/// TypeScript-to-JavaScript transformation.
pub struct TypeScriptRuntime {
    /// Registry of available tools
    tools: HashMap<String, Arc<dyn Tool>>,
    /// Execution timeout
    timeout: Duration,
}

impl TypeScriptRuntime {
    /// Create a new TypeScript runtime with default limits
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            timeout: MAX_EXECUTION_TIME,
        }
    }

    /// Register a tool that can be called from JavaScript
    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_owned(), tool);
    }

    /// Set the execution timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the memory limit (no-op for Boa, kept for API compatibility)
    #[must_use]
    pub fn with_memory_limit(self, _limit: usize) -> Self {
        self
    }

    /// Execute JavaScript/TypeScript code that calls tools
    ///
    /// # Errors
    /// Returns an error if:
    /// - The code cannot be parsed or executed
    /// - Tool calls fail to execute
    /// - Execution times out
    pub async fn execute(&self, code: &str) -> ToolResult<Value> {
        tracing::debug!("TypeScriptRuntime::execute called");

        let wrapped_code = wrap_code(code);
        let tools_clone = self.tools.clone();
        let timeout = self.timeout;

        // Execute with timeout
        time::timeout(timeout, async move {
            // Run in spawn_blocking since Boa context is !Send
            spawn_blocking(move || Self::execute_sync(&wrapped_code, &tools_clone))
                .await
                .map_err(|err| ToolError::ExecutionFailed(format!("Task join failed: {err}")))?
        })
        .await
        .map_err(|_| {
            ToolError::ExecutionFailed(format!(
                "Execution timed out after {} seconds",
                timeout.as_secs()
            ))
        })?
    }

    /// Execute JavaScript synchronously in Boa
    ///
    /// # Errors
    /// Returns error if execution fails
    fn execute_sync(code: &str, tools: &HashMap<String, Arc<dyn Tool>>) -> ToolResult<Value> {
        tracing::debug!("Creating Boa context");

        // Create context - Boa 0.21 handles job queue internally
        let mut context = Context::default();

        // Register tools as global functions
        register_tool_functions(&mut context, tools)?;

        tracing::debug!("Executing JavaScript code");

        // Execute the code
        let result = context
            .eval(Source::from_bytes(code))
            .map_err(|err| ToolError::ExecutionFailed(format!("JavaScript error: {err}")))?;

        // Run all pending jobs (resolve Promises)
        tracing::debug!("Running job queue to resolve Promises");
        let _result = context.run_jobs();

        // Extract Promise value if result is a Promise
        let final_result = extract_promise_if_needed(result, &mut context)?;

        // Convert result to JSON
        js_value_to_json(&final_result, &mut context)
    }

    /// Generate TypeScript type definitions for registered tools
    ///
    /// # Errors
    /// Returns an error if formatting fails (should never happen in practice)
    pub fn generate_type_definitions(&self) -> Result<String, ToolError> {
        let mut defs = String::from("// Available tool functions\n\n");

        for tool in self.tools.values() {
            writeln!(defs, "/**\n * {}\n */", tool.description()).map_err(|err| {
                ToolError::ExecutionFailed(format!("Failed to write type definitions: {err}"))
            })?;
            writeln!(
                defs,
                "declare function {}(params: any): any;\n",
                tool.name()
            )
            .map_err(|err| {
                ToolError::ExecutionFailed(format!("Failed to write type definitions: {err}"))
            })?;
        }

        Ok(defs)
    }
}

impl Default for TypeScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolInput, ToolOutput};
    use async_trait::async_trait;

    // Mock tool for testing
    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> &'static str {
            "Echoes back the input"
        }

        fn typescript_signature(&self) -> &'static str {
            "/**\n * Echoes back the input\n */\ndeclare function echo(params: any): Promise<any>;"
        }

        async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
            Ok(ToolOutput::success_with_data("echoed", input.params))
        }
    }

    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = TypeScriptRuntime::new();
        assert_eq!(runtime.timeout, MAX_EXECUTION_TIME);
    }

    #[tokio::test]
    async fn test_simple_javascript_execution() {
        let runtime = TypeScriptRuntime::new();
        let code = "1 + 1";
        let result = runtime.execute(code).await;
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_variable_declaration() {
        let runtime = TypeScriptRuntime::new();
        let code = "const x = 42; x * 2";
        let result = runtime.execute(code).await;
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        assert_eq!(result.unwrap(), serde_json::json!(84));
    }
}
