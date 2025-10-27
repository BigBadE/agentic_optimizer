//! Library interface for merlin-cli
//!
//! Exposes core components for integration testing
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

// Include all modules so their unit tests run
// These modules are only used by main.rs (the binary), but need to be
// included here so their unit tests are compiled and run
mod cli;
mod config;
mod handlers;
mod interactive;
pub mod ui;
mod utils;

// Public API exports for integration testing
pub use ui::event_source::InputEventSource;
pub use ui::renderer::FocusedPane;
pub use ui::state::UiState;
pub use ui::task_manager::TaskManager;
pub use ui::{TuiApp, UiChannel, UiEvent};
