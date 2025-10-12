//! TypeScript/JavaScript execution tool for natural LLM tool calling.
//!
//! This module provides a tool that allows LLMs to write TypeScript/JavaScript
//! code to orchestrate multiple tool calls with natural control flow.

use async_trait::async_trait;
use merlin_tools::{
    Tool as MerlinTool, ToolInput, ToolOutput, ToolResult, TypeScriptRuntime as QuickJsRuntime,
};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::{Result, RoutingError, Tool};

/// Tool that executes TypeScript/JavaScript code with access to other tools
pub struct TypeScriptTool {
    runtime: Arc<QuickJsRuntime>,
}

impl TypeScriptTool {
    /// Create a new TypeScript tool with the given tools available
    pub fn new(tools: Vec<Arc<dyn Tool>>) -> Self {
        let mut runtime = QuickJsRuntime::new();

        // Wrap each routing tool as a merlin_tools::Tool
        for tool in tools {
            let wrapped = ToolWrapper::new(tool);
            runtime.register_tool(Arc::new(wrapped));
        }

        Self {
            runtime: Arc::new(runtime),
        }
    }

    /// Get the runtime for direct access (useful for testing)
    pub fn runtime(&self) -> &Arc<QuickJsRuntime> {
        &self.runtime
    }
}

#[async_trait]
impl Tool for TypeScriptTool {
    fn name(&self) -> &'static str {
        "execute_typescript"
    }

    fn description(&self) -> &'static str {
        "Execute TypeScript/JavaScript code to orchestrate multiple tool calls. \
         Write natural code with loops, conditionals, and async/await. \
         All registered tools are available as async functions."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "TypeScript/JavaScript code to execute. Use await for tool calls."
                }
            },
            "required": ["code"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let code = args
            .get("code")
            .and_then(Value::as_str)
            .ok_or_else(|| RoutingError::Other("Missing 'code' parameter".to_owned()))?;

        self.runtime
            .execute(code)
            .map_err(|err| RoutingError::Other(format!("TypeScript execution failed: {err}")))
    }
}

/// Wrapper that adapts a routing Tool to a `merlin_tools::Tool`
struct ToolWrapper {
    tool: Arc<dyn Tool>,
}

impl ToolWrapper {
    fn new(tool: Arc<dyn Tool>) -> Self {
        Self { tool }
    }
}

#[async_trait]
impl MerlinTool for ToolWrapper {
    fn name(&self) -> &'static str {
        // This is a limitation - we need to leak the string to get 'static lifetime
        // In practice, tool names are static strings anyway
        Box::leak(self.tool.name().to_owned().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        // Same limitation as above
        Box::leak(self.tool.description().to_owned().into_boxed_str())
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        // Execute the routing tool
        match self.tool.execute(input.params).await {
            Ok(result) => Ok(ToolOutput::success_with_data(
                "Tool execution completed",
                result,
            )),
            Err(err) => Ok(ToolOutput::error(format!("Tool execution failed: {err}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ListFilesTool, ReadFileTool, WriteFileTool};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_typescript_tool_creation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(ReadFileTool::new(workspace.clone())),
            Arc::new(WriteFileTool::new(workspace.clone())),
            Arc::new(ListFilesTool::new(workspace)),
        ];

        let ts_tool = TypeScriptTool::new(tools);
        assert_eq!(ts_tool.name(), "execute_typescript");
    }

    #[tokio::test]
    async fn test_simple_javascript_execution() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(ReadFileTool::new(workspace))];

        let ts_tool = TypeScriptTool::new(tools);

        let code = "const x = 1 + 1; x * 2";
        let result = ts_tool.execute(json!({ "code": code })).await;

        result.unwrap();
    }

    #[tokio::test]
    async fn test_typescript_tool_schema() {
        let ts_tool = TypeScriptTool::new(vec![]);
        let schema = ts_tool.parameters_schema();

        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }
}
