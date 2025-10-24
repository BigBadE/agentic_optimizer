//! End-to-end tests for task list execution using fixtures.
//!
//! These tests actually execute the agent with mock providers and verify real outputs.
#![cfg_attr(
    test,
    allow(
        dead_code,
        unsafe_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        clippy::too_many_lines,
        clippy::expect_fun_call,
        clippy::min_ident_chars,
        clippy::redundant_closure_for_method_calls,
        reason = "Test allows"
    )
)]

mod task_list_fixture_runner;

use merlin_agent::agent::executor::AgentExecutorParams;
use merlin_agent::{AgentExecutor, ValidationPipeline, Validator};
use merlin_context::ContextFetcher;
use merlin_core::ui::{UiChannel, UiEvent};
use merlin_core::{ModelProvider, RoutingConfig, Task, TaskList};
use merlin_routing::{ProviderRegistry, StrategyRouter};
use merlin_tooling::ToolRegistry;
use num_cpus::get as get_cpu_count;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use task_list_fixture_runner::TaskListFixture;
use tokio::spawn;
use tokio::sync::{Semaphore, mpsc};

/// Set up test environment to use temp directory for caches
fn setup_test_env() {
    // Set MERLIN_FOLDER to temp directory to avoid polluting project directories with .merlin
    if env::var("MERLIN_FOLDER").is_err() {
        let temp_dir = env::temp_dir().join("merlin_agent_tests");
        // SAFETY: Setting env var once at test start before any concurrent access
        unsafe {
            env::set_var("MERLIN_FOLDER", &temp_dir);
            env::set_var("MERLIN_SKIP_EMBEDDINGS", "1");
        }
    }
}

/// Verify task list step descriptions
fn verify_step_descriptions(fixture_name: &str, task_list: &TaskList, expected_descs: &[String]) {
    for (i, step) in task_list.steps.iter().enumerate() {
        let expected_desc = &expected_descs[i];
        assert!(
            step.description.contains(expected_desc),
            "Fixture {fixture_name}: Step {} description '{}' does not contain expected '{expected_desc}'",
            i + 1,
            step.description
        );
    }
}

/// Drain UI channel to prevent blocking
async fn drain_ui_channel(mut rx: mpsc::UnboundedReceiver<UiEvent>) {
    while rx.recv().await.is_some() {
        // Drain events
    }
}

