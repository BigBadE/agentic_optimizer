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

pub mod benchmark;
mod builder;
pub mod context_inclusion;
pub mod embedding;
mod fs_utils;
pub mod models;
pub mod query;
pub mod subagent;

pub use builder::ContextBuilder;
pub use embedding::{
    EmbeddingClient, ProgressCallback, SearchResult, VectorSearchManager, VectorStore,
};
