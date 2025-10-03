//! Context expansion using ContextPlan and language backends.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};
use ignore::WalkBuilder;

use agentic_core::{FileContext, Result};
use agentic_languages::LanguageProvider;
use crate::{fs_utils::is_source_file, query::{ContextPlan, ExpansionStrategy}};

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
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        tracing::info!("Expanding context with strategy: {:?}", plan.strategy);
        tracing::debug!("Context plan: keywords={:?}, symbols={:?}, patterns={:?}", 
            plan.keywords, plan.symbols, plan.file_patterns);

        let mut files = HashSet::new();

        // Step 1: Find seed files based on patterns
        if !plan.file_patterns.is_empty() {
            spinner.set_message(format!("Searching for files matching patterns: {:?}...", plan.file_patterns));
            let pattern_files = self.find_files_by_patterns(&plan.file_patterns)?;
            eprintln!("  Found {} files matching patterns", pattern_files.len());
            tracing::info!("Found {} files matching patterns", pattern_files.len());
            files.extend(pattern_files);
        }

        // Step 2: Execute strategy-specific expansion
        match &plan.strategy {
            ExpansionStrategy::Focused { symbols } => {
                spinner.set_message(format!("Searching for symbols: {:?}...", symbols));
                let focused_files = self.expand_focused(symbols, &spinner)?;
                eprintln!("  Found {} files with focused symbols", focused_files.len());
                tracing::info!("Focused expansion found {} files", focused_files.len());
                files.extend(focused_files);
            }
            ExpansionStrategy::Broad { patterns } => {
                spinner.set_message(format!("Broad search with patterns: {:?}...", patterns));
                let broad_files = self.expand_broad(patterns)?;
                eprintln!("  Found {} files with broad search", broad_files.len());
                tracing::info!("Broad expansion found {} files", broad_files.len());
                files.extend(broad_files);
            }
            ExpansionStrategy::EntryPointBased { entry_files } => {
                spinner.set_message("Expanding from entry points...");
                let entry_based = self.expand_from_entry_points(entry_files, plan.max_depth)?;
                eprintln!("  Found {} files from entry points", entry_based.len());
                tracing::info!("Entry-point expansion found {} files", entry_based.len());
                files.extend(entry_based);
            }
            ExpansionStrategy::Semantic { query, top_k } => {
                spinner.set_message(format!("Semantic search: {}...", query));
                let semantic_files = self.expand_semantic(query, *top_k)?;
                eprintln!("  Found {} files with semantic search", semantic_files.len());
                tracing::info!("Semantic expansion found {} files", semantic_files.len());
                files.extend(semantic_files);
            }
        }

        // Step 3: Add test files if requested
        if plan.include_tests {
            spinner.set_message("Finding related test files...");
            let test_files = self.find_test_files(&files)?;
            eprintln!("  Found {} test files", test_files.len());
            tracing::info!("Found {} test files", test_files.len());
            files.extend(test_files);
        }

        // Step 4: Convert paths to FileContext
        spinner.set_message("Loading file contents...");
        let mut contexts: Vec<FileContext> = files
            .into_iter()
            .filter_map(|path| FileContext::from_path(&path).ok())
            .collect();

        // Step 5: Sort by relevance (files matching more keywords come first)
        spinner.set_message("Ranking files by relevance...");
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

        spinner.finish_with_message(format!("✓ Context expansion complete: {} files", contexts.len()));
        tracing::info!("Final context: {} files", contexts.len());
        Ok(contexts)
    }

    /// Find files matching any of the patterns
    fn find_files_by_patterns(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Use the same WalkBuilder as build_file_tree for consistency
        let walker = WalkBuilder::new(self.project_root)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .build();

        for entry_result in walker {
            match entry_result {
                Ok(entry) => {
                    let path = entry.path();
                    
                    // Only process files
                    if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                        continue;
                    }

                    // Only process source files
                    if !is_source_file(path) {
                        continue;
                    }

                    // Check if path matches any pattern (use relative path with forward slashes)
                    let rel_path = path.strip_prefix(self.project_root)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .replace('\\', "/")
                        .to_lowercase();
                    
                    if patterns.iter().any(|pattern| rel_path.contains(&pattern.to_lowercase())) {
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.len() <= self.max_file_size as u64 {
                                files.push(path.to_path_buf());
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Error walking directory: {}", e);
                }
            }
        }

        Ok(files)
    }

    /// Expand focused on specific symbols
    fn expand_focused(&self, symbols: &[String], spinner: &ProgressBar) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if let Some(backend) = self.backend {
            for symbol in symbols {
                // Skip if this looks like a file path or keyword, not a symbol
                if symbol.contains('/') || symbol.contains('\\') || symbol.contains('.') {
                    eprintln!("  Skipping '{}' - looks like a path, not a symbol", symbol);
                    continue;
                }
                
                // Skip very generic terms that would match too much
                if symbol.len() < 3 {
                    eprintln!("  Skipping '{}' - too short for symbol search", symbol);
                    continue;
                }
                
                spinner.set_message(format!("Searching for symbol: {}...", symbol));
                
                // Search for the symbol
                let search_query = agentic_languages::SearchQuery {
                    symbol_name: Some(symbol.clone()),
                    include_references: true,
                    include_implementations: true,
                    max_results: 20,
                };

                if let Ok(result) = backend.search_symbols(&search_query) {
                    eprintln!("    Symbol '{}': {} locations", symbol, result.symbols.len());
                    for symbol_info in result.symbols {
                        if is_source_file(&symbol_info.file_path) {
                            files.push(symbol_info.file_path);
                        }
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
                if is_source_file(path) {
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

}
