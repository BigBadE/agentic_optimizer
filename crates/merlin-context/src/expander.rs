//! Context expansion logic for following code relationships.

use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::time::Duration;
use walkdir::{DirEntry, WalkDir};

use merlin_core::FileContext;
use merlin_languages::LanguageProvider;

use crate::{
    fs_utils::is_source_file,
    query::{ContextPlan, ExpansionStrategy},
};

/// Expands context by following code relationships
pub struct ContextExpander<'expander> {
    /// Optional language backend for semantic analysis
    backend: Option<&'expander dyn LanguageProvider>,
    /// Project root directory
    project_root: &'expander Path,
    /// Maximum file size to include
    max_file_size: usize,
}

impl<'expander> ContextExpander<'expander> {
    /// Create a new context expander
    #[must_use]
    #[allow(
        dead_code,
        reason = "Module reserved for future context expansion features"
    )]
    pub fn new(
        backend: Option<&'expander dyn LanguageProvider>,
        project_root: &'expander Path,
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
    #[allow(
        dead_code,
        reason = "Module reserved for future context expansion features"
    )]
    pub fn expand(&self, plan: &ContextPlan) -> Vec<FileContext> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        tracing::info!("Expanding context with strategy: {:?}", plan.strategy);
        tracing::debug!(
            "Context plan: keywords={:?}, symbols={:?}, patterns={:?}",
            plan.keywords,
            plan.symbols,
            plan.file_patterns
        );

        let mut files = HashSet::new();

        // Step 1: Find seed files based on patterns
        if !plan.file_patterns.is_empty() {
            spinner.set_message(format!(
                "Searching for files matching patterns: {:?}...",
                plan.file_patterns
            ));
            let pattern_files = self.find_files_by_patterns(&plan.file_patterns);
            tracing::info!("Found {} files matching patterns", pattern_files.len());
            files.extend(pattern_files);
        }

        // Step 2: Execute strategy-specific expansion
        match &plan.strategy {
            ExpansionStrategy::Focused { symbols } => {
                spinner.set_message(format!("Searching for symbols: {symbols:?}..."));
                let focused_files = self.expand_focused(symbols, &spinner);
                tracing::info!("Found {} files with focused symbols", focused_files.len());
                files.extend(focused_files);
            }
            ExpansionStrategy::Broad { patterns } => {
                spinner.set_message(format!("Broad search with patterns: {patterns:?}..."));
                let broad_files = self.expand_broad(patterns);
                tracing::info!("Found {} files with broad search", broad_files.len());
                files.extend(broad_files);
            }
            ExpansionStrategy::EntryPointBased { entry_files } => {
                spinner.set_message("Expanding from entry points...");
                let entry_based = self.expand_from_entry_points(entry_files, plan.max_depth);
                tracing::info!("Found {} files from entry points", entry_based.len());
                files.extend(entry_based);
            }
            ExpansionStrategy::Semantic { query, top_k } => {
                spinner.set_message(format!("Semantic search: {query}..."));
                let semantic_files = Self::expand_semantic(query, *top_k);
                tracing::info!("Found {} files with semantic search", semantic_files.len());
                files.extend(semantic_files);
            }
        }

        // Step 3: Add test files if requested
        if plan.include_tests {
            spinner.set_message("Finding related test files...");
            let test_files = self.find_test_files(&files);
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
            let keyword_score: usize = plan
                .keywords
                .iter()
                .map(|keyword| {
                    let kw_lower = keyword.to_lowercase();
                    let path_matches = path_str.matches(&kw_lower).count();
                    let content_matches = content_lower.matches(&kw_lower).count().min(10);
                    path_matches * 10 + content_matches
                })
                .sum();

            // Higher scores should come first (reverse order)
            Reverse(keyword_score)
        });

        spinner.finish_with_message(format!(
            "✓ Context expansion complete: {} files",
            contexts.len()
        ));
        tracing::info!("Final context: {} files", contexts.len());
        contexts
    }

    #[allow(
        dead_code,
        reason = "Module reserved for future context expansion features"
    )]
    fn find_files_by_patterns(&self, _patterns: &[String]) -> Vec<PathBuf> {
        // Use the same WalkBuilder as build_file_tree for consistency
        let walker = WalkBuilder::new(self.project_root)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .build();

        let mut files = Vec::new();

        for entry in walker {
            match entry {
                Err(error) => {
                    tracing::warn!("Warning: Error walking directory: {error}");
                }
                Ok(dir_entry) => {
                    // Only process files
                    if !dir_entry
                        .file_type()
                        .is_some_and(|file_type| file_type.is_file())
                    {
                        continue;
                    }

                    let path = dir_entry.path();
                    if is_source_file(path) {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }

        files
    }

    /// Expand broadly across matching patterns
    fn expand_broad(&self, patterns: &[String]) -> Vec<PathBuf> {
        self.find_files_by_patterns(patterns)
    }

    /// Expand focusing on specific symbols by scanning source files
    fn expand_focused(&self, symbols: &[String], spinner: &ProgressBar) -> Vec<PathBuf> {
        spinner.set_message("Scanning for focused symbols...");

        let walker = WalkBuilder::new(self.project_root)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .build();

        let mut files = Vec::new();

        for entry in walker {
            let Ok(dir_entry) = entry else {
                continue;
            };

            if !dir_entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
            {
                continue;
            }

            let path = dir_entry.path();
            if !is_source_file(path) {
                continue;
            }

            if let Ok(metadata) = fs::metadata(path)
                && metadata.len() as usize > self.max_file_size
            {
                continue;
            }

            if let Ok(content) = fs::read_to_string(path)
                && symbols
                    .iter()
                    .any(|symbol| !symbol.is_empty() && content.contains(symbol))
            {
                files.push(path.to_path_buf());
            }
        }

        files
    }

    /// Expand from entry points by traversing imports
    #[allow(
        dead_code,
        reason = "Module reserved for future context expansion features"
    )]
    fn expand_from_entry_points(&self, entry_files: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
        let mut visited: HashSet<PathBuf> = HashSet::new();
        let mut to_process: Vec<(PathBuf, usize)> =
            entry_files.iter().cloned().map(|path| (path, 0)).collect();

        while let Some((file, depth)) = to_process.pop() {
            if visited.contains(&file) || depth >= max_depth {
                continue;
            }

            visited.insert(file.clone());

            // Get imports from this file
            if let Some(backend) = self.backend
                && let Ok(imports) = backend.extract_imports(&file)
            {
                imports
                    .into_iter()
                    .filter(|import| !visited.contains(import))
                    .for_each(|import| to_process.push((import, depth + 1)));
            }
        }

        visited.into_iter().collect()
    }

    /// Expand using semantic search
    #[allow(
        dead_code,
        reason = "Module reserved for future context expansion features"
    )]
    fn expand_semantic(_query: &str, _top_k: usize) -> Vec<PathBuf> {
        // TODO: Implement semantic search using embeddings
        Vec::new()
    }

    #[allow(
        dead_code,
        reason = "Module reserved for future context expansion features"
    )]
    fn find_test_files(&self, files: &HashSet<PathBuf>) -> Vec<PathBuf> {
        let mut test_files = Vec::new();

        for entry in WalkDir::new(self.project_root)
            .into_iter()
            .filter_entry(|entry| !Self::is_ignored(entry))
            .filter_map(StdResult::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Only consider paths that look like test/spec files and are source files
            if !(path_str.contains("test") || path_str.contains("spec")) || !is_source_file(path) {
                continue;
            }

            let Some(parent) = path.parent() else {
                continue;
            };
            let Some(name) = path.file_name().and_then(|str_path| str_path.to_str()) else {
                continue;
            };

            let stem = name.trim_end_matches(".rs");
            let candidate = parent.join(format!("{stem}.rs"));
            if files.contains(&candidate) {
                test_files.push(path.to_path_buf());
            }
        }

        test_files
    }

    /// Check if a directory entry should be ignored
    fn is_ignored(entry: &DirEntry) -> bool {
        const IGNORED_DIRS: &[&str] = &[
            "target",
            "node_modules",
            "dist",
            "build",
            ".git",
            ".idea",
            ".vscode",
        ];

        let file_name = entry.file_name().to_string_lossy();

        if file_name.starts_with('.') && entry.file_type().is_dir() {
            return true;
        }

        IGNORED_DIRS.iter().any(|dir| file_name == *dir)
    }
}
