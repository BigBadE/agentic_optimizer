//! Command handlers for CLI operations

use anyhow::Result;
use merlin_agent::RoutingOrchestrator;
use merlin_routing::RoutingConfig;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{
    EnvFilter, Registry, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

use crate::cli::Validation;
use crate::interactive::{InteractiveFlags, handle_interactive_agent};
use crate::utils::get_merlin_folder;

/// Handle interactive agent session with routing
///
/// # Errors
/// Returns an error if the orchestrator fails to initialize or process requests
pub async fn handle_interactive(
    project: PathBuf,
    validation: Validation,
    local_only: bool,
    context_dump: bool,
) -> Result<()> {
    // Initialize tracing - TUI mode logs to file
    let merlin_dir = get_merlin_folder(&project)?;
    fs::create_dir_all(&merlin_dir)?;

    let debug_log = merlin_dir.join("debug.log");
    if debug_log.exists() {
        fs::remove_file(&debug_log)?;
    }

    let log_file = fs::OpenOptions::new()
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
        tracing::warn!("Failed to load config from ~/.merlin/config.toml: {error}");
        tracing::warn!("Using default configuration");
        RoutingConfig::default()
    });

    config.validation.enabled = !matches!(validation, Validation::Disabled);
    config.workspace.root_path.clone_from(&project);
    config.execution.context_dump = context_dump;

    if local_only {
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;
    }

    let orchestrator = RoutingOrchestrator::new(config);

    let flags = InteractiveFlags { local_only };

    handle_interactive_agent(orchestrator?, project, flags).await
}
