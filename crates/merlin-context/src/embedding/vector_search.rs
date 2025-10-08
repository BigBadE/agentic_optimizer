//! Vector search manager with persistent caching.

use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::time::{Duration, SystemTime};
use tokio::task::JoinSet;
use tracing::{info, warn};

use crate::context_inclusion::MIN_SIMILARITY_SCORE;
use crate::embedding::chunking::{FileChunk, chunk_file};
use crate::embedding::{BM25Index, EmbeddingClient, SearchResult, VectorStore, generate_preview};
use crate::fs_utils::is_source_file;
use bincode::config::standard as bincode_config;
use bincode::{Decode, Encode, decode_from_slice, encode_to_vec};
use merlin_core::{Error, Result};

type ChunkResult = (PathBuf, FileChunk, Vec<f32>, String);

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
    /// Last modification time
    modified: SystemTime,
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
    const VERSION: u32 = 3; // Bumped for normalized relative paths

    fn is_valid(&self) -> bool {
        self.version == Self::VERSION
    }
}

/// Vector search manager with caching and BM25 keyword search
pub struct VectorSearchManager {
    /// In-memory vector store
    store: VectorStore,
    /// BM25 keyword search index
    bm25: BM25Index,
    /// File modification times for cache invalidation
    file_times: HashMap<PathBuf, SystemTime>,
    /// Embedding client
    client: EmbeddingClient,
    /// Project root
    project_root: PathBuf,
    /// Cache file path
    cache_path: PathBuf,
}

impl VectorSearchManager {
    /// Create a new vector search manager
    pub fn new(project_root: PathBuf) -> Self {
        let cache_path = project_root.join("..merlin").join("embeddings.bin");

        Self {
            store: VectorStore::default(),
            bm25: BM25Index::default(),
            file_times: HashMap::default(),
            client: EmbeddingClient::default(),
            project_root,
            cache_path,
        }
    }

