//! Symbol search and navigation helpers built on rust-analyzer.

use std::fs::read_to_string;
use std::path::Path;

use ra_ap_ide::{Analysis, SymbolKind as RaSymbolKind};
use ra_ap_ide_db::symbol_index::Query;

use agentic_core::Result;
use crate::provider::{SearchQuery, SearchResult, SymbolInfo, SymbolKind};
use super::RustLanguageProvider;

/// Helper type to perform symbol discovery and navigation queries.
pub struct SymbolSearcher<'analysis> {
    /// The analysis snapshot to query against
    analysis: &'analysis Analysis,
    /// The Rust provider for file/path resolution
    provider: &'analysis RustLanguageProvider,
}

impl<'analysis> SymbolSearcher<'analysis> {
    /// Create a new `SymbolSearcher` bound to an analysis snapshot.
    #[must_use]
    pub const fn new(analysis: &'analysis Analysis, provider: &'analysis RustLanguageProvider) -> Self {
        Self { analysis, provider }
    }

    /// Search for symbols according to the provided query.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer queries fail.
    pub fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let mut symbols = Vec::new();

        if let Some(symbol_name) = &query.symbol_name {
            symbols.extend(self.find_symbol_by_name(symbol_name)?);

            if query.include_references {
                symbols.extend(self.find_references(symbol_name)?);
            }
        } else {
            symbols = self.list_all_symbols()?;
        }

        symbols.truncate(query.max_results);

        let related_files = symbols
            .iter()
            .filter_map(|symbol| {
                read_to_string(&symbol.file_path).ok().map(|content| {
                    agentic_core::FileContext::new(symbol.file_path.clone(), content)
                })
            })
            .collect();

