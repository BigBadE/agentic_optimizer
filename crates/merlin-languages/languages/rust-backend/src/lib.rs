//! Rust language backend using rust-analyzer for semantic code analysis.
//!
//! This crate provides a concrete implementation of language analysis for Rust
//! codebases using rust-analyzer's APIs.

mod workspace;
mod symbol_search;
mod context_builder;
mod cache;

pub use cache::WorkspaceCache;
pub use workspace::{LoadConfig, WorkspaceLoader};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ra_ap_ide::{Analysis, AnalysisHost, FileId};
use ra_ap_vfs::Vfs;

use merlin_core::{Error, FileContext, Result};
/// Information about a code symbol (function, struct, etc.)
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// The name of the symbol
    pub name: String,
    /// The kind/type of symbol
    pub kind: SymbolKind,
    /// The file containing this symbol
    pub file_path: PathBuf,
    /// The line number where the symbol is defined
    pub line: u32,
    /// Optional documentation for the symbol
    pub documentation: Option<String>,
}

/// The kind of code symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Constant,
    Variable,
    Field,
    Method,
    Type,
}

/// Query parameters for symbol search
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Optional symbol name to search for
    pub symbol_name: Option<String>,
    /// Whether to include references to the symbol
    pub include_references: bool,
    /// Whether to include trait implementations
    pub include_implementations: bool,
    /// Maximum number of results to return
    pub max_results: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            symbol_name: None,
            include_references: false,
            include_implementations: false,
            max_results: 50,
        }
    }
}

/// Results from a symbol search
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The symbols found
    pub symbols: Vec<SymbolInfo>,
    /// Related file contexts
    pub related_files: Vec<FileContext>,
}

/// Rust language backend using rust-analyzer for semantic analysis
pub struct RustBackend {
    /// The rust-analyzer analysis host (thread-safe)
    analysis_host: Option<Arc<Mutex<AnalysisHost>>>,
    /// Virtual file system for the workspace
    vfs: Option<Arc<Vfs>>,
    /// Project root directory
    project_root: PathBuf,
    /// Mapping from file paths to rust-analyzer file IDs
    file_id_map: Arc<HashMap<PathBuf, FileId>>,
}

impl RustBackend {
    /// Create a new uninitialized Rust backend
    #[must_use]
    pub fn new() -> Self {
        Self {
            analysis_host: None,
            vfs: None,
            project_root: PathBuf::new(),
            file_id_map: Arc::new(HashMap::new()),
        }
    }

    /// Initialize with custom loading configuration
    ///
    /// # Errors
    /// Returns an error if the project root is invalid or if the workspace cannot be loaded
    pub fn initialize_with_config(&mut self, project_root: &Path, config: LoadConfig) -> Result<()> {
        tracing::info!("Initializing Rust workspace at: {}", project_root.display());
        
        self.project_root = project_root.to_path_buf();
        
        let loader = WorkspaceLoader::with_config(project_root, config);
        let (analysis_host, vfs, file_id_map) = loader.load()?;
        
        self.analysis_host = Some(Arc::new(Mutex::new(analysis_host)));
        self.vfs = Some(Arc::new(vfs));
        self.file_id_map = Arc::new(file_id_map);
        
        tracing::info!("Workspace initialized with {} files", self.file_id_map.len());
        
        Ok(())
    }

    /// Initialize the Rust backend for a project with default configuration
    ///
    /// # Errors
    /// Returns an error if the project root is invalid or if the workspace cannot be loaded
    pub fn initialize(&mut self, project_root: &Path) -> Result<()> {
        // Use default config with incremental loading enabled
        let config = LoadConfig {
            workspace_only: true,
            show_progress: true,
            use_cache: true,
        };
        self.initialize_with_config(project_root, config)
    }

    /// Get the rust-analyzer analysis instance
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the mutex is poisoned
    fn analysis(&self) -> Result<Analysis> {
        let host = self
            .analysis_host
            .as_ref()
            .ok_or_else(|| Error::Other("Workspace not initialized".into()))?;

        let guard = host
            .lock()
            .map_err(|error| Error::Other(format!("Failed to lock analysis host: {error}")))?;

        Ok(guard.analysis())
    }

    /// Get the rust-analyzer file ID for a path
    fn get_file_id(&self, path: &Path) -> Option<FileId> {
        self.file_id_map.get(path).copied()
    }

    /// Map a rust-analyzer file id back to a file path
    fn path_from_file_id(&self, file_id: FileId) -> Option<PathBuf> {
        self.file_id_map
            .iter()
            .find(|(_, id)| **id == file_id)
            .map(|(path, _)| path.clone())
    }

    /// Search for symbols in the project
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the search query is invalid
    pub fn search_symbols(&self, query: &SearchQuery) -> Result<SearchResult> {
        let analysis = self.analysis()?;
        let searcher = symbol_search::SymbolSearcher::new(&analysis, self);
        
        searcher.search(query)
    }

    /// Find the definition of a symbol
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the symbol is not found
    pub fn find_definition(&self, symbol_name: &str, file: &Path, line: u32) -> Result<Option<SymbolInfo>> {
        let analysis = self.analysis()?;
        let searcher = symbol_search::SymbolSearcher::new(&analysis, self);
        
        searcher.find_definition(symbol_name, file, line)
    }

    /// Find references to a symbol
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the symbol is not found
    pub fn find_references(&self, symbol_name: &str) -> Result<Vec<SymbolInfo>> {
        let analysis = self.analysis()?;
        let searcher = symbol_search::SymbolSearcher::new(&analysis, self);
        
        searcher.find_references(symbol_name)
    }

    /// Get related context for a file
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the file is not found
    pub fn get_related_context(&self, file: &Path) -> Result<Vec<FileContext>> {
        let analysis = self.analysis()?;
        let builder = context_builder::ContextBuilder::new(&analysis, self);
        
        builder.get_related_context(file)
    }

    /// Extract imports from a file
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the file is not found
    pub fn extract_imports(&self, file: &Path) -> Result<Vec<PathBuf>> {
        let analysis = self.analysis()?;
        let builder = context_builder::ContextBuilder::new(&analysis, self);
        
        builder.extract_imports(file)
    }

    /// List symbols in a file
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized or if the file is not found
    pub fn list_symbols_in_file(&self, file: &Path) -> Result<Vec<SymbolInfo>> {
        let analysis = self.analysis()?;
        let searcher = symbol_search::SymbolSearcher::new(&analysis, self);
        
        searcher.list_symbols_in_file(file)
    }

    /// Build import graph for a set of files
    /// Returns a map from file path to list of files it imports
    ///
    /// # Errors
    /// Returns an error if the workspace is not initialized
    pub fn build_import_graph(&self, files: &[PathBuf]) -> Result<ImportGraph> {
        let mut graph = HashMap::new();
        
        for file in files {
            if let Ok(imports) = self.extract_imports(file) {
                graph.insert(file.clone(), imports);
            }
        }
        
        Ok(graph)
    }
}

impl Default for RustBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// A graph of import relationships between files
type ImportGraph = HashMap<PathBuf, Vec<PathBuf>>;
