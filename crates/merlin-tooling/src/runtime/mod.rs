//! TypeScript/JavaScript runtime using Boa engine.

pub mod bulk_extraction;
mod conversion;
mod handle;
mod persistent;
mod promise;
mod tool_registration;
mod typescript;

pub use handle::JsValueHandle;
pub use persistent::PersistentTypeScriptRuntime;

use std::collections::HashMap;
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
        use std::time::Instant;
        let exec_start = Instant::now();
        tracing::debug!("TypeScriptRuntime::execute called");

        let wrap_start = Instant::now();
        let wrapped_code = wrap_code(code);
        let wrap_time = wrap_start.elapsed();

        let tools_clone = self.tools.clone();
        let timeout = self.timeout;

        // Execute with timeout
        let spawn_start = Instant::now();
        let result = time::timeout(timeout, async move {
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
        })?;

        let total_time = exec_start.elapsed();
        tracing::debug!(
            total_time_secs = total_time.as_secs_f64(),
            wrap_time_secs = wrap_time.as_secs_f64(),
            spawn_exec_time_secs = spawn_start.elapsed().as_secs_f64(),
            "[TS] timing breakdown"
        );

        result
    }

    /// Execute JavaScript synchronously in Boa
    ///
    /// # Errors
    /// Returns error if execution fails
    fn execute_sync(code: &str, tools: &HashMap<String, Arc<dyn Tool>>) -> ToolResult<Value> {
        use std::time::Instant;
        let sync_start = Instant::now();
        tracing::debug!("Creating Boa context");

        // Create context - Boa 0.21 handles job queue internally
        let ctx_start = Instant::now();
        let mut context = Context::default();
        let ctx_time = ctx_start.elapsed();

        // Register tools as global functions
        let reg_start = Instant::now();
        register_tool_functions(&mut context, tools)?;
        let reg_time = reg_start.elapsed();

        tracing::debug!("Executing JavaScript code");

        // Execute the code
        let eval_start = Instant::now();
        let result = context
            .eval(Source::from_bytes(code))
            .map_err(|err| ToolError::ExecutionFailed(format!("JavaScript error: {err}")))?;
        let eval_time = eval_start.elapsed();

        // Run all pending jobs (resolve Promises)
        tracing::debug!("Running job queue to resolve Promises");
        let job_start = Instant::now();
        let _result = context.run_jobs();
        let job_time = job_start.elapsed();

        // Extract Promise value if result is a Promise
        let extract_start = Instant::now();
        let final_result = extract_promise_if_needed(result, &mut context)?;
        let extract_time = extract_start.elapsed();

        // Convert result to JSON
        let conv_start = Instant::now();
        let json_result = js_value_to_json(&final_result, &mut context)?;
        let conv_time = conv_start.elapsed();

        tracing::debug!(
            total_time_secs = sync_start.elapsed().as_secs_f64(),
            ctx_time_secs = ctx_time.as_secs_f64(),
            reg_time_secs = reg_time.as_secs_f64(),
            eval_time_secs = eval_time.as_secs_f64(),
            job_time_secs = job_time.as_secs_f64(),
            extract_time_secs = extract_time.as_secs_f64(),
            conv_time_secs = conv_time.as_secs_f64(),
            "[TS_SYNC] timing breakdown"
        );

        Ok(json_result)
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
    use anyhow::Result;

    /// Tests TypeScript runtime initialization.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = TypeScriptRuntime::new();
        assert_eq!(runtime.timeout, MAX_EXECUTION_TIME);
    }

    /// Tests simple JavaScript expression execution.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_simple_javascript_execution() {
        let runtime = TypeScriptRuntime::new();
        let code = "1 + 1";
        let result = runtime.execute(code).await;
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    /// Tests variable declaration and computation.
    ///
    /// # Errors
    /// Returns an error if code execution fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_variable_declaration() -> Result<()> {
        let runtime = TypeScriptRuntime::new();
        let code = "const x = 42; x * 2";
        let result = runtime.execute(code).await?;
        assert_eq!(result, serde_json::json!(84));
        Ok(())
    }
}
