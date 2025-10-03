use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};
use ignore::WalkBuilder;

use agentic_core::{Context, FileContext, Query, Result};
use agentic_languages::LanguageProvider;
use core::result::Result as CoreResult;

use crate::fs_utils::is_source_file;
use crate::query::{QueryAnalyzer, QueryIntent};
use crate::subagent::LocalContextAgent;
use crate::expander::ContextExpander;
use crate::embedding::VectorSearchManager;
use crate::context_inclusion::{ContextManager, PrioritizedFile, FilePriority, MAX_CONTEXT_TOKENS};

/// Default system prompt used when constructing the base context.
const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful coding assistant. You help users understand and modify their codebase.\n\nWhen making changes:\n1. Be precise and accurate\n2. Explain your reasoning\n3. Provide complete, working code\n4. Follow the existing code style\n\nYou have access to the user's codebase context below.";

/// Directories ignored during project scan.
const IGNORED_DIRS: &[&str] = &["target", "node_modules", "dist", "build", ".git", ".idea", ".vscode"];

/// Builds a `Context` by scanning files under a project root.
pub struct ContextBuilder {
    /// Root directory of the project to scan
    project_root: PathBuf,
    /// Maximum number of files to include in context
    max_files: usize,
    /// Maximum file size in bytes to include
    max_file_size: usize,
    /// Optional language backend for semantic analysis
    language_backend: Option<Box<dyn LanguageProvider>>,
    /// Whether the language backend has been initialized
    language_backend_initialized: bool,
    /// Vector search manager for semantic search
    vector_manager: Option<VectorSearchManager>,
}

