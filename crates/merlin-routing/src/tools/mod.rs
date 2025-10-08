//! Tool implementations for agent capabilities.
//!
//! This module provides tools that agents can use to interact with the filesystem
//! and execute commands, including reading/writing files, listing directories,
//! and running shell commands.

use crate::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::convert::AsRef;
use std::sync::Arc;

/// Tool trait for agent capabilities
#[async_trait]
pub trait Tool: Send + Sync {
    /// Name of the tool
    fn name(&self) -> &str;

    /// Description of what the tool does
    fn description(&self) -> &str;

    /// JSON schema for the tool's arguments
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with given arguments
    async fn execute(&self, args: Value) -> Result<Value>;
}

/// Registry for managing available tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: ToolList,
}

type ToolList = Arc<Vec<Arc<dyn Tool>>>;

impl ToolRegistry {
    /// Creates an empty tool registry. Prefer `Default`.
    /// Adds a tool to the registry.
    #[must_use]
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        Arc::make_mut(&mut self.tools).push(tool);
        self
    }

    /// Gets a tool by name, if it exists.
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .iter()
            .find(|tool_ref| tool_ref.name() == name)
            .cloned()
    }

    /// Lists all available tools.
    pub fn list_tools(&self) -> Vec<&dyn Tool> {
        self.tools.iter().map(AsRef::as_ref).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self {
            tools: Arc::new(Vec::default()),
        }
    }
}

/// Command execution tool
pub mod command;
/// File operation tools (read, write, list)
pub mod file_ops;

pub use command::RunCommandTool;
pub use file_ops::{ListFilesTool, ReadFileTool, WriteFileTool};
