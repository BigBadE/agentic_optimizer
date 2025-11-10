//! Cache operations for vector embeddings.

use bincode::config::standard as bincode_config;
use bincode::{Decode, Encode, decode_from_slice, encode_to_vec};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::task::spawn_blocking;
use tracing::info;

use merlin_core::{CoreResult as Result, Error};

/// Cache entry for a chunk embedding
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct CachedEmbedding {
    /// File path
    pub path: PathBuf,
    /// Chunk identifier
    pub chunk_id: String,
    /// Start line
    pub start_line: usize,
    /// End line
    pub end_line: usize,
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Chunk content preview
    pub preview: String,
    /// Last modification time (for informational purposes)
    pub modified: SystemTime,
    /// Content hash (xxHash64 for fast validation)
    pub content_hash: u64,
}

/// Cached vector database
#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub struct VectorCache {
    /// Version identifier for cache invalidation
    pub version: u32,
    /// Cached embeddings
    pub embeddings: Vec<CachedEmbedding>,
}

impl Default for VectorCache {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            embeddings: Vec::default(),
        }
    }
}

impl VectorCache {
    /// Cache version identifier
    pub const VERSION: u32 = 5; // Bumped for content_hash field

    /// Check if cache version is valid
    pub fn is_valid(&self) -> bool {
        self.version == Self::VERSION
    }
}

/// Cache operations
pub struct CacheOperations {
    /// Cache file path
    cache_path: PathBuf,
    /// File modification times for cache invalidation
    pub file_times: HashMap<PathBuf, SystemTime>,
    /// File content hashes for validation
    pub file_hashes: HashMap<PathBuf, u64>,
}

impl CacheOperations {
    /// Create new cache operations
    pub fn new(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
            file_times: HashMap::default(),
            file_hashes: HashMap::default(),
        }
    }

    /// Load cache from disk
    ///
    /// # Errors
    /// Returns an error if the cache file cannot be read or deserialized
    pub async fn load_cache(&self) -> Result<VectorCache> {
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
    pub async fn save_cache_async(&self, embeddings: Vec<CachedEmbedding>) -> Result<()> {
        self.ensure_cache_dir()?;
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
    pub fn save_cache_sync(&self, embeddings: Vec<CachedEmbedding>) -> Result<()> {
        self.ensure_cache_dir()?;
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

    /// Validate cache entries and return (valid, invalid)
    pub fn validate_cache_entries(
        entries: &[CachedEmbedding],
        project_root: &Path,
    ) -> (Vec<CachedEmbedding>, Vec<PathBuf>) {
        let mut valid = Vec::default();
        let mut invalid_set: HashSet<PathBuf> = HashSet::default();

        for entry in entries {
            let absolute_path = project_root.join(&entry.path);

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

    /// Compute hash of file content for cache validation
    pub fn compute_file_hash(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash as _, Hasher as _};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
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
}
