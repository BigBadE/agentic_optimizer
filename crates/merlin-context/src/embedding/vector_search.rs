//! Vector search manager with persistent caching.

use crate::embedding::client::EmbeddingProvider;
use futures::stream::{FuturesUnordered, StreamExt as _};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::hash::{Hash as _, Hasher as _};
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::task::spawn_blocking;
use tracing::{info, warn};

use crate::context_inclusion::MIN_SIMILARITY_SCORE;
use crate::embedding::chunking::{FileChunk, chunk_file};
use crate::embedding::{BM25Index, EmbeddingClient, SearchResult, VectorStore, generate_preview};
use crate::fs_utils::is_source_file;
use bincode::config::standard as bincode_config;
use bincode::{Decode, Encode, decode_from_slice, encode_to_vec};
use merlin_core::{CoreResult as Result, Error};

type ChunkResult = (PathBuf, FileChunk, Vec<f32>, String, u64);
type FileChunksData = (PathBuf, String, Vec<FileChunk>, u64);
type FileChunkMap = HashMap<PathBuf, Vec<(usize, FileChunk, u64)>>;

/// Progress callback for embedding operations
pub type ProgressCallback = Arc<dyn Fn(&str, u64, Option<u64>) + Send + Sync>;

/// Helper struct to hold vector score data
struct VectorScoreData {
    scores: HashMap<PathBuf, f32>,
    previews: HashMap<PathBuf, String>,
    max_score: f32,
}

/// Parameters for computing combined scores
struct ScoreComputationParams<'score> {
    bm25_scores: &'score HashMap<PathBuf, f32>,
    vector_scores: &'score HashMap<PathBuf, f32>,
    previews: &'score HashMap<PathBuf, String>,
    max_bm25: f32,
    max_vector: f32,
    bm25_weight: f32,
    vector_weight: f32,
}

// Removed feature-gated import to avoid unexpected-cfg; we fall back to an empty graph instead.

/// Cache entry for a chunk embedding
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
struct CachedEmbedding {
    /// File path
    path: PathBuf,
    /// Chunk identifier
    chunk_id: String,
    /// Start line
    start_line: usize,
    /// End line
    end_line: usize,
    /// Embedding vector
    embedding: Vec<f32>,
    /// Chunk content preview
    preview: String,
    /// Last modification time (for informational purposes)
    modified: SystemTime,
    /// Content hash (xxHash64 for fast validation)
    content_hash: u64,
}

impl Default for VectorCache {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            embeddings: Vec::default(),
        }
    }
}

/// Cached vector database
#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
struct VectorCache {
    /// Version identifier for cache invalidation
    version: u32,
    /// Cached embeddings
    embeddings: Vec<CachedEmbedding>,
}

impl VectorCache {
    const VERSION: u32 = 5; // Bumped for content_hash field

    fn is_valid(&self) -> bool {
        self.version == Self::VERSION
    }
}

/// Vector search manager with caching and BM25 keyword search
pub struct VectorSearchManager<E: EmbeddingProvider = EmbeddingClient> {
    /// In-memory vector store
    store: VectorStore,
    /// BM25 keyword search index
    bm25: BM25Index,
    /// File modification times for cache invalidation
    file_times: HashMap<PathBuf, SystemTime>,
    /// File content hashes for validation
    file_hashes: HashMap<PathBuf, u64>,
    /// Embedding client
    client: E,
    /// Project root
    project_root: PathBuf,
    /// Cache file path
    cache_path: PathBuf,
    /// Optional progress callback
    progress_callback: Option<ProgressCallback>,
}

impl<E: EmbeddingProvider> VectorSearchManager<E> {
    /// Compute hash of file content for cache validation
    fn compute_file_hash(content: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Create a new vector search manager with a custom embedding provider
    pub fn with_provider(project_root: PathBuf, client: E) -> Self {
        let cache_path = Self::resolve_cache_path(&project_root);

        Self {
            store: VectorStore::default(),
            bm25: BM25Index::default(),
            file_times: HashMap::default(),
            file_hashes: HashMap::default(),
            client,
            project_root,
            cache_path,
            progress_callback: None,
        }
    }
}

impl VectorSearchManager<EmbeddingClient> {
    /// Create a new vector search manager with default Ollama client
    pub fn new(project_root: PathBuf) -> Self {
        Self::with_provider(project_root, EmbeddingClient::default())
    }
}

impl<E: EmbeddingProvider> VectorSearchManager<E> {
    /// Set a progress callback for embedding operations
    #[must_use]
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Report progress if callback is set
    fn report_progress(&self, stage: &str, current: u64, total: Option<u64>) {
        if let Some(callback) = &self.progress_callback {
            callback(stage, current, total);
        }
    }

    /// Resolve cache path with environment override support
    ///
    /// Env variables:
    /// - `MERLIN_FOLDER`: directory for the entire Merlin state (e.g. `.merlin`). We store embeddings at `{MERLIN_FOLDER}/cache/vector/embeddings.bin`
    fn resolve_cache_path(project_root: &Path) -> PathBuf {
        if let Ok(folder) = env::var("MERLIN_FOLDER") {
            let path = PathBuf::from(folder)
                .join("cache")
                .join("vector")
                .join("embeddings.bin");
            info!("Using MERLIN_FOLDER: {}", path.display());
            return path;
        }

        project_root
            .join(".merlin")
            .join("cache")
            .join("vector")
            .join("embeddings.bin")
    }

