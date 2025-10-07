//! Agent implementation for the agentic optimizer.
//!
//! This crate provides the core agent functionality that orchestrates
//! context gathering, provider interaction, and response generation.


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
