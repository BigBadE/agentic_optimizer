//! Embedding operations for files and chunks.

use merlin_deps::futures::stream::{FuturesUnordered, StreamExt as _};
use merlin_deps::tracing::{info, warn};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::task::spawn_blocking;

use crate::embedding::chunking::{FileChunk, chunk_file};
use crate::embedding::vector_search::cache::{CacheOperations, CachedEmbedding};
use crate::embedding::{EmbeddingProvider, generate_preview};
use merlin_core::CoreResult as Result;

type ChunkResult = (PathBuf, FileChunk, Vec<f32>, String, u64);
type FileChunksData = (PathBuf, String, Vec<FileChunk>, u64);
type FileChunkMap = HashMap<PathBuf, Vec<(usize, FileChunk, u64)>>;

/// Progress callback for embedding operations
pub type ProgressCallback = Arc<dyn Fn(&str, u64, Option<u64>) + Send + Sync>;

/// Embedding operations coordinator
pub struct EmbeddingOperations<E: EmbeddingProvider + Clone> {
    /// Embedding client
    client: E,
    /// Project root
    project_root: PathBuf,
    /// Optional progress callback
    progress_callback: Option<ProgressCallback>,
}

impl<E: EmbeddingProvider + Clone> EmbeddingOperations<E> {
    /// Create new embedding operations
    pub fn new(
        client: E,
        project_root: PathBuf,
        progress_callback: Option<ProgressCallback>,
    ) -> Self {
        Self {
            client,
            project_root,
            progress_callback,
        }
    }

    /// Report progress if callback is set
    fn report_progress(&self, stage: &str, current: u64, total: Option<u64>) {
        if let Some(callback) = &self.progress_callback {
            callback(stage, current, total);
        }
    }

    /// Embed a batch of files (chunked) - optimized version
    ///
    /// # Errors
    /// Returns an error if any embedding task fails
    pub async fn embed_files(&self, files: Vec<PathBuf>) -> Result<Vec<ChunkResult>> {
        let total_files = files.len();
        info!(
            "Starting optimized embedding pipeline for {} files",
            total_files
        );

        // Phase 1: Parallel file reading and chunking (CPU-bound)
        merlin_deps::tracing::info!("Reading and chunking files...");
        self.report_progress("Reading files", 0, Some(total_files as u64));
        let file_chunks_data = Self::parallel_read_and_chunk(files, &self.project_root).await;

        info!("Chunked {} files into chunks", file_chunks_data.len());
        self.report_progress(
            "Chunking complete",
            file_chunks_data.len() as u64,
            Some(total_files as u64),
        );

        // Phase 2: Cross-file chunk batching and embedding (I/O-bound)
        let (all_chunk_results, _file_chunk_map) = self.embed_chunk_batches(file_chunks_data).await;

        Ok(all_chunk_results)
    }

    /// Process chunk batches and generate embeddings
    ///
    /// Returns vector of chunk results and file-chunk mapping
    async fn embed_chunk_batches(
        &self,
        file_chunks_data: Vec<FileChunksData>,
    ) -> (Vec<ChunkResult>, FileChunkMap) {
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

        let content_hash = CacheOperations::compute_file_hash(&content);
        let chunks = chunk_file(&relative_path, &content);

        if chunks.is_empty() {
            return None;
        }

        Some((relative_path, content, chunks, content_hash))
    }

    /// Prepare embeddings for caching
    pub fn prepare_embeddings(
        store_entries: impl Iterator<Item = (PathBuf, Vec<f32>, String)>,
        file_times: &HashMap<PathBuf, SystemTime>,
        file_hashes: &HashMap<PathBuf, u64>,
    ) -> Vec<CachedEmbedding> {
        let mut result = Vec::new();
        for (path_buf, embedding, preview) in store_entries {
            let chunk_path_str = path_buf.to_str().unwrap_or("");
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
            let modified = file_times
                .get(&path)
                .copied()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let content_hash = file_hashes.get(&path).copied().unwrap_or(0);
            result.push(CachedEmbedding {
                path,
                chunk_id: format!("{start_line}-{end_line}"),
                start_line,
                end_line,
                embedding,
                preview,
                modified,
                content_hash,
            });
        }
        result
    }
}
