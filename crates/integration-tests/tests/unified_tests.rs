//! Unified integration tests.
//!
//! Discovers and runs all unified test fixtures.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::absolute_paths,
        clippy::min_ident_chars,
        clippy::tests_outside_test_module,
        clippy::excessive_nesting,
        reason = "Allow for tests"
    )
)]

use integration_tests::{UnifiedTestRunner, VerificationResult};
use std::path::PathBuf;

/// Get fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Run a single fixture
async fn run_fixture(fixture_path: PathBuf) -> Result<VerificationResult, String> {
    let fixture_name = fixture_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid fixture name".to_owned())?;

    println!("Running fixture: {fixture_name}");

    let fixture = UnifiedTestRunner::load_fixture(&fixture_path)
        .map_err(|e| format!("Failed to load fixture {fixture_name}: {e}"))?;

    let runner = UnifiedTestRunner::new(fixture)
        .map_err(|e| format!("Failed to create runner for {fixture_name}: {e}"))?;

    runner
        .run()
        .await
        .map_err(|e| format!("Failed to run fixture {fixture_name}: {e}"))
}

/// Test all basic fixtures
#[tokio::test]
#[cfg_attr(test, allow(clippy::unwrap_used))]
async fn test_basic_fixtures() {
    let basic_dir = fixtures_dir().join("basic");
    if !basic_dir.exists() {
        println!("No basic fixtures found");
        return;
    }

    let fixtures = UnifiedTestRunner::discover_fixtures(&basic_dir).unwrap();
    println!("Found {} basic fixtures", fixtures.len());

    for fixture_path in fixtures {
        let result = run_fixture(fixture_path).await;
        match result {
            Ok(verification) => {
                if !verification.passed {
                    println!("Failures:");
                    for failure in &verification.failures {
                        println!("  - {failure}");
                    }
                    panic!("Verification failed");
                }
                println!("  ✓ Passed");
            }
            Err(e) => {
                panic!("Test failed: {e}");
            }
        }
    }
}

/// Test all execution fixtures
#[tokio::test]
#[cfg_attr(test, allow(clippy::unwrap_used))]
async fn test_execution_fixtures() {
    let execution_dir = fixtures_dir().join("execution");
    if !execution_dir.exists() {
        println!("No execution fixtures found");
        return;
    }

    let fixtures = UnifiedTestRunner::discover_fixtures(&execution_dir).unwrap();
    println!("Found {} execution fixtures", fixtures.len());

    for fixture_path in fixtures {
        let result = run_fixture(fixture_path).await;
        match result {
            Ok(verification) => {
                if !verification.passed {
                    println!("Failures:");
                    for failure in &verification.failures {
                        println!("  - {failure}");
                    }
                    panic!("Verification failed");
                }
                println!("  ✓ Passed");
            }
            Err(e) => {
                panic!("Test failed: {e}");
            }
        }
    }
}

/// Test all task list fixtures
#[tokio::test]
#[cfg_attr(test, allow(clippy::unwrap_used))]
async fn test_task_list_fixtures() {
    let task_lists_dir = fixtures_dir().join("task_lists");
    if !task_lists_dir.exists() {
        println!("No task list fixtures found");
        return;
    }

    let fixtures = UnifiedTestRunner::discover_fixtures(&task_lists_dir).unwrap();
    println!("Found {} task list fixtures", fixtures.len());

    for fixture_path in fixtures {
        let result = run_fixture(fixture_path).await;
        match result {
            Ok(verification) => {
                if !verification.passed {
                    println!("Failures:");
                    for failure in &verification.failures {
                        println!("  - {failure}");
                    }
                    panic!("Verification failed");
                }
                println!("  ✓ Passed");
            }
            Err(e) => {
                panic!("Test failed: {e}");
            }
        }
    }
}

