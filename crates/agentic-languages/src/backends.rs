//! Backend implementations that wrap language-specific analyzers.

use std::path::{Path, PathBuf};
use agentic_core::{FileContext, Result};
use crate::provider::{LanguageProvider, SearchQuery, SearchResult, SymbolInfo};

/// Wrapper for RustBackend that implements LanguageProvider
pub struct RustBackendWrapper {
    backend: rust_backend::RustBackend,
}

impl RustBackendWrapper {
    /// Create a new Rust backend wrapper
    #[must_use]
    pub fn new() -> Self {
        Self {
            backend: rust_backend::RustBackend::new(),
        }
    }
}

impl Default for RustBackendWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageProvider for RustBackendWrapper {
    fn initialize(&mut self, project_root: &Path) -> Result<()> {
        self.backend.initialize(project_root)
    }
    
    fn search_symbols(&self, query: &SearchQuery) -> Result<SearchResult> {
        // Convert between the two SearchQuery types
        let backend_query = rust_backend::SearchQuery {
            symbol_name: query.symbol_name.clone(),
            include_references: query.include_references,
            include_implementations: query.include_implementations,
            max_results: query.max_results,
        };
        
        let backend_result = self.backend.search_symbols(&backend_query)?;
        
        // Convert backend SymbolInfo to provider SymbolInfo
        let symbols = backend_result.symbols.into_iter().map(|s| SymbolInfo {
            name: s.name,
            kind: convert_symbol_kind(s.kind),
            file_path: s.file_path,
            line: s.line,
            documentation: s.documentation,
        }).collect();
        
        Ok(SearchResult {
            symbols,
            related_files: backend_result.related_files,
        })
    }
    
    fn find_definition(&self, symbol_name: &str, file: &Path, line: u32) -> Result<Option<SymbolInfo>> {
        let result = self.backend.find_definition(symbol_name, file, line)?;
        
        Ok(result.map(|s| SymbolInfo {
            name: s.name,
            kind: convert_symbol_kind(s.kind),
            file_path: s.file_path,
            line: s.line,
            documentation: s.documentation,
        }))
    }
    
    fn find_references(&self, symbol_name: &str) -> Result<Vec<SymbolInfo>> {
        let results = self.backend.find_references(symbol_name)?;
        
        Ok(results.into_iter().map(|s| SymbolInfo {
            name: s.name,
            kind: convert_symbol_kind(s.kind),
            file_path: s.file_path,
            line: s.line,
            documentation: s.documentation,
        }).collect())
    }
    
    fn get_related_context(&self, file: &Path) -> Result<Vec<FileContext>> {
        self.backend.get_related_context(file)
    }
    
    fn extract_imports(&self, file: &Path) -> Result<Vec<PathBuf>> {
        self.backend.extract_imports(file)
    }
    
    fn list_symbols_in_file(&self, file: &Path) -> Result<Vec<SymbolInfo>> {
        let results = self.backend.list_symbols_in_file(file)?;
        
        Ok(results.into_iter().map(|s| SymbolInfo {
            name: s.name,
            kind: convert_symbol_kind(s.kind),
            file_path: s.file_path,
            line: s.line,
            documentation: s.documentation,
        }).collect())
    }
}

/// Convert rust-backend SymbolKind to provider SymbolKind
fn convert_symbol_kind(kind: rust_backend::SymbolKind) -> crate::provider::SymbolKind {
    match kind {
        rust_backend::SymbolKind::Function => crate::provider::SymbolKind::Function,
        rust_backend::SymbolKind::Struct => crate::provider::SymbolKind::Struct,
        rust_backend::SymbolKind::Enum => crate::provider::SymbolKind::Enum,
        rust_backend::SymbolKind::Trait => crate::provider::SymbolKind::Trait,
        rust_backend::SymbolKind::Module => crate::provider::SymbolKind::Module,
        rust_backend::SymbolKind::Constant => crate::provider::SymbolKind::Constant,
        rust_backend::SymbolKind::Variable => crate::provider::SymbolKind::Variable,
        rust_backend::SymbolKind::Field => crate::provider::SymbolKind::Field,
        rust_backend::SymbolKind::Method => crate::provider::SymbolKind::Method,
        rust_backend::SymbolKind::Type => crate::provider::SymbolKind::Type,
    }
}
