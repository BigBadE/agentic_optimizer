//! Tool registry for managing available tools.

use std::convert::AsRef;
use std::sync::Arc;

use super::Tool;

type ToolList = Arc<Vec<Arc<dyn Tool>>>;

/// Registry for managing available tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: ToolList,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Vec::new()),
        }
    }

    /// Add a tool to the registry
    #[must_use]
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        Arc::make_mut(&mut self.tools).push(tool);
        self
    }

    /// Get a tool by name, if it exists
    #[must_use]
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .iter()
            .find(|tool_ref| tool_ref.name() == name)
            .cloned()
    }

    /// List all available tools
    #[must_use]
    pub fn list_tools(&self) -> Vec<&dyn Tool> {
        self.tools.iter().map(AsRef::as_ref).collect()
    }

    /// Get number of registered tools
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ToolInput, ToolOutput, ToolResult};
    use async_trait::async_trait;

    struct MockTool {
        name: &'static str,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            self.name
        }

        fn typescript_signature(&self) -> &'static str {
            "/**\n * A mock tool for testing\n */\ndeclare function mockTool(params: any): Promise<any>;"
        }

        async fn execute(&self, _input: ToolInput) -> ToolResult<ToolOutput> {
            Ok(ToolOutput::success("test"))
        }
    }

    /// Tests empty registry initialization.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_empty_registry() {
        let registry = ToolRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    /// Tests adding a tool to the registry.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_add_tool() {
        let tool = Arc::new(MockTool { name: "test_tool" });
        let registry = ToolRegistry::default().with_tool(tool);

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    /// Tests retrieving tools from the registry by name.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_get_tool() {
        let tool = Arc::new(MockTool { name: "test_tool" });
        let registry = ToolRegistry::default().with_tool(tool);

        assert!(registry.get_tool("test_tool").is_some());
        assert!(registry.get_tool("nonexistent").is_none());
    }

    /// Tests listing all tools in the registry.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_list_tools() {
        let tool1 = Arc::new(MockTool { name: "tool1" });
        let tool2 = Arc::new(MockTool { name: "tool2" });
        let registry = ToolRegistry::default().with_tool(tool1).with_tool(tool2);

        let tool_list = registry.list_tools();
        assert_eq!(tool_list.len(), 2);
    }
}
