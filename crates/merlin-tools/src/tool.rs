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

    /// Executes the tool with the provided input parameters.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` if the input is invalid or execution fails.
    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput>;
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::absolute_paths,
    clippy::missing_panics_doc,
    clippy::min_ident_chars,
    reason = "Test code is allowed to use unwrap and has different conventions"
)]
mod tests {
    use super::*;
    use serde_json::{Value as JsonValue, from_str, json, to_string};

    #[test]
    fn test_tool_error_display() {
        let error1 = ToolError::InvalidInput("bad param".to_owned());
        assert_eq!(error1.to_string(), "Invalid input: bad param");

        let error2 = ToolError::ExecutionFailed("command failed".to_owned());
        assert_eq!(error2.to_string(), "Tool execution failed: command failed");
    }

    #[test]
    fn test_tool_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let error: ToolError = io_error.into();
        assert!(matches!(error, ToolError::Io(_)));
    }

    #[test]
    fn test_tool_input_serialization() {
        let input = ToolInput {
            params: json!({"key": "value"}),
        };
        let json = to_string(&input).unwrap();
        let deserialized: ToolInput = from_str(&json).unwrap();
        assert_eq!(input.params, deserialized.params);
    }

    #[test]
    fn test_tool_output_success() {
        let output = ToolOutput::success("operation completed");
        assert!(output.success);
        assert_eq!(output.message, "operation completed");
        assert!(output.data.is_none());
    }

    #[test]
    fn test_tool_output_success_with_data() {
        let data = json!({"result": 42});
        let output = ToolOutput::success_with_data("computed", data.clone());
        assert!(output.success);
        assert_eq!(output.message, "computed");
        assert_eq!(output.data, Some(data));
    }

    #[test]
    fn test_tool_output_error() {
        let output = ToolOutput::error("failed to execute");
        assert!(!output.success);
        assert_eq!(output.message, "failed to execute");
        assert!(output.data.is_none());
    }

    #[test]
    fn test_tool_output_serialization() {
        let output = ToolOutput::success_with_data("done", json!({"count": 5}));
        let json = to_string(&output).unwrap();
        let deserialized: ToolOutput = from_str(&json).unwrap();
        assert_eq!(output.success, deserialized.success);
        assert_eq!(output.message, deserialized.message);
        assert_eq!(output.data, deserialized.data);
    }

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
