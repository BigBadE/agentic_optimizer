//! System initialization for vector search.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::spawn;

use merlin_core::CoreResult as Result;

use crate::embedding::{ProgressCallback, VectorSearchManager};

/// Spawn background task for full embedding initialization
///
/// Note: Does not use progress callback to avoid UI blocking
pub fn spawn_background_embedding(project_root: PathBuf) {
    spawn(async move {
        let mut bg_manager = VectorSearchManager::new(&project_root);
        // Don't set progress callback - background task shouldn't update UI

        tracing::info!("Background: Starting full embedding initialization...");
        if let Err(bg_error) = bg_manager.initialize().await {
            tracing::warn!("Background embedding generation failed: {bg_error}");
        } else {
            tracing::info!("Background: Embedding generation completed successfully");
        }
    });
}

/// Initializes vector search system.
///
/// # Errors
/// Returns an error if critical initialization fails.
pub async fn initialize_systems_parallel(
    vector_manager: &mut Option<VectorSearchManager>,
    project_root: &Path,
    progress_callback: Option<&ProgressCallback>,
) -> Result<()> {
    let needs_vector_init = vector_manager.is_none();

    if !needs_vector_init {
        return Ok(());
    }

    tracing::info!("Initializing vector search...");

    // Vector search initialization (I/O-bound, async)
    // Truly non-blocking: loads cache if available, spawns background task otherwise
    tracing::info!("Loading embedding cache (non-blocking)...");
    let mut manager = VectorSearchManager::new(project_root);

    if let Some(callback) = progress_callback {
        manager = manager.with_progress_callback(Arc::clone(callback));
    }

    // Try partial init first (fast, uses cache only)
    match manager.initialize_partial().await {
        Ok(()) => {
            tracing::info!("Using cached embeddings immediately");
            *vector_manager = Some(manager);
        }
        Err(error) => {
            tracing::warn!("No cache available, spawning background embedding generation: {error}");
            spawn_background_embedding(project_root.to_path_buf());

            // Store the manager anyway (empty but ready for BM25 fallback)
            *vector_manager = Some(manager);
        }
    }

    tracing::info!("Vector search initialized (embeddings may continue in background)");
    Ok(())
}
