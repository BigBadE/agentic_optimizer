//! TypeScript-based tooling system for the agentic optimizer.
//!
//! This crate provides the core tooling infrastructure including:
//! - Tool trait and registry for managing available tools
//! - `BashTool` for shell command execution
//! - TypeScript runtime with `QuickJS` for executing agent code
//! - TypeScript signature generation from tool schemas
#![cfg_attr(
    test,
    allow(
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        reason = "Allow for tests"
    )
)]

/// Shell execution tool implementation.
mod bash;
/// Dynamic context request tool for agents.
pub mod context_request;
/// File deletion tool.
mod delete_tool;
/// File editing tool for find-and-replace operations.
mod edit_tool;
/// File operation tools (read, write, list).
mod file_ops;
/// Tool registry for managing available tools.
mod registry;
/// TypeScript/JavaScript runtime using QuickJS.
mod runtime;
/// TypeScript signature generation from tool schemas.
mod signatures;
/// Core abstractions shared by all tools.
mod tool;

pub use bash::BashTool;
pub use context_request::{
    ContextFile, ContextRequestArgs, ContextRequestResult, ContextRequestTool, ContextTracker,
};
pub use delete_tool::DeleteFileTool;
pub use edit_tool::EditFileTool;
pub use file_ops::{ListFilesTool, ReadFileTool, WriteFileTool};
pub use registry::ToolRegistry;
pub use runtime::TypeScriptRuntime;
pub use signatures::generate_typescript_signatures;
pub use tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};
