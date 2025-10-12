//! Command handlers for CLI operations

use anyhow::Result;
use console::{Term, style};
use merlin_agent::{Agent, AgentConfig};
use merlin_context::ContextBuilder;
use merlin_core::{Context, ModelProvider, Query};
use merlin_languages::{Language, create_backend};
use merlin_providers::OpenRouterProvider;
use merlin_routing::{RoutingConfig, RoutingOrchestrator};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use toml::to_string_pretty;

use crate::cli::{UiMode, Validation};
use crate::config::Config;
use crate::interactive::{
    InteractiveFlags, chat_loop, handle_interactive_agent, print_chat_header,
};
use crate::utils::{display_response_metrics, get_merlin_folder};

/// Run interactive chat session with a single provider-backed agent.
///
/// # Errors
/// Returns an error if configuration loading, provider initialization, IO, or agent execution fails.
pub async fn handle_chat(project: PathBuf, model: Option<String>) -> Result<()> {
    let term = Term::stdout();

    print_chat_header(&term, &project)?;

    let cli_config = Config::load_from_project(&project);

    term.write_line(&format!("{}", style("Initializing agent...").cyan()))?;

    let mut provider = OpenRouterProvider::from_config_or_env(cli_config.providers.openrouter_key)?;
    let model_to_use = model.or_else(|| cli_config.providers.high_model.clone());
    if let Some(model_name) = model_to_use {
        provider = provider.with_model(model_name);
    }
    let provider: Arc<dyn ModelProvider> = Arc::new(provider);

    let backend = create_backend(Language::Rust)?;

    let agent_config = AgentConfig::default()
        .with_system_prompt(
            "You are a helpful AI coding assistant. Analyze the provided code context and help the user with their requests. \
             Be concise but thorough. When making code changes, provide complete, working code."
        )
        .with_max_context_tokens(100_000)
        .with_top_k_context_files(15);

    let agent = Agent::with_config(provider, agent_config);
    let mut executor = agent.executor().with_language_backend(backend);

    term.write_line(&format!(
        "{}",
        style("\u{2713} Agent ready!").green().bold()
    ))?;
    term.write_line("")?;
    term.write_line(&format!(
        "{}",
        style("Type your message (or 'exit' to quit):").cyan()
    ))?;
    term.write_line("")?;

    chat_loop(&term, &mut executor, &project).await
}

/// Setup provider from configuration
///
/// # Errors
/// Returns an error if provider configuration is invalid or missing
fn setup_provider(project: &Path) -> Result<OpenRouterProvider> {
    let config = Config::load_from_project(project);
    let mut provider = OpenRouterProvider::from_config_or_env(config.providers.openrouter_key)?;
    if let Some(model_name) = config.providers.high_model {
        provider = provider.with_model(model_name);
    }
    Ok(provider)
}

/// Build context for query
///
/// # Errors
/// Returns an error if context building fails
async fn build_query_context(
    project: PathBuf,
    query_text: String,
    files: Vec<PathBuf>,
    max_files: Option<usize>,
) -> Result<(Query, Context)> {
    let mut builder = ContextBuilder::new(project);
    if let Some(max) = max_files {
        builder = builder.with_max_files(max);
    }

    let query = Query::new(query_text).with_files(files);

    tracing::info!("Building context...");
    let context = builder.build_context(&query).await?;
    tracing::info!(
        "Context built: {} files, ~{} tokens",
        context.files.len(),
        context.token_estimate()
    );

    Ok((query, context))
}

/// Handle the query command by building context and sending to provider.
///
/// # Errors
/// Returns an error if configuration loading, context building, or provider request fails.
pub async fn handle_query(
    query_text: String,
    project: PathBuf,
    files: Vec<PathBuf>,
    max_files: Option<usize>,
) -> Result<()> {
    tracing::info!("Processing query: {}", query_text);

    let provider = setup_provider(&project)?;
    let (query, context) = build_query_context(project, query_text, files, max_files).await?;

    let estimated_cost = provider.estimate_cost(&context);
    tracing::info!("Estimated cost: ${:.4}", estimated_cost);

    tracing::info!("Sending request to {}...", provider.name());
    let response = provider.generate(&query, &context).await?;

    display_response_metrics(&response);

    Ok(())
}

