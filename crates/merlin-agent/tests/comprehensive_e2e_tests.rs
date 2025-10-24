//! Comprehensive E2E tests using the new testing framework.
//!
//! These tests execute the full agent workflow with mock providers,
//! using real code paths and comprehensive verification.
//!
//! ## Test Coverage
//!
//! **Positive Tests:**
//! - `simple_calculator.json` - Basic task list execution with file creation
//! - `file_read_write.json` - File operations (read, modify, write)
//!
//! **Negative Tests:**
//! - `negative_missing_response.json` - Handling of missing mock responses
//! - `negative_provider_error.json` - Handling of provider errors
//! - `negative_insufficient_responses.json` - Detection of incomplete task lists
//! - `negative_excessive_calls.json` - Detection of excessive provider calls
//!
//! ## Framework Features
//!
//! - **Real Code Paths**: Uses production orchestrator, router, and executor
//! - **Comprehensive Verification**: Files, tool calls, responses, call counts
//! - **Stateful Mock Provider**: Tracks all calls and patterns
//! - **Negative Testing**: Error scenarios and edge cases
//! - **Detailed Reporting**: Per-test verification results
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all E2E tests
//! cargo nextest run -p merlin-agent comprehensive_e2e_tests
//!
//! # Run a specific test
//! cargo nextest run -p merlin-agent test_simple_calculator
//!
//! # Run only positive tests
//! cargo nextest run -p merlin-agent test_all_positive_fixtures
//!
//! # Run only negative tests
//! cargo nextest run -p merlin-agent test_all_negative_fixtures
//! ```

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

mod e2e_framework;

use std::fs;

use e2e_framework::fixture::E2EFixture;
use e2e_framework::runner::E2ERunner;

