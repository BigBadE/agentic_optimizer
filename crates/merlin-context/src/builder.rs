use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::spawn;
use tokio::task::{JoinError, spawn_blocking};
use walkdir::{DirEntry, WalkDir};

use core::result::Result as CoreResult;
use merlin_core::prompts::load_prompt;
use merlin_core::{Context, Error, FileContext, Query, Result};
use merlin_languages::LanguageProvider;

use crate::context_inclusion::{
    ContextManager, FilePriority, MAX_CONTEXT_TOKENS, PrioritizedFile, add_prioritized_files,
};
use crate::embedding::{ProgressCallback, SearchResult, VectorSearchManager};
use crate::fs_utils::is_source_file;
use crate::query::{QueryAnalyzer, QueryIntent};

type BackendJoinResult = CoreResult<(Option<Box<dyn LanguageProvider>>, Result<()>), JoinError>;
type VectorJoinResult = CoreResult<(VectorSearchManager, Result<()>), JoinError>;
type FileScoreInfo = (PathBuf, f32, Option<f32>, Option<f32>);
type ProcessSearchResultsReturn = (Vec<PrioritizedFile>, Vec<FileScoreInfo>);
type FileChunksMap = HashMap<PathBuf, Vec<(usize, usize, f32)>>;

/// Loads the default system prompt from the prompts directory
///
/// # Panics
/// Panics if the `coding_assistant` prompt cannot be loaded (should never happen as prompts are embedded)
fn load_default_system_prompt() -> String {
    load_prompt("coding_assistant")
        .unwrap_or_else(|err| panic!("Failed to load coding_assistant prompt: {err}"))
}

/// Directories ignored during project scan.
const IGNORED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    ".git",
    ".idea",
    ".vscode",
];

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
    /// Optional progress callback for embedding operations
    progress_callback: Option<ProgressCallback>,
}

