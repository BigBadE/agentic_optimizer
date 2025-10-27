//! System initialization for language backend and vector search.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::spawn;
use tokio::task::spawn_blocking;

use merlin_core::{CoreResult as Result, Error};
use merlin_languages::LanguageProvider;

use crate::embedding::{ProgressCallback, VectorSearchManager};

/// Spawn background task for full embedding initialization
///
/// Note: Does not use progress callback to avoid UI blocking
pub fn spawn_background_embedding(project_root: PathBuf) {
    spawn(async move {
        let mut bg_manager = VectorSearchManager::new(project_root);
        // Don't set progress callback - background task shouldn't update UI

        merlin_deps::tracing::info!("Background: Starting full embedding initialization...");
        if let Err(bg_error) = bg_manager.initialize().await {
            merlin_deps::tracing::warn!("Background embedding generation failed: {bg_error}");
        } else {
            merlin_deps::tracing::info!("Background: Embedding generation completed successfully");
        }
    });
}

/// Initialize backend with timeout
pub async fn initialize_backend_with_timeout(
    backend: Option<Box<dyn LanguageProvider>>,
    project_root: PathBuf,
) -> (Option<Box<dyn LanguageProvider>>, Result<()>) {
    use tokio::time::{Duration, timeout};

    let backend_task = spawn_blocking(move || {
        merlin_deps::tracing::info!("Initializing rust-analyzer...");
        let mut backend_mut = backend;
        if let Some(ref mut backend_ref) = backend_mut {
            merlin_deps::tracing::info!("Initializing language backend...");
            let result = backend_ref.initialize(&project_root);
            (backend_mut, result)
        } else {
            (backend_mut, Ok(()))
        }
    });

    // Timeout after 30 seconds
    match timeout(Duration::from_secs(30), backend_task).await {
        Ok(Ok(result)) => result,
        Ok(Err(join_error)) => {
            merlin_deps::tracing::error!("Backend task join error: {join_error}");
            (None, Err(Error::Other("Backend task panicked".into())))
        }
        Err(_timeout) => {
            merlin_deps::tracing::warn!("Backend initialization timed out after 30s");
            (
                None,
                Err(Error::Other("Backend initialization timeout".into())),
            )
        }
    }
}

/// Initializes systems (language backend and vector search) in parallel.
///
/// # Errors
/// Returns an error if critical initialization fails.
pub async fn initialize_systems_parallel(
    language_backend: &mut Option<Box<dyn LanguageProvider>>,
    language_backend_initialized: &mut bool,
    vector_manager: &mut Option<VectorSearchManager>,
    project_root: &Path,
    progress_callback: Option<&ProgressCallback>,
) -> Result<()> {
    let needs_backend_init = language_backend.is_some() && !*language_backend_initialized;
    let needs_vector_init = vector_manager.is_none();

    if !needs_backend_init && !needs_vector_init {
        return Ok(());
    }

    merlin_deps::tracing::info!("Initializing systems in parallel...");

    // Rust-analyzer initialization (CPU-bound, blocking) - spawn in background
    if needs_backend_init {
        let backend = language_backend.take();
        let project_root_clone = project_root.to_path_buf();

        spawn(async move {
            merlin_deps::tracing::info!("Background: Starting rust-analyzer initialization...");
            let (_backend, result) =
                initialize_backend_with_timeout(backend, project_root_clone).await;
            match result {
                Ok(()) => merlin_deps::tracing::info!(
                    "Background: rust-analyzer initialized successfully"
                ),
                Err(error) => {
                    merlin_deps::tracing::warn!(
                        "Background: rust-analyzer initialization failed: {error}"
                    );
                }
            }
        });

        // Mark as initialized to prevent re-initialization
        *language_backend_initialized = true;
    }

    // Vector search initialization (I/O-bound, async)
    // Truly non-blocking: loads cache if available, spawns background task otherwise
    if needs_vector_init {
        merlin_deps::tracing::info!("Loading embedding cache (non-blocking)...");
        let mut manager = VectorSearchManager::new(project_root.to_path_buf());

        if let Some(callback) = progress_callback {
            manager = manager.with_progress_callback(Arc::clone(callback));
        }

        // Try partial init first (fast, uses cache only)
        match manager.initialize_partial().await {
            Ok(()) => {
                merlin_deps::tracing::info!("Using cached embeddings immediately");
                *vector_manager = Some(manager);
            }
            Err(error) => {
                merlin_deps::tracing::warn!(
                    "No cache available, spawning background embedding generation: {error}"
                );
                spawn_background_embedding(project_root.to_path_buf());

                // Store the manager anyway (empty but ready for BM25 fallback)
                *vector_manager = Some(manager);
            }
        }
    }

    merlin_deps::tracing::info!(
        "Core systems initialized (backend and embeddings may continue in background)"
    );
    Ok(())
}
