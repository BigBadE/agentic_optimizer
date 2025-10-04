//! Backend implementations that wrap language-specific analyzers.

use std::path::{Path, PathBuf};

use merlin_core::{FileContext, Result};
use crate::provider::{LanguageProvider, SearchQuery, SearchResult, SymbolInfo, SymbolKind};

/// Wrapper for `RustBackend` that implements `LanguageProvider`
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
        
        let symbols = backend_result.symbols.into_iter().map(|symbol| SymbolInfo {
            name: symbol.name,
            kind: convert_symbol_kind(symbol.kind),
            file_path: symbol.file_path,
            line: symbol.line,
            documentation: symbol.documentation,
        }).collect();
        
        Ok(SearchResult {
            symbols,
            related_files: backend_result.related_files,
        })
    }
    
    fn find_definition(&self, symbol_name: &str, file: &Path, line: u32) -> Result<Option<SymbolInfo>> {
        let result = self.backend.find_definition(symbol_name, file, line)?;
        
        Ok(result.map(|symbol| SymbolInfo {
            name: symbol.name,
            kind: convert_symbol_kind(symbol.kind),
            file_path: symbol.file_path,
            line: symbol.line,
            documentation: symbol.documentation,
        }))
    }
    
    fn find_references(&self, symbol_name: &str) -> Result<Vec<SymbolInfo>> {
        let results = self.backend.find_references(symbol_name)?;
        
        Ok(results.into_iter().map(|symbol| SymbolInfo {
            name: symbol.name,
            kind: convert_symbol_kind(symbol.kind),
            file_path: symbol.file_path,
            line: symbol.line,
            documentation: symbol.documentation,
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
        
        Ok(results.into_iter().map(|symbol| SymbolInfo {
            name: symbol.name,
            kind: convert_symbol_kind(symbol.kind),
            file_path: symbol.file_path,
            line: symbol.line,
            documentation: symbol.documentation,
        }).collect())
    }
}

/// Convert rust-backend `SymbolKind` to provider `SymbolKind`
const fn convert_symbol_kind(kind: rust_backend::SymbolKind) -> SymbolKind {
    match kind {
        rust_backend::SymbolKind::Function => SymbolKind::Function,
        rust_backend::SymbolKind::Struct => SymbolKind::Struct,
        rust_backend::SymbolKind::Enum => SymbolKind::Enum,
        rust_backend::SymbolKind::Trait => SymbolKind::Trait,
        rust_backend::SymbolKind::Module => SymbolKind::Module,
        rust_backend::SymbolKind::Constant => SymbolKind::Constant,
        rust_backend::SymbolKind::Variable => SymbolKind::Variable,
        rust_backend::SymbolKind::Field => SymbolKind::Field,
        rust_backend::SymbolKind::Method => SymbolKind::Method,
        rust_backend::SymbolKind::Type => SymbolKind::Type,
    }
}

