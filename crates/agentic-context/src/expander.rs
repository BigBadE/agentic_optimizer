//! Context expansion using ContextPlan and language backends.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use agentic_core::{FileContext, Result};
use agentic_languages::LanguageProvider;
use crate::query::{ContextPlan, ExpansionStrategy};

/// Expands context based on a ContextPlan
pub struct ContextExpander<'a> {
    /// Optional language backend for semantic analysis
    backend: Option<&'a Box<dyn LanguageProvider>>,
    /// Project root directory
    project_root: &'a Path,
    /// Maximum file size to include
    max_file_size: usize,
}

impl<'a> ContextExpander<'a> {
    /// Create a new context expander
    #[must_use]
    pub const fn new(
        backend: Option<&'a Box<dyn LanguageProvider>>,
        project_root: &'a Path,
        max_file_size: usize,
    ) -> Self {
        Self {
            backend,
            project_root,
            max_file_size,
        }
    }

    /// Expand context based on the plan
    ///
    /// # Errors
    /// Returns an error if file operations or semantic analysis fails
    pub fn expand(&self, plan: &ContextPlan) -> Result<Vec<FileContext>> {
        tracing::info!("Expanding context with strategy: {:?}", plan.strategy);
        tracing::debug!("Context plan: keywords={:?}, symbols={:?}, patterns={:?}", 
            plan.keywords, plan.symbols_to_find, plan.file_patterns);

        let mut files = HashSet::new();

        // Step 1: Find seed files based on patterns
        if !plan.file_patterns.is_empty() {
            let pattern_files = self.find_files_by_patterns(&plan.file_patterns)?;
            tracing::info!("Found {} files matching patterns", pattern_files.len());
            files.extend(pattern_files);
        }

        // Step 2: Execute strategy-specific expansion
        match &plan.strategy {
            ExpansionStrategy::Focused { symbols } => {
                let focused_files = self.expand_focused(symbols)?;
                tracing::info!("Focused expansion found {} files", focused_files.len());
                files.extend(focused_files);
            }
            ExpansionStrategy::Broad { patterns } => {
                let broad_files = self.expand_broad(patterns)?;
                tracing::info!("Broad expansion found {} files", broad_files.len());
                files.extend(broad_files);
            }
            ExpansionStrategy::EntryPointBased { entry_files } => {
                let entry_based = self.expand_from_entry_points(entry_files, plan.max_depth)?;
                tracing::info!("Entry-point expansion found {} files", entry_based.len());
                files.extend(entry_based);
            }
            ExpansionStrategy::Semantic { query, top_k } => {
                let semantic_files = self.expand_semantic(query, *top_k)?;
                tracing::info!("Semantic expansion found {} files", semantic_files.len());
                files.extend(semantic_files);
            }
        }

        // Step 3: Add test files if requested
        if plan.include_tests {
            let test_files = self.find_test_files(&files)?;
            tracing::info!("Found {} test files", test_files.len());
            files.extend(test_files);
        }

        // Step 4: Convert paths to FileContext
        let mut contexts: Vec<FileContext> = files
            .into_iter()
            .filter_map(|path| FileContext::from_path(&path).ok())
            .collect();

        // Step 5: Sort by relevance (files matching more keywords come first)
        contexts.sort_by_cached_key(|ctx| {
            let path_str = ctx.path.to_string_lossy().to_lowercase();
            let content_lower = ctx.content.to_lowercase();
            
            // Count keyword matches in path and content
            let keyword_score: usize = plan.keywords.iter()
                .map(|kw| {
                    let kw_lower = kw.to_lowercase();
                    let path_matches = path_str.matches(&kw_lower).count();
                    let content_matches = content_lower.matches(&kw_lower).count().min(10);
                    path_matches * 10 + content_matches
                })
                .sum();
            
            // Higher scores should come first (reverse order)
            std::cmp::Reverse(keyword_score)
        });

        tracing::info!("Final context: {} files", contexts.len());
        Ok(contexts)
    }

