//! Integration tests for merlin-context
#![cfg_attr(
    test,
    allow(
        clippy::tests_outside_test_module,
        reason = "Allow for integration tests"
    )
)]

#[path = "modules/bm25_tokenization.rs"]
mod bm25_tokenization;

#[path = "modules/chunking_validation.rs"]
mod chunking_validation;

#[path = "modules/embedding_cache.rs"]
mod embedding_cache;
