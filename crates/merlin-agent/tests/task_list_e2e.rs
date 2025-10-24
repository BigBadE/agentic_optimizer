//! End-to-end tests for task list execution using fixtures.
//!
//! These tests execute the full agent workflow with mock providers:
//! 1. Agent generates task list from initial query
//! 2. TaskListExecutor executes each step in the task list
//! 3. Validation pipeline validates the results
//! 4. Test verifies files are created and validation passes
//!
//! ## Fixture Types
//!
//! **Full E2E Fixtures** (have tool calls in step responses):
//! - simple_implementation.json - Creates files, adds tests, runs validation
//! - test_failure_recovery.json - Tests error handling and retry logic
//! - multiple_failures_retry.json - Tests multiple retry attempts
//! - parallel_tasks.json - Tests parallel task execution
//!
//! **Structural Fixtures** (test decomposition only, no file creation):
//! - circular_dependency_detection.json - Tests cycle detection
//! - complex_dag.json - Tests complex dependency graphs
//! - deep_dependency_chain.json - Tests deep dependency chains
//! - empty_task_list.json - Tests empty task list handling
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

use merlin_agent::RoutingOrchestrator;
use merlin_core::ui::{UiChannel, UiEvent};
use merlin_core::{ModelProvider, RoutingConfig, Task, TaskList};
use merlin_routing::{ProviderRegistry, StrategyRouter};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use task_list_fixture_runner::TaskListFixture;
use tempfile::TempDir;
use tokio::spawn;
use tokio::sync::mpsc;

/// Set up shared test workspace with proper Rust project structure
fn setup_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path();

    // Create basic Rust project structure
    fs::create_dir_all(project_root.join("src")).expect("Failed to create src dir");

    // Create minimal Cargo.toml
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

    fs::write(project_root.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Create minimal main.rs
    fs::write(project_root.join("src/main.rs"), "fn main() {}\n").expect("Failed to write main.rs");

    // Set MERLIN_FOLDER to use this temp directory for caches
    // SAFETY: Setting env var once at test start before any concurrent access
    unsafe {
        env::set_var("MERLIN_FOLDER", project_root.join(".merlin"));
        env::set_var("MERLIN_SKIP_EMBEDDINGS", "1");
    }

    temp_dir
}

