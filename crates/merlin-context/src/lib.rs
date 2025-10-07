//! Context building utilities for assembling LLM prompts from a project tree.

pub mod benchmark;
mod builder;
pub mod context_inclusion;
pub mod embedding;
mod expander;
mod fs_utils;
pub mod models;
pub mod query;
pub mod subagent;

pub use builder::ContextBuilder;
pub use embedding::{EmbeddingClient, SearchResult, VectorSearchManager, VectorStore};
