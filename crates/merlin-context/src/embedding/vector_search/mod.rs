//! Vector search manager with persistent caching.

mod cache;
mod embedding;
mod initialization;
mod scoring;

pub use cache::{CachedEmbedding, VectorCache};
pub use embedding::ProgressCallback;

use merlin_deps::tracing::{info, warn};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use crate::embedding::client::EmbeddingProvider;
use crate::embedding::{BM25Index, EmbeddingClient, SearchResult, VectorStore};
use cache::CacheOperations;
use embedding::EmbeddingOperations;
use initialization::InitializationHelper;
use merlin_core::{CoreResult as Result, Error};
use scoring::ScoringUtils;

/// Vector search manager with caching and BM25 keyword search
pub struct VectorSearchManager<E: EmbeddingProvider + Clone = EmbeddingClient> {
    /// In-memory vector store
    store: VectorStore,
    /// BM25 keyword search index
    bm25: BM25Index,
    /// Embedding client
    client: E,
    /// Project root
    project_root: PathBuf,
    /// Cache operations
    cache_ops: CacheOperations,
    /// Optional progress callback
    progress_callback: Option<ProgressCallback>,
}

impl<E: EmbeddingProvider + Clone> VectorSearchManager<E> {
    /// Create a new vector search manager with a custom embedding provider
    pub fn with_provider(project_root: &Path, client: E) -> Self {
        let cache_path = InitializationHelper::resolve_cache_path(project_root);

        Self {
            store: VectorStore::default(),
            bm25: BM25Index::default(),
            client,
            project_root: project_root.to_path_buf(),
            cache_ops: CacheOperations::new(cache_path),
            progress_callback: None,
        }
    }
}

impl VectorSearchManager<EmbeddingClient> {
    /// Create a new vector search manager with default Ollama client
    pub fn new(project_root: &Path) -> Self {
        Self::with_provider(project_root, EmbeddingClient::default())
    }
}

impl<E: EmbeddingProvider + Clone> VectorSearchManager<E> {
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

