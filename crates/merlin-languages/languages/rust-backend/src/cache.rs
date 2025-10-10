//! Caching and persistence for rust-analyzer state.

use std::collections::HashMap;
use std::fs::{self as filesystem, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use merlin_core::Error as CoreError;
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer};

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
        #[cfg(test)]
        {
            // In tests, use a local .cache directory to avoid permission issues
            let cache_dir = project_root.join(".cache");
            cache_dir.join("rust-analyzer.cache")
        }

        #[cfg(not(test))]
        {
            let cache_dir = project_root
                .join("../../../../../target")
                .join(".agentic-cache");
            cache_dir.join("rust-analyzer.cache")
        }
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

        {
            let file = File::create(&cache_path).map_err(|error| {
                CoreError::Other(format!("Failed to create cache file: {error}"))
            })?;
            let writer = BufWriter::new(file);
            to_writer(writer, self)
                .map_err(|error| CoreError::Other(format!("Failed to serialize cache: {error}")))?;
        }

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
        let cache: Self = from_reader(reader)
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
            let metadata = filesystem::metadata(path);

            // If we can't get metadata, the file might be deleted or inaccessible
            if metadata.is_err() {
                tracing::debug!(
                    "Cache invalid: file {} is missing or inaccessible",
                    path.display()
                );
                return Ok(false);
            }

            if let Ok(modified) = metadata
                .map_err(|error| CoreError::Other(format!("Failed to get metadata: {error}")))?
                .modified()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write as _;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    /// Test project setup data
    type TestProject = (TempDir, PathBuf, HashMap<PathBuf, SystemTime>);

    /// Create a test directory structure with some source files
    fn create_test_project() -> TestProject {
        let temp_dir =
            TempDir::new().unwrap_or_else(|error| panic!("Failed to create temp dir: {error}"));
        let project_root = temp_dir.path().to_path_buf();

        // Create some test files
        let file1 = project_root.join("src/main.rs");
        let file2 = project_root.join("src/lib.rs");
        let file3 = project_root.join("Cargo.toml");

        fs::create_dir_all(project_root.join("src"))
            .unwrap_or_else(|error| panic!("Failed to create src dir: {error}"));

        for path in &[&file1, &file2, &file3] {
            let mut file =
                File::create(path).unwrap_or_else(|error| panic!("Failed to create file: {error}"));
            file.write_all(b"// test content")
                .unwrap_or_else(|error| panic!("Failed to write file: {error}"));
        }

        // Get modification times
        let mut file_metadata = HashMap::new();
        for path in &[&file1, &file2, &file3] {
            let modified = fs::metadata(path)
                .unwrap_or_else(|error| panic!("Failed to get metadata: {error}"))
                .modified()
                .unwrap_or_else(|error| panic!("Failed to get modified time: {error}"));
            file_metadata.insert((*path).clone(), modified);
        }

        (temp_dir, project_root, file_metadata)
    }

    #[test]
    fn test_cache_creation() {
        let (_temp_dir, project_root, file_metadata) = create_test_project();
        let cache = WorkspaceCache::new(project_root.clone(), file_metadata);

        assert_eq!(cache.project_root, project_root);
        assert_eq!(cache.file_count, 3);
        assert_eq!(cache.file_metadata.len(), 3);
    }

    #[test]
    fn test_cache_save_and_load() {
        let (_temp_dir, project_root, file_metadata) = create_test_project();
        let cache = WorkspaceCache::new(project_root.clone(), file_metadata);

        // Save the cache
        cache
            .save(&project_root)
            .map_or_else(|error| panic!("Failed to save cache: {error}"), |()| ());

        // Load the cache
        let loaded_cache = WorkspaceCache::load(&project_root)
            .unwrap_or_else(|error| panic!("Failed to load cache: {error}"));

        assert_eq!(loaded_cache.project_root, cache.project_root);
        assert_eq!(loaded_cache.file_count, cache.file_count);
        assert_eq!(loaded_cache.file_metadata.len(), cache.file_metadata.len());
    }

    #[test]
    fn test_cache_load_nonexistent() {
        let temp_dir =
            TempDir::new().unwrap_or_else(|error| panic!("Failed to create temp dir: {error}"));
        let project_root = temp_dir.path().to_path_buf();

        let result = WorkspaceCache::load(&project_root);
        result.unwrap_err();
    }

    #[test]
    fn test_cache_is_valid_unchanged() {
        let (_temp_dir, project_root, file_metadata) = create_test_project();
        let cache = WorkspaceCache::new(project_root.clone(), file_metadata);

        // Cache should be valid immediately after creation
        let is_valid = cache
            .is_valid(&project_root)
            .unwrap_or_else(|error| panic!("Failed to check validity: {error}"));
        assert!(is_valid);
    }

    #[test]
    fn test_cache_detects_file_changes() {
        let (_temp_dir, project_root, file_metadata) = create_test_project();
        let cache = WorkspaceCache::new(project_root.clone(), file_metadata);

        // Wait a bit to ensure modification time will be different
        thread::sleep(Duration::from_millis(100));

        // Modify one of the files
        let file_path = project_root.join("src/main.rs");
        let mut file =
            File::create(&file_path).unwrap_or_else(|error| panic!("Failed to open file: {error}"));
        file.write_all(b"// modified content")
            .unwrap_or_else(|error| panic!("Failed to write file: {error}"));

        // Cache should now be invalid
        let is_valid = cache
            .is_valid(&project_root)
            .unwrap_or_else(|error| panic!("Failed to check validity: {error}"));
        assert!(!is_valid);
    }

    #[test]
    fn test_cache_detects_deleted_files() {
        let (_temp_dir, project_root, file_metadata) = create_test_project();
        let cache = WorkspaceCache::new(project_root.clone(), file_metadata);

        // Delete one of the files
        let file_path = project_root.join("src/main.rs");
        fs::remove_file(&file_path)
            .unwrap_or_else(|error| panic!("Failed to remove file: {error}"));

        // Cache should be invalid because we can't verify the file
        let is_valid = cache
            .is_valid(&project_root)
            .unwrap_or_else(|error| panic!("Failed to check validity: {error}"));
        assert!(!is_valid);
    }

    #[test]
    fn test_cache_invalid_different_project() {
        let (_temp_dir, project_root, file_metadata) = create_test_project();
        let cache = WorkspaceCache::new(project_root, file_metadata);

        // Check validity with a different project root
        let other_temp =
            TempDir::new().unwrap_or_else(|error| panic!("Failed to create temp dir: {error}"));
        let other_root = other_temp.path().to_path_buf();

        let is_valid = cache
            .is_valid(&other_root)
            .unwrap_or_else(|error| panic!("Failed to check validity: {error}"));
        assert!(!is_valid);
    }

    #[test]
    fn test_cache_clear() {
        let (_temp_dir, project_root, file_metadata) = create_test_project();
        let cache = WorkspaceCache::new(project_root.clone(), file_metadata);

        // Save the cache
        cache
            .save(&project_root)
            .unwrap_or_else(|error| panic!("Failed to save cache: {error}"));

        // Verify it exists
        let cache_path = WorkspaceCache::cache_path(&project_root);
        assert!(cache_path.exists());

        // Clear the cache
        WorkspaceCache::clear(&project_root)
            .unwrap_or_else(|error| panic!("Failed to clear cache: {error}"));

        // Verify it's gone
        assert!(!cache_path.exists());
    }

    #[test]
    fn test_cache_clear_nonexistent() {
        let temp_dir =
            TempDir::new().unwrap_or_else(|error| panic!("Failed to create temp dir: {error}"));
        let project_root = temp_dir.path().to_path_buf();

        // Should not error when clearing non-existent cache
        let result = WorkspaceCache::clear(&project_root);
        result.unwrap();
    }

    #[test]
    fn test_cache_doesnt_rebuild_unchanged() {
        let (_temp_dir, project_root, file_metadata) = create_test_project();

        // Create and save initial cache
        let cache1 = WorkspaceCache::new(project_root.clone(), file_metadata.clone());
        cache1
            .save(&project_root)
            .unwrap_or_else(|error| panic!("Failed to save cache: {error}"));

        // Load the cache
        let loaded_cache = WorkspaceCache::load(&project_root)
            .unwrap_or_else(|error| panic!("Failed to load cache: {error}"));

        // Cache should be valid (no rebuilding needed)
        let is_valid = loaded_cache
            .is_valid(&project_root)
            .unwrap_or_else(|error| panic!("Failed to check validity: {error}"));
        assert!(is_valid);

        // File metadata should match
        assert_eq!(loaded_cache.file_metadata, file_metadata);
    }

    #[test]
    fn test_cache_with_many_files() {
        let temp_dir =
            TempDir::new().unwrap_or_else(|error| panic!("Failed to create temp dir: {error}"));
        let project_root = temp_dir.path().to_path_buf();

        fs::create_dir_all(project_root.join("src"))
            .unwrap_or_else(|error| panic!("Failed to create src dir: {error}"));

        // Create many files (more than the sample size of 10)
        let mut file_metadata = HashMap::new();
        for index in 0..20 {
            let file_path = project_root.join(format!("src/file{index}.rs"));
            let mut file = File::create(&file_path)
                .unwrap_or_else(|error| panic!("Failed to create file: {error}"));
            file.write_all(b"// test content")
                .unwrap_or_else(|error| panic!("Failed to write file: {error}"));

            let modified = fs::metadata(&file_path)
                .unwrap_or_else(|error| panic!("Failed to get metadata: {error}"))
                .modified()
                .unwrap_or_else(|error| panic!("Failed to get modified time: {error}"));
            file_metadata.insert(file_path, modified);
        }

        let cache = WorkspaceCache::new(project_root.clone(), file_metadata);
        assert_eq!(cache.file_count, 20);

        // Cache should be valid (uses sampling)
        let is_valid = cache
            .is_valid(&project_root)
            .unwrap_or_else(|error| panic!("Failed to check validity: {error}"));
        assert!(is_valid);
    }
}
