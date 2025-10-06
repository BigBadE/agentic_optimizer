use std::io::Error as IoError;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Error as SerdeJsonError, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("IO error: {0}")]
    Io(#[from] IoError),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] SerdeJsonError),
}

pub type ToolResult<T> = Result<T, ToolError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

impl ToolOutput {
    pub fn success<T: Into<String>>(message: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn success_with_data<T: Into<String>>(message: T, data: Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error<T: Into<String>>(message: T) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput>;
}
