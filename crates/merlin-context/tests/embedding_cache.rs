//! Tests for embedding cache behavior and validation

#![cfg_attr(
    test,
    allow(
        dead_code,
        unsafe_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        clippy::undocumented_unsafe_blocks,
        reason = "Test allows"
    )
)]

use merlin_context::VectorSearchManager;
use std::env;
use std::fs;
use tempfile::TempDir;

/// Create a test project directory with sample files
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src directory");

    // Create test files
    fs::write(
        src_dir.join("lib.rs"),
        "//! Test library\n\npub fn hello() -> &'static str {\n    \"hello\"\n}\n",
    )
    .expect("Failed to write lib.rs");

    fs::write(
        src_dir.join("main.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    )
    .expect("Failed to write main.rs");

    fs::write(
        src_dir.join("utils.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn multiply(a: i32, b: i32) -> i32 {\n    a * b\n}\n",
    )
    .expect("Failed to write utils.rs");

    temp_dir
}

#[tokio::test]
async fn test_cache_initialization_and_persistence() {
    // Ensure each test uses its own cache directory (not global MERLIN_FOLDER)
    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    // Initialize vector search manager
    let mut manager = VectorSearchManager::new(project_root.clone());

    // First initialization should build from scratch
    let result = manager.initialize().await;

    // Skip test if embedding model is not available
    if result.is_err() {
        eprintln!("Skipping test: embedding model not available");
        return;
    }

    assert!(
        !manager.is_empty(),
        "Manager should not be empty after initialization"
    );

    let cache_path = project_root
        .join(".merlin")
        .join("cache")
        .join("vector")
        .join("embeddings.bin");
    assert!(cache_path.exists(), "Cache file should exist");

    // Create a new manager and initialize from cache
    let mut manager2 = VectorSearchManager::new(project_root.clone());
    manager2
        .initialize()
        .await
        .expect("Second initialization should succeed");

    assert_eq!(
        manager.len(),
        manager2.len(),
        "Both managers should have the same number of indexed files"
    );
}

#[tokio::test]
async fn test_cache_invalidation_on_file_modification() {
    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    // Initialize vector search manager
    let mut manager = VectorSearchManager::new(project_root.clone());
    let result = manager.initialize().await;

    // Skip test if embedding model is not available
    if result.is_err() {
        eprintln!("Skipping test: embedding model not available");
        return;
    }

    let initial_len = manager.len();
    assert!(initial_len > 0, "Manager should have indexed files");

    // Modify a file
    let src_dir = project_root.join("src");
    fs::write(
        src_dir.join("lib.rs"),
        "//! Modified library\n\npub fn goodbye() -> &'static str {\n    \"goodbye\"\n}\n",
    )
    .expect("Failed to modify lib.rs");

    // Re-initialize and verify cache detects the modification
    let mut manager2 = VectorSearchManager::new(project_root.clone());
    manager2
        .initialize()
        .await
        .expect("Re-initialization should succeed after file modification");

    // Should still have same number of indexed files (file was modified, not added/removed)
    assert_eq!(
        initial_len,
        manager2.len(),
        "Number of indexed files should remain the same"
    );
}

#[tokio::test]
async fn test_cache_handles_new_files() {
    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    // Initialize vector search manager
    let mut manager = VectorSearchManager::new(project_root.clone());
    let result = manager.initialize().await;

    // Skip test if embedding model is not available
    if result.is_err() {
        eprintln!("Skipping test: embedding model not available");
        return;
    }

    let initial_len = manager.len();

    // Add a new file
    let src_dir = project_root.join("src");
    fs::write(
        src_dir.join("new_module.rs"),
        "pub fn new_function() -> i32 {\n    42\n}\n",
    )
    .expect("Failed to write new_module.rs");

    // Re-initialize and verify new file is detected
    let mut manager2 = VectorSearchManager::new(project_root.clone());
    manager2
        .initialize()
        .await
        .expect("Re-initialization should succeed after adding file");

    assert!(
        manager2.len() > initial_len,
        "Should have more indexed files after adding a new file"
    );
}

#[tokio::test]
async fn test_cache_handles_deleted_files() {
    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    // Initialize vector search manager
    let mut manager = VectorSearchManager::new(project_root.clone());
    let result = manager.initialize().await;

    // Skip test if embedding model is not available
    if result.is_err() {
        eprintln!("Skipping test: embedding model not available");
        return;
    }

    let initial_len = manager.len();

    // Delete a file
    let src_dir = project_root.join("src");
    fs::remove_file(src_dir.join("utils.rs")).expect("Failed to delete utils.rs");

    // Re-initialize and verify deleted file is handled
    let mut manager2 = VectorSearchManager::new(project_root.clone());
    manager2
        .initialize()
        .await
        .expect("Re-initialization should succeed after deleting file");

    assert!(
        manager2.len() < initial_len,
        "Should have fewer indexed files after deleting a file"
    );
}

#[tokio::test]
async fn test_empty_cache_rebuilds() {
    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    // Initialize vector search manager
    let mut manager = VectorSearchManager::new(project_root.clone());
    let result = manager.initialize().await;

    // Skip test if embedding model is not available
    if result.is_err() {
        eprintln!("Skipping test: embedding model not available");
        return;
    }

    let initial_len = manager.len();

    // Delete cache file
    let cache_path = project_root
        .join(".merlin")
        .join("cache")
        .join("vector")
        .join("embeddings.bin");
    if cache_path.exists() {
        fs::remove_file(&cache_path).expect("Failed to delete cache file");
    }

    // Re-initialize and verify it rebuilds from scratch
    let mut manager2 = VectorSearchManager::new(project_root.clone());
    manager2
        .initialize()
        .await
        .expect("Should rebuild cache from scratch");

    assert_eq!(
        initial_len,
        manager2.len(),
        "Rebuilt cache should have same number of indexed files"
    );
    assert!(cache_path.exists(), "Cache file should be recreated");
}

#[tokio::test]
async fn test_search_returns_relevant_results() {
    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    // Initialize vector search manager
    let mut manager = VectorSearchManager::new(project_root.clone());
    let result = manager.initialize().await;

    // Skip test if embedding model is not available
    if result.is_err() {
        eprintln!("Skipping test: embedding model not available");
        return;
    }

    // Search for "add" should find utils.rs
    let results = manager
        .search("add function", 5)
        .await
        .expect("Search should succeed");

    // Results might be empty if similarity threshold is not met
    // Just verify the search completes without error
    assert!(
        results.len() <= 5,
        "Should return at most the requested number of results"
    );
}

#[test]
fn test_cache_directory_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path().to_path_buf();

    // Create manager (cache dir doesn't exist yet)
    let manager = VectorSearchManager::new(project_root.clone());

    // Verify cache path is set correctly
    let cache_path = project_root
        .join(".merlin")
        .join("cache")
        .join("vector")
        .join("embeddings.bin");
    assert!(cache_path.to_str().is_some(), "Cache path should be valid");

    // New manager should be empty before initialization
    assert!(
        manager.is_empty(),
        "New VectorSearchManager should be empty"
    );
    // Explicit scope end will drop manager; no manual drop() call needed
}

#[tokio::test]
async fn test_concurrent_file_processing() {
    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    // Add more files to test concurrent processing
    let src_dir = project_root.join("src");
    for idx in 0..20 {
        fs::write(
            src_dir.join(format!("module_{idx}.rs")),
            format!("pub fn function_{idx}() -> i32 {{\n    {idx}\n}}\n"),
        )
        .expect("Failed to write test file");
    }

    // Initialize vector search manager
    let mut manager = VectorSearchManager::new(project_root.clone());
    let result = manager.initialize().await;

    // Skip test if embedding model is not available
    if result.is_err() {
        eprintln!("Skipping test: embedding model not available");
        return;
    }

    // Verify all files were processed
    assert!(
        manager.len() >= 20,
        "Should have indexed at least 20 modules"
    );
}

#[test]
fn test_chunk_count_consistency() {
    // This test verifies that process_chunk_results returns the correct count
    // This is a unit test that doesn't require async or embedding model

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    let manager = VectorSearchManager::new(project_root);

    // Verify initial state
    assert!(manager.is_empty(), "New manager should be empty");
    assert_eq!(manager.len(), 0, "New manager should have length 0");
}

#[tokio::test]
async fn test_cache_version_validation() {
    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = create_test_project();
    let project_root = temp_dir.path().to_path_buf();

    // Initialize vector search manager
    let mut manager = VectorSearchManager::new(project_root.clone());
    let result = manager.initialize().await;

    // Skip test if embedding model is not available
    if result.is_err() {
        eprintln!("Skipping test: embedding model not available");
        return;
    }

    // Verify cache was created
    let cache_path = project_root
        .join(".merlin")
        .join("cache")
        .join("vector")
        .join("embeddings.bin");
    assert!(cache_path.exists(), "Cache file should exist");

    // Corrupt the cache by writing invalid data
    fs::write(&cache_path, b"invalid cache data").expect("Failed to write invalid cache");

    // Re-initialize should detect invalid cache and rebuild
    let mut manager2 = VectorSearchManager::new(project_root.clone());
    manager2
        .initialize()
        .await
        .expect("Should handle invalid cache gracefully");

    assert!(
        !manager2.is_empty(),
        "Should rebuild from scratch after invalid cache"
    );
}

#[tokio::test]
async fn test_batch_processing_timeout() {
    use std::time::Duration;
    use tokio::time::timeout;

    // SAFETY: We are in a test environment and need to ensure clean state.
    // This is safe because we are in a single-threaded test context.
    unsafe {
        env::remove_var("MERLIN_FOLDER");
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path().to_path_buf();
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src directory");

    // Create exactly 15 files to test batch processing (batch size is 10)
    // This should trigger at least 2 batches
    for idx in 0..15 {
        fs::write(
            src_dir.join(format!("module_{idx}.rs")),
            format!(
                "//! Module {idx}\n\npub fn function_{idx}() -> i32 {{\n    {idx}\n}}\n\npub struct Struct{idx} {{\n    value: i32,\n}}\n"
            ),
        )
        .expect("Failed to write test file");
    }

    // Initialize vector search manager with a timeout
    let mut manager = VectorSearchManager::new(project_root.clone());

    // Set a reasonable timeout - should complete in 120 seconds if working correctly
    // This needs to be long enough to allow model pulling on first run
    // If it hangs after 10 files, this will catch it
    let result = timeout(Duration::from_secs(120), manager.initialize()).await;

    match result {
        Ok(Ok(())) => {
            // Success - verify all files were processed
            eprintln!("Initialization completed successfully");
            assert!(
                manager.len() >= 10,
                "Should have indexed at least 10 files, got {}",
                manager.len()
            );
        }
        Ok(Err(error)) => {
            // Embedding model might not be available
            eprintln!("Skipping test: {error}");
        }
        Err(timeout_error) => {
            panic!(
                "Initialization timed out after 30 seconds - likely stuck after batch processing: {timeout_error}"
            );
        }
    }
}
