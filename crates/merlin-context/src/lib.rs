//! Context building utilities for assembling LLM prompts from a project tree.

mod builder;
/// Context fetching with file reference extraction and semantic search
pub mod context_fetcher;
pub mod context_inclusion;
pub mod embedding;
mod fs_utils;
pub mod models;
pub mod query;

pub use builder::ContextBuilder;
pub use context_fetcher::ContextFetcher;
#[cfg(any(test, feature = "test-helpers"))]
pub use embedding::FakeEmbeddingClient;
pub use embedding::{
    EmbeddingClient, EmbeddingProvider, ProgressCallback, SearchResult, VectorSearchManager,
    VectorStore,
};
