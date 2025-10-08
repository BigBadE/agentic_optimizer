//! Integration tests for tool execution within the agent
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use async_trait::async_trait;
use merlin_agent::{Agent, AgentConfig};
use merlin_core::{Context, ModelProvider, Query, Response, Result, TokenUsage};
use std::sync::Arc;

struct MockProvider;

#[async_trait]
impl ModelProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn generate(&self, _query: &Query, _context: &Context) -> Result<Response> {
        Ok(Response {
            text: "Mock response".to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "mock".to_owned(),
            latency_ms: 0,
        })
    }

    fn estimate_cost(&self, _context: &Context) -> f64 {
        0.0
    }
}

/// Verify default tools are registered in the executor's tool registry.
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