    /// Initialize vector store by loading from cache or generating embeddings
    ///
    /// # Errors
    /// Returns an error if embedding model is unavailable or embedding/cache IO fails
    pub async fn initialize(&mut self) -> Result<()> {
        // Check if embedding model is available
        merlin_deps::tracing::info!("Checking embedding model availability...");
        self.client.ensure_model_available().await?;

        let cache_path = InitializationHelper::resolve_cache_path(&self.project_root);
        merlin_deps::tracing::info!(
            "Loading embedding cache (path: {})...",
            cache_path.display()
        );

        // Try to load from cache first
        if let Ok(cache) = self.cache_ops.load_cache().await
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
        let cache_path = InitializationHelper::resolve_cache_path(&self.project_root);
        merlin_deps::tracing::info!(
            "Loading embedding cache for partial init (path: {})...",
            cache_path.display()
        );

        // Try to load from cache - if it exists and is valid, use it immediately
        if let Ok(cache) = self.cache_ops.load_cache().await
            && cache.is_valid()
            && !cache.embeddings.is_empty()
        {
            info!(
                "  Loading {} cached embeddings for immediate use (no validation)",
                cache.embeddings.len()
            );

            // Load all entries without validation (trust the cache)
            InitializationHelper::load_valid_entries(
                &cache.embeddings,
                &mut self.store,
                &mut self.bm25,
                &mut self.cache_ops.file_times,
                &mut self.cache_ops.file_hashes,
            );
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

        merlin_deps::tracing::info!("Validating {} cached embeddings...", cache.embeddings.len());

        self.process_cached_embeddings(&cache).await?;

        Ok(true)
    }

    /// Process and validate cached embeddings
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn process_cached_embeddings(&mut self, cache: &VectorCache) -> Result<()> {
        let (valid, invalid) =
            CacheOperations::validate_cache_entries(&cache.embeddings, &self.project_root);

        // Add valid entries to store and BM25 index
        InitializationHelper::load_valid_entries(
            &valid,
            &mut self.store,
            &mut self.bm25,
            &mut self.cache_ops.file_times,
            &mut self.cache_ops.file_hashes,
        );

        // Finalize BM25 index
        self.bm25.finalize();
        info!("  BM25 index built with {} documents", self.bm25.len());
        info!("  Total embeddings in store: {}", self.store.len());

        // Handle new and invalid files
        let (new_files, new_count) =
            InitializationHelper::identify_new_files(cache, &self.project_root);

        self.update_cache_with_changes(new_files, invalid, new_count)
            .await?;

        self.save_cache_async().await?;
        Ok(())
    }

    /// Update cache with new and modified files
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn update_cache_with_changes(
        &mut self,
        new_files: Vec<PathBuf>,
        invalid: Vec<PathBuf>,
        new_count: usize,
    ) -> Result<()> {
        let invalid_count = invalid.len();

        if !new_files.is_empty() {
            info!("  Found {new_count} new files to embed");
            merlin_deps::tracing::info!("Embedding {new_count} new files...");
            self.report_progress("Embedding new files", 0, Some(new_count as u64));
            self.embed_files(new_files).await?;
        }

        if !invalid.is_empty() {
            merlin_deps::tracing::info!("Re-embedding {invalid_count} modified files...");
            self.report_progress("Re-embedding modified files", 0, Some(invalid_count as u64));
            self.embed_files(invalid).await?;
            merlin_deps::tracing::info!(
                "✓ Loaded cache + updated {} files",
                invalid_count + new_count
            );
        } else if new_count > 0 {
            merlin_deps::tracing::info!("✓ Loaded cache + added {new_count} new files");
        } else {
            merlin_deps::tracing::info!("✓ Loaded embeddings from cache");
        }

        Ok(())
    }

    /// Initialize from scratch by embedding entire codebase
    ///
    /// # Errors
    /// Returns an error if embedding operations fail
    async fn initialize_from_scratch(&mut self) -> Result<()> {
        info!("  No valid cache found - building from scratch");
        merlin_deps::tracing::info!("Building embedding index for codebase...");
        let files = InitializationHelper::collect_source_files(&self.project_root);

        info!("  Found {} source files to embed", files.len());
        merlin_deps::tracing::info!("Embedding {} source files...", files.len());
        self.report_progress("Embedding", 0, Some(files.len() as u64));
        self.embed_files(files).await?;

        info!("  Embedded {} files total", self.store.len());
        merlin_deps::tracing::info!("✓ Indexed {} files with embeddings", self.store.len());

        info!("  Saving cache to disk...");
        self.report_progress("Saving cache", 0, None);
        self.save_cache_async().await?;
        info!("  ✓ Cache saved");

        Ok(())
    }

    /// Embed a batch of files
    ///
    /// # Errors
    /// Returns an error if embedding fails
    async fn embed_files(&mut self, files: Vec<PathBuf>) -> Result<()> {
        let embedding_ops = EmbeddingOperations::new(
            self.client.clone(),
            self.project_root.clone(),
            self.progress_callback.clone(),
        );

        let chunk_results = embedding_ops.embed_files(files).await?;

        // Process results and update indices
        for (path, chunk, embedding, preview, content_hash) in chunk_results {
            let chunk_path: String =
                format!("{}:{}-{}", path.display(), chunk.start_line, chunk.end_line);

            // Track file metadata
            if let Ok(metadata) = fs::metadata(self.project_root.join(&path))
                && let Ok(modified) = metadata.modified()
            {
                self.cache_ops.file_times.insert(path.clone(), modified);
                self.cache_ops.file_hashes.insert(path, content_hash);
            }

            self.store
                .add(PathBuf::from(&chunk_path), embedding, preview.clone());
            self.bm25
                .add_document(PathBuf::from(chunk_path.clone()), &chunk.content);
        }

        self.bm25.finalize();

        Ok(())
    }

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
        let bm25_results = self.bm25.search(query, top_k * 2);
        info!("  BM25 found {} keyword matches", bm25_results.len());

        // Run vector semantic search
        let query_embedding = self.client.embed(query).await?;
        let vector_results = self.store.search(&query_embedding, top_k * 2);
        info!("  Vector found {} semantic matches", vector_results.len());

        // Combine results using adaptive weighted fusion
        let mut combined =
            ScoringUtils::reciprocal_rank_fusion(query, &bm25_results, &vector_results, top_k);

        // Build import graph for graph-based ranking
        let all_files: Vec<PathBuf> = combined
            .iter()
            .map(|result| result.file_path.clone())
            .collect();
        let import_graph = ScoringUtils::build_import_graph(&all_files);

        // Apply graph-based boost
        ScoringUtils::apply_graph_boost(&mut combined, &import_graph);

        // Apply import-based boosting using preview content
        for result in &mut combined {
            let import_boost = ScoringUtils::boost_by_imports(&result.preview, query);
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
        let filtered = ScoringUtils::filter_by_min_score(combined);

        info!("  After filtering: {} results", filtered.len());

        Ok(filtered)
    }

    /// Save cache to disk
    ///
    /// # Errors
    /// Returns an error if cache save fails
    async fn save_cache_async(&self) -> Result<()> {
        let embeddings = EmbeddingOperations::<E>::prepare_embeddings(
            self.store
                .iter()
                .map(|entry| (entry.path, entry.embedding, entry.preview)),
            &self.cache_ops.file_times,
            &self.cache_ops.file_hashes,
        );
        self.cache_ops.save_cache_async(embeddings).await
    }

    /// Save cache to disk (sync version for Drop)
    ///
    /// # Errors
    /// Returns an error if cache save fails
    fn save_cache_sync(&self) -> Result<()> {
        let embeddings = EmbeddingOperations::<E>::prepare_embeddings(
            self.store
                .iter()
                .map(|entry| (entry.path, entry.embedding, entry.preview)),
            &self.cache_ops.file_times,
            &self.cache_ops.file_hashes,
        );
        self.cache_ops.save_cache_sync(embeddings)
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

impl<E: EmbeddingProvider + Clone> Drop for VectorSearchManager<E> {
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
