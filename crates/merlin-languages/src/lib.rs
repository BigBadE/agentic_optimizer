//! Language-specific code analysis and context building.
//!
//! This crate provides language provider abstractions for semantic code analysis.
//! Language backends have been removed as they are not currently in use.

/// Language provider trait and types.
pub mod provider;

pub use provider::{LanguageProvider, SearchQuery, SearchResult, SymbolInfo, SymbolKind};
