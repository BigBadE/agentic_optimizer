//! Tool implementations for the agentic optimizer.
//!
//! This crate provides various tools that agents can use to interact with
//! the file system and execute commands.

mod tool;
mod edit;
mod show;
mod bash;

pub use tool::{Tool, ToolError, ToolResult};
pub use edit::EditTool;
pub use show::ShowTool;
pub use bash::BashTool;