/// Handle the prompt command - show relevant files without sending to LLM.
///
/// # Errors
/// Returns an error if context building or terminal IO fails.
pub async fn handle_prompt(
    query_text: String,
    project: PathBuf,
    files: Vec<PathBuf>,
    max_files: Option<usize>,
) -> Result<()> {
    let term = Term::stdout();

    term.write_line(&format!(
        "{} {}",
        style("Analyzing prompt:").cyan().bold(),
        style(&query_text).yellow()
    ))?;

    let mut builder = ContextBuilder::new(project);
    if let Some(max) = max_files {
        builder = builder.with_max_files(max);
    }

    // Always enable Rust semantic analysis
    let backend = create_backend(Language::Rust)?;
    builder = builder.with_language_backend(backend);

    let query = Query::new(query_text).with_files(files);

    term.write_line(&format!("{}", style("Building context...").cyan()))?;

    // Build context and capture any errors/warnings
    let context = match builder.build_context(&query).await {
        Ok(ctx) => ctx,
        Err(error) => {
            term.write_line(&format!(
                "{} {}",
                style("Error:").red().bold(),
                style(error.to_string()).red()
            ))?;
            return Err(error.into());
        }
    };

    term.write_line("")?;
    term.write_line(&format!(
        "{} {}",
        style("\u{2713} Context ready:").green().bold(),
        style(format!(
            "{} sections, ~{} tokens",
            context.files.len(),
            context.token_estimate()
        ))
        .yellow()
    ))?;
    term.write_line("")?;

    Ok(())
}

/// Output current configuration. If `full` is true, prints full TOML.
///
/// # Errors
/// Returns an error if serialization or logging fails.
pub fn handle_config(full: bool) -> Result<()> {
    let config = Config::from_env();

    if full {
        let toml = to_string_pretty(&config)?;
        tracing::info!("{toml}");
    } else {
        tracing::info!("Configuration:");
        tracing::info!(
            "  OpenRouter API Key: {status}",
            status = if config.providers.openrouter_key.is_some() {
                "Set"
            } else {
                "Not set"
            }
        );
        tracing::info!(
            "  High Model: {model}",
            model = config.providers.high_model.as_deref().unwrap_or("default")
        );
        tracing::info!(
            "  Medium Model: {model}",
            model = config
                .providers
                .medium_model
                .as_deref()
                .unwrap_or("default")
        );
    }

    Ok(())
}

/// Placeholder for future metrics handler.
pub fn handle_metrics(_daily: bool) {
    tracing::info!("Metrics tracking not yet implemented in MVP.");
    tracing::info!("This will be added in Phase 5 (Advanced Optimizations).");
}

/// Handle interactive agent session with routing
///
/// # Errors
/// Returns an error if the orchestrator fails to initialize or process requests
pub async fn handle_interactive(
    project: PathBuf,
    validation: Validation,
    ui: UiMode,
    local_only: bool,
    context_dump: bool,
) -> Result<()> {
    // Initialize tracing based on UI mode BEFORE creating orchestrator
    if matches!(ui, UiMode::Tui) {
        // TUI mode - log to file
        use std::fs;
        use std::sync::Arc;
        use tracing_subscriber::{
            EnvFilter, Registry, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _,
        };

        let merlin_dir = get_merlin_folder(&project);
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
    } else {
        // Plain modes - log to stdout
        use tracing_subscriber::{
            EnvFilter, Registry, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _,
        };
        Registry::default()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "agentic_optimizer=info".into()),
            )
            .with(fmt::layer())
            .init();
    }

    // Create routing configuration
    let mut config = RoutingConfig::default();
    config.validation.enabled = !matches!(validation, Validation::Disabled);
    config.workspace.root_path.clone_from(&project);
    config.execution.context_dump = context_dump;

    if local_only {
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;
    }

    let orchestrator = RoutingOrchestrator::new(config);

    let flags = InteractiveFlags {
        validation_enabled: !matches!(validation, Validation::Disabled),
        ui,
        local_only,
        context_dump,
    };

    handle_interactive_agent(orchestrator, project, flags).await
}
