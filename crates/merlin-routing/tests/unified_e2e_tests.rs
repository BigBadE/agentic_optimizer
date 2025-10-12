//! Unified E2E tests using JSON scenarios
//!
//! All E2E tests are now defined in JSON files in tests/fixtures/scenarios/
//! This provides a declarative, maintainable way to test the full system.

mod scenario_runner;

use scenario_runner::ScenarioRunner;

#[tokio::test]
async fn test_input_submission() {
    let runner = ScenarioRunner::load("input_submission").unwrap();
    runner.run().await.unwrap();
}

#[tokio::test]
async fn test_user_input_basic() {
    let runner = ScenarioRunner::load("user_input_basic").unwrap();
    runner.run().await.unwrap();
}

#[tokio::test]
async fn test_vector_cache_immediate_start() {
    let runner = ScenarioRunner::load("vector_cache_immediate_start").unwrap();
    runner.run().await.unwrap();
}

#[tokio::test]
async fn test_unified_example() {
    let runner = ScenarioRunner::load("unified_example").unwrap();
    runner.run().await.unwrap();
}
