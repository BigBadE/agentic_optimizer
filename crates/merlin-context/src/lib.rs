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
        reason = "Allow for tests"
    )
)]

mod builder;
/// Context fetching with file reference extraction and semantic search
pub mod context_fetcher;
pub mod context_inclusion;
/// Context management for dynamic file inclusion/exclusion
pub mod context_manager;
pub mod embedding;
mod fs_utils;
pub mod models;
pub mod query;
pub mod subagent;

pub use builder::ContextBuilder;
pub use context_fetcher::ContextFetcher;
pub use context_manager::{ContextManager, ContextStats};
pub use embedding::{
    EmbeddingClient, ProgressCallback, SearchResult, VectorSearchManager, VectorStore,
};
