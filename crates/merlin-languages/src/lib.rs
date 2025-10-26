//! Language-specific code analysis and context building.
//!
//! This crate provides language provider abstractions and implementations
//! for semantic code analysis using language-specific tools like rust-analyzer.
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

mod backends;
/// Language provider trait and types.
pub mod provider;

pub use provider::{LanguageProvider, SearchQuery, SearchResult, SymbolInfo, SymbolKind};

use merlin_core::CoreResult as Result;

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

#[cfg(test)]
mod tests {
    use super::*;

    // REMOVED: test_language_equality - Trait implementation test

    // REMOVED: test_language_debug - Trait implementation test

    // REMOVED: test_language_clone - Trait implementation test

    #[test]
    fn test_create_backend_rust() {
        let result = create_backend(Language::Rust);
        result.unwrap();
    }
}
