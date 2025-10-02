//! Language-specific code analysis and context building.
//!
//! This crate provides language provider abstractions and implementations
//! for semantic code analysis using language-specific tools like rust-analyzer.

pub mod languages;
pub mod provider;

pub use provider::{LanguageProvider, SymbolInfo, SymbolKind, SearchQuery, SearchResult};
