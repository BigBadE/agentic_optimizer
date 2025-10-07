//! Tool implementations for the agentic optimizer.
//!
//! This crate provides various tools that agents can use to interact with
//! the file system and execute commands.


/// Core abstractions shared by all tools.
mod tool;
/// Editing tool implementation.
mod edit;
/// File viewing tool implementation.
mod show;
/// Shell execution tool implementation.
mod bash;

pub use tool::{Tool, ToolError, ToolResult, ToolInput, ToolOutput};
pub use edit::EditTool;
pub use show::ShowTool;
pub use bash::BashTool;
