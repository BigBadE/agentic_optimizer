//! Tests for embedding cache behavior and validation
//!
//! These tests verify cache persistence, invalidation, and recovery with mocked embeddings.
//! They use minimal test files (2 tiny files) to reduce I/O time.
//! Embeddings are deterministic (content hash-based) using `FakeEmbeddingClient`.

use merlin_context::{EmbeddingProvider, VectorSearchManager};
use merlin_core::CoreResult as Result;
use merlin_deps::tempfile::TempDir;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash as _, Hasher as _};
use std::path::PathBuf;

/// Fake embedding client for testing (deterministic, hash-based)
#[derive(Clone)]
struct FakeEmbeddingClient;

impl EmbeddingProvider for FakeEmbeddingClient {
    async fn ensure_model_available(&self) -> Result<()> {
        Ok(())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        Ok(Self::fake_embedding(text))
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|text| Self::fake_embedding(text))
            .collect())
    }
}

impl FakeEmbeddingClient {
    fn fake_embedding(text: &str) -> Vec<f32> {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        let mut vec = Vec::with_capacity(384);
        for idx in 0..384 {
            let value = ((hash.wrapping_add(idx as u64)) % 1000) as f32 / 1000.0;
            vec.push(value);
        }
        vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_core::Error as CoreError;

    /// Create minimal test project (2 tiny files to minimize embedding time)
    ///
    /// # Errors
    ///
    /// Returns an error if directory or file creation fails.
    fn create_minimal_project() -> Result<TempDir> {
        let temp_dir = TempDir::new().map_err(CoreError::Io)?;
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).map_err(CoreError::Io)?;

        // Tiny files to minimize embedding generation time
        fs::write(src_dir.join("lib.rs"), "pub fn a() { }").map_err(CoreError::Io)?;
        fs::write(src_dir.join("main.rs"), "fn main() { }").map_err(CoreError::Io)?;

        Ok(temp_dir)
    }

    #[tokio::test]
    async fn test_cache_lifecycle() -> Result<()> {
        // Test cache creation, persistence, and reload in one test
        let temp_dir = create_minimal_project()?;
        let project_root = temp_dir.path().to_path_buf();

        // First init - builds cache with fake embeddings
        let mut manager1 = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager1.initialize().await?;

        // Resolve cache path the same way VectorSearchManager does
        let cache_path = env::var("MERLIN_FOLDER").map_or_else(
            |_| {
                project_root
                    .join(".merlin")
                    .join("cache")
                    .join("vector")
                    .join("embeddings.bin")
            },
            |folder| {
                PathBuf::from(folder)
                    .join("cache")
                    .join("vector")
                    .join("embeddings.bin")
            },
        );
        assert!(cache_path.exists(), "Cache file should exist");
        let len1 = manager1.len();

        // Second init - loads from cache (fast)
        let mut manager2 = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager2.initialize().await?;
        assert_eq!(len1, manager2.len(), "Cache reload should have same files");

        // Delete cache
        fs::remove_file(&cache_path).map_err(CoreError::Io)?;

        // Third init - rebuilds cache
        let mut manager3 = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager3.initialize().await?;
        assert_eq!(len1, manager3.len(), "Rebuild should have same files");
        assert!(cache_path.exists(), "Cache should be recreated");

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_file_changes() -> Result<()> {
        // Test modification, addition, and deletion in one test
        let temp_dir = create_minimal_project()?;
        let project_root = temp_dir.path().to_path_buf();
        let src_dir = project_root.join("src");

        // Initial state
        let mut manager = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager.initialize().await?;
        let initial_len = manager.len();

        // Test 1: Modification
        fs::write(src_dir.join("lib.rs"), "pub fn b() { }").map_err(CoreError::Io)?;
        let mut manager2 = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager2.initialize().await?;
        assert_eq!(
            initial_len,
            manager2.len(),
            "Modification shouldn't change file count"
        );

        // Test 2: Addition
        fs::write(src_dir.join("new.rs"), "pub fn c() { }").map_err(CoreError::Io)?;
        let mut manager3 = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager3.initialize().await?;
        assert!(
            manager3.len() > initial_len,
            "Addition should increase file count"
        );
        let after_add = manager3.len();

        // Test 3: Deletion
        fs::remove_file(src_dir.join("new.rs")).map_err(CoreError::Io)?;
        let mut manager4 = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager4.initialize().await?;
        assert!(
            manager4.len() < after_add,
            "Deletion should decrease file count"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_corrupted_cache_recovery() -> Result<()> {
        let temp_dir = create_minimal_project()?;
        let project_root = temp_dir.path().to_path_buf();

        // Build cache
        let mut manager = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager.initialize().await?;
        let initial_len = manager.len();

        // Corrupt cache - resolve path the same way VectorSearchManager does
        let cache_path = env::var("MERLIN_FOLDER").map_or_else(
            |_| {
                project_root
                    .join(".merlin")
                    .join("cache")
                    .join("vector")
                    .join("embeddings.bin")
            },
            |folder| {
                PathBuf::from(folder)
                    .join("cache")
                    .join("vector")
                    .join("embeddings.bin")
            },
        );
        fs::write(&cache_path, b"corrupted").map_err(CoreError::Io)?;

        // Should rebuild
        let mut manager2 = VectorSearchManager::with_provider(&project_root, FakeEmbeddingClient);
        manager2.initialize().await?;
        assert_eq!(
            initial_len,
            manager2.len(),
            "Should have same files after rebuild"
        );

        Ok(())
    }
}
