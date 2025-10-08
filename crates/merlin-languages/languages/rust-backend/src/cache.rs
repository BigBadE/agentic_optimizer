//! Caching and persistence for rust-analyzer state.

use std::collections::HashMap;
use std::fs::{self as filesystem, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bincode::{deserialize_from, serialize_into};
use merlin_core::Error as CoreError;
use serde::{Deserialize, Serialize};

/// Cached metadata about the rust-analyzer workspace state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceCache {
    /// Timestamp when the cache was created
    pub timestamp: SystemTime,
    /// Project root path
    pub project_root: PathBuf,
    /// Map of file paths to their modification times
    pub file_metadata: HashMap<PathBuf, SystemTime>,
    /// Number of files indexed
    pub file_count: usize,
}

impl WorkspaceCache {
    /// Create a new workspace cache
    pub fn new(project_root: PathBuf, file_metadata: HashMap<PathBuf, SystemTime>) -> Self {
        let file_count = file_metadata.len();
        Self {
            timestamp: SystemTime::now(),
            project_root,
            file_metadata,
            file_count,
        }
    }

    /// Get the cache file path for a project
    fn cache_path(project_root: &Path) -> PathBuf {
        let cache_dir = project_root
            .join("../../../../../target")
            .join(".agentic-cache");
        cache_dir.join("rust-analyzer.cache")
    }

    /// Save the cache to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created, the cache file cannot be
    /// written, or serialization fails.
    pub fn save(&self, project_root: &Path) -> Result<(), CoreError> {
        let cache_path = Self::cache_path(project_root);

        if let Some(parent) = cache_path.parent() {
            filesystem::create_dir_all(parent).map_err(|error| {
                CoreError::Other(format!("Failed to create cache directory: {error}"))
            })?;
        }

        let file = File::create(&cache_path)
            .map_err(|error| CoreError::Other(format!("Failed to create cache file: {error}")))?;
        let writer = BufWriter::new(file);
        serialize_into(writer, &self)
            .map_err(|error| CoreError::Other(format!("Failed to serialize cache: {error}")))?;

        tracing::info!("Saved rust-analyzer cache to {}", cache_path.display());
        Ok(())
    }

    /// Load the cache from disk
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file does not exist, cannot be opened, or cannot be
    /// deserialized.
    pub fn load(project_root: &Path) -> Result<Self, CoreError> {
        let cache_path = Self::cache_path(project_root);

        if !cache_path.exists() {
            return Err(CoreError::Other("Cache file does not exist".into()));
        }

        let file = File::open(&cache_path)
            .map_err(|error| CoreError::Other(format!("Failed to open cache file: {error}")))?;

        let reader = BufReader::new(file);
        let cache: Self = deserialize_from(reader)
            .map_err(|error| CoreError::Other(format!("Failed to deserialize cache: {error}")))?;

        tracing::info!("Loaded rust-analyzer cache from {}", cache_path.display());
        Ok(cache)
    }

    /// Check if the cache is still valid for the current project state
    ///
    /// # Errors
    ///
    /// Returns an error if file metadata cannot be read.
    pub fn is_valid(&self, project_root: &Path) -> Result<bool, CoreError> {
        if self.project_root != project_root {
            tracing::debug!("Cache invalid: project root mismatch");
            return Ok(false);
        }

        if let Ok(elapsed) = self.timestamp.elapsed()
            && elapsed.as_secs() > 86_400
        {
            tracing::debug!("Cache invalid: older than 24 hours");
            return Ok(false);
        }

        let sample_size = 10.min(self.file_metadata.len());
        let mut checked = 0;

        for (path, cached_time) in self.file_metadata.iter().take(sample_size) {
            if let Ok(metadata) = filesystem::metadata(path)
                && let Ok(modified) = metadata.modified()
            {
                if modified > *cached_time {
                    tracing::debug!("Cache invalid: file {} was modified", path.display());
                    return Ok(false);
                }
                checked += 1;
            }
        }

        if checked == 0 {
            tracing::debug!("Cache invalid: no files could be verified");
            return Ok(false);
        }

        tracing::info!("Cache is valid (checked {checked}/{sample_size} files)");
        Ok(true)
    }

    /// Clear the cache for a project
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file cannot be deleted.
    pub fn clear(project_root: &Path) -> Result<(), CoreError> {
        let cache_path = Self::cache_path(project_root);

        if cache_path.exists() {
            filesystem::remove_file(&cache_path)
                .map_err(|error| CoreError::Other(format!("Failed to delete cache: {error}")))?;
            tracing::info!("Cleared rust-analyzer cache");
        }

        Ok(())
    }
}
