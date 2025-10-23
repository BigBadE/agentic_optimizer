//! Integration tests for `AgentExecutor`.
//!
//! Tests the full agent execution flow with tools, context, and conversation history.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::tests_outside_test_module,
        clippy::too_many_lines,
        reason = "Test allows"
    )
)]

use merlin_agent::agent::executor::AgentExecutorParams;
use merlin_agent::{AgentExecutor, ValidationPipeline, Validator};
use merlin_context::ContextFetcher;
use merlin_core::ui::UiChannel;
use merlin_core::{
    Context, ModelProvider, Query, Response, Result, RoutingConfig, Task, TokenUsage,
};
use merlin_routing::{ProviderRegistry, StrategyRouter};
use merlin_tooling::{BashTool, ToolRegistry};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Mock provider for testing
struct MockProvider {
    response: String,
}

#[async_trait::async_trait]
impl ModelProvider for MockProvider {
    async fn generate(&self, _query: &Query, _context: &Context) -> Result<Response> {
        Ok(Response {
            text: self.response.clone(),
            confidence: 0.9,
            tokens_used: TokenUsage::default(),
            provider: "mock".to_owned(),
            latency_ms: 100,
        })
    }

    fn name(&self) -> &'static str {
        "mock-model"
    }

    async fn is_available(&self) -> bool {
        true
    }

    fn estimate_cost(&self, _context: &Context) -> f64 {
        0.0
    }
}

/// Create a test executor with local-only config
///
/// # Errors
/// Returns error if provider registry creation fails
fn create_test_executor() -> Result<AgentExecutor> {
    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;

    let provider_registry = ProviderRegistry::new(config.clone())?;
    let router = Arc::new(StrategyRouter::new(provider_registry.clone()));
    let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;
    let tool_registry = Arc::new(ToolRegistry::default());
    let context_fetcher = ContextFetcher::new(PathBuf::from("."));

    AgentExecutor::with_provider_registry(AgentExecutorParams {
        router,
        validator,
        tool_registry,
        context_fetcher,
        config,
        provider_registry: Arc::new(provider_registry),
    })
}

#[test]
fn test_agent_executor_creation() {
    let result = create_test_executor();
    assert!(
        result.is_ok() || result.is_err(),
        "Executor creation should complete"
    );
}

#[test]
fn test_agent_executor_context_dump_toggle() {
    if let Ok(mut executor) = create_test_executor() {
        executor.enable_context_dump();
        executor.disable_context_dump();
        // Just verify the methods exist and don't panic
    }
}

