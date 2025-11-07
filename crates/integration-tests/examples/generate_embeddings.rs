//! Generate embeddings for test workspaces
//!
//! This example generates embeddings for specified workspaces to enable
//! semantic search in integration tests.

use merlin_context::VectorSearchManager;
use merlin_context::embedding::FakeEmbeddingClient;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = PathBuf::from("test-workspaces/context-workspace");

    merlin_deps::tracing::info!("Generating embeddings for {}", workspace.display());

    VectorSearchManager::with_provider(&workspace, FakeEmbeddingClient)
        .initialize()
        .await?;

    merlin_deps::tracing::info!("✓ Embeddings generated successfully");

    Ok(())
}
