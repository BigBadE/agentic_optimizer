//! Rust language provider using rust-analyzer.

mod workspace;
mod symbol_search;
mod context_builder;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ra_ap_ide::{Analysis, AnalysisHost, FileId};
use ra_ap_vfs::Vfs;

use agentic_core::{FileContext, Result};
use crate::provider::{LanguageProvider, SearchQuery, SearchResult, SymbolInfo};

use workspace::WorkspaceLoader;
use symbol_search::SymbolSearcher;
use context_builder::ContextBuilder;

/// Rust language provider using rust-analyzer for semantic analysis
pub struct RustLanguageProvider {
    /// The rust-analyzer analysis host (thread-safe)
    analysis_host: Option<Arc<Mutex<AnalysisHost>>>,
    /// Virtual file system for the workspace
    vfs: Option<Arc<Vfs>>,
    /// Project root directory
    project_root: PathBuf,
    /// Mapping from file paths to rust-analyzer file IDs
    file_id_map: Arc<HashMap<PathBuf, FileId>>,
}

impl RustLanguageProvider {
    /// Create a new uninitialized Rust language provider
    #[must_use]
    pub fn new() -> Self {
        Self {
            analysis_host: None,
            vfs: None,
            project_root: PathBuf::new(),
            file_id_map: Arc::new(HashMap::new()),
        }
    }

    /// Get the rust-analyzer analysis instance
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the mutex is poisoned
    fn analysis(&self) -> Result<Analysis> {
        let host = self
            .analysis_host
            .as_ref()
            .ok_or_else(|| agentic_core::Error::Other("Workspace not initialized".into()))?;

        let guard = host
            .lock()
            .map_err(|error| agentic_core::Error::Other(format!("Failed to lock analysis host: {error}")))?;

        Ok(guard.analysis())
    }

    /// Get the rust-analyzer file ID for a path
    /// Internal: map a file system path to a rust-analyzer file id.
    fn get_file_id(&self, path: &Path) -> Option<FileId> {
        self.file_id_map.get(path).copied()
    }

    /// Internal: map a rust-analyzer file id back to a file path.
    fn path_from_file_id(&self, file_id: FileId) -> Option<PathBuf> {
        self.file_id_map
            .iter()
            .find(|(_, id)| **id == file_id)
            .map(|(path, _)| path.clone())
    }
}

impl Default for RustLanguageProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageProvider for RustLanguageProvider {
    /// Initialize the Rust language provider for a project
    ///
    /// # Errors
    /// Returns an error if the project root is invalid or if the workspace cannot be loaded
    fn initialize(&mut self, project_root: &Path) -> Result<()> {
        tracing::info!("Initializing Rust workspace at: {}", project_root.display());
        
        self.project_root = project_root.to_path_buf();
        
        let loader = WorkspaceLoader::new(project_root);
        let (analysis_host, vfs, file_id_map) = loader.load()?;
        
        self.analysis_host = Some(Arc::new(Mutex::new(analysis_host)));
        self.vfs = Some(Arc::new(vfs));
        self.file_id_map = Arc::new(file_id_map);
        
        tracing::info!("Workspace initialized with {} files", self.file_id_map.len());
        
        Ok(())
    }

    /// Search for symbols in the project
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the search query is invalid
    fn search_symbols(&self, query: &SearchQuery) -> Result<SearchResult> {
        let analysis = self.analysis()?;
        let searcher = SymbolSearcher::new(&analysis, self);
        
        searcher.search(query)
    }

    /// Find the definition of a symbol
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the symbol is not found
    fn find_definition(&self, symbol_name: &str, file: &Path, line: u32) -> Result<Option<SymbolInfo>> {
        let analysis = self.analysis()?;
        let searcher = SymbolSearcher::new(&analysis, self);
        
        searcher.find_definition(symbol_name, file, line)
    }

    /// Find references to a symbol
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the symbol is not found
    fn find_references(&self, symbol_name: &str) -> Result<Vec<SymbolInfo>> {
        let analysis = self.analysis()?;
        let searcher = SymbolSearcher::new(&analysis, self);
        
        searcher.find_references(symbol_name)
    }

    /// Get related context for a file
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the file is not found
    fn get_related_context(&self, file: &Path) -> Result<Vec<FileContext>> {
        let analysis = self.analysis()?;
        let builder = ContextBuilder::new(&analysis, self);
        
        builder.get_related_context(file)
    }

    /// Extract imports from a file
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the file is not found
    fn extract_imports(&self, file: &Path) -> Result<Vec<PathBuf>> {
        let analysis = self.analysis()?;
        let builder = ContextBuilder::new(&analysis, self);
        
        builder.extract_imports(file)
    }

    /// List symbols in a file
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the file is not found
    fn list_symbols_in_file(&self, file: &Path) -> Result<Vec<SymbolInfo>> {
        let analysis = self.analysis()?;
        let searcher = SymbolSearcher::new(&analysis, self);
        
        searcher.list_symbols_in_file(file)
    }
}
