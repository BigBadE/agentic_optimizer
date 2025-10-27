//! User interface (TUI) subsystem for Merlin.
//! Provides TUI rendering, event handling, state management, and persistence.

// Publicly exposed modules
/// Input event source abstraction (public so tests can inject events)
pub mod event_source;
/// Input management
pub mod input;
/// Rendering components
pub mod renderer;
/// UI state management
pub mod state;
/// Task management
pub mod task_manager;
/// Theme definitions
pub mod theme;

// Internal modules (visible for testing)
/// TUI application and main event loop (contains sub-modules)
mod app;
mod event_handler;
/// Layout calculation utilities
pub mod layout;
/// Task persistence
pub mod persistence;
/// Scrolling utilities
pub mod scroll;

// Re-exports
pub use app::TuiApp;
