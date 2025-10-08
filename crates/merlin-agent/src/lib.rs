//! Agent implementation for the agentic optimizer.
//!
//! This crate provides the core agent functionality that orchestrates
//! context gathering, provider interaction, and response generation.
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
mod agent;
mod config;
mod executor;
mod tools;
mod types;

pub use agent::Agent;
pub use config::AgentConfig;
pub use executor::AgentExecutor;
pub use tools::ToolRegistry;
pub use types::{AgentRequest, AgentResponse, ExecutionMetadata, ExecutionResult};
