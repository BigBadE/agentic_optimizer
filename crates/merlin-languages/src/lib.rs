//! Language-specific code analysis and context building.
//!
//! This crate provides language provider abstractions and implementations
//! for semantic code analysis using language-specific tools like rust-analyzer.

mod backends;
/// Language provider trait and types.
pub mod provider;

pub use provider::{LanguageProvider, SearchQuery, SearchResult, SymbolInfo, SymbolKind};

use merlin_core::Result;

/// Supported language types for backend creation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Rust language
    Rust,
    // Future: Java, Python, TypeScript, etc.
}

/// Create a language backend for the specified language
///
/// # Errors
/// Returns an error if the backend cannot be created
pub fn create_backend(language: Language) -> Result<Box<dyn LanguageProvider>> {
    match language {
        Language::Rust => {
            let backend = backends::RustBackendWrapper::default();
            Ok(Box::new(backend))
        }
    }
}