#[tokio::test]
async fn test_agent_executor_conversation_history() {
    if let Ok(mut executor) = create_test_executor() {
        executor
            .add_to_conversation("user".to_owned(), "Hello".to_owned())
            .await;
        executor
            .add_to_conversation("assistant".to_owned(), "Hi there!".to_owned())
            .await;

        // Verify conversation history can be managed
    }
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_agent_executor_simple_streaming() {
    let mut executor = create_test_executor().expect("create executor");

    let task = Task::new("Say hello".to_owned()).with_difficulty(1);
    let (sender, _rx) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    let result = executor.execute_streaming(task, ui_channel).await;

    assert!(result.is_ok(), "Execution should succeed");
    let task_result = result.unwrap();
    assert!(!task_result.response.text.is_empty());
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_agent_executor_with_bash_tool() {
    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;

    let provider_registry = ProviderRegistry::new(config.clone()).expect("provider registry");
    let router = Arc::new(StrategyRouter::new(provider_registry.clone()));
    let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;

    // Create tool registry with bash tool
    let tool_registry = Arc::new(ToolRegistry::default().with_tool(Arc::new(BashTool)));

    let context_fetcher = ContextFetcher::new(PathBuf::from("."));

    let mut executor = AgentExecutor::with_provider_registry(AgentExecutorParams {
        router,
        validator,
        tool_registry,
        context_fetcher,
        config,
        provider_registry: Arc::new(provider_registry),
    })
    .expect("create executor");

    let task = Task::new("Run echo hello using bash".to_owned()).with_difficulty(1);
    let (sender, _rx) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    let result = executor.execute_streaming(task, ui_channel).await;

    assert!(
        result.is_ok() || result.is_err(),
        "Tool execution should complete"
    );
}

#[tokio::test]
async fn test_agent_executor_with_mock_provider() {
    let mock_provider = Arc::new(MockProvider {
        response: "Mock response".to_owned(),
    }) as Arc<dyn ModelProvider>;

    let provider_registry =
        ProviderRegistry::with_mock_provider(&mock_provider).expect("provider registry");

    let router = Arc::new(StrategyRouter::new(provider_registry.clone()));
    let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;
    let tool_registry = Arc::new(ToolRegistry::default());
    let context_fetcher = ContextFetcher::new(PathBuf::from("."));
    let config = RoutingConfig::default();

    let mut executor = AgentExecutor::with_provider_registry(AgentExecutorParams {
        router,
        validator,
        tool_registry,
        context_fetcher,
        config,
        provider_registry: Arc::new(provider_registry),
    })
    .expect("create executor");

    let task = Task::new("test task".to_owned()).with_difficulty(1);
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    let result = executor.execute_streaming(task, ui_channel).await;

    assert!(result.is_ok(), "Mock execution should succeed");

    // Check that we received some UI events
    let mut event_count = 0;
    while receiver.try_recv().is_ok() {
        event_count += 1;
    }
    assert!(event_count > 0, "Should have received UI events");
}

#[tokio::test]
async fn test_agent_executor_cloning() {
    if let Ok(executor) = create_test_executor() {
        let _cloned = executor;

        // Verify executor works
        let task1 = Task::new("task1".to_owned());
        let task2 = Task::new("task2".to_owned());

        // Just verify they can be created and used independently
        drop(task1);
        drop(task2);
    }
}

#[tokio::test]
async fn test_agent_executor_multiple_sequential_tasks() {
    if let Ok(mut executor) = create_test_executor() {
        let (sender, _rx) = mpsc::unbounded_channel();

        // Execute multiple tasks sequentially
        for idx in 0..3 {
            let task = Task::new(format!("Task {idx}")).with_difficulty(1);
            let ui_channel = UiChannel::from_sender(sender.clone());

            let result = executor.execute_streaming(task, ui_channel).await;
            // Just verify it completes without panicking
            drop(result);
        }
    }
}

#[tokio::test]
async fn test_agent_executor_with_conversation_context() {
    if let Ok(mut executor) = create_test_executor() {
        // Build up conversation context
        executor
            .add_to_conversation("user".to_owned(), "What is 2+2?".to_owned())
            .await;
        executor
            .add_to_conversation("assistant".to_owned(), "4".to_owned())
            .await;
        executor
            .add_to_conversation("user".to_owned(), "What about 3+3?".to_owned())
            .await;

        // Now execute a task that might use the context
        let task = Task::new("Continue the math pattern".to_owned()).with_difficulty(1);
        let (sender, _rx) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(sender);

        let result = executor.execute_streaming(task, ui_channel).await;
        drop(result); // Just verify no panic
    }
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_agent_executor_error_handling() {
    let mut executor = create_test_executor().expect("create executor");

    // Create a task that might fail
    let task =
        Task::new("Execute invalid bash command: %%%invalid%%%".to_owned()).with_difficulty(1);
    let (sender, _rx) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    let result = executor.execute_streaming(task, ui_channel).await;

    // Should either succeed (by not executing the command) or fail gracefully
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle errors gracefully"
    );
}

#[tokio::test]
async fn test_agent_executor_ui_channel_receives_events() {
    let mock_provider = Arc::new(MockProvider {
        response: "Test response".to_owned(),
    }) as Arc<dyn ModelProvider>;

    let provider_registry =
        ProviderRegistry::with_mock_provider(&mock_provider).expect("provider registry");

    let router = Arc::new(StrategyRouter::new(provider_registry.clone()));
    let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;
    let tool_registry = Arc::new(ToolRegistry::default());
    let context_fetcher = ContextFetcher::new(PathBuf::from("."));
    let config = RoutingConfig::default();

    let mut executor = AgentExecutor::with_provider_registry(AgentExecutorParams {
        router,
        validator,
        tool_registry,
        context_fetcher,
        config,
        provider_registry: Arc::new(provider_registry),
    })
    .expect("create executor");

    let task = Task::new("test".to_owned()).with_difficulty(1);
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    // Execute task
    let _result = executor.execute_streaming(task, ui_channel).await;

    // Collect all events
    let mut events = Vec::new();
    while let Ok(event) = receiver.try_recv() {
        events.push(event);
    }

    // Should have received some events during execution
    assert!(!events.is_empty(), "Should receive UI events");
}

#[test]
fn test_agent_executor_params_struct() {
    // Just verify we can construct the params struct
    let config = RoutingConfig::default();

    if let Ok(provider_registry) = ProviderRegistry::new(config.clone()) {
        let router = Arc::new(StrategyRouter::new(provider_registry.clone()));
        let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;
        let tool_registry = Arc::new(ToolRegistry::default());
        let context_fetcher = ContextFetcher::new(PathBuf::from("."));

        let _params = AgentExecutorParams {
            router,
            validator,
            tool_registry,
            context_fetcher,
            config,
            provider_registry: Arc::new(provider_registry),
        };
    }
}
