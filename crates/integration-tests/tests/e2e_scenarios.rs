//! E2E scenario test runner
//!
//! Automatically discovers and runs all JSON scenarios in tests/fixtures/scenarios/

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
        unsafe_code,
        reason = "Test allows"
    )
)]

use integration_tests::ScenarioRunner;
use num_cpus::get as get_cpu_count;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs};
use tokio::{spawn, sync::Semaphore};
use tracing_subscriber::fmt;
use tracing_subscriber::{
    EnvFilter, layer::SubscriberExt as _, registry, util::SubscriberInitExt as _,
};

/// Initialize tracing for tests
fn init_tracing() {
    drop(
        registry()
            .with(fmt::layer().with_test_writer().with_target(false))
            .with(EnvFilter::from_default_env())
            .try_init(),
    );
}

/// Recursively scan directory for JSON files
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
                scenarios.push(scenario_name);
            }
        }
    }
}

/// Discover all scenarios in fixtures/scenarios directory
fn discover_scenarios() -> Vec<String> {
    let scenarios_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scenarios");

    let mut scenarios = Vec::new();
    if scenarios_dir.exists() {
        scan_dir(&scenarios_dir, &scenarios_dir, &mut scenarios);
    }
    scenarios.sort();
    scenarios
}

#[tokio::test]
async fn run_all_scenarios() {
    init_tracing();

    // Set MERLIN_FOLDER for all tests
    // SAFETY: Setting environment variable in test setup before any concurrent access
    unsafe {
        env::set_var(
            "MERLIN_FOLDER",
            env::temp_dir().join("merlin_integration_tests"),
        );
    }

    let scenarios = discover_scenarios();

    if scenarios.is_empty() {
        println!("No scenarios found in tests/fixtures/scenarios/");
        println!("Skipping E2E tests (this is expected for initial setup)");
        return;
    }

    println!("Discovered {} scenarios:", scenarios.len());
    for scenario in &scenarios {
        println!("  - {scenario}");
    }
    println!();

    // Run scenarios in parallel with a concurrency limit
    let cpu_count = get_cpu_count();
    let semaphore = Arc::new(Semaphore::new(cpu_count));
    let mut tasks = Vec::new();

    for scenario_name in scenarios {
        let sem = Arc::clone(&semaphore);
        let task = spawn(async move {
            let _permit = sem.acquire().await.expect("Failed to acquire semaphore");

            let result = match ScenarioRunner::load(&scenario_name) {
                Ok(runner) => match runner.run().await {
                    Ok(()) => Ok(()),
                    Err(error) => Err(error.to_string()),
                },
                Err(error) => Err(format!("Failed to load: {error:?}")),
            };

            (scenario_name, result)
        });
        tasks.push(task);
    }

    // Collect results
    let mut passed = 0;
    let mut failed = Vec::new();

    for task in tasks {
        match task.await {
            Ok((scenario_name, Ok(()))) => {
                passed += 1;
                println!("✓ {scenario_name} PASSED");
            }
            Ok((scenario_name, Err(error))) => {
                failed.push((scenario_name.clone(), error.clone()));
                println!("✗ {scenario_name} FAILED: {error}");
            }
            Err(join_error) => {
                failed.push(("unknown".to_string(), format!("Join error: {join_error:?}")));
                println!("✗ Task join failed: {join_error:?}");
            }
        }
    }

    println!("{}", "=".repeat(60));
    println!("Results: {passed} passed, {} failed", failed.len());

    if !failed.is_empty() {
        println!("\nFailed scenarios:");
        for (name, error) in &failed {
            println!("  ✗ {name}");
            println!("    {error}");
        }
        panic!("{} scenario(s) failed", failed.len());
    }
}
