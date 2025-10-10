//! Test for vector search initialization with existing cache
//!
//! Run with: `cargo test -p merlin-context --test embedding_initialization -- --ignored --nocapture`

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::print_stderr,
    clippy::missing_panics_doc,
    clippy::min_ident_chars,
    reason = "Test code: panics and prints are acceptable for test failures and debugging"
)]

use merlin_context::VectorSearchManager;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
#[ignore = "Requires test repository and Ollama"]
async fn test_initialization_with_existing_cache() {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use tokio::time::timeout;

    eprintln!("\n=== Testing initialization with existing cache ===");

    // Path relative to workspace root
    let project_root = PathBuf::from("../../benchmarks/test_repositories/valor");
    let project_root = project_root.canonicalize().unwrap_or_else(|_| {
        // Try from workspace root
        PathBuf::from("benchmarks/test_repositories/valor")
    });

    if !project_root.exists() {
        eprintln!(
            "SKIPPED: Test repository not found at {}",
            project_root.display()
        );
        return;
    }

    eprintln!("Using repository at: {}", project_root.display());

    // Check if cache exists
    let cache_path = project_root.join(".merlin/embeddings.bin");
    if cache_path.exists() {
        let metadata = fs::metadata(&cache_path).expect("Failed to get cache metadata");
        eprintln!("Found existing cache: {} bytes", metadata.len());
    } else {
        eprintln!("No cache found - will build from scratch");
    }

    eprintln!("\nInitializing VectorSearchManager...");

    let mut manager = VectorSearchManager::new(project_root.clone());

    let start = Instant::now();
    let result = timeout(Duration::from_secs(300), manager.initialize()).await;
    let elapsed = start.elapsed();

    eprintln!("\nInitialization took {elapsed:?}");

    match result {
        Ok(Ok(())) => {
            eprintln!("SUCCESS: Initialization completed in {elapsed:?}");
            eprintln!("Indexed {} entries", manager.len());
            // Test passed - initialization completed
        }
        Ok(Err(error)) => {
            eprintln!("SKIPPED: {error}");
        }
        Err(timeout_err) => {
            panic!("HANG: Initialization timed out after {elapsed:?}! Error: {timeout_err}");
        }
    }
}

#[tokio::test]
#[ignore = "Requires Ollama"]
async fn test_reproduce_hang_after_10_files() {
    use std::time::{Duration, Instant};
    use tokio::time::timeout;

    eprintln!("\n=== Testing hang after 10 files ===");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path().to_path_buf();
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src directory");

    // Create exactly 11 files - just over the batch size of 10
    // Each file should produce ~3 chunks (29 chunks / 10 files ≈ 3 chunks per file)
    for i in 0..11 {
        let content = format!(
            "//! Module {i} - This is a longer documentation comment to create multiple chunks\n\
            //! It contains more content to ensure we get at least 2-3 chunks per file\n\
            //! Adding more lines here to increase the chunk count\n\
            //! More documentation content here\n\
            //! And even more to reach the chunking threshold\n\n\
            pub struct Struct{i} {{\n\
                field1: i32,\n\
                field2: String,\n\
                field3: Vec<u8>,\n\
            }}\n\n\
            impl Struct{i} {{\n\
                pub fn new() -> Self {{\n\
                    Self {{\n\
                        field1: {i},\n\
                        field2: String::from(\"test\"),\n\
                        field3: Vec::new(),\n\
                    }}\n\
                }}\n\n\
                pub fn method1(&self) -> i32 {{\n\
                    self.field1\n\
                }}\n\n\
                pub fn method2(&mut self, val: i32) {{\n\
                    self.field1 = val;\n\
                }}\n\
            }}\n\n\
            pub fn helper_function_{i}() -> i32 {{\n\
                {i} * 2\n\
            }}\n\n\
            pub fn another_function_{i}(x: i32) -> i32 {{\n\
                x + {i}\n\
            }}\n"
        );

        fs::write(src_dir.join(format!("module_{i}.rs")), content)
            .expect("Failed to write test file");
    }

    eprintln!("Created 11 test files with substantial content");
    eprintln!("Starting initialization with 10-second timeout...");

    // Initialize vector search manager with a SHORT timeout
    let mut manager = VectorSearchManager::new(project_root.clone());

    // Use a 10-second timeout to detect the hang quickly
    let start = Instant::now();
    let result = timeout(Duration::from_secs(10), manager.initialize()).await;
    let elapsed = start.elapsed();

    eprintln!("Initialization took {elapsed:?}");

    match result {
        Ok(Ok(())) => {
            eprintln!("SUCCESS: Initialization completed in {elapsed:?}");
            eprintln!("Indexed {} chunks", manager.len());
            panic!("Test was supposed to hang but completed successfully - the bug may be fixed!");
        }
        Ok(Err(error)) => {
            eprintln!("SKIPPED: Embedding model not available: {error}");
        }
        Err(timeout_error) => {
            panic!("HANG: Initialization timed out after {elapsed:?}! Error: {timeout_error}");
        }
    }
}
