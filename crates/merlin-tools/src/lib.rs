//! Tool implementations for the agentic optimizer.
//!
//! This crate provides various tools that agents can use to interact with
//! the file system and execute commands.

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
