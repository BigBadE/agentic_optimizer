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
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use integration_tests::{UnifiedTestRunner, VerificationResult};
use std::fs::read_dir;
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
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Invalid fixture name".to_owned())?;

    let fixture = UnifiedTestRunner::load_fixture(&fixture_path)
        .map_err(|error| format!("Failed to load fixture {fixture_name}: {error}"))?;

    let runner = UnifiedTestRunner::new(fixture)
        .map_err(|error| format!("Failed to create runner for {fixture_name}: {error}"))?;

    runner
        .run()
        .await
        .map_err(|error| format!("Failed to run fixture {fixture_name}: {error}"))
}

/// Helper to run all fixtures in a directory
async fn run_fixtures_in_dir(dir: PathBuf) -> Vec<(String, Result<VerificationResult, String>)> {
    if !dir.exists() {
        return vec![];
    }

    let fixtures = UnifiedTestRunner::discover_fixtures(&dir).unwrap();
    let mut results = vec![];

    for fixture_path in fixtures {
        let fixture_name = fixture_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_owned();

        let result = run_fixture(fixture_path).await;
        results.push((fixture_name, result));
    }

    results
}

/// Run all fixtures in the fixtures directory
#[tokio::test]
async fn test_all_fixtures() {
    let fixtures_root = fixtures_dir();

    // Discover all subdirectories
    let subdirs = read_dir(&fixtures_root)
        .expect("Failed to read fixtures directory")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    let mut all_results = vec![];
    for subdir in subdirs {
        let results = run_fixtures_in_dir(subdir).await;
        all_results.extend(results);
    }

    // Collect all results for final report
    let mut failures_with_details = vec![];
    let mut passed = vec![];

    for (fixture_name, result) in all_results {
        match result {
            Ok(verification) if verification.passed => {
                passed.push(fixture_name);
            }
            Ok(verification) => {
                failures_with_details.push((fixture_name, verification));
            }
            Err(error) => {
                // Create a minimal VerificationResult for errors
                let mut error_result = VerificationResult::new();
                error_result.add_failure(error);
                failures_with_details.push((fixture_name, error_result));
            }
        }
    }

    // Print complete summary at the end
    println!("\n=== Test Summary ===");
    println!(
        "Total fixtures: {}",
        passed.len() + failures_with_details.len()
    );
    println!("Passed: {}", passed.len());
    println!("Failed: {}", failures_with_details.len());

    if failures_with_details.is_empty() {
        println!("\nAll fixtures passed! ✓");
    } else {
        println!("\n=== Failed Fixtures ===");
        for (fixture_name, verification) in &failures_with_details {
            println!("\n{fixture_name}:");
            for failure in &verification.failures {
                println!("  - {failure}");
            }
        }

        println!("\n=== Passed Fixtures ===");
        for name in &passed {
            println!("  ✓ {name}");
        }

        panic!("{} fixture(s) failed", failures_with_details.len());
    }
}
