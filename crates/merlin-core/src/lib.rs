//! Core types and traits for the agentic optimizer.
//!
//! This crate provides fundamental types, error handling, and trait definitions
//! used across the agentic optimizer system.
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

/// Error types and result definitions.
pub mod error;
/// Trait definitions for model providers.
pub mod traits;
/// Core data types for queries, responses, and context.
pub mod types;

pub use error::{Error, Result};
pub use traits::ModelProvider;
pub use types::{Context, FileContext, Query, Response, TokenUsage};
