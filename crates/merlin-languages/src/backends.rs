//! Backend implementations that wrap language-specific analyzers.

use std::path::{Path, PathBuf};

use crate::provider::{LanguageProvider, SearchQuery, SearchResult, SymbolInfo, SymbolKind};
use merlin_core::{FileContext, Result};
use rust_backend::{RustBackend, SearchQuery as RustSearchQuery, SymbolKind as RustSymbolKind};

/// Wrapper for `RustBackend` that implements `LanguageProvider`
#[derive(Default)]
pub struct RustBackendWrapper {
    backend: RustBackend,
}

impl LanguageProvider for RustBackendWrapper {
    fn initialize(&mut self, project_root: &Path) -> Result<()> {
        self.backend.initialize(project_root)
    }

    fn search_symbols(&self, query: &SearchQuery) -> Result<SearchResult> {
        // Convert between the two SearchQuery types
        let backend_query = RustSearchQuery {
            symbol_name: query.symbol_name.clone(),
            include_references: query.include_references,
            include_implementations: query.include_implementations,
            max_results: query.max_results,
        };

        let backend_result = self.backend.search_symbols(&backend_query)?;

        let symbols = backend_result
            .symbols
            .into_iter()
            .map(|symbol| SymbolInfo {
                name: symbol.name,
                kind: convert_symbol_kind(symbol.kind),
                file_path: symbol.file_path,
                line: symbol.line,
                documentation: symbol.documentation,
            })
            .collect();

        Ok(SearchResult {
            symbols,
            related_files: backend_result.related_files,
        })
    }

    fn find_definition(
        &self,
        symbol_name: &str,
        file: &Path,
        line: u32,
    ) -> Result<Option<SymbolInfo>> {
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

        Ok(results
            .into_iter()
            .map(|symbol| SymbolInfo {
                name: symbol.name,
                kind: convert_symbol_kind(symbol.kind),
                file_path: symbol.file_path,
                line: symbol.line,
                documentation: symbol.documentation,
            })
            .collect())
    }

    fn get_related_context(&self, file: &Path) -> Result<Vec<FileContext>> {
        self.backend.get_related_context(file)
    }

    fn extract_imports(&self, file: &Path) -> Result<Vec<PathBuf>> {
        self.backend.extract_imports(file)
    }

    fn list_symbols_in_file(&self, file: &Path) -> Result<Vec<SymbolInfo>> {
        let results = self.backend.list_symbols_in_file(file)?;

        Ok(results
            .into_iter()
            .map(|symbol| SymbolInfo {
                name: symbol.name,
                kind: convert_symbol_kind(symbol.kind),
                file_path: symbol.file_path,
                line: symbol.line,
                documentation: symbol.documentation,
            })
            .collect())
    }
}

/// Convert rust-backend `SymbolKind` to provider `SymbolKind`
fn convert_symbol_kind(kind: RustSymbolKind) -> SymbolKind {
    match kind {
        RustSymbolKind::Function => SymbolKind::Function,
        RustSymbolKind::Struct => SymbolKind::Struct,
        RustSymbolKind::Enum => SymbolKind::Enum,
        RustSymbolKind::Trait => SymbolKind::Trait,
        RustSymbolKind::Module => SymbolKind::Module,
        RustSymbolKind::Constant => SymbolKind::Constant,
        RustSymbolKind::Variable => SymbolKind::Variable,
        RustSymbolKind::Field => SymbolKind::Field,
        RustSymbolKind::Method => SymbolKind::Method,
        RustSymbolKind::Type => SymbolKind::Type,
    }
}
