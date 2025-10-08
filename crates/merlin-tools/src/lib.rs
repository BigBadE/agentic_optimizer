//! Tool implementations for the agentic optimizer.
//!
//! This crate provides various tools that agents can use to interact with
//! the file system and execute commands.
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

/// Shell execution tool implementation.
mod bash;
/// Editing tool implementation.
mod edit;
/// File viewing tool implementation.
mod show;
/// Core abstractions shared by all tools.
mod tool;

pub use bash::BashTool;
pub use edit::EditTool;
pub use show::ShowTool;
pub use tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};
