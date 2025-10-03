//! Vector search manager with persistent caching.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::task::JoinSet;

use agentic_core::Result;
use crate::embedding::{EmbeddingClient, VectorStore, SearchResult, generate_preview};
use crate::embedding::chunking::chunk_file;
use crate::fs_utils::is_source_file;
use crate::context_inclusion::MIN_SIMILARITY_SCORE;

/// Cache entry for a chunk embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Cached vector database
#[derive(Debug, Serialize, Deserialize)]
struct VectorCache {
    /// Version identifier for cache invalidation
    version: u32,
    /// Cached embeddings
    embeddings: Vec<CachedEmbedding>,
}

impl VectorCache {
    const VERSION: u32 = 2;  // Bumped for chunk-based embeddings

    fn new() -> Self {
        Self {
            version: Self::VERSION,
            embeddings: Vec::new(),
        }
    }

    fn is_valid(&self) -> bool {
        self.version == Self::VERSION
    }
}

/// Vector search manager with caching
pub struct VectorSearchManager {
    /// In-memory vector store
    store: VectorStore,
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
        let cache_path = project_root.join(".agentic_cache").join("embeddings.bin");
        
        Self {
            store: VectorStore::new(),
            file_times: HashMap::new(),
            client: EmbeddingClient::new(),
            project_root,
            cache_path,
        }
    }

    /// Initialize vector store by loading from cache or generating embeddings
    pub async fn initialize(&mut self) -> Result<()> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        // Check if embedding model is available
        spinner.set_message("Checking embedding model availability...");
        if let Err(e) = self.client.ensure_model_available().await {
            spinner.finish_and_clear();
            return Err(e);
        }
        
        spinner.set_message("Loading embedding cache...");

        // Try to load from cache first
        if let Ok(cache) = self.load_cache() {
            eprintln!("  Cache file found with {} embeddings (version: {})", cache.embeddings.len(), cache.version);
            
            if cache.embeddings.is_empty() {
                eprintln!("  ⚠️  Cache is empty - will rebuild index");
            }
            
            if cache.is_valid() && !cache.embeddings.is_empty() {
                spinner.set_message(format!("Validating {} cached embeddings...", cache.embeddings.len()));
                
                let (valid, invalid) = self.validate_cache_entries(&cache.embeddings)?;
                
                // Add valid entries to store
                for entry in &valid {
                    let chunk_path = format!("{}:{}-{}", entry.path.display(), entry.start_line, entry.end_line);
                    eprintln!("  Loaded: {} [{}] (dim: {})", chunk_path, entry.chunk_id, entry.embedding.len());
                    self.file_times.insert(entry.path.clone(), entry.modified);
                    self.store.add(PathBuf::from(chunk_path), entry.embedding.clone(), entry.preview.clone());
                }
                
                eprintln!("  Total embeddings in store: {}", self.store.len());

                // Check for new files not in cache
                let all_files = self.collect_source_files()?;
                let cached_paths: std::collections::HashSet<_> = cache.embeddings.iter()
                    .map(|e| &e.path)
                    .collect();
                let new_files: Vec<_> = all_files.into_iter()
                    .filter(|f| !cached_paths.contains(f))
                    .collect();
                
                let new_count = new_files.len();
                let invalid_count = invalid.len();
                
                if !new_files.is_empty() {
                    eprintln!("  Found {} new files to embed", new_count);
                    spinner.set_message(format!("Embedding {} new files...", new_count));
                    self.embed_files(new_files, &spinner).await?;
                }

                if !invalid.is_empty() {
                    // Re-embed invalid files
                    spinner.set_message(format!("Re-embedding {} modified files...", invalid_count));
                    self.embed_files(invalid, &spinner).await?;
                    
                    spinner.finish_with_message(format!("✓ Loaded cache + updated {} files", invalid_count + new_count));
                } else if new_count > 0 {
                    spinner.finish_with_message(format!("✓ Loaded cache + added {} new files", new_count));
                } else {
                    spinner.finish_with_message(format!("✓ Loaded {} embeddings from cache", cache.embeddings.len()));
                }
                
                self.save_cache()?;
                return Ok(());
            }
            
            // Cache is valid but empty - fall through to rebuild
            eprintln!("  Cache is empty - falling through to rebuild");
        }

        // No valid cache - embed entire codebase
        eprintln!("  No valid cache found - building from scratch");
        spinner.set_message("Building embedding index for codebase...");
        let files = self.collect_source_files()?;
        
        eprintln!("  Found {} source files to embed", files.len());
        spinner.set_message(format!("Embedding {} source files...", files.len()));
        self.embed_files(files, &spinner).await?;
        
        eprintln!("  Embedded {} files total", self.store.len());
        spinner.finish_with_message(format!("✓ Indexed {} files with embeddings", self.store.len()));
        
        eprintln!("  Saving cache to disk...");
        self.save_cache()?;
        eprintln!("  ✓ Cache saved");
        
        Ok(())
    }

    /// Search for similar files with minimum score threshold
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        eprintln!("  Vector search: store has {} embeddings", self.store.len());
        
        if self.store.is_empty() {
            eprintln!("  ⚠️  Vector store is empty - no results");
            return Ok(Vec::new());
        }
        
        let query_embedding = self.client.embed(query).await?;
        eprintln!("  Query embedded (dim: {})", query_embedding.len());
        
        let results = self.store.search(&query_embedding, top_k);
        eprintln!("  Found {} results before filtering", results.len());
        
        if !results.is_empty() {
            eprintln!("  Top scores: {:?}", results.iter().take(5).map(|r| r.score).collect::<Vec<_>>());
        }
        
        // Filter by minimum similarity score
        let filtered: Vec<_> = results.into_iter()
            .filter(|r| r.score >= MIN_SIMILARITY_SCORE)
            .collect();
        
        eprintln!("  After filtering (score >= {}): {} results", MIN_SIMILARITY_SCORE, filtered.len());
        
        Ok(filtered)
    }

    /// Collect all source files in the project
    fn collect_source_files(&self) -> Result<Vec<PathBuf>> {
        use ignore::WalkBuilder;
        
        let mut files = Vec::new();
        
        let walker = WalkBuilder::new(&self.project_root)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .build();

        for entry in walker.filter_map(std::result::Result::ok) {
            let path = entry.path();
            
            if entry.file_type().map_or(false, |ft| ft.is_file()) && is_source_file(path) {
                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    /// Embed a batch of files (chunked)
    async fn embed_files(&mut self, files: Vec<PathBuf>, spinner: &ProgressBar) -> Result<()> {
        const BATCH_SIZE: usize = 10;
        let total_files = files.len();
        let mut processed_files = 0;
        let mut total_chunks = 0;

        for file_batch in files.chunks(BATCH_SIZE) {
            let mut tasks = JoinSet::new();
            
            for file_path in file_batch {
                let path = file_path.clone();
                let client = EmbeddingClient::new();
                
                tasks.spawn(async move {
                    let content = match fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("Warning: Failed to read {}: {}", path.display(), e);
                            return Vec::new();
                        }
                    };

                    // Skip empty files
                    if content.trim().is_empty() {
                        return Vec::new();
                    }

                    // Chunk the file
                    let chunks = chunk_file(&path, &content);
                    let mut chunk_results = Vec::new();
                    
                    for chunk in chunks {
                        let preview = generate_preview(&chunk.content, 200);
                        
                        match client.embed(&chunk.content).await {
                            Ok(embedding) => {
                                chunk_results.push((path.clone(), chunk, embedding, preview));
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to embed chunk in {}: {}", path.display(), e);
                            }
                        }
                    }
                    
                    chunk_results
                });
            }

            // Collect results
            while let Some(result) = tasks.join_next().await {
                match result {
                    Ok(chunk_results) => {
                        if !chunk_results.is_empty() {
                            let file_path = &chunk_results[0].0;
                            
                            // Track file modification time
                            if let Ok(metadata) = fs::metadata(file_path) {
                                if let Ok(modified) = metadata.modified() {
                                    self.file_times.insert(file_path.clone(), modified);
                                }
                            }
                            
                            for (path, chunk, embedding, preview) in chunk_results {
                                let chunk_path = format!("{}:{}-{}", path.display(), chunk.start_line, chunk.end_line);
                                eprintln!("    Embedded: {} [{}] (dim: {})", chunk_path, chunk.identifier, embedding.len());
                                self.store.add(PathBuf::from(chunk_path), embedding, preview);
                                total_chunks += 1;
                            }
                            
                            processed_files += 1;
                            spinner.set_message(format!("Embedding files... {}/{} ({} chunks)", processed_files, total_files, total_chunks));
                        }
                    }
                    Err(e) => {
                        eprintln!("    Task error: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate cache entries and return (valid, invalid)
    fn validate_cache_entries(&self, entries: &[CachedEmbedding]) -> Result<(Vec<CachedEmbedding>, Vec<PathBuf>)> {
        let mut valid = Vec::new();
        let mut invalid = Vec::new();

        for entry in entries {
            // Check if file still exists
            if !entry.path.exists() {
                continue;
            }

            // Check if file was modified
            match fs::metadata(&entry.path) {
                Ok(metadata) => {
                    match metadata.modified() {
                        Ok(modified) => {
                            if modified > entry.modified {
                                invalid.push(entry.path.clone());
                            } else {
                                valid.push(entry.clone());
                            }
                        }
                        Err(_) => invalid.push(entry.path.clone()),
                    }
                }
                Err(_) => continue, // File doesn't exist anymore
            }
        }

        Ok((valid, invalid))
    }

    /// Load cache from disk
    fn load_cache(&self) -> Result<VectorCache> {
        let data = fs::read(&self.cache_path)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to read cache: {e}")))?;
        
        bincode::deserialize(&data)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to deserialize cache: {e}")))
    }

    /// Save cache to disk
    fn save_cache(&self) -> Result<()> {
        // Create cache directory if needed
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| agentic_core::Error::Other(format!("Failed to create cache dir: {e}")))?;
        }

        // Build cache entries from store
        let mut cache = VectorCache::new();
        
        for entry in self.store.iter() {
            // Parse chunk path: "file_path:start-end"
            let path_str = entry.path.display().to_string();
            if let Some((file_part, range_part)) = path_str.rsplit_once(':') {
                if let Some((start_str, end_str)) = range_part.split_once('-') {
                    if let (Ok(start), Ok(end)) = (start_str.parse(), end_str.parse()) {
                        let file_path = PathBuf::from(file_part);
                        let modified = self.file_times.get(&file_path)
                            .copied()
                            .unwrap_or_else(SystemTime::now);
                        
                        cache.embeddings.push(CachedEmbedding {
                            path: file_path,
                            chunk_id: String::from("chunk"),  // We don't store this separately
                            start_line: start,
                            end_line: end,
                            embedding: entry.embedding,
                            preview: entry.preview,
                            modified,
                        });
                        continue;
                    }
                }
            }
            
            // Fallback for non-chunked entries (shouldn't happen)
            eprintln!("Warning: Could not parse chunk path: {}", path_str);
        }
        
        let data = bincode::serialize(&cache)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to serialize cache: {e}")))?;
        
        fs::write(&self.cache_path, data)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to write cache: {e}")))?;

        Ok(())
    }

    /// Get the number of indexed files
    #[must_use]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the store is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}
