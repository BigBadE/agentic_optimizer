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
mod task_execution;
mod task_operations;

mod thread_operations;
pub mod tui_app;

pub use tui_app::TuiApp;