/// Test all TUI fixtures
#[tokio::test]
#[cfg_attr(test, allow(clippy::unwrap_used))]
async fn test_tui_fixtures() {
    let tui_dir = fixtures_dir().join("tui");
    if !tui_dir.exists() {
        println!("No TUI fixtures found");
        return;
    }

    let fixtures = UnifiedTestRunner::discover_fixtures(&tui_dir).unwrap();
    println!("Found {} TUI fixtures", fixtures.len());

    for fixture_path in fixtures {
        let result = run_fixture(fixture_path).await;
        match result {
            Ok(verification) => {
                if !verification.passed {
                    println!("Failures:");
                    for failure in &verification.failures {
                        println!("  - {failure}");
                    }
                    panic!("Verification failed");
                }
                println!("  ✓ Passed");
            }
            Err(e) => {
                panic!("Test failed: {e}");
            }
        }
    }
}

/// Test all TypeScript fixtures
#[tokio::test]
#[cfg_attr(test, allow(clippy::unwrap_used))]
async fn test_typescript_fixtures() {
    let typescript_dir = fixtures_dir().join("typescript");
    if !typescript_dir.exists() {
        println!("No TypeScript fixtures found");
        return;
    }

    let fixtures = UnifiedTestRunner::discover_fixtures(&typescript_dir).unwrap();
    println!("Found {} TypeScript fixtures", fixtures.len());

    for fixture_path in fixtures {
        let result = run_fixture(fixture_path).await;
        match result {
            Ok(verification) => {
                if !verification.passed {
                    println!("Failures:");
                    for failure in &verification.failures {
                        println!("  - {failure}");
                    }
                    panic!("Verification failed");
                }
                println!("  ✓ Passed");
            }
            Err(e) => {
                panic!("Test failed: {e}");
            }
        }
    }
}

/// Test all tool fixtures
#[tokio::test]
#[cfg_attr(test, allow(clippy::unwrap_used))]
async fn test_tool_fixtures() {
    let tools_dir = fixtures_dir().join("tools");
    if !tools_dir.exists() {
        println!("No tool fixtures found");
        return;
    }

    let fixtures = UnifiedTestRunner::discover_fixtures(&tools_dir).unwrap();
    println!("Found {} tool fixtures", fixtures.len());

    for fixture_path in fixtures {
        let result = run_fixture(fixture_path).await;
        match result {
            Ok(verification) => {
                if !verification.passed {
                    println!("Failures:");
                    for failure in &verification.failures {
                        println!("  - {failure}");
                    }
                    panic!("Verification failed");
                }
                println!("  ✓ Passed");
            }
            Err(e) => {
                panic!("Test failed: {e}");
            }
        }
    }
}

/// Test all context fixtures
#[tokio::test]
#[cfg_attr(test, allow(clippy::unwrap_used))]
async fn test_context_fixtures() {
    let context_dir = fixtures_dir().join("context");
    if !context_dir.exists() {
        println!("No context fixtures found");
        return;
    }

    let fixtures = UnifiedTestRunner::discover_fixtures(&context_dir).unwrap();
    println!("Found {} context fixtures", fixtures.len());

    for fixture_path in fixtures {
        let result = run_fixture(fixture_path).await;
        match result {
            Ok(verification) => {
                if !verification.passed {
                    println!("Failures:");
                    for failure in &verification.failures {
                        println!("  - {failure}");
                    }
                    panic!("Verification failed");
                }
                println!("  ✓ Passed");
            }
            Err(e) => {
                panic!("Test failed: {e}");
            }
        }
    }
}

/// Discover all fixtures across all categories
#[tokio::test]
#[cfg_attr(test, allow(clippy::unwrap_used))]
async fn test_discover_all_fixtures() {
    let fixtures = UnifiedTestRunner::discover_fixtures(&fixtures_dir()).unwrap();
    println!("Total fixtures discovered: {}", fixtures.len());
    assert!(!fixtures.is_empty(), "Should discover at least one fixture");

    for fixture_path in &fixtures {
        println!(
            "  - {}",
            fixture_path.file_name().unwrap().to_string_lossy()
        );
    }
}