    /// Initialize vector store by loading from cache or generating embeddings
    ///
    /// # Errors
    /// Returns an error if embedding model is unavailable or embedding/cache IO fails
    #[allow(
        clippy::too_many_lines,
        reason = "Complex initialization with caching and progress"
    )]
    pub async fn initialize(&mut self) -> Result<()> {
        let spinner = Self::create_spinner();

        // Check if embedding model is available
        spinner.set_message("Checking embedding model availability...");
        if let Err(model_error) = self.client.ensure_model_available().await {
            spinner.finish_and_clear();
            return Err(model_error);
        }

        spinner.set_message("Loading embedding cache...");

        // Try to load from cache first
        if let Ok(cache) = self.load_cache()
            && self.try_initialize_from_cache(cache, &spinner).await?
        {
            return Ok(());
        }

        // No valid cache - embed entire codebase
        self.initialize_from_scratch(&spinner).await?;

        Ok(())
    }

    /// Create a configured progress spinner
    fn create_spinner() -> ProgressBar {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        spinner.enable_steady_tick(Duration::from_millis(100));
        spinner
    }

    /// Try to initialize from cached embeddings
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn try_initialize_from_cache(
        &mut self,
        cache: VectorCache,
        spinner: &ProgressBar,
    ) -> Result<bool> {
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

        spinner.set_message(format!(
            "Validating {} cached embeddings...",
            cache.embeddings.len()
        ));

        self.process_cached_embeddings(&cache, spinner).await?;

        Ok(true)
    }

    /// Process and validate cached embeddings
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn process_cached_embeddings(
        &mut self,
        cache: &VectorCache,
        spinner: &ProgressBar,
    ) -> Result<()> {
        let (valid, invalid) = self.validate_cache_entries(&cache.embeddings);

        // Add valid entries to store and BM25 index
        self.load_valid_entries(&valid);

        // Finalize BM25 index
        self.bm25.finalize();
        info!("  BM25 index built with {} documents", self.bm25.len());
        info!("  Total embeddings in store: {}", self.store.len());

        // Handle new and invalid files
        let (new_files, _new_count, _invalid_count) = self.identify_new_files(cache);

        self.update_cache_with_changes(new_files, invalid, spinner, cache)
            .await?;

        self.save_cache()?;
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
        spinner: &ProgressBar,
        cache: &VectorCache,
    ) -> Result<()> {
        let new_count = new_files.len();
        let invalid_count = invalid.len();

        if !new_files.is_empty() {
            info!("  Found {new_count} new files to embed");
            spinner.set_message(format!("Embedding {new_count} new files..."));
            self.embed_files(new_files, spinner).await?;
        }

        if !invalid.is_empty() {
            spinner.set_message(format!("Re-embedding {invalid_count} modified files..."));
            self.embed_files(invalid, spinner).await?;
            spinner.finish_with_message(format!(
                "✓ Loaded cache + updated {} files",
                invalid_count + new_count
            ));
        } else if new_count > 0 {
            spinner
                .finish_with_message(format!("✓ Loaded cache + added {new_count} new files"));
        } else {
            spinner.finish_with_message(format!(
                "✓ Loaded {} embeddings from cache",
                cache.embeddings.len()
            ));
        }

        Ok(())
    }

    /// Initialize from scratch by embedding entire codebase
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn initialize_from_scratch(&mut self, spinner: &ProgressBar) -> Result<()> {
        info!("  No valid cache found - building from scratch");
        spinner.set_message("Building embedding index for codebase...");
        let files = self.collect_source_files();

        info!("  Found {} source files to embed", files.len());
        spinner.set_message(format!("Embedding {} source files...", files.len()));
        self.embed_files(files, spinner).await?;

        info!("  Embedded {} files total", self.store.len());
        spinner.finish_with_message(format!(
            "✓ Indexed {} files with embeddings",
            self.store.len()
        ));

        info!("  Saving cache to disk...");
        self.save_cache()?;
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
            "toml" | "yaml" | "yml" | "json" | "xml" => 0.5,
            "md" | "txt" => 0.1, // Heavy penalty for all documentation
            _ => 1.0,
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

    /// Combine BM25 keyword scores with vector semantic scores using weighted normalization
    #[allow(clippy::too_many_lines, reason = "Complex ranking fusion algorithm")]
    fn reciprocal_rank_fusion(
        query: &str,
        bm25_results: &[(PathBuf, f32)],
        vector_results: &[SearchResult],
        top_k: usize,
    ) -> Vec<SearchResult> {
        let (bm25_weight, vector_weight) = Self::calculate_adaptive_weights(query);

        let mut bm25_scores: HashMap<PathBuf, f32> = HashMap::default();
        let mut vector_scores: HashMap<PathBuf, f32> = HashMap::default();
        let mut previews: HashMap<PathBuf, String> = HashMap::default();
        let mut paths: HashSet<PathBuf> = HashSet::default();

        let mut max_bm25 = 0.0f32;
        for (path, score) in bm25_results {
            if *score > 0.0 {
                bm25_scores.insert(path.clone(), *score);
                if *score > max_bm25 {
                    max_bm25 = *score;
                }
                paths.insert(path.clone());
            }
        }

        let mut max_vector = 0.0f32;
        for result in vector_results {
            if result.score > 0.0 {
                vector_scores.insert(result.file_path.clone(), result.score);
                if result.score > max_vector {
                    max_vector = result.score;
                }
                paths.insert(result.file_path.clone());
            }
            previews.insert(result.file_path.clone(), result.preview.clone());
        }

        let mut combined: Vec<SearchResult> = paths
            .into_iter()
            .map(|path| {
                let bm25_raw = bm25_scores.get(&path).copied().unwrap_or(0.0);
                let vector_raw = vector_scores.get(&path).copied().unwrap_or(0.0);

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

                // Apply minimum BM25 threshold - weak matches don't contribute
                // Tuned threshold: 0.75 balances precision and recall
                let mut bm25_contribution = if bm25_raw >= 0.75 {
                    bm25_normalized * bm25_weight
                } else {
                    0.0
                };

                // Exact match bonus: check if preview contains exact query terms
                bm25_contribution =
                    Self::apply_exact_match_bonus(bm25_contribution, query, previews.get(&path));

                let vector_contribution = vector_normalized * vector_weight;

                let preview = previews.get(&path).cloned().unwrap_or_default();
                let file_boost = Self::calculate_file_boost(&path);
                let query_alignment = Self::calculate_query_file_alignment(query, &path, &preview);
                let pattern_boost = Self::calculate_pattern_boost(&preview);
                let chunk_quality = Self::calculate_chunk_quality(&preview);
                let combined_score = (bm25_contribution + vector_contribution)
                    * file_boost
                    * query_alignment
                    * pattern_boost
                    * chunk_quality;

                SearchResult {
                    file_path: path,
                    score: combined_score,
                    preview,
                    bm25_score: (bm25_contribution > 0.0).then_some(bm25_contribution),
                    vector_score: (vector_contribution > 0.0).then_some(vector_contribution),
                }
            })
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

    /// Embed a single file and return chunks with embeddings
    async fn embed_single_file(relative_path: PathBuf, absolute_path: PathBuf) -> Vec<ChunkResult> {
        let client = EmbeddingClient::default();

        let content = match fs::read_to_string(&absolute_path) {
            Ok(content) => content,
            Err(error) => {
                warn!(
                    "Warning: Failed to read {}: {error}",
                    relative_path.display()
                );
                return Vec::default();
            }
        };

        // Skip empty files
        if content.trim().is_empty() {
            return Vec::default();
        }

        // Chunk the file
        let chunks = chunk_file(&relative_path, &content);
        let mut chunk_results: Vec<ChunkResult> = Vec::default();

        for chunk in chunks {
            let preview = generate_preview(&chunk.content, 200);

            match client.embed(&chunk.content).await {
                Ok(embedding) => {
                    chunk_results.push((relative_path.clone(), chunk, embedding, preview));
                }
                Err(error) => {
                    warn!(
                        "Warning: Failed to embed chunk in {}: {error}",
                        relative_path.display()
                    );
                }
            }
        }

        chunk_results
    }

    /// Process embedding results for a single file
    fn process_chunk_results(&mut self, chunk_results: Vec<ChunkResult>, total_chunks: &mut usize) {
        let relative_path = &chunk_results[0].0;
        let absolute_path = self.project_root.join(relative_path);

        // Track file modification time
        if let Ok(metadata) = fs::metadata(&absolute_path)
            && let Ok(modified) = metadata.modified()
        {
            self.file_times.insert(relative_path.clone(), modified);
        }

        for (path, chunk, embedding, preview) in chunk_results {
            let chunk_path = format!("{}:{}-{}", path.display(), chunk.start_line, chunk.end_line);

            // Add to vector store
            self.store
                .add(PathBuf::from(&chunk_path), embedding, preview);

            // Add to BM25 index
            self.bm25
                .add_document(PathBuf::from(chunk_path), &chunk.content);

            *total_chunks += 1;
        }
    }

    /// Embed a batch of files (chunked)
    /// Embed a batch of files (chunked)
    ///
    /// # Errors
    /// Returns an error if any embedding task fails
    async fn embed_files(&mut self, files: Vec<PathBuf>, spinner: &ProgressBar) -> Result<()> {
        const BATCH_SIZE: usize = 10;
        let total_files = files.len();
        let mut processed_files = 0;
        let mut total_chunks = 0;

        for file_batch in files.chunks(BATCH_SIZE) {
            let mut tasks = JoinSet::default();

            for file_path in file_batch {
                let relative_path = file_path.clone();
                let absolute_path = self.project_root.join(file_path);

                tasks.spawn(Self::embed_single_file(relative_path, absolute_path));
            }

            // Collect results
            while let Some(result) = tasks.join_next().await {
                match result {
                    Ok(chunk_results) if !chunk_results.is_empty() => {
                        self.process_chunk_results(chunk_results, &mut total_chunks);
                        processed_files += 1;
                        spinner.set_message(format!("Embedding files... {processed_files}/{total_files} ({total_chunks} chunks)"));
                    }
                    Err(task_error) => {
                        warn!("    Task error: {task_error}");
                    }
                    Ok(_) => {} // Empty results, skip
                }
            }
        }

        // Finalize BM25 index (compute IDF scores)
        self.bm25.finalize();
        info!("  BM25 index finalized with {} documents", self.bm25.len());

        Ok(())
    }

    /// Validate cache entries and return (valid, invalid)
    fn validate_cache_entries(
        &self,
        entries: &[CachedEmbedding],
    ) -> (Vec<CachedEmbedding>, Vec<PathBuf>) {
        let mut valid = Vec::default();
        let mut invalid = Vec::default();

        for entry in entries {
            let absolute_path = self.project_root.join(&entry.path);

            // Check if file still exists
            if !absolute_path.exists() {
                continue;
            }

            // Check if file was modified
            let Ok(metadata) = fs::metadata(&absolute_path) else {
                continue; // File doesn't exist anymore
            };

            let Ok(modified) = metadata.modified() else {
                invalid.push(entry.path.clone());
                continue;
            };

            if modified > entry.modified {
                invalid.push(entry.path.clone());
            } else {
                valid.push(entry.clone());
            }
        }

        (valid, invalid)
    }

    /// Load cache from disk
    ///
    /// # Errors
    /// Returns an error if the cache file cannot be read or deserialized
    fn load_cache(&self) -> Result<VectorCache> {
        let data = fs::read(&self.cache_path)
            .map_err(|error| Error::Other(format!("Failed to read cache: {error}")))?;

        let cache: VectorCache = decode_from_slice(&data, bincode_config())
            .map_err(|error| Error::Other(format!("Failed to deserialize cache: {error}")))?
            .0;
        Ok(cache)
    }

    /// Save cache to disk
    ///
    /// # Errors
    /// Returns an error if the cache directory cannot be created or serialization fails
    fn save_cache(&self) -> Result<()> {
        // Create cache directory if needed
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                Error::Other(format!("Failed to create cache directory: {error}"))
            })?;
        }

        let cache = VectorCache::default();

        let data = encode_to_vec(&cache, bincode_config())
            .map_err(|error| Error::Other(format!("Failed to serialize cache: {error}")))?;
        fs::write(&self.cache_path, data)
            .map_err(|error| Error::Other(format!("Failed to write cache: {error}")))?;
        Ok(())
    }

    /// Get the number of indexed files
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}