        Ok(SearchResult {
            symbols,
            related_files,
        })
    }

    /// Find the definition of the named symbol located around a given file/line.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer queries fail.
    pub fn find_definition(&self, symbol_name: &str, file: &Path, line: u32) -> Result<Option<SymbolInfo>> {
        let file_id = self.provider.get_file_id(file)
            .ok_or_else(|| agentic_core::Error::FileNotFound(file.display().to_string()))?;

        let line_index = self
            .analysis
            .file_line_index(file_id)
            .map_err(|error| agentic_core::Error::Other(error.to_string()))?;

        let Some(offset) = line_index.offset(ra_ap_ide::LineCol { line: line.saturating_sub(1), col: 0 }) else {
            return Ok(None);
        };

        let position = ra_ap_ide::FilePosition { file_id, offset };

        let nav_targets = self
            .analysis
            .goto_definition(position)
            .map_err(|error| agentic_core::Error::Other(error.to_string()))?;

        if let Some(nav_info) = nav_targets {
            for nav in nav_info.info {
                if nav.name.to_string().contains(symbol_name)
                    && let Some(path) = self.provider.path_from_file_id(nav.file_id)
                {
                    let kind = nav.kind.unwrap_or(RaSymbolKind::Module);

                    return Ok(Some(SymbolInfo {
                        name: nav.name.to_string(),
                        kind: convert_symbol_kind(kind),
                        file_path: path,
                        line: nav.focus_range.map_or(0, |range| {
                            self.analysis
                                .file_line_index(nav.file_id)
                                .ok()
                                .map_or(0, |index| index.line_col(range.start()).line)
                        }),
                        documentation: nav.description,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Find references to a symbol by name using a structural scan.
    pub fn find_references(&self, symbol_name: &str) -> Result<Vec<SymbolInfo>> {
        let mut results = Vec::new();

        for (_path, file_id) in self.provider.file_id_map.iter() {
            let symbols = self.list_symbols_in_file_by_id(*file_id)?;
            
            for symbol in symbols {
                if symbol.name.contains(symbol_name) {
                    results.push(symbol);
                }
            }

            if results.len() > 100 {
                break;
            }
        }

        Ok(results)
    }

    /// List symbols for a given file path.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer fails to provide file structure.
    pub fn list_symbols_in_file(&self, file: &Path) -> Result<Vec<SymbolInfo>> {
        let file_id = self.provider.get_file_id(file)
            .ok_or_else(|| agentic_core::Error::FileNotFound(file.display().to_string()))?;

        self.list_symbols_in_file_by_id(file_id)
    }

    /// Internal: list symbols using a rust-analyzer file id.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer queries fail.
    fn list_symbols_in_file_by_id(&self, file_id: ra_ap_ide::FileId) -> Result<Vec<SymbolInfo>> {
        let structure = self
            .analysis
            .file_structure(file_id)
            .map_err(|error| agentic_core::Error::Other(error.to_string()))?;

        let path = self.provider.path_from_file_id(file_id)
            .ok_or_else(|| agentic_core::Error::Other("File path not found".into()))?;

        let line_index = self
            .analysis
            .file_line_index(file_id)
            .map_err(|error| agentic_core::Error::Other(error.to_string()))?;

        Ok(structure
            .into_iter()
            .map(|node_info| {
                let line = line_index.line_col(node_info.node_range.start()).line;
                
                SymbolInfo {
                    name: node_info.label,
                    kind: convert_structure_kind(node_info.kind),
                    file_path: path.clone(),
                    line,
                    documentation: node_info.detail,
                }
            })
            .collect())
    }

    /// Internal: list symbols for all known files (bounded size).
    fn list_all_symbols(&self) -> Result<Vec<SymbolInfo>> {
        let mut symbols = Vec::new();

        for file_id in self.provider.file_id_map.values() {
            symbols.extend(self.list_symbols_in_file_by_id(*file_id)?);
            
            if symbols.len() > 1000 {
                break;
            }
        }

        Ok(symbols)
    }

    /// Internal: find symbols by name using the global index.
    fn find_symbol_by_name(&self, name: &str) -> Result<Vec<SymbolInfo>> {
        let query = Query::new(name.to_owned());
        
        let symbols = self
            .analysis
            .symbol_search(query, 100)
            .map_err(|error| agentic_core::Error::Other(error.to_string()))?;

        Ok(symbols
            .into_iter()
            .filter_map(|nav| {
                let path = self.provider.path_from_file_id(nav.file_id)?;
                
                let line = self
                    .analysis
                    .file_line_index(nav.file_id)
                    .ok()
                    .and_then(|index| nav.focus_range.map(|range| index.line_col(range.start()).line))
                    .unwrap_or(0);

                let kind = nav.kind.unwrap_or(RaSymbolKind::Module);

                Some(SymbolInfo {
                    name: nav.name.to_string(),
                    kind: convert_symbol_kind(kind),
                    file_path: path,
                    line,
                    documentation: nav.description,
                })
            })
            .collect())
    }
}

/// Convert rust-analyzer symbol kind to our public symbol kind.
const fn convert_symbol_kind(kind: RaSymbolKind) -> SymbolKind {
    match kind {
        RaSymbolKind::Function => SymbolKind::Function,
        RaSymbolKind::Struct => SymbolKind::Struct,
        RaSymbolKind::Enum => SymbolKind::Enum,
        RaSymbolKind::Trait => SymbolKind::Trait,
        RaSymbolKind::Module => SymbolKind::Module,
        RaSymbolKind::Const => SymbolKind::Constant,
        RaSymbolKind::Field => SymbolKind::Field,
        RaSymbolKind::Method => SymbolKind::Method,
        RaSymbolKind::TypeAlias => SymbolKind::Type,
        _ => SymbolKind::Variable,
    }
}

/// Convert rust-analyzer structure node kind to our public symbol kind.
const fn convert_structure_kind(kind: ra_ap_ide::StructureNodeKind) -> SymbolKind {
    match kind {
        ra_ap_ide::StructureNodeKind::SymbolKind(sk) => convert_symbol_kind(sk),
        ra_ap_ide::StructureNodeKind::Region => SymbolKind::Module,
    }
}