impl ContextBuilder {
    /// Create a new builder with defaults.
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_files: 50,
            max_file_size: 100_000,
            language_backend: None,
            language_backend_initialized: false,
            vector_manager: None,
            progress_callback: None,
        }
    }

    /// Override the maximum number of files included in context.
    #[must_use]
    pub fn with_max_files(mut self, max_files: usize) -> Self {
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

    /// Set a progress callback for embedding operations
    #[must_use]
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Build a `Context` for the provided query.
    ///
    /// # Errors
    /// Returns an error if file scanning or reading fails.
    pub async fn build_context(&mut self, query: &Query) -> Result<Context> {
        // Step 1: Analyze the query to extract intent
        let analyzer = QueryAnalyzer;
        let intent = analyzer.analyze(&query.text);

        tracing::info!(
            "Query intent: action={:?}, scope={:?}, complexity={:?}",
            intent.action,
            intent.scope,
            intent.complexity
        );
        tracing::debug!(
            "Keywords: {:?}, Entities: {:?}",
            intent.keywords,
            intent.entities
        );

        let mut files = if query.files_context.is_empty() {
            // Step 2: Initialize backend and vector search IN PARALLEL
            self.initialize_systems_parallel().await?;

            // Step 3: Use hybrid search for context (vector search works without backend)
            let agent_files = self.use_subagent_for_context(&intent, &query.text).await?;
            tracing::info!(
                "Intelligent context fetching found {} files",
                agent_files.len()
            );
            agent_files
        } else {
            // User provided specific files
            let mut collected = Vec::new();
            for file_path in &query.files_context {
                if let Ok(file_context) = FileContext::from_path(file_path) {
                    collected.push(file_context);
                }
            }
            if collected.is_empty() {
                let all_files = self.collect_all_files();
                tracing::info!("Collected {} files from project scan", all_files.len());
                all_files
            } else {
                collected
            }
        };

        files.truncate(self.max_files);
        tracing::info!(
            "Final context: {} files (max: {})",
            files.len(),
            self.max_files
        );

        Ok(Context::new(load_default_system_prompt()).with_files(files))
    }

    /// Use hybrid search to intelligently gather context
    ///
    /// # Errors
    /// Returns an error if hybrid search fails
    async fn use_subagent_for_context(
        &self,
        _intent: &QueryIntent,
        query_text: &str,
    ) -> Result<Vec<FileContext>> {
        // Perform hybrid search
        let semantic_matches = self.perform_hybrid_search(query_text).await?;

        // Process search results into prioritized chunks
        let (search_prioritized, file_scores) = self.process_search_results(&semantic_matches);

        // Use context manager to add hybrid search results
        let mut context_mgr = ContextManager::new(MAX_CONTEXT_TOKENS);

        let added = add_prioritized_files(&mut context_mgr, search_prioritized);
        tracing::info!(
            "Added {} chunks from hybrid search ({} tokens used)",
            added,
            context_mgr.token_count()
        );

        // Show relevant sections for prompt
        tracing::info!("RELEVANT SECTIONS FOR PROMPT");
        tracing::info!(
            "Total: {} chunks ({} tokens)",
            context_mgr.file_count(),
            context_mgr.token_count()
        );

        // List all chunks with their sections and scores
        for (index, file) in context_mgr.files().iter().enumerate() {
            let tokens = ContextManager::estimate_tokens(&file.content);

            // Find the scores for this file
            let (total_score, bm25, vector) = file_scores
                .iter()
                .find(|(path, _, _, _)| path == &file.path)
                .map_or((0.0, None, None), |(_, total, bm25, vector)| {
                    (*total, *bm25, *vector)
                });

            // Extract section info from content
            let section_info = file.content.lines().next().map_or_else(
                || "chunk".to_owned(),
                |first_line| {
                    if first_line.starts_with("--- Context: lines") {
                        // Code file with context
                        first_line
                            .trim_start_matches("--- Context: lines ")
                            .trim_end_matches(" ---")
                            .to_owned()
                    } else if first_line.starts_with("--- Lines") {
                        // Text file without context
                        first_line
                            .trim_start_matches("--- Lines ")
                            .trim_end_matches(" ---")
                            .to_owned()
                    } else if file.content.lines().count() < 100 {
                        // Small content without markers is likely a chunk
                        format!("chunk (~{} lines)", file.content.lines().count())
                    } else {
                        "full file".to_owned()
                    }
                },
            );

            // Format score display
            // Note: component scores are raw RRF contributions, total is normalized to 0-1
            let score_display = match (bm25, vector) {
                (Some(bm25_score), Some(vec_score)) => {
                    let sum = bm25_score + vec_score;
                    format!(
                        "score: {total_score:.3} (bm25: {bm25_score:.3} + vec: {vec_score:.3} = {sum:.3})"
                    )
                }
                (Some(bm25_score), None) => {
                    format!("score: {total_score:.3} (bm25: {bm25_score:.3})")
                }
                (None, Some(vec_score)) => format!("score: {total_score:.3} (vec: {vec_score:.3})"),
                (None, None) => format!("score: {total_score:.3}"),
            };

            tracing::info!(
                "{}. {} [{}] ({}, {} tokens)",
                index + 1,
                file.path.display(),
                section_info,
                score_display,
                tokens
            );
        }
        tracing::info!("=====================================");

        let files = context_mgr.into_files();

        Ok(files)
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
    fn merge_overlapping_chunks(chunks: Vec<(usize, usize, f32)>) -> Vec<(usize, usize, f32)> {
        const CONTEXT_LINES: usize = 50;

        if chunks.is_empty() {
            return Vec::new();
        }

        let mut merged = Vec::new();
        let mut current_start = chunks[0].0;
        let mut current_end = chunks[0].1;
        let mut max_score = chunks[0].2;

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
    ///
    /// # Errors
    /// Returns an error if file cannot be read
    fn extract_chunk_with_context(
        file_path: &PathBuf,
        start_line: usize,
        end_line: usize,
        include_context: bool,
    ) -> Result<FileContext> {
        use std::fs;

        let content = fs::read_to_string(file_path)
            .map_err(|read_error| Error::Other(format!("Failed to read file: {read_error}")))?;

        let lines: Vec<&str> = content.lines().collect();

        // Calculate context window (±50 lines for code, exact chunk for text)
        let (context_start, context_end) = if include_context {
            const CONTEXT_LINES: usize = 50;
            (
                (start_line.saturating_sub(CONTEXT_LINES)).max(1),
                (end_line + CONTEXT_LINES).min(lines.len()),
            )
        } else {
            // Text files: exact chunk only
            (start_line, end_line)
        };

        // Extract lines with context
        let chunk_lines: Vec<&str> = lines
            .iter()
            .enumerate()
            .filter(|(line_index, _)| *line_index + 1 >= context_start && *line_index < context_end)
            .map(|(_, line)| *line)
            .collect();

        let chunk_content = chunk_lines.join("\n");

        // Create a marker to show the actual matched chunk (only if we added context)
        let marker = if include_context && (context_start < start_line || context_end > end_line) {
            format!("\n\n--- Matched chunk: lines {start_line}-{end_line} ---\n")
        } else {
            String::default()
        };

        let final_content = if !marker.is_empty() {
            format!("--- Context: lines {context_start}-{context_end} ---\n{chunk_content}{marker}")
        } else if include_context {
            format!("--- Context: lines {context_start}-{context_end} ---\n{chunk_content}")
        } else {
            // Text files without context - still show line range
            format!("--- Lines {context_start}-{context_end} ---\n{chunk_content}")
        };

        Ok(FileContext {
            path: file_path.clone(),
            content: final_content,
        })
    }

    /// Check if a chunk should be included based on size and score
    fn should_include_chunk(tokens: usize, score: f32) -> bool {
        if tokens < 50 {
            return false; // Always filter tiny chunks
        }
        if tokens < 100 && score < 0.7 {
            return false; // Filter small low-score chunks
        }
        true
    }

    /// Check if a file is a code file (not documentation/text)
    fn is_code_file(path: &Path) -> bool {
        let Some(ext) = path.extension() else {
            return false;
        };
        ext.to_str().is_some_and(|ext| {
            matches!(
                ext,
                "rs" | "py"
                    | "js"
                    | "ts"
                    | "jsx"
                    | "tsx"
                    | "java"
                    | "c"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "go"
                    | "rb"
                    | "php"
                    | "cs"
                    | "swift"
                    | "kt"
                    | "scala"
                    | "toml"
                    | "yaml"
                    | "yml"
                    | "json"
                    | "xml"
            )
        })
    }

    /// Check if a directory entry should be ignored
    fn is_ignored(entry: &DirEntry) -> bool {
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

    /// Handle backend initialization result
    fn handle_backend_result(&mut self, backend_result: BackendJoinResult) {
        if let Ok((backend, Ok(()))) = backend_result {
            self.language_backend = backend;
            self.language_backend_initialized = true;
            tracing::info!("Rust-analyzer initialized");
        } else if let Ok((backend, Err(backend_error))) = backend_result {
            self.language_backend = backend;
            tracing::warn!("Failed to initialize rust-analyzer: {backend_error}");
            tracing::warn!("Falling back to basic file scanning");
        } else if let Err(join_error) = backend_result {
            tracing::error!("Backend task join error: {join_error}");
        }
    }

    /// Handle vector search initialization result
    fn handle_vector_result(&mut self, vector_result: VectorJoinResult) {
        if let Ok((manager, Ok(()))) = vector_result {
            self.vector_manager = Some(manager);
            tracing::info!("Embedding index ready");
        } else if let Ok((_, Err(vector_error))) = vector_result {
            tracing::warn!("Failed to initialize vector search: {vector_error}");
        } else if let Err(join_error) = vector_result {
            tracing::error!("Vector search task join error: {join_error}");
        }
    }

    /// Initialize backend with timeout
    async fn initialize_backend_with_timeout(
        backend: Option<Box<dyn LanguageProvider>>,
        project_root: PathBuf,
    ) -> (Option<Box<dyn LanguageProvider>>, Result<()>) {
        use tokio::time::{Duration, timeout};

        let backend_task = spawn_blocking(move || {
            tracing::info!("Initializing rust-analyzer...");
            let mut backend_mut = backend;
            if let Some(ref mut backend_ref) = backend_mut {
                tracing::info!("Initializing language backend...");
                let result = backend_ref.initialize(&project_root);
                (backend_mut, result)
            } else {
                (backend_mut, Ok(()))
            }
        });

        // Timeout after 30 seconds
        match timeout(Duration::from_secs(30), backend_task).await {
            Ok(Ok(result)) => result,
            Ok(Err(join_error)) => {
                tracing::error!("Backend task join error: {join_error}");
                (None, Err(Error::Other("Backend task panicked".into())))
            }
            Err(_timeout) => {
                tracing::warn!("Backend initialization timed out after 30s");
                (
                    None,
                    Err(Error::Other("Backend initialization timeout".into())),
                )
            }
        }
    }

    /// Initializes systems (language backend and vector search) in parallel.
    ///
    /// # Errors
    /// Returns an error if critical initialization fails.
    async fn initialize_systems_parallel(&mut self) -> Result<()> {
        let needs_backend_init =
            self.language_backend.is_some() && !self.language_backend_initialized;
        let needs_vector_init = self.vector_manager.is_none();

        if !needs_backend_init && !needs_vector_init {
            return Ok(());
        }

        tracing::info!("Initializing systems in parallel...");

        // Rust-analyzer initialization (CPU-bound, blocking) with timeout
        let backend_handle = needs_backend_init.then(|| {
            let backend = self.language_backend.take();
            let project_root = self.project_root.clone();

            spawn(async move { Self::initialize_backend_with_timeout(backend, project_root).await })
        });

        // Vector search initialization (I/O-bound, async)
        // Use partial initialization to avoid blocking on full rebuild
        let vector_handle = needs_vector_init.then(|| {
            tracing::info!("Loading embedding cache (non-blocking)...");
            let mut manager = VectorSearchManager::new(self.project_root.clone());

            if let Some(callback) = self.progress_callback.clone() {
                manager = manager.with_progress_callback(callback);
            }

            spawn(async move {
                // Try partial init first (fast, uses cache only)
                let result = match manager.initialize_partial().await {
                    Ok(()) => {
                        tracing::info!("Using cached embeddings immediately");
                        Ok(())
                    }
                    Err(error) => {
                        tracing::warn!(
                            "No cache available, will proceed without embeddings: {error}"
                        );
                        Err(error)
                    }
                };
                (manager, result)
            })
        });

        // Wait for BOTH tasks to complete simultaneously
        match (backend_handle, vector_handle) {
            (Some(backend_hdl), Some(vector_hdl)) => {
                let (backend_result, vector_result) = tokio::join!(backend_hdl, vector_hdl);
                self.handle_backend_result(backend_result);
                self.handle_vector_result(vector_result);
            }
            (Some(backend_hdl), None) => {
                let backend_result = backend_hdl.await;
                self.handle_backend_result(backend_result);
            }
            (None, Some(vector_hdl)) => {
                let vector_result = vector_hdl.await;
                self.handle_vector_result(vector_result);
            }
            (None, None) => {
                // Nothing to initialize
            }
        }

        tracing::info!("All systems initialized");
        Ok(())
    }

    /// Search for relevant context without building full context (for benchmarking)
    ///
    /// # Errors
    /// Returns an error if search initialization or execution fails.
    pub async fn search_context(&mut self, query: &str) -> Result<Vec<SearchResult>> {
        if self.vector_manager.is_none() {
            tracing::info!("Initializing vector search...");
            let mut manager = VectorSearchManager::new(self.project_root.clone());

            if let Some(callback) = self.progress_callback.clone() {
                manager = manager.with_progress_callback(callback);
            }

            manager.initialize().await?;
            self.vector_manager = Some(manager);
        }

        let Some(manager) = self.vector_manager.as_ref() else {
            return Err(Error::Other("Vector manager should be initialized".into()));
        };
        let results = manager.search(query, 50).await?;

        Ok(results)
    }

    /// Performs hybrid search (BM25 + vector) for relevant code chunks.
    ///
    /// # Errors
    /// Returns an error if hybrid search fails
    async fn perform_hybrid_search(&self, query_text: &str) -> Result<Vec<SearchResult>> {
        tracing::info!("Running hybrid search (BM25 + Vector)...");
        tracing::info!("Using hybrid BM25 + Vector search for context");

        let semantic_matches = if let Some(manager) = &self.vector_manager {
            match manager.search(query_text, 50).await {
                Ok(results) => results,
                Err(search_error) => {
                    tracing::warn!("Hybrid search failed: {search_error}");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        if semantic_matches.is_empty() {
            tracing::info!("Hybrid search: no results (store may be empty)");
        } else {
            tracing::info!("Hybrid search found {} matches", semantic_matches.len());
            for (idx, result) in semantic_matches.iter().enumerate().take(10) {
                tracing::debug!(
                    "  {}. {} (score: {:.3})",
                    idx + 1,
                    result.file_path.display(),
                    result.score
                );
            }
            if semantic_matches.len() > 10 {
                tracing::debug!("  ... and {} more", semantic_matches.len() - 10);
            }
        }

        tracing::info!("Hybrid search complete");
        Ok(semantic_matches)
    }

    /// Helper to process merged chunks for a single file.
    fn process_merged_chunks(
        file_path: &PathBuf,
        merged: Vec<(usize, usize, f32)>,
        is_code: bool,
        search_prioritized: &mut Vec<PrioritizedFile>,
    ) {
        for (start, end, score) in merged {
            match Self::extract_chunk_with_context(file_path, start, end, is_code) {
                Ok(chunk_ctx) => {
                    let priority = if is_code {
                        FilePriority::High
                    } else {
                        FilePriority::Medium
                    };

                    search_prioritized
                        .push(PrioritizedFile::with_score(chunk_ctx, priority, score));
                }
                Err(extract_error) => {
                    tracing::warn!(
                        "Failed to extract chunk from {}: {extract_error}",
                        file_path.display()
                    );
                }
            }
        }
    }

    /// Processes search results into prioritized file chunks.
    fn process_search_results(
        &self,
        semantic_matches: &[SearchResult],
    ) -> ProcessSearchResultsReturn {
        // Filter out low-quality small chunks
        let filtered_matches: Vec<_> = semantic_matches
            .iter()
            .filter(|result| {
                if let Some(path_str) = result.file_path.to_str()
                    && let Some((_, range_part)) = path_str.rsplit_once(':')
                    && let Some((start_str, end_str)) = range_part.split_once('-')
                    && let (Ok(start), Ok(end)) =
                        (start_str.parse::<usize>(), end_str.parse::<usize>())
                {
                    let line_count = end - start;
                    let estimated_tokens = line_count * 10;
                    return Self::should_include_chunk(estimated_tokens, result.score);
                }
                true
            })
            .collect();

        tracing::info!(
            "After quality filtering: {} chunks (removed {} low-quality)",
            filtered_matches.len(),
            semantic_matches.len() - filtered_matches.len()
        );

        // Group chunks by file
        let mut file_chunks: FileChunksMap = HashMap::new();

        for result in &filtered_matches {
            if let Some(path_str) = result.file_path.to_str()
                && let Some((file_part, range_part)) = path_str.rsplit_once(':')
            {
                // Convert relative path to absolute by joining with project root
                let relative_path = PathBuf::from(file_part);
                let absolute_path = self.project_root.join(relative_path);
                if let Some((start_str, end_str)) = range_part.split_once('-')
                    && let (Ok(start), Ok(end)) =
                        (start_str.parse::<usize>(), end_str.parse::<usize>())
                {
                    file_chunks
                        .entry(absolute_path)
                        .or_default()
                        .push((start, end, result.score));
                }
            }
        }

        // Merge overlapping chunks and extract
        let mut search_prioritized = Vec::new();

        for (file_path, mut chunks) in file_chunks {
            chunks.sort_by_key(|(start, _, _)| *start);
            let merged = Self::merge_overlapping_chunks(chunks);
            let is_code = Self::is_code_file(&file_path);

            Self::process_merged_chunks(&file_path, merged, is_code, &mut search_prioritized);
        }

        // Track scores for display
        let file_scores: Vec<FileScoreInfo> = filtered_matches
            .iter()
            .filter_map(|result| {
                let path_str = result.file_path.to_str()?;
                let (file_part, _) = path_str.rsplit_once(':')?;
                Some((
                    PathBuf::from(file_part),
                    result.score,
                    result.bm25_score,
                    result.vector_score,
                ))
            })
            .collect();

        (search_prioritized, file_scores)
    }
}