    /// Initialize vector store by loading from cache or generating embeddings
    ///
    /// # Errors
    /// Returns an error if embedding model is unavailable or embedding/cache IO fails
    pub async fn initialize(&mut self) -> Result<()> {
        // Check if embedding model is available
        tracing::info!("Checking embedding model availability...");
        self.client.ensure_model_available().await?;

        tracing::info!(
            "Loading embedding cache (path: {})...",
            self.cache_path.display()
        );

        // Try to load from cache first
        if let Ok(cache) = self.load_cache().await
            && self.try_initialize_from_cache(cache).await?
        {
            return Ok(());
        }

        // No valid cache - embed entire codebase
        self.initialize_from_scratch().await?;

        Ok(())
    }

    /// Initialize vector store using only existing cache, without blocking for full rebuild
    ///
    /// This allows immediate use of partial/incomplete embeddings while full indexing
    /// continues in the background. Returns immediately after loading cache.
    ///
    /// Note: Skips model availability check for non-blocking operation.
    ///
    /// # Errors
    /// Returns an error if cache loading fails or cache is invalid/empty
    pub async fn initialize_partial(&mut self) -> Result<()> {
        // Skip model availability check - we're just loading from cache
        // Model check will happen in background task if needed
        tracing::info!(
            "Loading embedding cache for partial init (path: {})...",
            self.cache_path.display()
        );

        // Try to load from cache - if it exists and is valid, use it immediately
        // IMPORTANT: Skip validation to avoid blocking on file I/O
        // Trust the cache and let background task handle updates
        if let Ok(cache) = self.load_cache().await
            && cache.is_valid()
            && !cache.embeddings.is_empty()
        {
            info!(
                "  Loading {} cached embeddings for immediate use (no validation)",
                cache.embeddings.len()
            );

            // Load all entries without validation (trust the cache)
            self.load_valid_entries(&cache.embeddings);
            self.bm25.finalize();

            info!(
                "  Partial index ready: {} embeddings, {} BM25 docs",
                self.store.len(),
                self.bm25.len()
            );
            return Ok(());
        }

        // No cache available - return error but don't block
        Err(Error::Other(
            "No valid cache available for partial initialization".into(),
        ))
    }

    /// Try to initialize from cached embeddings
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn try_initialize_from_cache(&mut self, cache: VectorCache) -> Result<bool> {
        info!(
            "  Cache file found with {} embeddings (version: {})",
            cache.embeddings.len(),
            cache.version
        );

        if cache.embeddings.is_empty() {
            warn!("  Cache is empty - will rebuild index");
            return Ok(false);
        }

        if !cache.is_valid() {
            return Ok(false);
        }

        tracing::info!("Validating {} cached embeddings...", cache.embeddings.len());

        self.process_cached_embeddings(&cache).await?;

        Ok(true)
    }

    /// Process and validate cached embeddings
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn process_cached_embeddings(&mut self, cache: &VectorCache) -> Result<()> {
        let (valid, invalid) = self.validate_cache_entries(&cache.embeddings);

        // Add valid entries to store and BM25 index
        self.load_valid_entries(&valid);

        // Finalize BM25 index
        self.bm25.finalize();
        info!("  BM25 index built with {} documents", self.bm25.len());
        info!("  Total embeddings in store: {}", self.store.len());

        // Handle new and invalid files
        let (new_files, _new_count, _invalid_count) = self.identify_new_files(cache);

        self.update_cache_with_changes(new_files, invalid, cache)
            .await?;

        self.save_cache_async().await?;
        Ok(())
    }

    /// Load valid cache entries into the store
    fn load_valid_entries(&mut self, valid: &[CachedEmbedding]) {
        for entry in valid {
            let chunk_path = format!(
                "{}:{}-{}",
                entry.path.display(),
                entry.start_line,
                entry.end_line
            );
            self.file_times.insert(entry.path.clone(), entry.modified);
            self.file_hashes
                .insert(entry.path.clone(), entry.content_hash);
            self.store.add(
                PathBuf::from(&chunk_path),
                entry.embedding.clone(),
                entry.preview.clone(),
            );

            // Rebuild BM25 index from preview (approximation)
            self.bm25
                .add_document(PathBuf::from(chunk_path), &entry.preview);
        }
    }

    /// Identify new files that need embedding
    fn identify_new_files(&self, cache: &VectorCache) -> (Vec<PathBuf>, usize, usize) {
        let all_files = self.collect_source_files();
        let cached_paths: HashSet<_> = cache.embeddings.iter().map(|entry| &entry.path).collect();
        let new_files: Vec<_> = all_files
            .into_iter()
            .filter(|f| !cached_paths.contains(f))
            .collect();
        let new_count = new_files.len();
        (new_files, new_count, 0)
    }

    /// Update cache with new and modified files
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn update_cache_with_changes(
        &mut self,
        new_files: Vec<PathBuf>,
        invalid: Vec<PathBuf>,
        cache: &VectorCache,
    ) -> Result<()> {
        let new_count = new_files.len();
        let invalid_count = invalid.len();

        if !new_files.is_empty() {
            info!("  Found {new_count} new files to embed");
            tracing::info!("Embedding {new_count} new files...");
            self.report_progress("Embedding new files", 0, Some(new_count as u64));
            self.embed_files(new_files).await?;
        }

        if !invalid.is_empty() {
            tracing::info!("Re-embedding {invalid_count} modified files...");
            self.report_progress("Re-embedding modified files", 0, Some(invalid_count as u64));
            self.embed_files(invalid).await?;
            tracing::info!(
                "✓ Loaded cache + updated {} files",
                invalid_count + new_count
            );
        } else if new_count > 0 {
            tracing::info!("✓ Loaded cache + added {new_count} new files");
        } else {
            tracing::info!("✓ Loaded {} embeddings from cache", cache.embeddings.len());
        }

        Ok(())
    }

