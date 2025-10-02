//! Context building utilities for assembling LLM prompts from a project tree.

mod builder;
mod expander;
pub mod query;
pub mod subagent;

pub use builder::ContextBuilder;
