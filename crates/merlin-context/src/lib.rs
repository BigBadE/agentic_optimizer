//! Context building utilities for assembling LLM prompts from a project tree.
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
        reason = "Test allows"
    )
)]

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
#[cfg(test)]
pub use embedding::FakeEmbeddingClient;
pub use embedding::{
    EmbeddingClient, EmbeddingProvider, ProgressCallback, SearchResult, VectorSearchManager,
    VectorStore,
};
