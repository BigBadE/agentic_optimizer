//! Language-specific code analysis and context building.
//!
//! This crate provides language provider abstractions for semantic code analysis.
//! Language backends have been removed as they are not currently in use.
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

/// Language provider trait and types.
pub mod provider;

pub use provider::{LanguageProvider, SearchQuery, SearchResult, SymbolInfo, SymbolKind};
