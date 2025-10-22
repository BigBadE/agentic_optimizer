//! Merlin CLI library
//!
//! Exposes internal modules for testing purposes.

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

pub mod config;
pub mod ui;

// Re-export commonly used types for testing
pub use ui::TuiApp;
