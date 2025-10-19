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
/// Tool registry for managing available tools.
mod registry;
/// TypeScript/JavaScript runtime using QuickJS.
mod runtime;
/// TypeScript signature generation from tool schemas.
mod signatures;
/// Core abstractions shared by all tools.
mod tool;

pub use bash::BashTool;
pub use registry::ToolRegistry;
pub use runtime::TypeScriptRuntime;
pub use signatures::generate_typescript_signatures;
pub use tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};
