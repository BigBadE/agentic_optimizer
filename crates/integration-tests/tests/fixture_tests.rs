//! Unified integration tests.
//!
//! Discovers and runs all unified test fixtures.
#![cfg_attr(
    test,
    allow(
        clippy::tests_outside_test_module,
        reason = "Allow for integration tests"
    )
)]

use futures::stream::{self, StreamExt as _};
use integration_tests::{UnifiedTestRunner, VerificationResult};
use std::fs::read_dir;
use std::path::PathBuf;
use std::time::Instant;

/// Run a single fixture
///
/// # Errors
/// Returns an error if fixture loading or execution fails
async fn run_fixture(fixture_path: PathBuf) -> Result<VerificationResult, String> {
    let fixture_name = fixture_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Invalid fixture name".to_owned())?;

    let start = Instant::now();
    let fixture = UnifiedTestRunner::load_fixture(&fixture_path)
        .map_err(|error| format!("Failed to load fixture {fixture_name}: {error}"))?;

    let mut runner = UnifiedTestRunner::new(fixture)
        .await
        .map_err(|error| format!("Failed to create runner for {fixture_name}: {error}"))?;

    let result = runner
        .run()
        .await
        .map_err(|error| format!("Failed to run fixture {fixture_name}: {error}"));

    let elapsed = start.elapsed();
    if elapsed.as_secs() >= 1 {
        tracing::debug!("[SLOW] {fixture_name} took {elapsed:?}");
    }

    result
}

/// Helper to run all fixtures in a directory in parallel
async fn run_fixtures_in_dir(dir: PathBuf) -> Vec<(String, Result<VerificationResult, String>)> {
    let dir_name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "unknown".to_owned(), ToString::to_string);
    let fixtures = UnifiedTestRunner::discover_fixtures(&dir).unwrap_or_default();
    let start = Instant::now();

    // Run fixtures in parallel with buffer_unordered
    let results = stream::iter(fixtures)
        .map(|fixture_path| async move {
            let fixture_name = fixture_path
                .file_name()
                .and_then(|name| name.to_str())
                .map_or_else(|| "unknown".to_owned(), ToString::to_string);

            let result = run_fixture(fixture_path).await;
            (fixture_name, result)
        })
        .buffer_unordered(16) // Balanced concurrency for optimal throughput
        .collect()
        .await;

    let elapsed = start.elapsed();
    tracing::debug!("[DIR] {dir_name} took {elapsed:?}");

    results
}

/// Run all fixtures in the fixtures directory
///
/// # Panics
/// Panics if any fixtures fail verification
#[tokio::test]
async fn test_all_fixtures() {
    let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    // Discover all subdirectories
    let subdirs = match read_dir(&fixtures_root) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.path())
            .collect::<Vec<_>>(),
        Err(err) => {
            tracing::error!("Failed to read fixtures directory: {err}");
            Vec::new()
        }
    };

    let subdir_count = subdirs.len();

    // Run all subdirectories in parallel
    let all_results: Vec<_> = stream::iter(subdirs)
        .map(|subdir| async move { run_fixtures_in_dir(subdir).await })
        .buffer_unordered(subdir_count.max(1)) // Process all subdirs concurrently
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect();

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
    tracing::info!("\n=== Test Summary ===");
    tracing::info!("{} passed", passed.len());

    if failures_with_details.is_empty() {
        tracing::info!("\nAll fixtures passed! ✓");
    } else {
        tracing::error!("{} failed\n", failures_with_details.len());
        tracing::error!("=== Failed Fixtures ===");
        for (fixture_name, verification) in &failures_with_details {
            tracing::error!("\n{fixture_name}:");
            for failure in &verification.failures {
                tracing::error!("  - {failure}");
            }
        }

        assert!(
            failures_with_details.is_empty(),
            "{} fixture(s) failed",
            failures_with_details.len()
        );
    }
}
