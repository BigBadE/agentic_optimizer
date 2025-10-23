//! Integration test framework for Merlin
//!
//! Provides unified E2E testing combining:
//! - TUI interaction testing (via `InputEventSource`)
//! - Agent execution testing (via `MockProvider`)
//! - Orchestrator flow testing
//! - Full stack integration scenarios

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
        reason = "Allow for tests"
    )
)]

pub mod scenario;
pub mod tui_helpers;
pub mod types;

pub use scenario::ScenarioRunner;
pub use types::{Scenario, ScenarioStep, StepAction, StepExpectations};

// Re-export for type definitions
pub use serde_json;
