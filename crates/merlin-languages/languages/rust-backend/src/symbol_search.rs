//! Symbol search and navigation helpers built on rust-analyzer.

use std::fs::read_to_string;
use std::path::Path;

use ra_ap_ide::{
    Analysis, FileId, FilePosition, FileStructureConfig, LineCol, ReferenceSearchResult,
    StructureNodeKind, SymbolKind as RaSymbolKind,
};
use ra_ap_ide_db::symbol_index::Query;

use crate::{RustBackend, SearchQuery, SearchResult, SymbolInfo, SymbolKind};
use merlin_core::{CoreResult as Result, Error, FileContext};

/// Helper type to perform symbol discovery and navigation queries.
pub struct SymbolSearcher<'analysis> {
    /// The analysis snapshot to query against
    analysis: &'analysis Analysis,
    /// The Rust backend for file/path resolution
    backend: &'analysis RustBackend,
}

impl SymbolSearcher<'_> {
    /// Collect references to a symbol starting from its definition.
    fn collect_references(
        &self,
        symbol_name: &str,
        definition: &SymbolInfo,
        results: &mut Vec<SymbolInfo>,
    ) {
        let Some(file_id) = self.backend.get_file_id(&definition.file_path) else {
            return;
        };

        let Ok(line_index) = self.analysis.file_line_index(file_id) else {
            return;
        };

        let Some(offset) = line_index.offset(LineCol {
            line: definition.line,
            col: 0,
        }) else {
            return;
        };

        let position = FilePosition { file_id, offset };

        let Ok(Some(reference_sets)) = self.analysis.find_all_refs(position, None) else {
            return;
        };

        for reference_set in reference_sets {
            self.collect_references_for_set(symbol_name, definition, &reference_set, results);
        }
    }

    /// Collect references from a reference search result set.
    fn collect_references_for_set(
        &self,
        symbol_name: &str,
        definition: &SymbolInfo,
        reference_set: &ReferenceSearchResult,
        results: &mut Vec<SymbolInfo>,
    ) {
        for (ref_file_id, ranges) in &reference_set.references {
            let Some(path) = self.backend.path_from_file_id(*ref_file_id) else {
                continue;
            };

            let Ok(line_index) = self.analysis.file_line_index(*ref_file_id) else {
                continue;
            };

            for (text_range, _) in ranges {
                let line = line_index.line_col(text_range.start()).line;

                results.push(SymbolInfo {
                    name: symbol_name.to_owned(),
                    kind: definition.kind,
                    file_path: path.clone(),
                    line,
                    documentation: None,
                });
            }
        }
    }
}

impl<'analysis> SymbolSearcher<'analysis> {
    /// Create a new `SymbolSearcher` bound to an analysis snapshot.
    pub fn new(analysis: &'analysis Analysis, backend: &'analysis RustBackend) -> Self {
        Self { analysis, backend }
    }

    /// Search for symbols according to the provided query.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer queries fail.
    pub fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let mut symbols = Vec::default();

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
                read_to_string(&symbol.file_path)
                    .ok()
                    .map(|content| FileContext::new(symbol.file_path.clone(), content))
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
    pub fn find_definition(
        &self,
        symbol_name: &str,
        file: &Path,
        line: u32,
    ) -> Result<Option<SymbolInfo>> {
        let file_id = self
            .backend
            .get_file_id(file)
            .ok_or_else(|| Error::FileNotFound(file.display().to_string()))?;

        let line_index = self
            .analysis
            .file_line_index(file_id)
            .map_err(|error| Error::Other(error.to_string()))?;

        let Some(offset) = line_index.offset(LineCol {
            line: line - 1,
            col: 0,
        }) else {
            return Ok(None);
        };

        let position = FilePosition { file_id, offset };

        let nav_targets = self
            .analysis
            .goto_definition(position)
            .map_err(|error| Error::Other(error.to_string()))?;

        let Some(nav_info) = nav_targets else {
            return Ok(None);
        };

        for nav in nav_info.info {
            if !nav.name.to_string().contains(symbol_name) {
                continue;
            }

            let Some(path) = self.backend.path_from_file_id(nav.file_id) else {
                continue;
            };

            let def_line = nav.focus_range.map_or(0, |range| {
                self.analysis
                    .file_line_index(nav.file_id)
                    .ok()
                    .map_or(0, |index| index.line_col(range.start()).line)
            });

            let kind = nav.kind.unwrap_or(RaSymbolKind::Module);

            return Ok(Some(SymbolInfo {
                name: nav.name.to_string(),
                kind: convert_symbol_kind(kind),
                file_path: path,
                line: def_line,
                documentation: nav.description,
            }));
        }

        Ok(None)
    }

    /// Find references to a symbol by name using rust-analyzer's reference index.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer queries fail.
    pub fn find_references(&self, symbol_name: &str) -> Result<Vec<SymbolInfo>> {
        let mut results = Vec::default();

        // First, find the symbol definition using the global index
        let definitions = self.find_symbol_by_name(symbol_name)?;

        // For each definition, use rust-analyzer's find_all_refs to get usages
        for def in definitions {
            self.collect_references(symbol_name, &def, &mut results);

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
        let file_id = self
            .backend
            .get_file_id(file)
            .ok_or_else(|| Error::FileNotFound(file.display().to_string()))?;

        self.list_symbols_in_file_by_id(file_id)
    }

    /// Internal: list symbols using a rust-analyzer file id.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer queries fail.
    fn list_symbols_in_file_by_id(&self, file_id: FileId) -> Result<Vec<SymbolInfo>> {
        let config = FileStructureConfig {
            exclude_locals: false,
        };
        let structure = self
            .analysis
            .file_structure(&config, file_id)
            .map_err(|error| Error::Other(error.to_string()))?;

        let path = self
            .backend
            .path_from_file_id(file_id)
            .ok_or_else(|| Error::Other("File path not found".into()))?;

        let line_index = self
            .analysis
            .file_line_index(file_id)
            .map_err(|error| Error::Other(error.to_string()))?;

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
    ///
    /// # Errors
    /// Returns an error if rust-analyzer queries fail.
    fn list_all_symbols(&self) -> Result<Vec<SymbolInfo>> {
        let mut symbols = Vec::default();

        for file_id in self.backend.file_id_map.values() {
            symbols.extend(self.list_symbols_in_file_by_id(*file_id)?);

            if symbols.len() > 1000 {
                break;
            }
        }

        Ok(symbols)
    }

    /// Internal: find symbols by name using the global index.
    ///
    /// # Errors
    /// Returns an error if rust-analyzer queries fail.
    fn find_symbol_by_name(&self, name: &str) -> Result<Vec<SymbolInfo>> {
        let query = Query::new(name.to_owned());

        let symbols = self
            .analysis
            .symbol_search(query, 100)
            .map_err(|error| Error::Other(error.to_string()))?;
        Ok(symbols
            .into_iter()
            .filter_map(|nav| {
                let path = self.backend.path_from_file_id(nav.file_id)?;

                let line = self
                    .analysis
                    .file_line_index(nav.file_id)
                    .ok()
                    .and_then(|index| {
                        nav.focus_range
                            .map(|range| index.line_col(range.start()).line)
                    })
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
fn convert_symbol_kind(kind: RaSymbolKind) -> SymbolKind {
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
fn convert_structure_kind(kind: StructureNodeKind) -> SymbolKind {
    match kind {
        StructureNodeKind::SymbolKind(symbol_kind) => convert_symbol_kind(symbol_kind),
        StructureNodeKind::Region | StructureNodeKind::ExternBlock => SymbolKind::Module,
    }
}