/// Test all fixtures with real agent execution using mock providers.
#[tokio::test]
async fn test_all_fixtures_with_agent_execution() {
    setup_test_env();
    let fixtures = TaskListFixture::discover_fixtures("tests/fixtures/task_lists")
        .expect("Failed to discover fixtures");

    assert!(
        !fixtures.is_empty(),
        "No fixtures found in tests/fixtures/task_lists"
    );

    // Create shared resources once (these are expensive to create)
    let validator = Arc::new(ValidationPipeline::with_default_stages()) as Arc<dyn Validator>;
    let tool_registry = Arc::new(ToolRegistry::default());
    let config = Arc::new(RoutingConfig::default());

    // Run fixtures in parallel with concurrency limit
    let cpu_count = get_cpu_count();
    let semaphore = Arc::new(Semaphore::new(cpu_count));
    let mut tasks = Vec::new();

    for fixture in fixtures {
        let sem = Arc::clone(&semaphore);
        let validator = Arc::clone(&validator);
        let tool_registry = Arc::clone(&tool_registry);
        let config = Arc::clone(&config);

        let task = spawn(async move {
            let _permit = sem.acquire().await.expect("Failed to acquire semaphore");

            // Create mock provider from fixture
            let mock_provider = Arc::new(fixture.create_mock_provider()) as Arc<dyn ModelProvider>;

            // Create provider registry with mock provider
            let provider_registry =
                ProviderRegistry::with_mock_provider(&mock_provider).expect(&format!(
                    "Failed to create provider registry for fixture: {}",
                    fixture.name
                ));

            // Create router with mock provider registry (not default strategies!)
            let router = Arc::new(StrategyRouter::new(provider_registry.clone()));

            // Create agent executor with injected mock provider
            let mut agent_executor = AgentExecutor::with_provider_registry(AgentExecutorParams {
                router,
                validator: Arc::clone(&validator),
                tool_registry: Arc::clone(&tool_registry),
                context_fetcher: ContextFetcher::new(PathBuf::from(".")),
                config: (*config).clone(),
                provider_registry: Arc::new(provider_registry),
            })
            .expect(&format!(
                "Failed to create agent executor for fixture: {}",
                fixture.name
            ));

            // Create task and execute (no UI channel needed for tests)
            let task = Task::new(fixture.initial_query.clone());
            let (tx, rx) = mpsc::unbounded_channel();
            let ui_channel = UiChannel::from_sender(tx);

            // Spawn a task to drain the UI channel to prevent blocking
            let _drain_handle = spawn(drain_ui_channel(rx));

            // Execute task through real agent executor
            let task_result = agent_executor
                .execute_streaming(task, ui_channel)
                .await
                .expect(&format!(
                    "Agent execution failed for fixture: {}",
                    fixture.name
                ));

            // Verify response is not empty
            assert!(
                !task_result.response.text.is_empty(),
                "Fixture {} produced empty response",
                fixture.name
            );

            // Extract and verify the generated task list
            if let Some(task_list) = task_result.task_list {
                // Verify task list has reasonable structure
                assert!(
                    !task_list.title.is_empty(),
                    "Fixture {}: TaskList has empty title",
                    fixture.name
                );

                // Verify task count matches expected
                assert_eq!(
                    task_list.steps.len(),
                    fixture.expected_task_list.total_tasks,
                    "Fixture {}: Expected {} tasks, but generated {}",
                    fixture.name,
                    fixture.expected_task_list.total_tasks,
                    task_list.steps.len()
                );

                // Verify each step description contains expected substring
                verify_step_descriptions(
                    &fixture.name,
                    &task_list,
                    &fixture.expected_task_list.task_descriptions,
                );
            } else if fixture.expected_task_list.total_tasks > 0 {
                panic!(
                    "Fixture {}: No TaskList generated but expected {} tasks",
                    fixture.name, fixture.expected_task_list.total_tasks
                );
            }

            fixture.name
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        let fixture_name = task.await.expect("Fixture task panicked");
        println!("✓ Fixture {fixture_name} passed");
    }
}

/// Test fixture structure validation without agent execution.
#[test]
fn test_all_fixtures_structure() {
    let fixtures = TaskListFixture::discover_fixtures("tests/fixtures/task_lists")
        .expect("Failed to discover fixtures");

    println!("\n========================================");
    println!("Validating structure of {} fixtures", fixtures.len());
    println!("========================================\n");

    for fixture in &fixtures {
        println!("📋 {}", fixture.name);

        // Validate structure
        assert!(!fixture.name.is_empty(), "Fixture has empty name");
        assert!(!fixture.initial_query.is_empty(), "Fixture has empty query");
        assert_eq!(
            fixture.expected_task_list.task_descriptions.len(),
            fixture.expected_task_list.total_tasks,
            "Fixture {} has mismatched task count",
            fixture.name
        );
        assert_eq!(
            fixture.expected_task_list.dependency_chain.len(),
            fixture.expected_task_list.total_tasks,
            "Fixture {} has mismatched dependency chain length",
            fixture.name
        );

        // Generate and verify tasks
        let tasks = fixture.create_task_descriptions();
        fixture
            .verify_task_list(&tasks)
            .unwrap_or_else(|e| panic!("Fixture {} failed verification: {}", fixture.name, e));

        // Validate dependencies
        for (i, deps) in fixture
            .expected_task_list
            .dependency_chain
            .iter()
            .enumerate()
        {
            for dep in deps {
                assert!(
                    *dep > 0 && *dep <= fixture.expected_task_list.total_tasks as u32,
                    "Fixture {} task {} has invalid dependency {}",
                    fixture.name,
                    i + 1,
                    dep
                );
            }
        }

        println!(
            "   ✅ {} tasks, {} dependencies validated\n",
            tasks.len(),
            fixture
                .expected_task_list
                .dependency_chain
                .iter()
                .map(|d| d.len())
                .sum::<usize>()
        );
    }

    println!("========================================");
    println!("✅ All {} fixtures have valid structure!", fixtures.len());
    println!("========================================");
}
