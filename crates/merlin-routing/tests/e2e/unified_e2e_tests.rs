//! Unified E2E tests using JSON scenarios
//!
//! All E2E tests are now defined in JSON files in tests/fixtures/scenarios/
//! This provides a declarative, maintainable way to test the full system.
//!
//! This test automatically discovers and runs all .json files in the scenarios directory.

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
        reason = "Test allows"
    )
)]

use crate::ScenarioRunner;
use crate::common::init_tracing;
use std::fs;
use std::path::PathBuf;
use tracing::info;

/// Recursively scan a directory for `.json` files and collect scenario names.
fn scan_dir(dir: &PathBuf, base: &PathBuf, scenarios: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, base, scenarios);
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && let Ok(relative) = path.strip_prefix(base)
                && let Some(relative_str) = relative.to_str()
            {
                let scenario_name = relative_str
                    .replace('\\', "/")
                    .trim_end_matches(".json")
                    .to_string();
                if !scenario_name.contains("SCHEMA") {
                    scenarios.push(scenario_name);
                }
            }
        }
    }
}

/// Discovers all JSON scenario files in the fixtures/scenarios directory (including subdirectories)
fn discover_scenarios() -> Vec<String> {
    let scenarios_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scenarios");

    let mut scenarios = Vec::new();
    scan_dir(&scenarios_dir, &scenarios_dir, &mut scenarios);
    scenarios.sort();
    scenarios
}

#[tokio::test]
async fn run_all_json_scenarios() {
    // Ensure tracing is initialized for test logs
    init_tracing();

    let scenarios = discover_scenarios();

    assert!(
        !scenarios.is_empty(),
        "No JSON scenarios found in tests/fixtures/scenarios/"
    );

    info!("Discovered {} scenarios:", scenarios.len());
    for scenario in &scenarios {
        info!("  - {scenario}");
    }
    info!("");

    let mut passed = 0;
    let mut failed = Vec::new();

    for scenario_name in &scenarios {
        info!("{}", "=".repeat(60));
        match ScenarioRunner::load(scenario_name) {
            Ok(runner) => match runner.run().await {
                Ok(()) => {
                    passed += 1;
                    info!("✓ {scenario_name} PASSED");
                }
                Err(error) => {
                    failed.push((scenario_name.clone(), error.clone()));
                    info!("✗ {scenario_name} FAILED: {error}");
                }
            },
            Err(error) => {
                failed.push((scenario_name.clone(), format!("Failed to load: {error}")));
                info!("✗ {scenario_name} FAILED TO LOAD: {error}");
            }
        }
    }

    info!("{}", "=".repeat(60));
    let failed_count = failed.len();
    info!("Results: {passed} passed, {failed_count} failed");

    if !failed.is_empty() {
        info!("Failed scenarios:");
        for (name, error) in &failed {
            info!("  ✗ {name}: {error}");
        }
        panic!("{} scenario(s) failed", failed.len());
    }
}
