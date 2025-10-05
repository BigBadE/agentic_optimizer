use std::collections::HashMap;
use std::sync::Arc;

use merlin_tools::{BashTool, EditTool, ShowTool, Tool, ToolInput, ToolOutput};
use tracing::info;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    #[must_use] 
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        registry.register(Arc::new(EditTool::new()));
        registry.register(Arc::new(ShowTool::new()));
        registry.register(Arc::new(BashTool::new()));

        registry
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_owned();
        info!("Registering tool: {}", name);
        self.tools.insert(name, tool);
    }

    #[must_use] 
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    #[must_use] 
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
    pub async fn execute(&self, tool_name: &str, input: ToolInput) -> anyhow::Result<ToolOutput> {
        let tool = self
            .get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {tool_name}"))?;

        let output = tool.execute(input).await?;
        Ok(output)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