/// Reset workspace files to clean state between tests
fn reset_workspace(workspace_root: &Path, files_to_reset: &[String]) {
    for file in files_to_reset {
        let file_path = workspace_root.join(file);
        if file_path.exists() {
            fs::remove_file(&file_path)
                .unwrap_or_else(|e| panic!("Failed to remove file {file}: {e}"));
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

/// Create an orchestrator with mock provider for testing
///
/// This is a thin wrapper that uses the exact same setup as production code
fn create_test_orchestrator(
    mock_provider: &Arc<dyn ModelProvider>,
    workspace_root: &Path,
) -> RoutingOrchestrator {
    // Create provider registry with mock provider
    let provider_registry = ProviderRegistry::with_mock_provider(mock_provider)
        .expect("Failed to create provider registry");

    // Create router with mock provider (same as production)
    let router = Arc::new(StrategyRouter::new(provider_registry));

    // Create config with workspace path and dummy API keys for testing
    let mut config = RoutingConfig::default();
    config.workspace.root_path = workspace_root.to_path_buf();

    // Set dummy API keys to avoid failing in ProviderRegistry::new()
    // These won't be used because we override the router with our mock
    config.api_keys.groq_api_key = Some("test_groq_key".to_owned());
    config.api_keys.openrouter_api_key = Some("test_openrouter_key".to_owned());

    // Create orchestrator using production code path
    RoutingOrchestrator::new(config)
        .expect("Failed to create orchestrator")
        .with_router(router)
}

/// Verify expected outcomes from fixture
fn verify_outcomes(
    fixture_name: &str,
    workspace_root: &Path,
    fixture: &TaskListFixture,
    task_list_success: bool,
) {
    let outcomes = &fixture.expected_outcomes;

    // Verify all tasks completed if expected
    if outcomes.all_tasks_completed {
        assert!(
            task_list_success,
            "Fixture {fixture_name}: Expected all tasks to complete successfully"
        );
    }

    // Only verify file creation for fixtures that have actual tool calls
    // Some fixtures test structure/decomposition without actual execution
    let has_tool_calls = fixture
        .mock_responses
        .iter()
        .any(|r| r.response.contains("await bash") || r.response.contains("await writeFile"));

    if has_tool_calls {
        // Verify files were created
        for expected_file in &outcomes.files_created {
            let file_path = workspace_root.join(expected_file);
            assert!(
                file_path.exists(),
                "Fixture {fixture_name}: Expected file '{expected_file}' was not created"
            );

            // Log file contents for debugging
            if let Ok(contents) = fs::read_to_string(&file_path) {
                tracing::debug!(
                    "Fixture {fixture_name}: File {expected_file} created with {} bytes",
                    contents.len()
                );
            }
        }
    } else if !outcomes.files_created.is_empty() {
        tracing::warn!(
            "Fixture {fixture_name}: Expects files {:?} but has no tool calls - skipping file verification",
            outcomes.files_created
        );
    }

    // Log success
    tracing::info!("Fixture {fixture_name}: All expected outcomes verified");
}

/// Test a single fixture for debugging
#[tokio::test]
#[ignore]
async fn test_single_fixture_debug() {
    // Set up workspace
    let workspace = setup_test_workspace();
    let workspace_root = workspace.path().to_path_buf();

    // Load just one fixture for testing
    let fixture = TaskListFixture::load("tests/fixtures/task_lists/simple_implementation.json")
        .expect("Failed to load fixture");

    let fixture_name = &fixture.name;
    let initial_query = &fixture.initial_query;
    let mock_responses_len = fixture.mock_responses.len();
    println!("\n=== Testing fixture: {fixture_name} ===");
    println!("Query: {initial_query}");
    println!("Mock responses: {mock_responses_len}");

    // Create orchestrator with mock provider (thin wrapper around production code)
    let mock_provider = Arc::new(fixture.create_mock_provider()) as Arc<dyn ModelProvider>;
    let orchestrator = create_test_orchestrator(&mock_provider, &workspace_root);

    // Create task and UI channel
    let task = Task::new(fixture.initial_query.clone());
    let (tx, rx) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(tx);

    // Spawn a task to drain the UI channel
    let _drain_handle = spawn(drain_ui_channel(rx));

    // Execute task using orchestrator (handles task list execution automatically)
    println!("\n--- Executing task with orchestrator ---");
    let task_result = orchestrator
        .execute_task_streaming(task, ui_channel)
        .await
        .expect("Orchestrator execution failed");

    let response_text = &task_result.response.text;
    let validation_passed = task_result.validation.passed;
    println!("\n--- Result ---");
    println!("Response: {response_text}");
    println!("Validation passed: {validation_passed}");

    // Step 3: Check files
    println!("\n--- Step 3: Checking files ---");
    println!("Workspace root: {}", workspace_root.display());

    // List all files in workspace
    println!("\nAll files in workspace:");
    for entry in fs::read_dir(&workspace_root).unwrap() {
        let entry = entry.unwrap();
        println!("  {}", entry.path().display());
        if entry.path().is_dir() {
            for sub_entry in fs::read_dir(entry.path()).unwrap() {
                let sub_entry = sub_entry.unwrap();
                println!("    {}", sub_entry.path().display());
            }
        }
    }

    println!("\nExpected files:");
    for expected_file in &fixture.expected_outcomes.files_created {
        let file_path = workspace_root.join(expected_file);
        println!("Checking: {}", file_path.display());
        if file_path.exists() {
            let contents = fs::read_to_string(&file_path).unwrap();
            println!("  ✓ Exists ({} bytes)", contents.len());
            println!("  Contents:\n{}", contents);
        } else {
            println!("  ✗ Does not exist");
        }
    }

    // Verify outcomes
    verify_outcomes(
        &fixture.name,
        &workspace_root,
        &fixture,
        task_result.validation.passed,
    );
}

/// Test all fixtures with real agent execution using mock providers.
///
/// This is a thin wrapper around the production orchestrator - it uses the exact
/// same code path as the real application, just with mock providers instead of real APIs.
#[tokio::test]
async fn test_all_fixtures_with_agent_execution() {
    // Set up shared workspace with proper Rust project structure
    let workspace = setup_test_workspace();
    let workspace_root = workspace.path().to_path_buf();

    let fixtures = TaskListFixture::discover_fixtures("tests/fixtures/task_lists")
        .expect("Failed to discover fixtures");

    assert!(
        !fixtures.is_empty(),
        "No fixtures found in tests/fixtures/task_lists"
    );

    // Run fixtures sequentially to avoid workspace conflicts
    for fixture in fixtures {
        let workspace_root = workspace_root.clone();
        let fixture_name = fixture.name.clone();

        // Create orchestrator with mock provider (thin wrapper)
        let mock_provider = Arc::new(fixture.create_mock_provider()) as Arc<dyn ModelProvider>;
        let orchestrator = create_test_orchestrator(&mock_provider, &workspace_root);

        // Create task and UI channel
        let task = Task::new(fixture.initial_query.clone());
        let (tx, rx) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(tx);

        // Spawn task to drain UI channel
        let _drain_handle = spawn(drain_ui_channel(rx));

        // Execute task using orchestrator (handles everything: task list generation,
        // step execution, validation, and retry logic)
        let task_result = orchestrator
            .execute_task_streaming(task, ui_channel)
            .await
            .expect(&format!(
                "Orchestrator execution failed for fixture: {}",
                fixture.name
            ));

        // Verify response is not empty
        assert!(
            !task_result.response.text.is_empty(),
            "Fixture {} produced empty response",
            fixture.name
        );

        // Verify expected outcomes (files created, validation passed)
        verify_outcomes(
            &fixture.name,
            &workspace_root,
            &fixture,
            task_result.validation.passed,
        );

        // Clean up files for next fixture
        reset_workspace(&workspace_root, &fixture.expected_outcomes.files_created);

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
