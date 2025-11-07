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
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::abort;
use std::time::{Duration, Instant};
use tokio::runtime::Builder;
use tokio::task::{LocalSet, spawn_blocking};
use tracing::{debug, error, info};

#[cfg(feature = "timing-layer")]
use integration_tests::{TimingData, TimingLayer};
#[cfg(feature = "timing-layer")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "timing-layer")]
use tracing::subscriber::set_global_default;
#[cfg(feature = "timing-layer")]
use tracing_subscriber::layer::SubscriberExt as _;
#[cfg(feature = "timing-layer")]
use tracing_subscriber::registry;

/// Result with timing and category information
struct FixtureRunResult {
    result: Result<VerificationResult, String>,
    duration: Duration,
    category: String,
    name: String,
}

/// Run a single fixture
///
/// # Errors
/// Returns an error if fixture loading or execution fails
async fn run_fixture(fixture_path: PathBuf) -> FixtureRunResult {
    let fixture_name = fixture_path
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "unknown".to_owned(), ToString::to_string);

    // Extract category from parent directory
    let category = fixture_path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .map_or_else(|| "unknown".to_owned(), ToString::to_string);

    let start = Instant::now();

    let result = async {
        let fixture = UnifiedTestRunner::load_fixture(&fixture_path)
            .map_err(|error| format!("Failed to load fixture {fixture_name}: {error}"))?;

        let mut runner = UnifiedTestRunner::new(fixture)
            .map_err(|error| format!("Failed to create runner for {fixture_name}: {error}"))?;

        runner
            .run()
            .await
            .map_err(|error| format!("Failed to run fixture {fixture_name}: {error}"))
    }
    .await;

    let duration = start.elapsed();
    if duration.as_secs() >= 1 {
        debug!("[SLOW] {fixture_name} took {duration:?}");
    }

    FixtureRunResult {
        result,
        duration,
        category,
        name: fixture_name,
    }
}

/// Collect and run all fixtures from subdirectories
async fn collect_and_run_fixtures() -> Vec<FixtureRunResult> {
    let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    // Discover all subdirectories
    let subdirs = read_dir(&fixtures_root).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.path())
                .collect::<Vec<_>>()
        },
    );

    let subdir_count = subdirs.len();

    // Run all subdirectories in parallel
    stream::iter(subdirs)
        .map(|subdir| async move { run_fixtures_in_dir(subdir).await })
        .buffer_unordered(subdir_count.max(1))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// Print category timing breakdown
#[cfg(feature = "timing-layer")]
fn print_category_stats(
    category_stats: &HashMap<String, (usize, Duration)>,
    timing_data: &Arc<Mutex<TimingData>>,
) {
    info!("\n=== Per-Category Timing Breakdown ===");
    info!(
        "{:<20} {:>6} {:>10} {:>10}",
        "Category", "Count", "Total", "Avg"
    );
    info!("{}", "-".repeat(52));

    let mut categories: Vec<_> = category_stats.iter().collect();
    categories.sort_by_key(|(_, (_, total))| Reverse(*total));

    for (category, (count, total)) in categories {
        let avg = total.as_secs_f64() / (*count as f64);
        info!(
            "{:<20} {:>6} {:>9.2}s {:>9.3}s",
            category,
            count,
            total.as_secs_f64(),
            avg
        );
    }
    info!("{}", "=".repeat(52));

    // Print timing report
    let Ok(timing) = timing_data.lock() else {
        return;
    };
    timing.print_report();
}

#[cfg(not(feature = "timing-layer"))]
fn print_category_stats(category_stats: &HashMap<String, (usize, Duration)>) {
    info!("\n=== Per-Category Timing Breakdown ===");
    info!(
        "{:<20} {:>6} {:>10} {:>10}",
        "Category", "Count", "Total", "Avg"
    );
    info!("{}", "-".repeat(52));

    let mut categories: Vec<_> = category_stats.iter().collect();
    categories.sort_by_key(|(_, (_, total))| Reverse(*total));

    for (category, (count, total)) in categories {
        let avg = total.as_secs_f64() / (*count as f64);
        info!(
            "{:<20} {:>6} {:>9.2}s {:>9.3}s",
            category,
            count,
            total.as_secs_f64(),
            avg
        );
    }
    info!("{}", "=".repeat(52));
}

