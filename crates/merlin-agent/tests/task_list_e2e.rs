//! End-to-end tests for task list execution using fixtures.
//!
//! These tests actually execute the agent with mock providers and verify real outputs.
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
        clippy::too_many_lines,
        clippy::expect_fun_call,
        clippy::min_ident_chars,
        clippy::redundant_closure_for_method_calls,
        reason = "Test allows"
    )
)]

mod task_list_fixture_runner;

use merlin_agent::agent::executor::AgentExecutorParams;
use merlin_agent::{AgentExecutor, ValidationPipeline};
use merlin_context::ContextFetcher;
use merlin_core::ui::UiChannel;
use merlin_core::{ModelProvider, RoutingConfig, Task};
use merlin_routing::{ProviderRegistry, StrategyRouter};
use merlin_tooling::ToolRegistry;
use std::path::PathBuf;
use std::sync::Arc;
use task_list_fixture_runner::TaskListFixture;
use tokio::sync::mpsc;

/// Test all fixtures with real agent execution using mock providers.
#[tokio::test]
async fn test_all_fixtures_with_agent_execution() {
    let fixtures = TaskListFixture::discover_fixtures("tests/fixtures/task_lists")
        .expect("Failed to discover fixtures");

    assert!(
        !fixtures.is_empty(),
        "No fixtures found in tests/fixtures/task_lists"
    );

    println!("\n========================================");
    println!(
        "Testing {} fixtures with real agent execution",
        fixtures.len()
    );
    println!("========================================\n");

    for fixture in &fixtures {
        println!("📋 {}", fixture.name);
        println!("   {}", fixture.description);

        // 1. Create mock provider from fixture
        let mock_provider = Arc::new(fixture.create_mock_provider()) as Arc<dyn ModelProvider>;

        // 2. Create provider registry with mock provider
        let provider_registry =
            ProviderRegistry::with_mock_provider(&mock_provider).expect(&format!(
                "Failed to create provider registry for fixture: {}",
                fixture.name
            ));

        // 3. Create router with mock provider registry (not default strategies!)
        let router = Arc::new(StrategyRouter::new(provider_registry.clone()));

        let validator = Arc::new(ValidationPipeline::with_default_stages());
        let tool_registry = Arc::new(ToolRegistry::default());
        let context_fetcher = ContextFetcher::new(PathBuf::from("."));
        let config = RoutingConfig::default();

        // 4. Create agent executor with injected mock provider
        let mut agent_executor = AgentExecutor::with_provider_registry(AgentExecutorParams {
            router,
            validator,
            tool_registry,
            context_fetcher,
            config,
            provider_registry: Arc::new(provider_registry),
        })
        .expect(&format!(
            "Failed to create agent executor for fixture: {}",
            fixture.name
        ));

        // 5. Create task and UI channel
        let task = Task::new(fixture.initial_query.clone());
        let (tx, _rx) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(tx);

        // 6. Execute task through real agent executor
        println!("   🚀 Executing task through agent...");
        let task_result = agent_executor
            .execute_streaming(task, ui_channel)
            .await
            .expect(&format!(
                "Agent execution failed for fixture: {}",
                fixture.name
            ));

        println!(
            "   📝 Agent response: {} chars",
            task_result.response.text.len()
        );

        // 7. Verify response is not empty
        assert!(
            !task_result.response.text.is_empty(),
            "Fixture {} produced empty response",
            fixture.name
        );

        // 8. Extract and verify the generated task list
        if let Some(task_list) = task_result.task_list {
            println!("   📊 Generated TaskList: {} steps", task_list.steps.len());

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
            for (i, step) in task_list.steps.iter().enumerate() {
                let expected_desc = &fixture.expected_task_list.task_descriptions[i];
                assert!(
                    step.description.contains(expected_desc),
                    "Fixture {}: Step {} description '{}' does not contain expected '{}'",
                    fixture.name,
                    i + 1,
                    step.description,
                    expected_desc
                );
            }

            println!(
                "   ✅ Verified {} generated tasks match expected structure",
                task_list.steps.len()
            );
        } else if fixture.expected_task_list.total_tasks > 0 {
            panic!(
                "Fixture {}: No TaskList generated but expected {} tasks",
                fixture.name, fixture.expected_task_list.total_tasks
            );
        } else {
            println!("   ✅ No TaskList expected or generated (correct)");
        }

        println!();
    }

    println!("========================================");
    println!("✅ All {} fixtures executed!", fixtures.len());
    println!("========================================");
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
