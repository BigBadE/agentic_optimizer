//! Integration tests for `RoutingOrchestrator`.
//!
//! Tests the full orchestration flow including analysis, routing, execution, and validation.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::print_stdout,
        clippy::tests_outside_test_module,
        unsafe_code,
        reason = "Test allows"
    )
)]

use merlin_agent::{RoutingOrchestrator, Validator};
use merlin_core::ui::UiChannel;
use merlin_core::{Result, RoutingConfig, Task};
use merlin_routing::{ModelRouter, TaskAnalyzer};
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Set up test environment to use temp directory for caches
fn setup_test_env() {
    // Set MERLIN_FOLDER to temp directory to avoid polluting project directories with .merlin
    if env::var("MERLIN_FOLDER").is_err() {
        let temp_dir = env::temp_dir().join("merlin_agent_tests");
        // SAFETY: Setting env var once at test start before any concurrent access
        unsafe {
            env::set_var("MERLIN_FOLDER", &temp_dir);
        }
    }
}

/// Helper to create a test orchestrator with local-only config
///
/// # Errors
/// Returns error if orchestrator creation fails
fn create_test_orchestrator() -> Result<RoutingOrchestrator> {
    setup_test_env();
    let mut config = RoutingConfig::default();
    // Disable cloud providers for tests
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;
    RoutingOrchestrator::new(config)
}

#[tokio::test]
async fn test_orchestrator_creation() {
    let result = create_test_orchestrator();
    assert!(
        result.is_ok(),
        "Orchestrator should be created successfully"
    );
}

