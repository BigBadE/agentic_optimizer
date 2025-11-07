//! Command handlers for CLI operations

use merlin_agent::{RoutingOrchestrator, ThreadStore};
use merlin_deps::anyhow::Result;
use merlin_routing::RoutingConfig;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::fs as async_fs;
use tracing_subscriber::{
    EnvFilter, Registry, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

use crate::cli::Validation;
use crate::interactive::run_tui_interactive;
use crate::utils::get_merlin_folder;

/// Handle interactive agent session with routing
///
/// # Errors
/// Returns an error if the orchestrator fails to initialize or process requests
pub async fn handle_interactive(
    project: PathBuf,
    _validation: Validation,
    local_only: bool,
    _context_dump: bool,
) -> Result<()> {
    // Initialize tracing - TUI mode logs to file
    let merlin_dir = get_merlin_folder(&project)?;
    async_fs::create_dir_all(&merlin_dir).await?;

    let debug_log = merlin_dir.join("debug.log");
    if async_fs::try_exists(&debug_log).await.unwrap_or(false) {
        async_fs::remove_file(&debug_log).await?;
    }

    // Open log file synchronously for tracing writer (needs sync File)
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_log)?;

    Registry::default()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "merlin_context=info,merlin_routing=info,agentic_optimizer=info".into()
        }))
        .with(
            fmt::layer()
                .with_writer(Arc::new(log_file))
                .with_ansi(false)
                .with_target(true)
                .with_level(true),
        )
        .init();

    // Load or create routing configuration from ~/.merlin/config.toml
    let mut config = RoutingConfig::load_or_create().unwrap_or_else(|error| {
        merlin_deps::tracing::warn!("Failed to load config from ~/.merlin/config.toml: {error}");
        merlin_deps::tracing::warn!("Using default configuration");
        RoutingConfig::default()
    });

    if local_only {
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;
    }

    // Create thread store
    let thread_storage_path = merlin_dir.join("threads");
    let thread_store = Arc::new(Mutex::new(ThreadStore::new(thread_storage_path)?));

    // Create orchestrator with thread store
    let orchestrator =
        RoutingOrchestrator::new(config)?.with_thread_store(Arc::clone(&thread_store));

    run_tui_interactive(orchestrator, project, true).await
}
