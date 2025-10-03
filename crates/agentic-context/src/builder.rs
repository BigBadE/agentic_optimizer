use std::path::PathBuf;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};

use agentic_core::{Context, FileContext, Query, Result};
use agentic_languages::LanguageProvider;
use core::result::Result as CoreResult;

use crate::fs_utils::is_source_file;
use crate::query::{QueryAnalyzer, QueryIntent};
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

    /// Use hybrid search to intelligently gather context
    async fn use_subagent_for_context(&self, _intent: &QueryIntent, query_text: &str) -> Result<Vec<FileContext>> {
        // Initialize the language backend if needed
        if !self.language_backend_initialized {
            return Err(agentic_core::Error::Other("Language backend not initialized".into()));
        }

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        spinner.set_message("Running hybrid search (BM25 + Vector)...");
        tracing::info!("Using hybrid BM25 + Vector search for context");

        // Use hybrid search (BM25 + vector embeddings)
        let semantic_matches = if let Some(manager) = &self.vector_manager {
            match manager.search(query_text, 50).await {
                Ok(results) => results,
                Err(e) => {
                    eprintln!("Warning: Hybrid search failed: {}", e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        if !semantic_matches.is_empty() {
            eprintln!("--- Hybrid search found {} matches", semantic_matches.len());
            for (i, result) in semantic_matches.iter().enumerate().take(10) {
                eprintln!("  {}. {} (score: {:.3})", i + 1, result.file_path.display(), result.score);
            }
            if semantic_matches.len() > 10 {
                eprintln!("  ... and {} more", semantic_matches.len() - 10);
            }
        } else {
            eprintln!("--- Hybrid search: no results (store may be empty)");
        }

        spinner.finish_with_message("✓ Hybrid search complete");
        
        // Filter out low-quality small chunks before processing
        let filtered_matches: Vec<_> = semantic_matches.iter()
            .filter(|result| {
                // Estimate chunk size from line range in path
                if let Some(path_str) = result.file_path.to_str() {
                    if let Some((_, range_part)) = path_str.rsplit_once(':') {
                        if let Some((start_str, end_str)) = range_part.split_once('-') {
                            if let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>()) {
                                let line_count = end.saturating_sub(start);
                                // Estimate ~10 tokens per line
                                let estimated_tokens = line_count * 10;
                                return Self::should_include_chunk(estimated_tokens, result.score);
                            }
                        }
                    }
                }
                // If we can't parse, include it
                true
            })
            .cloned()
            .collect();
        
        eprintln!("  After quality filtering: {} chunks (removed {} low-quality)", 
            filtered_matches.len(), 
            semantic_matches.len() - filtered_matches.len());
        
        // Use context manager to add hybrid search results
        let mut context_mgr = ContextManager::new(MAX_CONTEXT_TOKENS);
        
        // Add hybrid search results with priority based on file type
        // Merge overlapping chunks from the same file
        use std::collections::HashMap;
        
        // Group chunks by file (using filtered matches)
        let mut file_chunks: HashMap<PathBuf, Vec<(usize, usize, f32)>> = HashMap::new();
        
        for result in &filtered_matches {
            // Parse chunk path: "file.rs:start-end"
            if let Some(path_str) = result.file_path.to_str() {
                if let Some((file_part, range_part)) = path_str.rsplit_once(':') {
                    let path = PathBuf::from(file_part);
                    if let Some((start_str, end_str)) = range_part.split_once('-') {
                        if let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>()) {
                            file_chunks.entry(path).or_insert_with(Vec::new).push((start, end, result.score));
                        }
                    }
                }
            }
        }
        
        // Merge overlapping chunks for each file
        let mut search_prioritized = Vec::new();
        
        for (file_path, mut chunks) in file_chunks {
            // Sort chunks by start line
            chunks.sort_by_key(|(start, _, _)| *start);
            
            // Merge overlapping chunks
            let merged = self.merge_overlapping_chunks(chunks);
            
            // Extract merged chunks
            let is_code = Self::is_code_file(&file_path);
            for (start, end, score) in merged {
                match self.extract_chunk_with_context(&file_path, start, end, is_code) {
                    Ok(chunk_ctx) => {
                        let priority = if is_code {
                            FilePriority::High
                        } else {
                            FilePriority::Medium
                        };
                        
                        search_prioritized.push(PrioritizedFile::with_score(
                            chunk_ctx,
                            priority,
                            score,
                        ));
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to extract chunk from {}: {}", file_path.display(), e);
                    }
                }
            }
        }
        
        // Keep track of scores for display (total, bm25, vector)
        let file_scores: Vec<(PathBuf, f32, Option<f32>, Option<f32>)> = filtered_matches.iter()
            .filter_map(|result| {
                if let Some(path_str) = result.file_path.to_str() {
                    if let Some((file_part, _)) = path_str.rsplit_once(':') {
                        Some((
                            PathBuf::from(file_part),
                            result.score,
                            result.bm25_score,
                            result.vector_score,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        let added = crate::context_inclusion::add_prioritized_files(&mut context_mgr, search_prioritized);
        eprintln!("  Added {} chunks from hybrid search ({} tokens used)", added, context_mgr.token_count());
        
        // Show relevant sections for prompt
        eprintln!("\n=== RELEVANT SECTIONS FOR PROMPT ===");
        eprintln!("Total: {} chunks ({} tokens)\n", context_mgr.file_count(), context_mgr.token_count());
        
        // List all chunks with their sections and scores
        for (i, file) in context_mgr.files().iter().enumerate() {
            let tokens = crate::context_inclusion::ContextManager::estimate_tokens(&file.content);
            
            // Find the scores for this file
            let (total_score, bm25, vector) = file_scores.iter()
                .find(|(path, _, _, _)| path == &file.path)
                .map(|(_, total, bm25, vector)| (*total, *bm25, *vector))
                .unwrap_or((0.0, None, None));
            
            // Extract section info from content
            let section_info = if let Some(first_line) = file.content.lines().next() {
                if first_line.starts_with("--- Context: lines") {
                    // Code file with context
                    first_line.trim_start_matches("--- Context: lines ").trim_end_matches(" ---").to_string()
                } else if first_line.starts_with("--- Lines") {
                    // Text file without context
                    first_line.trim_start_matches("--- Lines ").trim_end_matches(" ---").to_string()
                } else if file.content.lines().count() < 100 {
                    // Small content without markers is likely a chunk
                    format!("chunk (~{} lines)", file.content.lines().count())
                } else {
                    "full file".to_string()
                }
            } else {
                "chunk".to_string()
            };
            
            // Format score display
            // Note: component scores are raw RRF contributions, total is normalized to 0-1
            let score_display = match (bm25, vector) {
                (Some(b), Some(v)) => {
                    let sum = b + v;
                    format!("score: {:.3} (bm25: {:.3} + vec: {:.3} = {:.3})", total_score, b, v, sum)
                },
                (Some(b), None) => format!("score: {:.3} (bm25: {:.3})", total_score, b),
                (None, Some(v)) => format!("score: {:.3} (vec: {:.3})", total_score, v),
                (None, None) => format!("score: {:.3}", total_score),
            };
            
            eprintln!("{}. {} [{}] ({}, {} tokens)", 
                i + 1, 
                file.path.display(), 
                section_info,
                score_display,
                tokens
            );
        }
        eprintln!("=====================================\n");
        
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

    /// Merge overlapping chunks considering context expansion
    fn merge_overlapping_chunks(&self, chunks: Vec<(usize, usize, f32)>) -> Vec<(usize, usize, f32)> {
        if chunks.is_empty() {
            return Vec::new();
        }
        
        let mut merged = Vec::new();
        let mut current_start = chunks[0].0;
        let mut current_end = chunks[0].1;
        let mut max_score = chunks[0].2;
        
        const CONTEXT_LINES: usize = 50;
        
        for (start, end, score) in chunks.into_iter().skip(1) {
            // Check if chunks overlap when considering context expansion
            // Two chunks overlap if: start - CONTEXT <= current_end + CONTEXT
            let expanded_current_end = current_end + CONTEXT_LINES;
            let expanded_start = start.saturating_sub(CONTEXT_LINES);
            
            if expanded_start <= expanded_current_end {
                // Merge: extend current chunk
                current_end = current_end.max(end);
                max_score = max_score.max(score);
            } else {
                // No overlap: save current and start new
                merged.push((current_start, current_end, max_score));
                current_start = start;
                current_end = end;
                max_score = score;
            }
        }
        
        // Add the last chunk
        merged.push((current_start, current_end, max_score));
        
        merged
    }

    /// Extract a chunk with surrounding context (only for code files)
    fn extract_chunk_with_context(&self, file_path: &PathBuf, start_line: usize, end_line: usize, include_context: bool) -> Result<FileContext> {
        use std::fs;
        
        let content = fs::read_to_string(file_path)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to read file: {}", e)))?;
        
        let lines: Vec<&str> = content.lines().collect();
        
        // Calculate context window (±50 lines for code, exact chunk for text)
        let (context_start, context_end) = if include_context {
            const CONTEXT_LINES: usize = 50;
            (
                start_line.saturating_sub(CONTEXT_LINES).max(1),
                (end_line + CONTEXT_LINES).min(lines.len())
            )
        } else {
            // Text files: exact chunk only
            (start_line, end_line)
        };
        
        // Extract lines with context
        let chunk_lines: Vec<&str> = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i + 1 >= context_start && *i + 1 <= context_end)
            .map(|(_, line)| *line)
            .collect();
        
        let chunk_content = chunk_lines.join("\n");
        
        // Create a marker to show the actual matched chunk (only if we added context)
        let marker = if include_context && (context_start < start_line || context_end > end_line) {
            format!("\n\n--- Matched chunk: lines {}-{} ---\n", start_line, end_line)
        } else {
            String::new()
        };
        
        let final_content = if !marker.is_empty() {
            format!("--- Context: lines {}-{} ---\n{}{}", context_start, context_end, chunk_content, marker)
        } else if include_context {
            format!("--- Context: lines {}-{} ---\n{}", context_start, context_end, chunk_content)
        } else {
            // Text files without context - still show line range
            format!("--- Lines {}-{} ---\n{}", context_start, context_end, chunk_content)
        };
        
        Ok(FileContext {
            path: file_path.clone(),
            content: final_content,
        })
    }

    /// Check if a chunk should be included based on size and score
    fn should_include_chunk(tokens: usize, score: f32) -> bool {
        if tokens < 50 {
            return false;  // Always filter tiny chunks
        }
        if tokens < 100 && score < 0.7 {
            return false;  // Filter small low-score chunks
        }
        true
    }

    /// Check if a file is a code file (not documentation/text)
    fn is_code_file(path: &PathBuf) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(ext, 
                "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "java" | "c" | "cpp" | 
                "h" | "hpp" | "go" | "rb" | "php" | "cs" | "swift" | "kt" | "scala" |
                "toml" | "yaml" | "yml" | "json" | "xml"
            )
        } else {
            false
        }
    }

    /// Check if a directory entry should be ignored
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

    /// Search for relevant context without building full context (for benchmarking)
    ///
    /// # Errors
    /// Returns an error if search initialization or execution fails.
    pub async fn search_context(&mut self, query: &str) -> Result<Vec<crate::embedding::SearchResult>> {
        if self.vector_manager.is_none() {
            eprintln!("⚙️  Initializing vector search...");
            let mut manager = VectorSearchManager::new(self.project_root.clone());
            manager.initialize().await?;
            self.vector_manager = Some(manager);
        }

        let manager = self.vector_manager.as_ref().unwrap();
        let results = manager.search(query, 50).await?;

        Ok(results)
    }

}
