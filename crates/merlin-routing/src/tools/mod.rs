use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use crate::Result;

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
    tools: Arc<Vec<Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Vec::new()),
        }
    }
    
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        Arc::make_mut(&mut self.tools).push(tool);
        self
    }
    
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }
    
    pub fn list_tools(&self) -> Vec<&dyn Tool> {
        self.tools.iter().map(|t| t.as_ref()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub mod file_ops;
pub mod command;

pub use file_ops::{ReadFileTool, WriteFileTool, ListFilesTool};
pub use command::RunCommandTool;
