//! Integration tests for merlin-context
#![cfg_attr(
    test,
    allow(
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        reason = "Allow for tests"
    )
)]

#[path = "modules/bm25_tokenization.rs"]
mod bm25_tokenization;

#[path = "modules/chunking_validation.rs"]
mod chunking_validation;

#[path = "modules/embedding_cache.rs"]
mod embedding_cache;