    /// Initialize from scratch by embedding entire codebase
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn initialize_from_scratch(&mut self) -> Result<()> {
        info!("  No valid cache found - building from scratch");
        tracing::info!("Building embedding index for codebase...");
        let files = self.collect_source_files();

        info!("  Found {} source files to embed", files.len());
        tracing::info!("Embedding {} source files...", files.len());
        self.report_progress("Embedding", 0, Some(files.len() as u64));
        self.embed_files(files).await?;

        info!("  Embedded {} files total", self.store.len());
        tracing::info!("✓ Indexed {} files with embeddings", self.store.len());

        info!("  Saving cache to disk...");
        self.report_progress("Saving cache", 0, None);
        self.save_cache_async().await?;
        info!("  ✓ Cache saved");

        Ok(())
    }

    /// Hybrid search combining BM25 keyword search and vector semantic search
    /// Hybrid search combining BM25 keyword search and vector semantic search
    ///
    /// # Errors
    /// Returns an error if embedding the query fails
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        info!(
            "  Hybrid search: {} embeddings, {} BM25 docs",
            self.store.len(),
            self.bm25.len()
        );

        if self.store.is_empty() {
            warn!("  Vector store is empty - no results");
            return Ok(Vec::default());
        }

        // Run BM25 keyword search
        let bm25_results = self.bm25.search(query, top_k * 2); // Get more for ranking
        info!("  BM25 found {} keyword matches", bm25_results.len());

        // Run vector semantic search
        let query_embedding = self.client.embed(query).await?;
        let vector_results = self.store.search(&query_embedding, top_k * 2);
        info!("  Vector found {} semantic matches", vector_results.len());

        // Combine results using adaptive weighted fusion
        let mut combined =
            Self::reciprocal_rank_fusion(query, &bm25_results, &vector_results, top_k);

        // Build import graph for graph-based ranking
        let all_files: Vec<PathBuf> = combined
            .iter()
            .map(|result| result.file_path.clone())
            .collect();
        let import_graph = Self::build_import_graph(&all_files);

        // Apply graph-based boost
        Self::apply_graph_boost(&mut combined, &import_graph);

        // Apply import-based boosting using preview content
        for result in &mut combined {
            let import_boost = Self::boost_by_imports(&result.preview, query);
            result.score *= import_boost;
        }

        // Re-sort after boosting
        combined.sort_by(|result_a, result_b| {
            result_b
                .score
                .partial_cmp(&result_a.score)
                .unwrap_or(Ordering::Equal)
        });

        // Re-normalize after boosting
        if let Some(max_score) = combined.first().map(|result| result.score)
            && max_score > 0.0
        {
            for result in &mut combined {
                result.score /= max_score;
            }
        }

        info!(
            "  Combined {} results using RRF + import boost",
            combined.len()
        );
        if !combined.is_empty() {
            let top_scores: Vec<f32> = combined.iter().take(5).map(|result| result.score).collect();
            info!("  Top scores: {:?}", top_scores);
        }

        // Filter by minimum similarity score
        let filtered: Vec<_> = combined
            .into_iter()
            .filter(|result| result.score >= MIN_SIMILARITY_SCORE)
            .collect();

        info!(
            "  After filtering (score >= {}): {} results",
            MIN_SIMILARITY_SCORE,
            filtered.len()
        );

        Ok(filtered)
    }

