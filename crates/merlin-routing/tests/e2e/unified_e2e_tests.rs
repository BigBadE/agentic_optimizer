//! Unified E2E tests using JSON scenarios
//!
//! All E2E tests are now defined in JSON files in tests/fixtures/scenarios/
//! This provides a declarative, maintainable way to test the full system.
//!
//! This test automatically discovers and runs all .json files in the scenarios directory.

#[path = "../scenario_runner.rs"]
mod scenario_runner;

use scenario_runner::ScenarioRunner;
use std::fs;
use std::path::PathBuf;

/// Discovers all JSON scenario files in the fixtures/scenarios directory (including subdirectories)
fn discover_scenarios() -> Vec<String> {
    let scenarios_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scenarios");

    let mut scenarios = Vec::new();

    fn scan_dir(dir: &PathBuf, base: &PathBuf, scenarios: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, base, scenarios);
                } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(relative) = path.strip_prefix(base) {
                        if let Some(relative_str) = relative.to_str() {
                            // Convert path to forward slashes and remove .json extension
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
        }
    }

    scan_dir(&scenarios_dir, &scenarios_dir, &mut scenarios);
    scenarios.sort();
    scenarios
}

#[tokio::test]
async fn run_all_json_scenarios() {
    let scenarios = discover_scenarios();

    if scenarios.is_empty() {
        panic!("No JSON scenarios found in tests/fixtures/scenarios/");
    }

    println!("\nDiscovered {} scenarios:", scenarios.len());
    for scenario in &scenarios {
        println!("  - {}", scenario);
    }
    println!();

    let mut passed = 0;
    let mut failed = Vec::new();

    for scenario_name in &scenarios {
        println!("\n{}", "=".repeat(60));
        match ScenarioRunner::load(scenario_name) {
            Ok(runner) => match runner.run().await {
                Ok(()) => {
                    passed += 1;
                    println!("✓ {} PASSED", scenario_name);
                }
                Err(e) => {
                    failed.push((scenario_name.clone(), e.clone()));
                    println!("✗ {} FAILED: {}", scenario_name, e);
                }
            },
            Err(e) => {
                failed.push((scenario_name.clone(), format!("Failed to load: {}", e)));
                println!("✗ {} FAILED TO LOAD: {}", scenario_name, e);
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Results: {} passed, {} failed", passed, failed.len());

    if !failed.is_empty() {
        println!("\nFailed scenarios:");
        for (name, error) in &failed {
            println!("  ✗ {}: {}", name, error);
        }
        panic!("{} scenario(s) failed", failed.len());
    }
}
