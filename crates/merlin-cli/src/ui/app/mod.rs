//! TUI application module
//!
//! This module contains the main TUI application logic, organized into focused sub-modules.

pub mod conversation;
pub mod input_handler;
pub mod navigation;

// TUI application implementation modules
mod event_loop;
mod key_handling;
mod lifecycle;
mod task_operations;
mod test_helpers;
#[cfg(feature = "test-util")]
mod test_util;
mod thread_operations;
mod tui_app;

pub use tui_app::TuiApp;
