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

use integration_tests::UnifiedTestRunner;
use std::path::PathBuf;
use std::env;
use tracing::info;

/// Get routing fixtures directory
fn routing_fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("routing")
        .join("fixtures")
}

/// Run a single fixture
async fn run_fixture(fixture_path: PathBuf) -> Result<(), String> {
    let fixture_name = fixture_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid fixture name".to_owned())?;

    info!("Running fixture: {fixture_name}");

    let fixture = UnifiedTestRunner::load_fixture(&fixture_path)
        .map_err(|e| format!("Failed to load fixture {fixture_name}: {e}"))?;

    let runner = UnifiedTestRunner::new(fixture)
        .map_err(|e| format!("Failed to create runner for {fixture_name}: {e}"))?;

    let result = runner
        .run()
        .await
        .map_err(|e| format!("Failed to run fixture {fixture_name}: {e}"))?;

    if !result.passed {
        let failures_msg = result.failures.join("\n  - ");
        return Err(format!("Verification failed:\n  - {failures_msg}"));
    }

    Ok(())
}

#[tokio::test]
async fn test_routing_fixtures() {
    let fixtures_root = routing_fixtures_dir();
    if !fixtures_root.exists() {
        info!("No routing fixtures found at {}", fixtures_root.display());
        return;
    }

    let fixtures = match UnifiedTestRunner::discover_fixtures(&fixtures_root) {
        Ok(f) => f,
        Err(e) => {
            info!("Failed to discover fixtures: {e}");
            return;
        }
    };

    if fixtures.is_empty() {
        info!("No fixtures found in {}", fixtures_root.display());
        return;
    }

    info!("Discovered {} routing fixtures", fixtures.len());

    let mut passed = 0;
    let mut failed = Vec::new();

    for fixture_path in fixtures {
        info!("{}", "=".repeat(60));
        match run_fixture(fixture_path.clone()).await {
            Ok(()) => {
                passed += 1;
                info!("✓ PASSED");
            }
            Err(error) => {
                let name = fixture_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                failed.push((name.to_owned(), error.clone()));
                info!("✗ FAILED: {error}");
            }
        }
    }

    info!("{}", "=".repeat(60));
    let failed_count = failed.len();
    info!("Results: {passed} passed, {failed_count} failed");

    if !failed.is_empty() {
        info!("Failed fixtures:");
        for (name, error) in &failed {
            info!("  ✗ {name}: {error}");
        }
        panic!("{} fixture(s) failed", failed.len());
    }
}
