//! Integration tests for merlin-context

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

#[path = "modules/bm25_tokenization.rs"]
mod bm25_tokenization;

#[path = "modules/chunking_validation.rs"]
mod chunking_validation;

#[path = "modules/embedding_cache.rs"]
mod embedding_cache;