impl ContextBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    pub const fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_files: 50,
            max_file_size: 100_000,
            language_backend: None,
            language_backend_initialized: false,
            vector_manager: None,
        }
    }

    /// Override the maximum number of files included in context.
    #[must_use]
    pub const fn with_max_files(mut self, max_files: usize) -> Self {
        self.max_files = max_files;
        self
    }

    /// Enable a language backend for semantic analysis.
    /// 
    /// This accepts any implementation of the `LanguageProvider` trait,
    /// allowing support for multiple languages (Rust, Java, Python, etc.)
    #[must_use]
    pub fn with_language_backend(mut self, backend: Box<dyn LanguageProvider>) -> Self {
        self.language_backend = Some(backend);
        self
    }

    /// Build a `Context` for the provided query.
    ///
    /// # Errors
    /// Returns an error if file scanning or reading fails.
    pub async fn build_context(&mut self, query: &Query) -> Result<Context> {
        // Step 1: Analyze the query to extract intent
        let analyzer = QueryAnalyzer::new();
        let intent = analyzer.analyze(&query.text);
        
        tracing::info!("Query intent: action={:?}, scope={:?}, complexity={:?}", 
            intent.action, intent.scope, intent.complexity);
        tracing::debug!("Keywords: {:?}, Entities: {:?}", intent.keywords, intent.entities);

        let mut files = if query.files_context.is_empty() {
            // Step 2: Initialize backend and vector search IN PARALLEL
            let needs_backend_init = self.language_backend.is_some() && !self.language_backend_initialized;
            let needs_vector_init = self.vector_manager.is_none();

            if needs_backend_init || needs_vector_init {
                eprintln!("⚙️  Initializing systems in parallel...");
                
                // Rust-analyzer initialization (CPU-bound, blocking)
                // Take ownership temporarily to run in parallel
                let backend_handle = if needs_backend_init {
                    let mut backend = self.language_backend.take();
                    let project_root = self.project_root.clone();
                    
                    Some(tokio::task::spawn_blocking(move || {
                        eprintln!("  → Initializing rust-analyzer...");
                        if let Some(ref mut b) = backend {
                            tracing::info!("Initializing language backend...");
                            let result = b.initialize(&project_root);
                            (backend, result)
                        } else {
                            (backend, Ok(()))
                        }
                    }))
                } else {
                    None
                };

                // Vector search initialization (I/O-bound, async)
                let vector_handle = if needs_vector_init {
                    eprintln!("  → Building embedding index...");
                    let mut manager = VectorSearchManager::new(self.project_root.clone());
                    
                    Some(tokio::spawn(async move {
                        let result = manager.initialize().await;
                        (manager, result)
                    }))
                } else {
                    None
                };

                // Wait for BOTH tasks to complete simultaneously
                match (backend_handle, vector_handle) {
                    (Some(bh), Some(vh)) => {
                        // Both tasks running - wait for both
                        let (backend_result, vector_result) = tokio::join!(bh, vh);
                        
                        // Handle rust-analyzer result
                        match backend_result {
                            Ok((backend, Ok(()))) => {
                                self.language_backend = backend;
                                self.language_backend_initialized = true;
                                eprintln!("  ✓ Rust-analyzer initialized");
                                tracing::info!("Language backend initialized successfully");
                            }
                            Ok((backend, Err(e))) => {
                                self.language_backend = backend;
                                eprintln!("  ⚠️  Warning: Failed to initialize rust-analyzer: {}", e);
                                eprintln!("     Falling back to basic file scanning");
                                tracing::warn!("Failed to initialize language backend: {}", e);
                            }
                            Err(e) => {
                                eprintln!("  ⚠️  Task join error: {}", e);
                            }
                        }
                        
                        // Handle vector search result
                        match vector_result {
                            Ok((manager, Ok(()))) => {
                                self.vector_manager = Some(manager);
                                eprintln!("  ✓ Embedding index ready");
                            }
                            Ok((_, Err(e))) => {
                                eprintln!("  ⚠️  Warning: Failed to initialize vector search: {}", e);
                                tracing::warn!("Failed to initialize vector search: {}", e);
                            }
                            Err(e) => {
                                eprintln!("  ⚠️  Task join error: {}", e);
                            }
                        }
                    }
                    (Some(bh), None) => {
                        // Only backend
                        match bh.await {
                            Ok((backend, Ok(()))) => {
                                self.language_backend = backend;
                                self.language_backend_initialized = true;
                                eprintln!("  ✓ Rust-analyzer initialized");
                                tracing::info!("Language backend initialized successfully");
                            }
                            Ok((backend, Err(e))) => {
                                self.language_backend = backend;
                                eprintln!("  ⚠️  Warning: Failed to initialize rust-analyzer: {}", e);
                                eprintln!("     Falling back to basic file scanning");
                                tracing::warn!("Failed to initialize language backend: {}", e);
                            }
                            Err(e) => {
                                eprintln!("  ⚠️  Task join error: {}", e);
                            }
                        }
                    }
                    (None, Some(vh)) => {
                        // Only vector search
                        match vh.await {
                            Ok((manager, Ok(()))) => {
                                self.vector_manager = Some(manager);
                                eprintln!("  ✓ Embedding index ready");
                            }
                            Ok((_, Err(e))) => {
                                eprintln!("  ⚠️  Warning: Failed to initialize vector search: {}", e);
                                tracing::warn!("Failed to initialize vector search: {}", e);
                            }
                            Err(e) => {
                                eprintln!("  ⚠️  Task join error: {}", e);
                            }
                        }
                    }
                    (None, None) => {
                        // Nothing to initialize
                    }
                }
                
                eprintln!("✓ All systems initialized");
            }

            // Step 3: Use subagent to generate context plan (backend is required)
            if self.language_backend.is_some() && self.language_backend_initialized {
                let agent_files = self.use_subagent_for_context(&intent, &query.text).await?;
                eprintln!("✓ Intelligent context fetching found {} files", agent_files.len());
                tracing::info!("Subagent found {} files", agent_files.len());
                agent_files
            } else {
                return Err(agentic_core::Error::Other(
                    "Language backend not initialized. This should not happen.".into()
                ));
            }
        } else {
            // User provided specific files
            let mut collected = Vec::new();
            for file_path in &query.files_context {
                if let Ok(file_context) = FileContext::from_path(file_path) {
                    collected.push(file_context);
                }
            }
            if collected.is_empty() {
                let collected = self.collect_all_files();
                tracing::info!("Collected {} files from project scan", collected.len());
                collected
            } else {
                collected
            }
        };

        files.truncate(self.max_files);
        tracing::info!("Final context: {} files (max: {})", files.len(), self.max_files);

        Ok(Context::new(DEFAULT_SYSTEM_PROMPT).with_files(files))
    }

    /// Use the subagent to intelligently gather context
    async fn use_subagent_for_context(&self, intent: &QueryIntent, query_text: &str) -> Result<Vec<FileContext>> {
        // Initialize the language backend if needed
        if !self.language_backend_initialized {
            return Err(agentic_core::Error::Other("Language backend not initialized".into()));
        }

        // Create and check subagent availability
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        spinner.set_message("Checking Ollama availability...");
        let agent = LocalContextAgent::new();
        
        if !agent.is_available().await? {
            spinner.finish_and_clear();
            return Err(agentic_core::Error::Other(
                "Ollama not available. Please ensure Ollama is running with: ollama serve".into()
            ));
        }

        spinner.set_message("Generating context plan and searching embeddings...");
        tracing::info!("Using LocalContextAgent (Ollama) to generate context plan");

        // Build lightweight file tree for context
        let file_tree = self.build_file_tree(intent);

        // Run plan generation and vector search IN PARALLEL
        let plan_future = agent.generate_plan(intent, query_text, &file_tree);
        let vector_search_future = async {
            // Use vector manager if available
            if let Some(manager) = &self.vector_manager {
                // Request more results to fill context up to token limit
                match manager.search(query_text, 50).await {
                    Ok(results) => Ok::<Vec<crate::embedding::SearchResult>, agentic_core::Error>(results),
                    Err(e) => {
                        eprintln!("Warning: Vector search failed: {}", e);
                        Ok(Vec::new())
                    }
                }
            } else {
                Ok(Vec::new())
            }
        };

        // Run both in parallel using tokio::join
        let (plan_result, vector_results) = tokio::join!(plan_future, vector_search_future);
        let plan = plan_result?;
        let semantic_matches = vector_results?;

        if !semantic_matches.is_empty() {
            eprintln!("--- Semantic search found {} matches", semantic_matches.len());
            for (i, result) in semantic_matches.iter().enumerate() {
                eprintln!("  {}. {} (score: {:.3})", i + 1, result.file_path.display(), result.score);
            }
        } else {
            eprintln!("--- Semantic search: no results (store may be empty)");
        }

        spinner.finish_with_message(format!("✓ Context plan: {}", plan.reasoning));
        eprintln!("  Keywords: {:?}", plan.keywords);
        eprintln!("  Symbols: {:?}", plan.symbols);
        eprintln!("  Patterns: {:?}", plan.file_patterns);
        
        tracing::info!("Context plan generated: {}", plan.reasoning);
        tracing::debug!("Plan details: keywords={:?}, symbols={:?}, patterns={:?}, depth={}", 
            plan.keywords, plan.symbols, plan.file_patterns, plan.max_depth);

        // Use expander to execute the plan
        let expander = ContextExpander::new(
            self.language_backend.as_ref(),
            &self.project_root,
            self.max_file_size,
        );

        let plan_files = expander.expand(&plan)?;
        
        // Use context manager to intelligently add files based on priority and token limits
        let mut context_mgr = ContextManager::new(MAX_CONTEXT_TOKENS);
        
        // First, add plan files with high priority
        let mut plan_prioritized = Vec::new();
        for file in plan_files {
            plan_prioritized.push(PrioritizedFile::new(file, FilePriority::High));
        }
        
        let plan_added = crate::context_inclusion::add_prioritized_files(&mut context_mgr, plan_prioritized);
        eprintln!("  Added {} plan files ({} tokens used)", plan_added, context_mgr.token_count());
        
        // Then, fill remaining context with semantic search results
        if !context_mgr.is_full() && !semantic_matches.is_empty() {
            let mut semantic_prioritized = Vec::new();
            
            for result in semantic_matches {
                if let Ok(file_context) = FileContext::from_path(&result.file_path) {
                    // Only add if not already in context
                    if !context_mgr.files().iter().any(|f| f.path == file_context.path) {
                        semantic_prioritized.push(PrioritizedFile::with_score(
                            file_context,
                            FilePriority::Medium,
                            result.score,
                        ));
                    }
                }
            }
            
            let semantic_added = crate::context_inclusion::add_prioritized_files(&mut context_mgr, semantic_prioritized);
            eprintln!("  Added {} semantic matches ({} tokens total / {} max)", 
                semantic_added, context_mgr.token_count(), MAX_CONTEXT_TOKENS);
        }
        
        Ok(context_mgr.into_files())
    }


    /// Collect a list of readable code files under the project root.
    fn collect_all_files(&self) -> Vec<FileContext> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.project_root)
            .into_iter()
            .filter_entry(|entry_var| !Self::is_ignored(entry_var))
            .filter_map(CoreResult::ok)
        {
            if entry.file_type().is_dir() {
                continue;
            }

            if !is_source_file(entry.path()) {
                continue;
            }

            if let Ok(metadata) = entry.metadata()
                && metadata.len() > self.max_file_size as u64
            {
                continue;
            }

            if let Ok(file_context) = FileContext::from_path(&entry.path().to_path_buf()) {
                files.push(file_context);
            }

            if files.len() >= self.max_files {
                break;
            }
        }

        files
    }

    /// Determine whether a directory entry should be ignored.
    fn is_ignored(entry: &walkdir::DirEntry) -> bool {
        let file_name = entry.file_name().to_string_lossy();

        // Don't filter the root directory itself (depth 0)
        if entry.depth() == 0 {
            return false;
        }

        if file_name.starts_with('.') {
            return true;
        }

        if entry.file_type().is_dir() && IGNORED_DIRS.contains(&file_name.as_ref()) {
            return true;
        }

        false
    }

    /// Build a lightweight file tree for context planning
    fn build_file_tree(&self, intent: &QueryIntent) -> String {
        const MAX_ENTRIES: usize = 1000;
        const MAX_PER_DIR: usize = 10;
        
        let (entries, filtered_count) = self.collect_file_tree_entries(MAX_ENTRIES, MAX_PER_DIR);
        let filtered_entries = self.filter_empty_directories(entries);
        
        let dir_count = filtered_entries.iter().filter(|(_, is_dir)| *is_dir).count();
        let file_count = filtered_entries.len() - dir_count;
        
        eprintln!("File tree stats: {} dirs, {} files, {} total entries, {} filtered", 
            dir_count, file_count, filtered_entries.len(), filtered_count);

        self.categorize_and_format_tree(intent, &filtered_entries, filtered_count, MAX_ENTRIES)
    }

    fn collect_file_tree_entries(&self, max_entries: usize, max_per_dir: usize) -> (Vec<(String, bool)>, usize) {
        let mut entries: Vec<(String, bool)> = Vec::new();
        let mut filtered_count = 0;
        let mut dir_entry_counts: HashMap<String, usize> = HashMap::new();

        let walker = WalkBuilder::new(&self.project_root)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .filter_entry(|entry| {
                if let Some(file_name) = entry.path().file_name() {
                    let name = file_name.to_string_lossy();
                    if entry.file_type().map_or(false, |ft| ft.is_dir()) 
                        && IGNORED_DIRS.contains(&name.as_ref()) {
                        return false;
                    }
                }
                true
            })
            .build();

        for entry_result in walker {
            match entry_result {
                Ok(entry) => {
                    if entries.len() >= max_entries {
                        break;
                    }

                    let path = entry.path();

                    if let Ok(rel_path) = path.strip_prefix(&self.project_root) {
                        let path_str = rel_path.to_string_lossy().replace('\\', "/");
                        
                        if path_str.is_empty() {
                            continue;
                        }
                        
                        let is_dir = entry.file_type().map_or(false, |ft| ft.is_dir());

                        if !is_dir && !is_source_file(path) {
                            filtered_count += 1;
                            continue;
                        }
                        
                        let parent_dir = if let Some(parent) = Path::new(&path_str).parent() {
                            parent.to_string_lossy().to_string()
                        } else {
                            String::from(".")
                        };
                        
                        let dir_count = dir_entry_counts.entry(parent_dir.clone()).or_insert(0);
                        if *dir_count >= max_per_dir {
                            filtered_count += 1;
                            continue;
                        }
                        *dir_count += 1;
                        
                        entries.push((path_str, is_dir));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Error walking directory: {}", e);
                }
            }
        }

        (entries, filtered_count)
    }

    fn filter_empty_directories(&self, entries: Vec<(String, bool)>) -> Vec<(String, bool)> {

        let mut non_empty_dirs = std::collections::HashSet::new();
        
        for (path, is_dir) in &entries {
            if !is_dir {
                let mut current = Path::new(path);
                while let Some(parent) = current.parent() {
                    let parent_str = parent.to_string_lossy().replace('\\', "/");
                    if !parent_str.is_empty() {
                        non_empty_dirs.insert(parent_str);
                    }
                    current = parent;
                }
            }
        }
        
        entries.into_iter()
            .filter(|(path, is_dir)| {
                if *is_dir {
                    non_empty_dirs.contains(path)
                } else {
                    true
                }
            })
            .collect()
    }

    fn categorize_and_format_tree(&self, intent: &QueryIntent, filtered_entries: &[(String, bool)], _filtered_count: usize, max_entries: usize) -> String {

        // Categorize entries by match type
        let mut keyword_matches: Vec<String> = Vec::new();
        let mut symbol_matches: Vec<String> = Vec::new();
        let mut folder_matches: Vec<String> = Vec::new();
        let mut file_matches: Vec<String> = Vec::new();
        
        for (path, is_dir) in filtered_entries {
            let path_lower = path.to_lowercase();
            let name = Path::new(path).file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            let name_lower = name.to_lowercase();
            
            let mut matched = false;
            
            // Check for keyword matches (from intent.keywords)
            for keyword in &intent.keywords {
                let kw_lower = keyword.to_lowercase();
                if name_lower.contains(&kw_lower) || path_lower.contains(&kw_lower) {
                    if *is_dir {
                        keyword_matches.push(format!("{}/", path));
                    } else {
                        keyword_matches.push(path.clone());
                    }
                    matched = true;
                    break;
                }
            }
            
            if matched {
                continue;
            }
            
            // Check for entity/symbol matches (from intent.entities)
            for entity in &intent.entities {
                let entity_lower = entity.to_lowercase();
                if name_lower.contains(&entity_lower) || path_lower.contains(&entity_lower) {
                    if *is_dir {
                        symbol_matches.push(format!("{}/", path));
                    } else {
                        symbol_matches.push(path.clone());
                    }
                    matched = true;
                    break;
                }
            }
            
            if matched {
                continue;
            }
            
            // Categorize remaining by type
            if *is_dir {
                folder_matches.push(format!("{}/", path));
            } else {
                file_matches.push(path.clone());
            }
        }

        // Build categorized output
        let mut tree = String::new();
        
        if !keyword_matches.is_empty() {
            tree.push_str("Keyword Matches:\n");
            for item in &keyword_matches {
                tree.push_str(&format!("  {}\n", item));
            }
            tree.push('\n');
        }
        
        if !symbol_matches.is_empty() {
            tree.push_str("Symbol/Entity Matches:\n");
            for item in &symbol_matches {
                tree.push_str(&format!("  {}\n", item));
            }
            tree.push('\n');
        }
        
        
        tree.push_str("All Folders:\n");
        for item in &folder_matches {
            tree.push_str(&format!("  {}\n", item));
        }
        tree.push('\n');
        
        tree.push_str("All Files:\n");
        let display_count = file_matches.len().min(100);
        for item in file_matches.iter().take(display_count) {
            tree.push_str(&format!("  {}\n", item));
        }
        
        if file_matches.len() > display_count {
            tree.push_str(&format!("  ... and {} more files\n", file_matches.len() - display_count));
        }

        if filtered_entries.len() >= max_entries {
            tree.push_str(&format!("\n(Truncated at {} entries)\n", max_entries));
        }

        tree
    }
}