    /// Find files matching any of the patterns
    fn find_files_by_patterns(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(self.project_root)
            .into_iter()
            .filter_entry(|e| !Self::is_ignored(e))
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            if !Self::is_code_file(path) {
                continue;
            }

            // Check if path matches any pattern
            let path_str = path.to_string_lossy().to_lowercase();
            if patterns.iter().any(|pattern| path_str.contains(&pattern.to_lowercase())) {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.len() <= self.max_file_size as u64 {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(files)
    }

    /// Expand focused on specific symbols
    fn expand_focused(&self, symbols: &[String]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if let Some(backend) = self.backend {
            for symbol in symbols {
                // Search for the symbol
                let search_query = agentic_languages::SearchQuery {
                    symbol_name: Some(symbol.clone()),
                    include_references: true,
                    include_implementations: false,
                    max_results: 50,
                };

                if let Ok(result) = backend.search_symbols(&search_query) {
                    for symbol_info in result.symbols {
                        files.push(symbol_info.file_path);
                    }
                }
            }
        }

        Ok(files)
    }

    /// Expand broadly across matching patterns
    fn expand_broad(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
        self.find_files_by_patterns(patterns)
    }

    /// Expand from entry points by traversing imports
    fn expand_from_entry_points(&self, entry_files: &[PathBuf], max_depth: usize) -> Result<Vec<PathBuf>> {
        let mut files = HashSet::new();
        let mut to_process: Vec<(PathBuf, usize)> = entry_files.iter()
            .map(|p| (p.clone(), 0))
            .collect();

        while let Some((file, depth)) = to_process.pop() {
            if files.contains(&file) || depth >= max_depth {
                continue;
            }

            files.insert(file.clone());

            // Get imports from this file
            if let Some(backend) = self.backend {
                if let Ok(imports) = backend.extract_imports(&file) {
                    for import in imports {
                        if !files.contains(&import) {
                            to_process.push((import, depth + 1));
                        }
                    }
                }
            }
        }

        Ok(files.into_iter().collect())
    }

    /// Expand using semantic search
    fn expand_semantic(&self, _query: &str, _top_k: usize) -> Result<Vec<PathBuf>> {
        // TODO: Implement semantic search using embeddings
        // For now, return empty - this is a future enhancement
        tracing::warn!("Semantic search not yet implemented");
        Ok(Vec::new())
    }

    /// Find test files related to the given files
    fn find_test_files(&self, files: &HashSet<PathBuf>) -> Result<Vec<PathBuf>> {
        let mut test_files = Vec::new();

        for entry in WalkDir::new(self.project_root)
            .into_iter()
            .filter_entry(|e| !Self::is_ignored(e))
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Check if it's a test file
            if path_str.contains("test") || path_str.contains("spec") {
                if Self::is_code_file(path) {
                    // Check if it's related to any of our files
                    let file_name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    if files.iter().any(|f| {
                        f.file_stem()
                            .and_then(|s| s.to_str())
                            .map_or(false, |stem| file_name.contains(stem))
                    }) {
                        test_files.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(test_files)
    }

    /// Check if a directory entry should be ignored
    fn is_ignored(entry: &walkdir::DirEntry) -> bool {
        const IGNORED_DIRS: &[&str] = &["target", "node_modules", "dist", "build", ".git", ".idea", ".vscode"];
        
        let file_name = entry.file_name().to_string_lossy();
        
        if file_name.starts_with('.') && entry.file_type().is_dir() {
            return true;
        }
        
        IGNORED_DIRS.iter().any(|dir| file_name == *dir)
    }

    /// Check if a file is a code file
    fn is_code_file(path: &Path) -> bool {
        const CODE_EXTENSIONS: &[&str] = &[
            "rs", "toml", "md", "txt", "json", "yaml", "yml",
            "js", "ts", "jsx", "tsx", "py", "java", "go", "c", "cpp", "h", "hpp"
        ];

        path.extension()
            .and_then(|ext| ext.to_str())
            .map_or(false, |ext| CODE_EXTENSIONS.contains(&ext))
    }
}
