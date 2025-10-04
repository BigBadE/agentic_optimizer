use std::sync::Arc;

use agentic_agent::{Agent, AgentConfig};
use agentic_core::{Context, ModelProvider, Query, Response, Result, TokenUsage};
use async_trait::async_trait;

struct MockProvider;

#[async_trait]
impl ModelProvider for MockProvider {
    async fn generate(&self, _query: &Query, _context: &Context) -> Result<Response> {
        Ok(Response {
            text: "Mock response".to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "mock".to_owned(),
            latency_ms: 0,
        })
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[test]
fn test_tools_in_system_prompt() {
    let provider = Arc::new(MockProvider);
    let config = AgentConfig::default();
    let agent = Agent::with_config(provider, config);
    let executor = agent.executor();

    let tools = executor.tool_registry().list_tools();
    
    assert_eq!(tools.len(), 3, "Should have 3 tools registered");
    
    let tool_names: Vec<&str> = tools.iter().map(|(name, _)| *name).collect();
    assert!(tool_names.contains(&"edit"), "Should have edit tool");
    assert!(tool_names.contains(&"show"), "Should have show tool");
    assert!(tool_names.contains(&"bash"), "Should have bash tool");
}
