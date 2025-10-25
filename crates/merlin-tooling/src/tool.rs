use std::io::Error as IoError;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Error as SerdeJsonError, Value};
use thiserror::Error;

/// Errors that can occur during tool execution.
#[derive(Debug, Error)]
pub enum ToolError {
    /// An I/O operation failed.
    #[error("IO error: {0}")]
    Io(#[from] IoError),

    /// The provided input parameters were invalid or malformed.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// The tool failed to execute its operation.
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Failed to serialize or deserialize data.
    #[error("Serialization error: {0}")]
    Serialization(#[from] SerdeJsonError),
}

/// Result type for tool operations.
pub type ToolResult<T> = Result<T, ToolError>;

/// Input parameters provided to a tool for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    /// JSON value containing the tool-specific parameters.
    pub params: Value,
}

/// Output returned by a tool after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Whether the tool execution succeeded.
    pub success: bool,
    /// Human-readable message describing the result.
    pub message: String,
    /// Optional JSON data containing tool-specific output.
    pub data: Option<Value>,
}

impl ToolOutput {
    /// Creates a successful output with the given message and no data.
    pub fn success<T: Into<String>>(message: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    /// Creates a successful output with the given message and associated data.
    pub fn success_with_data<T: Into<String>>(message: T, data: Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Creates an error output with the given message.
    pub fn error<T: Into<String>>(message: T) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

/// Trait for implementing executable tools that can be invoked by the system.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique identifier for this tool.
    fn name(&self) -> &'static str;

    /// Returns a human-readable description of what this tool does and its parameters.
    fn description(&self) -> &'static str;

    /// Returns the TypeScript function signature for this tool.
    ///
    /// This signature is used by the TypeScript runtime to provide proper type information.
    /// The signature should include:
    /// - `JSDoc` comment with the tool description
    /// - `declare function` with proper parameter and return types
    ///
    /// # Examples
    ///
    /// ```text
    /// /**
    ///  * Reads a file from the filesystem
    ///  */
    /// declare function readFile(path: string): Promise<string>;
    /// ```
    fn typescript_signature(&self) -> &'static str;

    /// Executes the tool with the provided input parameters.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` if the input is invalid or execution fails.
    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value as JsonValue, from_str, json, to_string};

    // REMOVED: test_tool_error_display - Low value trait test


    // REMOVED: test_tool_error_from_io - Low value trait test


    // REMOVED: test_tool_input_serialization - Low value serde test


    // REMOVED: test_tool_output_success - Trivial test


    // REMOVED: test_tool_output_success_with_data - Trivial test


    #[test]
    fn test_tool_output_error() {
        let output = ToolOutput::error("failed to execute");
        assert!(!output.success);
        assert_eq!(output.message, "failed to execute");
        assert!(output.data.is_none());
    }

    // REMOVED: test_tool_output_serialization - Low value serde test


    // Mock tool for testing the trait
    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            "mock_tool"
        }

        fn description(&self) -> &'static str {
            "A mock tool for testing"
        }

        fn typescript_signature(&self) -> &'static str {
            "/**\n * A mock tool for testing\n */\ndeclare function mockTool(params: any): Promise<any>;"
        }

        async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
            if input.params.get("fail").and_then(JsonValue::as_bool) == Some(true) {
                Err(ToolError::ExecutionFailed("intentional failure".to_owned()))
            } else {
                Ok(ToolOutput::success("mock executed"))
            }
        }
    }

    #[tokio::test]
    async fn test_tool_trait_implementation() {
        let tool = MockTool;
        assert_eq!(tool.name(), "mock_tool");
        assert_eq!(tool.description(), "A mock tool for testing");

        let input = ToolInput { params: json!({}) };
        let result = tool.execute(input).await;
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[tokio::test]
    async fn test_tool_trait_error_handling() {
        let tool = MockTool;
        let input = ToolInput {
            params: json!({"fail": true}),
        };
        let result = tool.execute(input).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::ExecutionFailed(_)));
    }
}
