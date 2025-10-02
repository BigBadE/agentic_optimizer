//! Agentic Optimizer application crate.
//! Provides CLI, configuration, context building, and core types/providers.

pub mod cli;
pub mod config;
pub mod context;
pub mod core;
pub mod providers;

pub use config::Config;
pub use context::ContextBuilder;
pub use core::{Context, Error, ModelProvider, Query, Response, Result, TokenUsage};
pub use providers::AnthropicProvider;
