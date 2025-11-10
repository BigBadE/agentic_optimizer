//! Library interface for merlin-cli
//!
//! Exposes core components for integration testing

// Modules needed by the UI and integration tests
pub mod config;

// UI module is public for integration testing
pub mod ui;

// Public API exports for integration testing
pub use ui::TuiApp;
pub use ui::app;
pub use ui::event_handler;
pub use ui::event_source::{EventFuture, InputEventSource};
pub use ui::input::InputManager;
pub use ui::layout;
pub use ui::persistence::TaskPersistence;
pub use ui::renderer::{FocusedPane, Renderer};
pub use ui::state::{ConversationRole, UiState};
pub use ui::task_manager::TaskManager;