    /// Check if file content has imports matching query terms
    fn boost_by_imports(content: &str, query: &str) -> f32 {
        let mut boost = 1.0;
        let query_terms: Vec<&str> = query
            .split_whitespace()
            .filter(|term| term.len() > 3)
            .collect();

        if query_terms.is_empty() {
            return boost;
        }

        // Extract import lines
        let imports: Vec<&str> = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("use ")
                    || trimmed.starts_with("import ")
                    || trimmed.starts_with("from ")
                    || trimmed.starts_with("require(")
            })
            .collect();

        // Check if imports match query terms
        for term in &query_terms {
            let term_lower = term.to_lowercase();
            if imports
                .iter()
                .any(|import_line| import_line.to_lowercase().contains(&term_lower))
            {
                boost += 0.2;
            }
        }

        boost.min(2.0)
    }

    /// Calculate file type and location boost
    fn calculate_file_boost(path: &Path) -> f32 {
        let path_str = path.to_str().unwrap_or("");
        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        // Heavy penalty for test files
        if path_str.contains("/tests/") || path_str.contains("\\tests\\") {
            return 0.1;
        }

        // Heavy penalty for benchmark files
        if path_str.contains("/benches/")
            || path_str.contains("\\benches\\")
            || path_str.contains("/benchmarks/")
            || path_str.contains("\\benchmarks\\")
        {
            return 0.1;
        }

        let mut type_boost = match ext {
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "java" | "c" | "cpp" | "h" | "hpp"
            | "go" | "rb" | "php" | "cs" | "swift" | "kt" | "scala" => 1.7,
            "toml" | "yaml" | "yml" | "json" | "xml" => 0.25, // Reduced by 50%
            "md" | "txt" => 0.05,                             // Reduced by 50%
            _ => 0.5,                                         // Reduced by 50%
        };

        // Boost module entry points
        if path_str.ends_with("/lib.rs") || path_str.ends_with("\\lib.rs") {
            type_boost *= 1.3; // Entry points are important
        } else if path_str.ends_with("/mod.rs") || path_str.ends_with("\\mod.rs") {
            type_boost *= 1.2; // Module definitions
        }

        let location_boost = if path_str.contains("/src/") || path_str.contains("\\src\\") {
            1.3
        } else if path_str.contains("/docs/")
            || path_str.contains("\\docs\\")
            || path_str.contains("/examples/")
            || path_str.contains("\\examples\\")
        {
            0.5
        } else {
            1.0
        };

        type_boost * location_boost
    }

    /// Calculate query-file alignment based on keyword matching
    fn calculate_query_file_alignment(query: &str, file_path: &Path, preview: &str) -> f32 {
        let mut alignment = 1.0;
        let query_lower = query.to_lowercase();

        // Extract query keywords (words longer than 3 chars)
        let keywords: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|word| {
                word.len() > 3
                    && !matches!(
                        *word,
                        "the" | "and" | "for" | "with" | "from" | "that" | "this"
                    )
            })
            .collect();

        if keywords.is_empty() {
            return alignment;
        }

        // Check if filename contains query keywords
        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_lowercase();

        for keyword in &keywords {
            if filename.contains(keyword) {
                alignment *= 1.4; // Filename match is strong signal
            }
        }

        // Check parent directory names
        if let Some(parent) = file_path.parent() {
            let parent_str = parent.to_str().unwrap_or("").to_lowercase();
            for keyword in &keywords {
                if parent_str.contains(keyword) {
                    alignment *= 1.2; // Directory match is good signal
                }
            }
        }

        // Keyword density in preview
        let preview_lower = preview.to_lowercase();
        let keyword_count = keywords
            .iter()
            .filter(|keyword| preview_lower.contains(*keyword))
            .count();

        if keyword_count > 0 {
            let density_boost = (keyword_count as f32).mul_add(0.1, 1.0);
            alignment *= density_boost.min(1.5); // Cap at 1.5x
        }

        alignment
    }

    /// Calculate pattern-based importance boost for code structure
    fn calculate_pattern_boost(preview: &str) -> f32 {
        let mut boost = 1.0;

        // Implementation pattern detection
        let has_impl = preview.contains("impl ") || preview.contains("impl<");
        let has_trait = preview.contains("trait ");
        let has_struct = preview.contains("pub struct") || preview.contains("pub enum");
        let has_main_fn = preview.contains("fn main(") || preview.contains("pub fn new(");

        if has_impl && has_struct {
            boost *= 1.3; // Core implementation file
        }

        if has_trait {
            boost *= 1.2; // Trait definitions are important
        }

        if has_main_fn {
            boost *= 1.25; // Entry point functions
        }

        // Count pub items (public API)
        let pub_count = preview.matches("pub fn").count()
            + preview.matches("pub struct").count()
            + preview.matches("pub enum").count();

        if pub_count > 5 {
            boost *= 1.2; // Rich public API
        }

        // Module-level documentation at start
        if preview.trim_start().starts_with("//!") {
            boost *= 1.15; // Module docs indicate important file
        }

        boost
    }

    /// Build import graph from Rust source files.
    /// Currently returns an empty graph when rust-analyzer backend is not available.
    fn build_import_graph(_files: &[PathBuf]) -> HashMap<PathBuf, Vec<PathBuf>> {
        // Graph ranking is an enhancement; safe to return empty graph.
        HashMap::default()
    }

    /// Apply graph-based boost to results
    fn apply_graph_boost(results: &mut [SearchResult], graph: &HashMap<PathBuf, Vec<PathBuf>>) {
        // Build reverse graph (who imports this file)
        let mut reverse_graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::default();
        for (file, imports) in graph {
            for imported in imports {
                reverse_graph
                    .entry(imported.clone())
                    .or_default()
                    .push(file.clone());
            }
        }

        // Boost files based on graph relationships
        for result in &mut *results {
            let mut graph_boost = 1.0;

            // Boost if many files import this (central/important)
            if let Some(importers) = reverse_graph.get(&result.file_path) {
                let import_count = importers.len();
                if import_count > 5 {
                    graph_boost *= 1.3; // Heavily imported = important
                } else if import_count > 2 {
                    graph_boost *= 1.15; // Moderately imported
                }
            }

            // Boost if this file imports many others (coordinator/orchestrator)
            if let Some(imports) = graph.get(&result.file_path) {
                let import_count = imports.len();
                if import_count > 10 {
                    graph_boost *= 1.2; // Orchestrator file
                }
            }

            result.score *= graph_boost;
        }
    }

    /// Calculate chunk quality boost based on content
    fn calculate_chunk_quality(preview: &str) -> f32 {
        let mut boost = 1.0;

        // Boost chunks with definitions
        if preview.contains("pub struct")
            || preview.contains("pub enum")
            || preview.contains("pub trait")
        {
            boost *= 1.4;
        }

        if preview.contains("pub fn") || preview.contains("pub async fn") {
            boost *= 1.3;
        }

        // Boost module-level documentation
        if preview.trim_start().starts_with("///") || preview.trim_start().starts_with("//!") {
            boost *= 1.2;
        }

        // Penalize chunks that are mostly comments or whitespace
        let non_whitespace_lines = preview
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*")
            })
            .count();

        if non_whitespace_lines < 3 {
            boost *= 0.5; // Mostly empty or comments
        }

        boost
    }

    /// Detect query intent from keywords
    fn detect_query_intent(query: &str) -> &'static str {
        let query_lower = query.to_lowercase();

        if query_lower.starts_with("how") || query_lower.contains(" work") {
            "explanation"
        } else if query_lower.starts_with("implement") || query_lower.starts_with("add") {
            "implementation"
        } else if query_lower.starts_with("fix")
            || query_lower.starts_with("debug")
            || query_lower.starts_with("where")
        {
            "debugging"
        } else {
            "general"
        }
    }

    /// Calculate adaptive weights based on query characteristics
    fn calculate_adaptive_weights(query: &str) -> (f32, f32) {
        // Detect special tokens that indicate exact matching is important
        let has_special_tokens =
            query.contains("::") || query.contains("--") || query.contains("#[");
        let intent = Self::detect_query_intent(query);

        if has_special_tokens {
            // Favor BM25 for exact matches
            (0.7, 0.3)
        } else {
            match intent {
                "explanation" => (0.3, 0.7),    // Favor semantics for "how does X work"
                "implementation" => (0.5, 0.5), // Balanced for "implement X"
                "debugging" => (0.6, 0.4),      // Favor keywords for "fix/where is X"
                _ => (0.4, 0.6),                // Default
            }
        }
    }

    /// Apply exact match bonus if preview contains special tokens from query
    fn apply_exact_match_bonus(
        bm25_contribution: f32,
        query: &str,
        preview: Option<&String>,
    ) -> f32 {
        if bm25_contribution <= 0.0 {
            return bm25_contribution;
        }

        let Some(preview) = preview else {
            return bm25_contribution;
        };

        let preview_lower = preview.to_lowercase();
        let query_lower = query.to_lowercase();

        // Check for special tokens (--flags, ::paths, #[attributes])
        let special_tokens: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|token| token.contains("--") || token.contains("::") || token.contains("#["))
            .collect();

        for token in special_tokens {
            if preview_lower.contains(token) {
                return bm25_contribution * 1.5; // Exact match bonus
            }
        }

        bm25_contribution
    }

    /// Collect BM25 scores into a map and find max score
    fn collect_bm25_scores(
        bm25_results: &[(PathBuf, f32)],
        paths: &mut HashSet<PathBuf>,
    ) -> (HashMap<PathBuf, f32>, f32) {
        let mut bm25_scores = HashMap::default();
        let mut max_bm25 = 0.0f32;

        for (path, score) in bm25_results {
            if *score > 0.0 {
                bm25_scores.insert(path.clone(), *score);
                max_bm25 = max_bm25.max(*score);
                paths.insert(path.clone());
            }
        }
        (bm25_scores, max_bm25)
    }

    /// Collect vector scores and previews into maps and find max score
    fn collect_vector_scores(
        vector_results: &[SearchResult],
        paths: &mut HashSet<PathBuf>,
    ) -> VectorScoreData {
        let mut vector_scores = HashMap::default();
        let mut previews = HashMap::default();
        let mut max_vector = 0.0f32;

        for result in vector_results {
            if result.score > 0.0 {
                vector_scores.insert(result.file_path.clone(), result.score);
                max_vector = max_vector.max(result.score);
                paths.insert(result.file_path.clone());
            }
            previews.insert(result.file_path.clone(), result.preview.clone());
        }
        VectorScoreData {
            scores: vector_scores,
            previews,
            max_score: max_vector,
        }
    }

    /// Compute the final combined score for a search result
    fn compute_combined_score(
        path: &PathBuf,
        query: &str,
        score_params: &ScoreComputationParams<'_>,
    ) -> SearchResult {
        let bm25_scores = score_params.bm25_scores;
        let vector_scores = score_params.vector_scores;
        let previews = score_params.previews;
        let max_bm25 = score_params.max_bm25;
        let max_vector = score_params.max_vector;
        let bm25_weight = score_params.bm25_weight;
        let vector_weight = score_params.vector_weight;
        let bm25_raw = bm25_scores.get(path).copied().unwrap_or(0.0);
        let vector_raw = vector_scores.get(path).copied().unwrap_or(0.0);

        let bm25_normalized = if max_bm25 > 0.0 {
            bm25_raw / max_bm25
        } else {
            0.0
        };
        let vector_normalized = if max_vector > 0.0 {
            vector_raw / max_vector
        } else {
            0.0
        };

        // Apply minimum BM25 threshold - weak matches don't contribute (tuned: 0.75)
        let mut bm25_contribution = if bm25_raw >= 0.75 {
            bm25_normalized * bm25_weight
        } else {
            0.0
        };

        bm25_contribution =
            Self::apply_exact_match_bonus(bm25_contribution, query, previews.get(path));
        let vector_contribution = vector_normalized * vector_weight;

        let preview = previews.get(path).cloned().unwrap_or_default();
        let file_boost = Self::calculate_file_boost(path);
        let query_alignment = Self::calculate_query_file_alignment(query, path, &preview);
        let pattern_boost = Self::calculate_pattern_boost(&preview);
        let chunk_quality = Self::calculate_chunk_quality(&preview);
        let combined_score = (bm25_contribution + vector_contribution)
            * file_boost
            * query_alignment
            * pattern_boost
            * chunk_quality;

        SearchResult {
            file_path: path.clone(),
            score: combined_score,
            preview,
            bm25_score: (bm25_contribution > 0.0).then_some(bm25_contribution),
            vector_score: (vector_contribution > 0.0).then_some(vector_contribution),
        }
    }

    /// Combine BM25 keyword scores with vector semantic scores using weighted normalization
    fn reciprocal_rank_fusion(
        query: &str,
        bm25_results: &[(PathBuf, f32)],
        vector_results: &[SearchResult],
        top_k: usize,
    ) -> Vec<SearchResult> {
        let (bm25_weight, vector_weight) = Self::calculate_adaptive_weights(query);
        let mut paths = HashSet::default();

        let (bm25_scores, max_bm25) = Self::collect_bm25_scores(bm25_results, &mut paths);
        let vector_data = Self::collect_vector_scores(vector_results, &mut paths);

        let score_params = ScoreComputationParams {
            bm25_scores: &bm25_scores,
            vector_scores: &vector_data.scores,
            previews: &vector_data.previews,
            max_bm25,
            max_vector: vector_data.max_score,
            bm25_weight,
            vector_weight,
        };

        let mut combined: Vec<SearchResult> = paths
            .into_iter()
            .map(|path| Self::compute_combined_score(&path, query, &score_params))
            .collect();

        combined.sort_by(|result_a, result_b| {
            result_b
                .score
                .partial_cmp(&result_a.score)
                .unwrap_or(Ordering::Equal)
        });

        if let Some(max_score) = combined.first().map(|result| result.score)
            && max_score > 0.0
        {
            for result in &mut combined {
                result.score /= max_score;
                if let Some(bm25_score) = result.bm25_score.as_mut() {
                    *bm25_score /= max_score;
                }
                if let Some(vector_score) = result.vector_score.as_mut() {
                    *vector_score /= max_score;
                }
            }
        }

        combined.truncate(top_k);

        combined
    }

    /// Collect all source files in the project
    ///
    /// # Errors
    /// Returns an error if file collection fails
    fn collect_source_files(&self) -> Vec<PathBuf> {
        use ignore::WalkBuilder;

        let mut files = Vec::default();

        let walker = WalkBuilder::new(&self.project_root)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .build();

        for entry in walker.filter_map(StdResult::ok) {
            let path = entry.path();

            if entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
                && is_source_file(path)
            {
                let normalized_path = path
                    .strip_prefix(&self.project_root)
                    .map_or_else(|_| path.to_path_buf(), PathBuf::from);
                files.push(normalized_path);
            }
        }

        files
    }

    /// Process embedding results for a single file
    fn process_chunk_results(&mut self, chunk_results: Vec<ChunkResult>) -> usize {
        if chunk_results.is_empty() {
            return 0;
        }

        let relative_path = &chunk_results[0].0;
        let absolute_path = self.project_root.join(relative_path);
        let content_hash = chunk_results[0].4;

        // Track file modification time and content hash
        if let Ok(metadata) = fs::metadata(&absolute_path)
            && let Ok(modified) = metadata.modified()
        {
            self.file_times.insert(relative_path.clone(), modified);
            self.file_hashes.insert(relative_path.clone(), content_hash);
        }

        let chunk_count = chunk_results.len();

        for (path, chunk, embedding, preview, _hash) in chunk_results {
            let chunk_path = format!("{}:{}-{}", path.display(), chunk.start_line, chunk.end_line);

            // Add to vector store
            self.store
                .add(PathBuf::from(&chunk_path), embedding, preview);

            // Add to BM25 index
            self.bm25
                .add_document(PathBuf::from(chunk_path), &chunk.content);
        }

        chunk_count
    }

    /// Process chunk batches and generate embeddings
    ///
    /// Returns vector of chunk results and file-chunk mapping
    async fn embed_chunk_batches(
        &self,
        file_chunks_data: Vec<FileChunksData>,
    ) -> (
        Vec<(PathBuf, FileChunk, Vec<f32>, String, u64)>,
        FileChunkMap,
    ) {
        const CHUNK_BATCH_SIZE: usize = 50;
        let mut all_chunk_results = Vec::new();
        let mut chunk_queue = Vec::new();
        let mut file_chunk_map: FileChunkMap = HashMap::new();

        // Collect all chunks with their file association
        for (relative_path, _content, chunks, content_hash) in file_chunks_data {
            for (idx, chunk) in chunks.into_iter().enumerate() {
                file_chunk_map
                    .entry(relative_path.clone())
                    .or_default()
                    .push((idx, chunk.clone(), content_hash));
                chunk_queue.push((relative_path.clone(), chunk, content_hash));
            }
        }

        let total_chunks = chunk_queue.len();
        info!("Total chunks to embed: {}", total_chunks);
        self.report_progress("Embedding chunks", 0, Some(total_chunks as u64));

        // Embed chunks in mega-batches
        for batch_start in (0..chunk_queue.len()).step_by(CHUNK_BATCH_SIZE) {
            let batch_end = (batch_start + CHUNK_BATCH_SIZE).min(chunk_queue.len());
            let batch = &chunk_queue[batch_start..batch_end];

            let chunk_texts: Vec<String> = batch
                .iter()
                .map(|(_, chunk, _)| chunk.content.clone())
                .collect();

            let embeddings = match self.client.embed_batch(chunk_texts).await {
                Ok(embs) => embs,
                Err(error) => {
                    warn!("Failed to embed batch: {error}");
                    continue;
                }
            };

            // Store results
            for ((relative_path, chunk, content_hash), embedding) in
                batch.iter().zip(embeddings.into_iter())
            {
                let preview = generate_preview(&chunk.content, 200);
                all_chunk_results.push((
                    relative_path.clone(),
                    chunk.clone(),
                    embedding,
                    preview,
                    *content_hash,
                ));
            }

            self.report_progress(
                "Embedding chunks",
                all_chunk_results.len() as u64,
                Some(total_chunks as u64),
            );
        }

        info!(
            "Successfully embedded {} chunks (received from model)",
            all_chunk_results.len()
        );

        (all_chunk_results, file_chunk_map)
    }

    /// Embed a batch of files (chunked) - optimized version
    ///
    /// # Errors
    /// Returns an error if any embedding task fails
    async fn embed_files(&mut self, files: Vec<PathBuf>) -> Result<()> {
        let total_files = files.len();
        info!(
            "Starting optimized embedding pipeline for {} files",
            total_files
        );

        // Phase 1: Parallel file reading and chunking (CPU-bound)
        tracing::info!("Reading and chunking files...");
        self.report_progress("Reading files", 0, Some(total_files as u64));
        let file_chunks_data = Self::parallel_read_and_chunk(files, &self.project_root).await;

        info!("Chunked {} files into chunks", file_chunks_data.len());
        self.report_progress(
            "Chunking complete",
            file_chunks_data.len() as u64,
            Some(total_files as u64),
        );

        // Phase 2: Cross-file chunk batching and embedding (I/O-bound)
        let (all_chunk_results, file_chunk_map) = self.embed_chunk_batches(file_chunks_data).await;

        // Phase 3: Process results and update indices
        let processed_chunks = self.process_and_index_chunks(all_chunk_results).await;

        tracing::info!(
            "Completed: {} files, {processed_chunks} chunks",
            file_chunk_map.len()
        );
        self.report_progress(
            "Complete",
            processed_chunks as u64,
            Some(processed_chunks as u64),
        );

        Ok(())
    }

    /// Process chunk results, build indices, and save checkpoints
    ///
    /// # Errors
    /// Returns an error if processing fails
    async fn process_and_index_chunks(&mut self, all_chunk_results: Vec<ChunkResult>) -> usize {
        const CHECKPOINT_INTERVAL_CHUNKS: usize = 500;

        tracing::info!("Building search indices and writing cache...");
        self.report_progress("Building indices", 0, Some(all_chunk_results.len() as u64));

        let mut processed_chunks: usize = 0;
        let mut next_checkpoint = CHECKPOINT_INTERVAL_CHUNKS;

        for chunk_result in all_chunk_results {
            let chunk_count = self.process_chunk_results(vec![chunk_result]);
            processed_chunks += chunk_count;

            // Progressive cache saving by chunks without modulo
            if processed_chunks >= next_checkpoint {
                if let Err(error) = self.save_cache_async().await {
                    warn!("Failed to save checkpoint: {error}");
                } else {
                    info!("Checkpoint saved at {processed_chunks} chunks");
                }
                next_checkpoint = processed_chunks.saturating_add(CHECKPOINT_INTERVAL_CHUNKS);
            }
        }

        // Finalize BM25 index (compute IDF scores)
        self.bm25.finalize();
        info!("BM25 index finalized with {} documents", self.bm25.len());

        // Ensure final cache save at the end of embedding
        self.report_progress("Saving cache", 0, None);
        if let Err(error) = self.save_cache_async().await {
            warn!("Failed to save final cache: {error}");
        }

        processed_chunks
    }

    /// Parallel file reading and chunking using blocking tasks
    async fn parallel_read_and_chunk(
        files: Vec<PathBuf>,
        project_root: &Path,
    ) -> Vec<FileChunksData> {
        const MAX_CONCURRENT_READS: usize = 20;

        let mut tasks = FuturesUnordered::new();
        let mut results = Vec::new();
        let mut file_iter = files.into_iter();

        // Start initial batch
        for _ in 0..MAX_CONCURRENT_READS {
            if let Some(relative_path) = file_iter.next() {
                let absolute_path = project_root.join(&relative_path);
                let relative_clone = relative_path.clone();

                tasks.push(spawn_blocking(move || {
                    Self::read_and_chunk_file(relative_clone, &absolute_path)
                }));
            }
        }

        // Process results and spawn new tasks
        while let Some(result) = tasks.next().await {
            if let Ok(Some(file_data)) = result {
                results.push(file_data);
            }

            // Spawn next task to maintain concurrency
            if let Some(relative_path) = file_iter.next() {
                let absolute_path = project_root.join(&relative_path);
                let relative_clone = relative_path.clone();

                tasks.push(spawn_blocking(move || {
                    Self::read_and_chunk_file(relative_clone, &absolute_path)
                }));
            }
        }

        results
    }

    /// Read and chunk a single file (CPU-bound, runs in blocking task)
    fn read_and_chunk_file(relative_path: PathBuf, absolute_path: &Path) -> Option<FileChunksData> {
        let content = match fs::read_to_string(absolute_path) {
            Ok(content) => content,
            Err(error) => {
                warn!("Failed to read {}: {error}", relative_path.display());
                return None;
            }
        };

        if content.trim().is_empty() {
            return None;
        }

        let content_hash = Self::compute_file_hash(&content);
        let chunks = chunk_file(&relative_path, &content);

        if chunks.is_empty() {
            return None;
        }

        Some((relative_path, content, chunks, content_hash))
    }

    /// Validate cache entries and return (valid, invalid)
    fn validate_cache_entries(
        &self,
        entries: &[CachedEmbedding],
    ) -> (Vec<CachedEmbedding>, Vec<PathBuf>) {
        let mut valid = Vec::default();
        let mut invalid_set: HashSet<PathBuf> = HashSet::default();

        for entry in entries {
            let absolute_path = self.project_root.join(&entry.path);

            // Check if file still exists
            if !absolute_path.exists() {
                continue;
            }

            // Read file content and compute hash
            let Ok(content) = fs::read_to_string(&absolute_path) else {
                invalid_set.insert(entry.path.clone());
                continue;
            };

            let current_hash = Self::compute_file_hash(&content);

            // Compare content hash - this is the most reliable check
            if current_hash != entry.content_hash {
                invalid_set.insert(entry.path.clone());
                continue;
            }

            // File is valid if not already marked invalid
            if !invalid_set.contains(&entry.path) {
                valid.push(entry.clone());
            }
        }

        let invalid: Vec<PathBuf> = invalid_set.into_iter().collect();
        (valid, invalid)
    }

    /// Load cache from disk
    ///
    /// # Errors
    /// Returns an error if the cache file cannot be read or deserialized
    async fn load_cache(&self) -> Result<VectorCache> {
        use tokio::fs as async_fs;

        // Read cache file asynchronously to avoid blocking
        let data = async_fs::read(&self.cache_path)
            .await
            .map_err(|error| Error::Other(format!("Failed to read cache: {error}")))?;

        // Deserialize in blocking task (CPU-bound operation)
        let cache = spawn_blocking(move || {
            decode_from_slice(&data, bincode_config())
                .map_err(|error| Error::Other(format!("Failed to deserialize cache: {error}")))
                .map(|(cache, _)| cache)
        })
        .await
        .map_err(|error| Error::Other(format!("Task join error: {error}")))??;

        Ok(cache)
    }

    /// Save cache to disk (async version)
    ///
    /// # Errors
    /// Returns an error if the cache directory cannot be created or serialization fails
    async fn save_cache_async(&self) -> Result<()> {
        self.ensure_cache_dir()?;
        let embeddings = self.prepare_embeddings();
        let cache = VectorCache {
            version: VectorCache::VERSION,
            embeddings,
        };
        info!(
            "  Saving cache with {} embeddings to {}",
            cache.embeddings.len(),
            self.cache_path.display()
        );

        let bytes = spawn_blocking(move || {
            encode_to_vec(&cache, bincode_config())
                .map_err(|error| Error::Other(format!("Failed to serialize cache: {error}")))
        })
        .await
        .map_err(|error| Error::Other(format!("Task join error: {error}")))??;

        self.write_cache_bytes_async(&bytes).await?;
        info!("  ✓ Cache saved successfully ({} bytes)", bytes.len());
        Ok(())
    }

    /// Save cache to disk (sync version for Drop)
    ///
    /// # Errors
    /// Returns an error if the cache directory cannot be created or serialization fails
    fn save_cache_sync(&self) -> Result<()> {
        self.ensure_cache_dir()?;
        let embeddings = self.prepare_embeddings();
        let cache = VectorCache {
            version: VectorCache::VERSION,
            embeddings,
        };
        info!(
            "  Saving cache with {} embeddings to {}",
            cache.embeddings.len(),
            self.cache_path.display()
        );
        let bytes = encode_to_vec(&cache, bincode_config())
            .map_err(|error| Error::Other(format!("Failed to serialize cache: {error}")))?;
        self.write_cache_bytes_sync(&bytes)?;
        info!("  ✓ Cache saved successfully ({} bytes)", bytes.len());
        Ok(())
    }

    /// Ensure the cache directory exists
    ///
    /// # Errors
    /// Returns an error if the cache directory cannot be created
    fn ensure_cache_dir(&self) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                Error::Other(format!("Failed to create cache directory: {error}"))
            })?;
        }
        Ok(())
    }

    fn prepare_embeddings(&self) -> Vec<CachedEmbedding> {
        let mut result = Vec::with_capacity(self.store.len());
        for entry in self.store.iter() {
            let chunk_path_str = entry.path.to_str().unwrap_or("");
            let Some((path_str, range)) = chunk_path_str.rsplit_once(':') else {
                continue;
            };
            let Some((start_str, end_str)) = range.split_once('-') else {
                continue;
            };
            let Ok(start_line) = start_str.parse::<usize>() else {
                continue;
            };
            let Ok(end_line) = end_str.parse::<usize>() else {
                continue;
            };
            let path = PathBuf::from(path_str);
            let modified = self
                .file_times
                .get(&path)
                .copied()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let content_hash = self.file_hashes.get(&path).copied().unwrap_or(0);
            result.push(CachedEmbedding {
                path,
                chunk_id: format!("{start_line}-{end_line}"),
                start_line,
                end_line,
                embedding: entry.embedding,
                preview: entry.preview,
                modified,
                content_hash,
            });
        }
        result
    }

    /// Write cache bytes to current `cache_path` (async version)
    ///
    /// # Errors
    /// Returns an error if the write fails even after ensuring parent dir exists
    async fn write_cache_bytes_async(&self, data: &[u8]) -> Result<()> {
        use tokio::fs as async_fs;

        let cache_path = self.cache_path.clone();
        let data_vec = data.to_vec();

        if let Err(write_error) = async_fs::write(&cache_path, &data_vec).await {
            if let Some(parent) = cache_path.parent() {
                async_fs::create_dir_all(parent).await.map_err(|error| {
                    Error::Other(format!("Failed to create cache directory: {error}"))
                })?;
            }
            async_fs::write(&cache_path, &data_vec)
                .await
                .map_err(|error| {
                    Error::Other(format!(
                        "Failed to write cache to {}: {error}. Prior error: {write_error}",
                        cache_path.display()
                    ))
                })?;
        }
        Ok(())
    }

    /// Write cache bytes to current `cache_path` (sync version for Drop)
    ///
    /// # Errors
    /// Returns an error if the write fails even after ensuring parent dir exists
    fn write_cache_bytes_sync(&self, data: &[u8]) -> Result<()> {
        if let Err(write_error) = fs::write(&self.cache_path, data) {
            if let Some(parent) = self.cache_path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    Error::Other(format!("Failed to create cache directory: {error}"))
                })?;
            }
            fs::write(&self.cache_path, data).map_err(|error| {
                Error::Other(format!(
                    "Failed to write cache to {}: {error}. Prior error: {write_error}",
                    self.cache_path.display()
                ))
            })?;
        }
        Ok(())
    }

    /// Get the number of indexed files
    pub fn len(&self) -> usize {
        self.bm25.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

impl<E: EmbeddingProvider> Drop for VectorSearchManager<E> {
    fn drop(&mut self) {
        if !self.store.is_empty() {
            if let Err(error) = self.save_cache_sync() {
                warn!("Failed to save cache on drop: {error}");
            } else {
                info!("Cache saved on drop");
            }
        }
    }
}
