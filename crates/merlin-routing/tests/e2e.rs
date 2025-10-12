//! End-to-end tests

#[path = "common/mod.rs"]
pub mod common;

#[path = "scenario_runner.rs"]
mod scenario_runner;

// Tools tests
#[path = "e2e/tools/tools_e2e_tests.rs"]
mod tools_e2e_tests;

// TypeScript tests
#[path = "e2e/typescript/typescript_integration_tests.rs"]
mod typescript_integration_tests;

// Progress tests
#[path = "e2e/progress/progress_callback_e2e_tests.rs"]
mod progress_callback_e2e_tests;

// Unified JSON scenario tests
#[path = "e2e/unified_e2e_tests.rs"]
mod unified_e2e_tests;
