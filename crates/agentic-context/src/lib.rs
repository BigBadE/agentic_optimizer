//! Context building utilities for assembling LLM prompts from a project tree.

mod builder;
pub mod embedding;
mod expander;
mod fs_utils;
pub mod models;
pub mod query;
pub mod subagent;
pub mod context_inclusion;

pub use builder::ContextBuilder;
pub use embedding::{EmbeddingClient, VectorStore, SearchResult, VectorSearchManager};
