//! Vector search manager initialization logic.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::time::SystemTime;
use tokio::spawn;
use tracing::info;

use crate::embedding::chunking::FileChunk;
use crate::embedding::client::EmbeddingProvider;
use crate::embedding::vector_search::cache::{CacheOperations, CachedEmbedding, VectorCache};
use crate::embedding::vector_search::embedding::EmbeddingOperations;
use crate::embedding::{BM25Index, VectorStore};
use crate::fs_utils::is_source_file;

/// Chunk result tuple: (path, chunk, embedding, preview, `content_hash`)
type ChunkResult = (PathBuf, FileChunk, Vec<f32>, String, u64);

/// Processing result tuple: (store, `file_times`, `file_hashes`)
type ProcessingResult = (
    VectorStore,
    HashMap<PathBuf, SystemTime>,
    HashMap<PathBuf, u64>,
);

/// Initialization helper
pub struct InitializationHelper;

#[allow(dead_code, reason = "Helper methods used by background tasks")]
impl InitializationHelper {
    /// Resolve cache path with environment override support
    ///
    /// Env variables:
    /// - `MERLIN_FOLDER`: directory for the entire Merlin state (e.g. `.merlin`). We store embeddings at `{MERLIN_FOLDER}/cache/vector/embeddings.bin`
    pub fn resolve_cache_path(project_root: &Path) -> PathBuf {
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

    /// Collect all source files in the project
    pub fn collect_source_files(project_root: &Path) -> Vec<PathBuf> {
        use ignore::WalkBuilder;

        let mut files = Vec::default();

        let walker = WalkBuilder::new(project_root)
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
                    .strip_prefix(project_root)
                    .map_or_else(|_| path.to_path_buf(), PathBuf::from);
                files.push(normalized_path);
            }
        }

        files
    }

    /// Load valid cache entries into the store
    pub fn load_valid_entries(
        valid: &[CachedEmbedding],
        store: &mut VectorStore,
        bm25: &mut BM25Index,
        file_times: &mut HashMap<PathBuf, SystemTime>,
        file_hashes: &mut HashMap<PathBuf, u64>,
    ) {
        for entry in valid {
            let chunk_path = format!(
                "{}:{}-{}",
                entry.path.display(),
                entry.start_line,
                entry.end_line
            );
            file_times.insert(entry.path.clone(), entry.modified);
            file_hashes.insert(entry.path.clone(), entry.content_hash);
            store.add(
                PathBuf::from(&chunk_path),
                entry.embedding.clone(),
                entry.preview.clone(),
            );

            // Rebuild BM25 index from preview (approximation)
            bm25.add_document(PathBuf::from(chunk_path), &entry.preview);
        }
    }

    /// Identify new files that need embedding
    pub fn identify_new_files(cache: &VectorCache, project_root: &Path) -> (Vec<PathBuf>, usize) {
        let all_files = Self::collect_source_files(project_root);
        let cached_paths: HashSet<_> = cache.embeddings.iter().map(|entry| &entry.path).collect();
        let new_files: Vec<_> = all_files
            .into_iter()
            .filter(|f| !cached_paths.contains(f))
            .collect();
        let new_count = new_files.len();
        (new_files, new_count)
    }

    /// Spawn background task for full embedding initialization
    ///
    /// Note: Does not use progress callback to avoid UI blocking
    pub fn spawn_background_embedding<E: EmbeddingProvider + Clone + 'static>(
        project_root: PathBuf,
        client: E,
        cache_path: PathBuf,
    ) {
        spawn(async move {
            let cache_ops = CacheOperations::new(cache_path.clone());
            let embedding_ops =
                EmbeddingOperations::new(client.clone(), project_root.clone(), None);

            tracing::info!("Background: Starting full embedding initialization...");

            let files = Self::collect_source_files(&project_root);
            let result = embedding_ops.embed_files(files).await;

            let Ok(chunk_results) = result else {
                tracing::warn!("Background embedding generation failed: {:?}", result.err());
                return;
            };

            // Build cache entries
            let (store, file_times, file_hashes) =
                Self::process_chunk_results(chunk_results, &project_root);

            // Prepare and save cache
            let entries = EmbeddingOperations::<E>::prepare_embeddings(
                store
                    .iter()
                    .map(|entry| (entry.path, entry.embedding, entry.preview)),
                &file_times,
                &file_hashes,
            );

            if let Err(bg_error) = cache_ops.save_cache_async(entries).await {
                tracing::warn!("Background: Failed to save cache: {bg_error}");
            } else {
                tracing::info!("Background: Embedding generation completed successfully");
            }
        });
    }

    /// Process chunk results into store and metadata maps
    fn process_chunk_results(
        chunk_results: Vec<ChunkResult>,
        project_root: &Path,
    ) -> ProcessingResult {
        let mut store = VectorStore::default();
        let mut file_times = HashMap::new();
        let mut file_hashes = HashMap::new();

        for (path, chunk, embedding, preview, content_hash) in chunk_results {
            let chunk_path: String =
                format!("{}:{}-{}", path.display(), chunk.start_line, chunk.end_line);

            // Track file metadata
            let metadata_result = fs::metadata(project_root.join(&path))
                .ok()
                .and_then(|metadata| metadata.modified().ok());

            if let Some(modified) = metadata_result {
                file_times.insert(path.clone(), modified);
                file_hashes.insert(path.clone(), content_hash);
            }

            store.add(PathBuf::from(chunk_path), embedding, preview);
        }

        (store, file_times, file_hashes)
    }
}