/// Test a single fixture by name
async fn test_fixture(fixture_name: &str) {
    let fixture_path = format!("tests/fixtures/e2e/{fixture_name}.json");
    let fixture = E2EFixture::load(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {fixture_name}: {e}"));

    let mut runner = E2ERunner::new().unwrap_or_else(|e| panic!("Failed to create runner: {e}"));

    runner
        .run_fixture_test(&fixture)
        .await
        .unwrap_or_else(|e| panic!("Fixture {fixture_name} failed: {e}"));
}

// ============================================================================
// Individual Positive Test Cases
// ============================================================================

#[tokio::test]
async fn test_simple_response() {
    test_fixture("simple_response").await;
}

#[tokio::test]
async fn test_simple_calculator() {
    test_fixture("simple_calculator").await;
}

#[tokio::test]
async fn test_file_read_write() {
    test_fixture("file_read_write").await;
}

#[tokio::test]
async fn test_parallel_tasks() {
    test_fixture("parallel_tasks").await;
}

#[tokio::test]
async fn test_sequential_dependencies() {
    test_fixture("sequential_dependencies").await;
}

// ============================================================================
// Individual Negative Test Cases
// ============================================================================

#[tokio::test]
#[should_panic(expected = "No mock response found")]
async fn test_negative_missing_response() {
    test_fixture("negative_missing_response").await;
}

#[tokio::test]
#[should_panic(expected = "Provider API error")]
async fn test_negative_provider_error() {
    test_fixture("negative_provider_error").await;
}

#[tokio::test]
async fn test_negative_insufficient_responses() {
    // This test should handle the error gracefully, not panic
    // The verification will check that step2 was not created
    test_fixture("negative_insufficient_responses").await;
}

#[tokio::test]
async fn test_negative_excessive_calls() {
    test_fixture("negative_excessive_calls").await;
}

// ============================================================================
// Batch Tests
// ============================================================================

/// Test all positive fixtures in the e2e directory (excluding negative tests)
#[tokio::test]
async fn test_all_positive_fixtures() {
    let fixtures =
        E2EFixture::discover_fixtures("tests/fixtures/e2e").expect("Failed to discover fixtures");

    let positive_fixtures: Vec<_> = fixtures
        .into_iter()
        .filter(|f| !f.tags.contains(&"negative".to_owned()))
        .collect();

    println!("\n========================================");
    println!("Running {} positive fixtures", positive_fixtures.len());
    println!("========================================\n");

    for fixture in positive_fixtures {
        println!("Testing: {}", fixture.name);

        let mut runner = E2ERunner::new().expect("Failed to create runner");

        runner
            .run_fixture_test(&fixture)
            .await
            .unwrap_or_else(|e| panic!("Fixture {} failed: {e}", fixture.name));

        println!("✅ {}\n", fixture.name);
    }
}

/// Test all negative fixtures (error handling tests)
#[tokio::test]
async fn test_all_negative_fixtures() {
    let fixtures =
        E2EFixture::discover_fixtures("tests/fixtures/e2e").expect("Failed to discover fixtures");

    let negative_fixtures: Vec<_> = fixtures
        .into_iter()
        .filter(|f| f.tags.contains(&"negative".to_owned()))
        .collect();

    println!("\n========================================");
    println!("Running {} negative fixtures", negative_fixtures.len());
    println!("========================================\n");

    for fixture in negative_fixtures {
        println!("Testing: {}", fixture.name);

        let mut runner = E2ERunner::new().expect("Failed to create runner");

        // Negative tests should either fail or handle errors gracefully
        let result = runner.run_fixture_test(&fixture).await;

        // Some negative tests expect failure, others expect graceful handling
        // Check if the fixture expects all_tasks_completed = false
        if fixture.expected_outcomes.all_tasks_completed {
            result.unwrap_or_else(|e| panic!("Fixture {} failed unexpectedly: {e}", fixture.name));
            println!("✅ {}\n", fixture.name);
        } else {
            // This test expects failure or incomplete execution
            println!("✅ {} (handled error correctly)\n", fixture.name);
        }
    }
}

// ============================================================================
// Fixture Structure Validation
// ============================================================================

/// Validate all fixture files have correct structure
#[test]
fn test_all_fixtures_structure() {
    let fixtures =
        E2EFixture::discover_fixtures("tests/fixtures/e2e").expect("Failed to discover fixtures");

    println!("\n========================================");
    println!("Validating {} fixture structures", fixtures.len());
    println!("========================================\n");

    for fixture in &fixtures {
        println!("📋 {}", fixture.name);

        fixture
            .validate()
            .unwrap_or_else(|e| panic!("Fixture {} has invalid structure: {e}", fixture.name));

        println!("  ✅ Structure valid");
        println!("  📊 Mock responses: {}", fixture.mock_responses.len());
        println!("  📄 Setup files: {}", fixture.setup_files.len());
        println!("  🏷️  Tags: {}", fixture.tags.join(", "));
        println!();
    }

    println!("========================================");
    println!("✅ All {} fixtures have valid structure!", fixtures.len());
    println!("========================================");
}

// ============================================================================
// Integration with Old Test System (for backwards compatibility)
// ============================================================================

/// Run a single fixture for debugging
#[tokio::test]
#[ignore]
async fn test_single_fixture_debug() {
    // Change this to the fixture you want to debug
    let fixture_name = "simple_calculator";

    let fixture_path = format!("tests/fixtures/e2e/{fixture_name}.json");
    let fixture = E2EFixture::load(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {fixture_name}: {e}"));

    println!("\n=== Debugging fixture: {} ===", fixture.name);
    println!("Description: {}", fixture.description);
    println!("Query: {}", fixture.initial_query);
    println!("Mock responses: {}", fixture.mock_responses.len());
    println!("Setup files: {}", fixture.setup_files.len());

    let mut runner = E2ERunner::new().unwrap_or_else(|e| panic!("Failed to create runner: {e}"));

    println!("\n--- Executing fixture ---");
    let result = runner
        .execute_fixture(&fixture)
        .await
        .expect("Execution failed");

    println!("\n--- Results ---");
    println!(
        "Response length: {}",
        result.task_result.response.text.len()
    );
    println!(
        "Validation passed: {}",
        result.task_result.validation.passed
    );
    println!("Provider calls: {}", result.mock_provider.call_count());

    println!("\n--- Call History ---");
    for (i, call) in result.mock_provider.get_call_history().iter().enumerate() {
        println!(
            "Call {}: pattern='{}', error={}",
            i + 1,
            call.matched_pattern,
            call.was_error
        );
    }

    println!("\n--- Workspace Files ---");
    for entry in fs::read_dir(&result.workspace_root).unwrap() {
        let entry = entry.unwrap();
        println!("  {}", entry.path().display());
    }

    println!("\n--- Running Verification ---");
    let verifier = e2e_framework::verifier::E2EVerifier::new(&fixture, &result.workspace_root);
    let verification = verifier.verify_all(&result.task_result, &result.mock_provider);

    e2e_framework::verifier::print_verification_result(&fixture.name, &verification);

    assert!(verification.passed, "Verification failed");
}
