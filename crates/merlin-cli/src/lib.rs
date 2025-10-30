//! Library interface for merlin-cli
//!
//! Exposes core components for integration testing
#![cfg_attr(
    test,
    allow(
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        reason = "Allow for tests"
    )
)]

// Modules needed by the UI (pub(crate) so lib can use them)
pub(crate) mod config;

// UI module is public for integration testing
pub mod ui;

// Public API exports for integration testing
pub use ui::TuiApp;
pub use ui::event_source::InputEventSource;
pub use ui::renderer::FocusedPane;
pub use ui::state::{ConversationRole, UiState};
pub use ui::task_manager::TaskManager;