/// Helper to run all fixtures in a directory in parallel
///
/// # Panics
/// Panics if any spawned fixture task panics during execution
async fn run_fixtures_in_dir(dir: PathBuf) -> Vec<FixtureRunResult> {
    let dir_name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "unknown".to_owned(), ToString::to_string);
    let fixtures = UnifiedTestRunner::discover_fixtures(&dir).unwrap_or_default();
    let start = Instant::now();

    // Run fixtures in parallel with true multi-threading
    // Each fixture runs in its own thread with its own LocalSet
    let task_results = stream::iter(fixtures)
        .map(|fixture_path| {
            spawn_blocking(move || {
                // Create a new single-threaded runtime for this fixture
                let runtime = Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap_or_else(|build_error| {
                        // Tests are allowed to abort on critical failures
                        error!("FATAL: Failed to create runtime for fixture: {build_error}");
                        abort();
                    });

                // Run the fixture in a LocalSet
                LocalSet::new().block_on(&runtime, run_fixture(fixture_path))
            })
        })
        .buffer_unordered(16) // Balanced concurrency for optimal throughput
        .collect::<Vec<_>>()
        .await;

    // Unwrap task join results - if any task panicked, propagate it
    let mut results = Vec::with_capacity(task_results.len());
    for task_result in task_results {
        match task_result {
            Ok(fixture_result) => results.push(fixture_result),
            Err(join_err) => {
                // Task panicked - create a failed fixture result
                let error_msg = format!("Fixture task panicked: {join_err}");
                results.push(FixtureRunResult {
                    result: Err(error_msg),
                    duration: Duration::ZERO,
                    category: "unknown".to_owned(),
                    name: "panicked_fixture".to_owned(),
                });
            }
        }
    }

    let elapsed = start.elapsed();
    tracing::debug!("[DIR] {dir_name} took {elapsed:?}");

    results
}

/// Run all fixtures in the fixtures directory
///
/// # Panics
/// Panics if any fixtures fail verification
#[tokio::test(flavor = "multi_thread")]
async fn test_all_fixtures() {
    // Each fixture runs in its own thread with its own LocalSet
    // This provides true parallelism while supporting !Send TypeScript runtime
    run_all_fixtures_impl().await;
}

/// Implementation of fixture runner
///
/// # Panics
/// Panics if any fixtures fail verification
async fn run_all_fixtures_impl() {
    #[cfg(feature = "timing-layer")]
    let timing_data = {
        use tracing_subscriber::fmt;

        let (timing_layer, timing_data) = TimingLayer::new();
        let subscriber = registry().with(timing_layer).with(fmt::layer());
        drop(set_global_default(subscriber));
        Some(timing_data)
    };

    let all_results = collect_and_run_fixtures().await;

    // Collect all results for final report and timing analysis
    let mut failures_with_details = vec![];
    let mut passed = vec![];
    let mut category_stats: HashMap<String, (usize, Duration)> = HashMap::new();

    for fixture_result in all_results {
        // Update category statistics
        let entry = category_stats
            .entry(fixture_result.category.clone())
            .or_insert((0, Duration::ZERO));
        entry.0 += 1;
        entry.1 += fixture_result.duration;

        match fixture_result.result {
            Ok(verification) if verification.passed => {
                passed.push(fixture_result.name);
            }
            Ok(verification) => {
                failures_with_details.push((fixture_result.name, verification));
            }
            Err(error) => {
                // Create a minimal VerificationResult for errors
                let mut error_result = VerificationResult::new();
                error_result.add_failure(error);
                failures_with_details.push((fixture_result.name, error_result));
            }
        }
    }

    // Print complete summary at the end
    info!("\n=== Test Summary ===");
    info!("{} passed", passed.len());

    if failures_with_details.is_empty() {
        info!("\nAll fixtures passed! ✓");
    } else {
        // Log failures before assertion for better debugging
        tracing::error!("\n{} failed\n", failures_with_details.len());
        tracing::error!("=== Failed Fixtures ===");
        for (fixture_name, verification) in &failures_with_details {
            tracing::error!("\n{fixture_name}:");
            for failure in &verification.failures {
                tracing::error!("  - {failure}");
            }
        }

        // Build detailed failure message by iterating fixtures
        let failure_count = failures_with_details.len();
        let mut failure_msg = String::with_capacity(failure_count * 100);
        failure_msg.push_str(&failure_count.to_string());
        failure_msg.push_str(" fixture(s) failed:\n");

        for (fixture_name, verification) in &failures_with_details {
            failure_msg.push('\n');
            failure_msg.push_str(fixture_name);
            failure_msg.push_str(":\n");

            for failure in &verification.failures {
                failure_msg.push_str("  - ");
                failure_msg.push_str(failure);
                failure_msg.push('\n');
            }
        }

        assert!(failures_with_details.is_empty(), "{failure_msg}");
    }

    #[cfg(feature = "timing-layer")]
    if let Some(timing_data) = timing_data {
        print_category_stats(&category_stats, &timing_data);
    }

    #[cfg(not(feature = "timing-layer"))]
    print_category_stats(&category_stats);
}
