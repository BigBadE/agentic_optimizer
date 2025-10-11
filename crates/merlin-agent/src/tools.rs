use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use merlin_tools::{
    BashTool, DeleteTool, EditTool, ListTool, ShowTool, Tool, ToolInput, ToolOutput,
};
use tracing::info;

/// Registry for managing and executing tools available to the agent
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Register a tool by name
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_owned();
        info!("Registering tool: {}", name);
        self.tools.insert(name, tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// List all registered tools with their descriptions
    pub fn list_tools(&self) -> Vec<(&str, &str)> {
        self.tools
            .values()
            .map(|tool| (tool.name(), tool.description()))
            .collect()
    }

    /// Execute a tool by name
    ///
    /// # Errors
    /// Returns an error if the tool is not found or execution fails
    pub async fn execute(&self, tool_name: &str, input: ToolInput) -> Result<ToolOutput> {
        let tool = self
            .get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {tool_name}"))?;

        let output = tool.execute(input).await?;
        Ok(output)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut registry = Self {
            tools: HashMap::default(),
        };

        // Register all available tools
        registry.register(Arc::new(ShowTool));
        registry.register(Arc::new(EditTool));
        registry.register(Arc::new(DeleteTool));
        registry.register(Arc::new(ListTool));
        registry.register(Arc::new(BashTool));

        registry
    }
}