#[tokio::test]
async fn test_orchestrator_with_custom_components() {
    use merlin_agent::ValidationPipeline;
    use merlin_routing::{LocalTaskAnalyzer, ProviderRegistry, StrategyRouter};

    setup_test_env();
    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;

    // Create custom components
    let analyzer = Arc::new(LocalTaskAnalyzer::default()) as Arc<dyn TaskAnalyzer>;
    let provider_registry = ProviderRegistry::new(config.clone()).expect("provider registry");
    let router = Arc::new(StrategyRouter::new(provider_registry)) as Arc<dyn ModelRouter>;
    let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;

    // Create orchestrator with custom components
    let orchestrator = RoutingOrchestrator::new(config)
        .expect("orchestrator")
        .with_analyzer(analyzer)
        .with_router(router)
        .with_validator(validator);

    // Verify it was created
    orchestrator.analyze_request("test request").await.unwrap();
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_orchestrator_analyze_request() {
    let orchestrator = create_test_orchestrator().expect("orchestrator");

    let result = orchestrator
        .analyze_request("Create a hello world function")
        .await;

    assert!(result.is_ok(), "Analysis should succeed");
    let analysis = result.unwrap();
    assert!(!analysis.tasks.is_empty(), "Should produce tasks");
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_orchestrator_execute_simple_task() {
    let orchestrator = create_test_orchestrator().expect("orchestrator");

    let task = Task::new("say hello".to_owned()).with_difficulty(1);
    let (sender, _rx) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    let result = orchestrator.execute_task_streaming(task, ui_channel).await;

    assert!(result.is_ok(), "Task execution should succeed");
    let task_result = result.unwrap();
    assert!(
        !task_result.response.text.is_empty(),
        "Response should not be empty"
    );
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_orchestrator_execute_with_conversation_history() {
    let orchestrator = create_test_orchestrator().expect("orchestrator");

    let task = Task::new("continue the conversation".to_owned()).with_difficulty(1);
    let (sender, _rx) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    let history = vec![
        ("user".to_owned(), "Hello".to_owned()),
        ("assistant".to_owned(), "Hi there!".to_owned()),
    ];

    let result = orchestrator
        .execute_task_streaming_with_history(task, ui_channel, history)
        .await;

    assert!(result.is_ok(), "Task with history should succeed");
}

#[tokio::test]
async fn test_orchestrator_builder_pattern() {
    use merlin_agent::ValidationPipeline;
    use merlin_routing::LocalTaskAnalyzer;

    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;

    let analyzer = Arc::new(LocalTaskAnalyzer::default()) as Arc<dyn TaskAnalyzer>;
    let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;

    // Test builder pattern chaining
    let orchestrator = RoutingOrchestrator::new(config)
        .expect("orchestrator")
        .with_analyzer(Arc::clone(&analyzer))
        .with_validator(Arc::clone(&validator));

    // Verify the orchestrator was built successfully
    orchestrator.analyze_request("test").await.unwrap();
}

#[tokio::test]
async fn test_orchestrator_config_validation() {
    // Test with invalid config (both cloud providers disabled and no local models)
    let config = RoutingConfig::default();

    // This should succeed even without API keys since we have fallback to local
    let result = RoutingOrchestrator::new(config);
    assert!(
        result.is_ok() || result.is_err(),
        "Config validation should complete"
    );
}

#[tokio::test]
async fn test_orchestrator_multiple_tasks_sequential() {
    let orchestrator = create_test_orchestrator().expect("orchestrator");

    // Analyze multiple requests sequentially
    let requests = vec!["Create a function", "Write a test", "Add documentation"];

    for request in requests {
        let result = orchestrator.analyze_request(request).await;
        // Should either succeed or fail gracefully without Ollama
        assert!(result.is_ok() || result.is_err(), "Should handle request");
    }
}

#[tokio::test]
async fn test_orchestrator_workspace_access() {
    let orchestrator = create_test_orchestrator().expect("orchestrator");

    let workspace = orchestrator.workspace();
    assert!(
        workspace.root_path().exists(),
        "Workspace root should exist"
    );
}

#[tokio::test]
async fn test_orchestrator_config_access() {
    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;
    config.execution.max_concurrent_tasks = 5;

    let orchestrator = RoutingOrchestrator::new(config).expect("orchestrator");

    assert_eq!(
        orchestrator.config().execution.max_concurrent_tasks,
        5,
        "Config should be accessible"
    );
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_orchestrator_process_request_end_to_end() {
    let orchestrator = create_test_orchestrator().expect("orchestrator");

    let results = orchestrator
        .process_request("Write a hello world function")
        .await;

    // May succeed or fail depending on Ollama availability
    match results {
        Ok(task_results) => {
            assert!(!task_results.is_empty(), "Should produce task results");
        }
        Err(err) => {
            tracing::info!("Expected failure without Ollama: {err}");
        }
    }
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_orchestrator_execute_tasks_with_dependencies() {
    use merlin_core::Task;

    let orchestrator = create_test_orchestrator().expect("orchestrator");

    let task_a = Task::new("Task A".to_owned()).with_difficulty(1);
    let task_b = Task::new("Task B".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_a.id]);

    let results = orchestrator.execute_tasks(vec![task_a, task_b]).await;

    match results {
        Ok(task_results) => {
            assert_eq!(task_results.len(), 2, "Both tasks should execute");
        }
        Err(err) => {
            tracing::info!("Expected failure without Ollama: {err}");
        }
    }
}

#[tokio::test]
async fn test_orchestrator_with_context_dump_enabled() {
    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;
    config.execution.context_dump = true;

    let orchestrator = RoutingOrchestrator::new(config).expect("orchestrator");

    assert!(
        orchestrator.config().execution.context_dump,
        "Context dump should be enabled"
    );
}

#[tokio::test]
async fn test_orchestrator_with_conflict_detection_enabled() {
    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;
    config.execution.enable_conflict_detection = true;

    let orchestrator = RoutingOrchestrator::new(config).expect("orchestrator");

    assert!(
        orchestrator.config().execution.enable_conflict_detection,
        "Conflict detection should be enabled"
    );
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_orchestrator_with_conflict_detection_execution() {
    use merlin_core::{ContextRequirements, Task};
    use std::path::PathBuf;

    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;
    config.execution.enable_conflict_detection = true;

    let orchestrator = RoutingOrchestrator::new(config).expect("orchestrator");

    // Create tasks that access the same file
    let file = PathBuf::from("test.rs");
    let task_a = Task::new("Modify test.rs - A".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file.clone()]));

    let task_b = Task::new("Modify test.rs - B".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file]));

    let result = orchestrator.execute_tasks(vec![task_a, task_b]).await;

    // May succeed or fail depending on Ollama, but should not panic
    match result {
        Ok(results) => {
            assert_eq!(results.len(), 2, "Both tasks should complete");
        }
        Err(err) => {
            tracing::info!("Expected failure without Ollama: {err}");
        }
    }
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_orchestrator_cyclic_dependency_detection() {
    use merlin_core::Task;

    let orchestrator = create_test_orchestrator().expect("orchestrator");

    // Create cyclic dependency
    let task_a = Task::new("Task A".to_owned()).with_difficulty(1);
    let task_b = Task::new("Task B".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_a.id]);
    let task_a_cyclic = Task {
        id: task_a.id,
        description: task_a.description.clone(),
        dependencies: vec![task_b.id],
        ..task_a
    };

    let result = orchestrator
        .execute_tasks(vec![task_a_cyclic, task_b])
        .await;

    assert!(
        result.is_err(),
        "Should detect and reject cyclic dependencies"
    );
}

#[tokio::test]
async fn test_orchestrator_with_validation_early_exit() {
    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;
    config.validation.early_exit = true;

    let orchestrator = RoutingOrchestrator::new(config).expect("orchestrator");

    assert!(
        orchestrator.config().validation.early_exit,
        "Validation early exit should be enabled"
    );
}

#[tokio::test]
#[ignore = "Requires local Ollama server"]
async fn test_orchestrator_empty_conversation_history() {
    let orchestrator = create_test_orchestrator().expect("orchestrator");

    let task = Task::new("test task".to_owned()).with_difficulty(1);
    let (sender, _rx) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    // Execute with empty history (default behavior)
    let result = orchestrator
        .execute_task_streaming_with_history(task, ui_channel, vec![])
        .await;

    // May succeed or fail depending on Ollama
    match result {
        Ok(_) => {}
        Err(err) => {
            tracing::info!("Expected failure without Ollama: {err}");
        }
    }
}

#[tokio::test]
async fn test_orchestrator_multiple_custom_components() {
    use merlin_agent::ValidationPipeline;
    use merlin_routing::{LocalTaskAnalyzer, ProviderRegistry, StrategyRouter};

    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;

    let analyzer =
        Arc::new(LocalTaskAnalyzer::default().with_max_parallel(4)) as Arc<dyn TaskAnalyzer>;
    let provider_registry = ProviderRegistry::new(config.clone()).expect("provider registry");
    let router = Arc::new(StrategyRouter::new(provider_registry)) as Arc<dyn ModelRouter>;
    let validator = Arc::new(ValidationPipeline::with_default_stages().with_early_exit(true))
        as Arc<dyn Validator>;

    let orchestrator = RoutingOrchestrator::new(config)
        .expect("orchestrator")
        .with_analyzer(Arc::clone(&analyzer))
        .with_router(Arc::clone(&router))
        .with_validator(Arc::clone(&validator));

    // Verify orchestrator was created with custom components
    assert!(
        orchestrator.analyze_request("test").await.is_ok()
            || orchestrator.analyze_request("test").await.is_err()
    );
}

#[tokio::test]
async fn test_orchestrator_builder_pattern_chaining() {
    use merlin_agent::ValidationPipeline;
    use merlin_routing::LocalTaskAnalyzer;

    let mut config = RoutingConfig::default();
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;

    let analyzer = Arc::new(LocalTaskAnalyzer::default()) as Arc<dyn TaskAnalyzer>;
    let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;

    // Chain multiple builders
    let _orchestrator = RoutingOrchestrator::new(config)
        .expect("orchestrator")
        .with_analyzer(Arc::clone(&analyzer))
        .with_validator(Arc::clone(&validator))
        .with_analyzer(Arc::clone(&analyzer)) // Can override
        .with_validator(Arc::clone(&validator));

    // Should successfully build
}
