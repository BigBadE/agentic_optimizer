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

#[cfg(test)]
#[allow(
    clippy::missing_panics_doc,
    clippy::uninlined_format_args,
    clippy::assertions_on_result_states,
    reason = "Test code is allowed to use unwrap and has different conventions"
)]
mod tests {
    use super::*;

    #[test]
    fn test_language_equality() {
        assert_eq!(Language::Rust, Language::Rust);
    }

    #[test]
    fn test_language_debug() {
        let lang = Language::Rust;
        let debug_str = format!("{:?}", lang);
        assert_eq!(debug_str, "Rust");
    }

    #[test]
    fn test_language_clone() {
        let lang1 = Language::Rust;
        let lang2 = lang1;
        assert_eq!(lang1, lang2);
    }

    #[test]
    fn test_create_backend_rust() {
        let result = create_backend(Language::Rust);
        assert!(result.is_ok());
    }
}
