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
/// File deletion tool implementation.
mod delete;
/// Editing tool implementation.
mod edit;
/// Directory listing tool implementation.
mod list;
/// File viewing tool implementation.
mod show;
/// Core abstractions shared by all tools.
mod tool;
/// TypeScript runtime for natural LLM tool calling.
mod typescript_runtime;

pub use bash::BashTool;
pub use delete::DeleteTool;
pub use edit::EditTool;
pub use list::ListTool;
pub use show::ShowTool;
pub use tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};
pub use typescript_runtime::{TypeScriptRuntime, TypeScriptTool};
