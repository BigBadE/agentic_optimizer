//! Merlin CLI - Interactive AI coding assistant command-line interface
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        reason = "Allow for tests"
    )
)]

use anyhow::Result;
use console::{Term, style};
use dialoguer::Input;
use merlin_agent::{Agent, AgentConfig, AgentExecutor, AgentRequest};
use merlin_context::ContextBuilder;
use merlin_core::{Context, ModelProvider, Query, Response, TokenUsage};
use merlin_languages::{Language, create_backend};
use merlin_providers::OpenRouterProvider;
use merlin_routing::{
    MessageLevel, RoutingConfig, RoutingOrchestrator, Task, TaskId, TaskResult, TuiApp, UiChannel,
    UiEvent,
};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::spawn;
use toml::to_string_pretty;
use tracing_subscriber::{
    EnvFilter, Registry, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

mod cli;
mod config;

use clap::Parser as _;
use cli::{Cli, Commands, UiMode, Validation};
use config::Config;

const MAX_TASKS: usize = 50;

#[tokio::main]
async fn main() -> Result<()> {
    Registry::default()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "agentic_optimizer=info".into()))
        .with(fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Chat { project, model }) => {
            handle_chat(project, model).await?;
        }
        Some(Commands::Query {
            query,
            project,
            files,
            max_files,
        }) => {
            handle_query(query, project, files, max_files).await?;
        }
        Some(Commands::Prompt {
            query,
            project,
            files,
            max_files,
        }) => {
            handle_prompt(query, project, files, max_files).await?;
        }
        Some(Commands::Config { full }) => {
            handle_config(full)?;
        }
        Some(Commands::Metrics { daily }) => {
            handle_metrics(daily);
        }
        None => {
            // No subcommand - start interactive agent session
            let flags = InteractiveFlags {
                validation: cli.validation,
                ui: cli.ui,
                local_only: cli.local,
            };
            handle_interactive_agent(cli.project, flags).await?;
        }
    }
    Ok(())
}

/// Run interactive chat session with a single provider-backed agent.
///
/// # Errors
/// Returns an error if configuration loading, provider initialization, IO, or agent execution fails.
async fn handle_chat(project: PathBuf, model: Option<String>) -> Result<()> {
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

/// Display response metrics
fn display_response_metrics(response: &Response) {
    tracing::info!("\n{sep}\n", sep = "=".repeat(80));
    tracing::info!("{text}", text = response.text);
    tracing::info!("{sep}", sep = "=".repeat(80));

    tracing::info!("\nMetrics:");
    tracing::info!("  Provider: {provider}", provider = response.provider);
    tracing::info!(
        "  Confidence: {confidence:.2}",
        confidence = response.confidence
    );
    tracing::info!("  Latency: {latency}ms", latency = response.latency_ms);
    tracing::info!("  Tokens:");
    tracing::info!("    Input: {input}", input = response.tokens_used.input);
    tracing::info!("    Output: {output}", output = response.tokens_used.output);
    tracing::info!(
        "    Cache Read: {cache_read}",
        cache_read = response.tokens_used.cache_read
    );
    tracing::info!(
        "    Cache Write: {cache_write}",
        cache_write = response.tokens_used.cache_write
    );
    tracing::info!("    Total: {total}", total = response.tokens_used.total());

    let actual_cost = calculate_cost(&response.tokens_used);
    tracing::info!("  Cost: ${actual_cost:.4}");
}

/// Handle the query command by building context and sending to provider.
///
/// # Errors
/// Returns an error if configuration loading, context building, or provider request fails.
async fn handle_query(
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
/// Handle the prompt command - show relevant files without sending to LLM.
///
/// # Errors
/// Returns an error if context building or terminal IO fails.
async fn handle_prompt(
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
fn handle_config(full: bool) -> Result<()> {
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
fn handle_metrics(_daily: bool) {
    tracing::info!("Metrics tracking not yet implemented in MVP.");
    tracing::info!("This will be added in Phase 5 (Advanced Optimizations).");
}
/// Handle interactive agent session with multi-model routing
/// Handle interactive agent session with multi-model routing
///
/// # Errors
/// Returns an error if TUI or IO operations fail, or if the orchestrator returns an error.
// Group the flags into a single struct to avoid excessive bool parameters
struct InteractiveFlags {
    validation: Validation,
    ui: UiMode,
    local_only: bool,
}

/// Handle interactive agent mode
///
/// # Errors
/// Returns an error if the orchestrator fails to initialize or process requests
async fn handle_interactive_agent(project: PathBuf, flags: InteractiveFlags) -> Result<()> {
    // Create routing configuration
    let mut config = RoutingConfig::default();
    config.validation.enabled = !matches!(flags.validation, Validation::Disabled);
    config.workspace.root_path.clone_from(&project);

    if flags.local_only {
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;
    }

    let orchestrator = RoutingOrchestrator::new(config);

    if matches!(flags.ui, UiMode::Tui) {
        // TUI mode (DEFAULT) - fully self-contained
        run_tui_interactive(
            orchestrator,
            project,
            flags.local_only,
            matches!(flags.ui, UiMode::PlainVerbose),
        )
        .await?;
    } else {
        // Plain console mode
        let term = Term::stdout();

        term.write_line(&format!(
            "{}",
            style("=== Merlin - Interactive AI Coding Assistant ===")
                .cyan()
                .bold()
        ))?;
        term.write_line(&format!("Project: {}", project.display()))?;
        term.write_line(&format!(
            "Mode: {}",
            if flags.local_only {
                "Local Only"
            } else {
                "Multi-Model Routing"
            }
        ))?;
        term.write_line("")?;
        term.write_line("\u{2713} Agent ready!")?;
        term.write_line("")?;
        term.write_line("Type your request (or 'exit' to quit):")?;
        term.write_line("")?;

        loop {
            term.write_line("You:")?;

            let input = Input::<String>::new().with_prompt(">").interact_text()?;

            let trimmed = input.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
                term.write_line("Goodbye!")?;
                break;
            }

            term.write_line("")?;

            match orchestrator.process_request(trimmed).await {
                Ok(results) => {
                    print_results_plain(&term, &results, matches!(flags.ui, UiMode::PlainVerbose))?;
                }
                Err(error) => {
                    term.write_line(&format!("Error: {error}"))?;
                    term.write_line("")?;
                }
            }
        }
    }

    Ok(())
}

/// Clean up old task files to prevent disk space waste
///
/// # Errors
/// Returns an error if the tasks directory cannot be read.
fn cleanup_old_tasks(merlin_dir: &Path) -> Result<()> {
    let tasks_dir = merlin_dir.join("tasks");
    if !tasks_dir.exists() {
        return Ok(());
    }

    // Get all task files sorted by modification time
    let mut task_files: Vec<_> = fs::read_dir(&tasks_dir)?
        .filter_map(StdResult::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "gz")
        })
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            meta.modified().ok().map(|time| (entry.path(), time))
        })
        .collect();

    // Sort by modification time (newest first)
    task_files.sort_by(|left, right| right.1.cmp(&left.1));

    // Keep only the 50 most recent, delete the rest
    for (path, _) in task_files.iter().skip(MAX_TASKS) {
        if let Err(error) = fs::remove_file(path) {
            tracing::warn!("failed to remove old task file {:?}: {}", path, error);
        }
    }

    Ok(())
}

/// Initialize logging for TUI session
///
/// # Errors
/// Returns error if file operations fail
fn init_tui_logging(merlin_dir: &Path, project: &Path, local_only: bool) -> Result<fs::File> {
    let debug_log = merlin_dir.join("debug.log");
    if debug_log.exists() {
        fs::remove_file(&debug_log)?;
    }
    let mut log_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&debug_log)?;

    writeln!(
        log_file,
        "=== Session started at {:?} ===",
        SystemTime::now()
    )?;
    writeln!(log_file, "Project: {}", project.display())?;
    writeln!(
        log_file,
        "Mode: {}",
        if local_only {
            "Local Only"
        } else {
            "Multi-Model"
        }
    )?;
    Ok(log_file)
}

/// Execute a task from user input and handle the result
async fn execute_user_task(
    orchestrator: RoutingOrchestrator,
    ui_channel: UiChannel,
    mut log_file: fs::File,
    user_input: String,
    parent_task_id: Option<TaskId>,
) {
    let task = Task::new(user_input.clone());
    let task_id = task.id;

    if let Err(error) = writeln!(log_file, "Created task: {user_input}") {
        let () = ui_channel.send(UiEvent::SystemMessage {
            level: MessageLevel::Warning,
            message: format!("Failed to write to log: {error}"),
        });
    }

    ui_channel.task_started_with_parent(task_id, user_input.clone(), parent_task_id);

    ui_channel.send(UiEvent::TaskOutput {
        task_id,
        output: format!("Prompt: {user_input}\n"),
    });

    match orchestrator
        .execute_task_streaming(task, ui_channel.clone())
        .await
    {
        Ok(result) => {
            ui_channel.completed(result.task_id, result.clone());
            try_write_log(
                &ui_channel,
                &mut log_file,
                &format!("Response: {}", result.response.text),
            );
            try_write_log(
                &ui_channel,
                &mut log_file,
                &format!(
                    "Tier: {} | Duration: {}ms | Tokens: {}",
                    result.tier_used,
                    result.duration_ms,
                    result.response.tokens_used.total()
                ),
            );
        }
        Err(error) => {
            try_write_log(&ui_channel, &mut log_file, &format!("Error: {error}"));
            ui_channel.send(UiEvent::SystemMessage {
                level: MessageLevel::Error,
                message: format!("Error: {error}"),
            });
            ui_channel.failed(task_id, error.to_string());
        }
    }
}

/// Run fully self-contained TUI interactive session
///
/// # Errors
/// Returns an error if filesystem, TUI, or async operations fail.
async fn run_tui_interactive(
    orchestrator: RoutingOrchestrator,
    project: PathBuf,
    local_only: bool,
    _verbose: bool,
) -> Result<()> {
    // Create .merlin directory for logs and task storage
    let merlin_dir = project.join(".merlin");
    fs::create_dir_all(&merlin_dir)?;

    let mut log_file = init_tui_logging(&merlin_dir, &project, local_only)?;

    // Clean up old task files (keep last 50 tasks)
    cleanup_old_tasks(&merlin_dir)?;

    // Create TUI with task storage
    let tasks_dir = merlin_dir.join("tasks");
    fs::create_dir_all(&tasks_dir)?;
    let (mut tui_app, ui_channel) = TuiApp::new_with_storage(tasks_dir)?;

    // Enable raw mode before loading
    tui_app.enable_raw_mode()?;

    // Load tasks in background
    tui_app.load_tasks_async().await;

    // Main event loop - event-driven
    loop {
        // Tick the TUI (handles rendering and input)
        let should_quit = tui_app.tick()?;
        if should_quit {
            break;
        }

        // Check if user submitted input
        if let Some(user_input) = tui_app.take_pending_input() {
            writeln!(log_file, "User: {user_input}")?;

            let parent_task_id = tui_app.get_selected_task_id();
            let log_clone = log_file.try_clone()?;

            spawn(execute_user_task(
                orchestrator.clone(),
                ui_channel.clone(),
                log_clone,
                user_input,
                parent_task_id,
            ));
        }
    }

    // Disable raw mode and clean up
    tui_app.disable_raw_mode()?;
    writeln!(log_file, "=== Session ended ===")?;

    Ok(())
}

/// Calculate estimated cost based on token usage.
fn calculate_cost(usage: &TokenUsage) -> f64 {
    const INPUT_COST: f64 = 3.0 / 1_000_000.0;
    const OUTPUT_COST: f64 = 15.0 / 1_000_000.0;
    const CACHE_READ_COST: f64 = 0.3 / 1_000_000.0;
    const CACHE_WRITE_COST: f64 = 3.75 / 1_000_000.0;

    (usage.cache_write as f64).mul_add(
        CACHE_WRITE_COST,
        (usage.cache_read as f64).mul_add(
            CACHE_READ_COST,
            (usage.output as f64).mul_add(OUTPUT_COST, usage.input as f64 * INPUT_COST),
        ),
    )
}

/// Print results in plain console mode and optionally metrics when verbose is true.
///
/// # Errors
/// Returns an error if terminal write fails.
fn print_results_plain(term: &Term, results: &[TaskResult], verbose: bool) -> Result<()> {
    term.write_line("Merlin:")?;
    term.write_line("")?;

    for result in results {
        term.write_line(&result.response.text)?;
        term.write_line("")?;

        if verbose {
            term.write_line(&format!(
                "Tier: {} | Duration: {}ms | Tokens: {}",
                result.tier_used,
                result.duration_ms,
                result.response.tokens_used.total()
            ))?;
        }
    }
    Ok(())
}

/// Write to the log file; if it fails, emit a UI warning.
fn try_write_log(ui: &UiChannel, writer: &mut fs::File, message: &str) {
    if let Err(error) = writeln!(writer, "{message}") {
        let () = ui.send(UiEvent::SystemMessage {
            level: MessageLevel::Warning,
            message: format!("Failed to write to log: {error}"),
        });
    }
}

/// Print the chat header for interactive chat mode.
///
/// # Errors
/// Returns an error if terminal write fails.
fn print_chat_header(term: &Term, project: &Path) -> Result<()> {
    term.write_line(&format!(
        "{}",
        style("=== Agentic Optimizer - Interactive Chat ===")
            .cyan()
            .bold()
    ))?;
    term.write_line(&format!(
        "{} {}",
        style("Project:").cyan(),
        style(project.display()).yellow()
    ))?;
    term.write_line("")?;
    Ok(())
}

/// Chat interaction loop for interactive chat mode.
///
/// # Errors
/// Returns an error if terminal IO or agent execution fails.
async fn chat_loop(term: &Term, executor: &mut AgentExecutor, project: &Path) -> Result<()> {
    loop {
        term.write_line(&format!("{}", style("You:").green().bold()))?;

        let input = Input::<String>::new().with_prompt(">").interact_text()?;

        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
            term.write_line(&format!("{}", style("Goodbye!").cyan()))?;
            break;
        }

        term.write_line("")?;
        term.write_line(&format!("{}", style("Agent:").blue().bold()))?;

        let request = AgentRequest::new(trimmed.to_owned(), project.to_path_buf());

        match executor.execute(request).await {
            Ok(result) => {
                term.write_line(&result.response.content)?;
                term.write_line("")?;

                term.write_line(&format!("{}", style("---").dim()))?;
                term.write_line(&format!(
                    "{} {} | {} {}ms | {} {} tokens",
                    style("Provider:").dim(),
                    style(&result.response.provider_used).dim(),
                    style("Latency:").dim(),
                    style(result.metadata.total_time_ms).dim(),
                    style("Tokens:").dim(),
                    style(result.response.tokens_used.total()).dim()
                ))?;

                if result.response.tokens_used.cache_read > 0 {
                    term.write_line(&format!(
                        "{} {} tokens ({}% cache hit)",
                        style("Cache:").dim(),
                        style(result.response.tokens_used.cache_read).dim(),
                        style(format!(
                            "{:.1}",
                            (result.response.tokens_used.cache_read as f64
                                / result.response.tokens_used.total() as f64)
                                * 100.0
                        ))
                        .dim()
                    ))?;
                }

                term.write_line(&format!("{}", style("---").dim()))?;
                term.write_line("")?;
            }
            Err(error) => {
                term.write_line(&format!(
                    "{} {}",
                    style("Error:").red().bold(),
                    style(error.to_string()).red()
                ))?;
                term.write_line("")?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread::sleep;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_cost_basic() {
        const EXPECTED_COST: f64 = 0.0105;
        const TOLERANCE: f64 = 0.0001;

        let usage = TokenUsage {
            input: 1000,
            output: 500,
            cache_read: 0,
            cache_write: 0,
        };
        let cost = calculate_cost(&usage);
        assert!(
            (cost - EXPECTED_COST).abs() < TOLERANCE,
            "Expected cost ~{EXPECTED_COST}, got {cost}"
        );
    }

    #[test]
    fn test_calculate_cost_with_cache() {
        const EXPECTED_COST: f64 = 0.01485;
        const TOLERANCE: f64 = 0.00001;

        let usage = TokenUsage {
            input: 1000,
            output: 500,
            cache_read: 2000,
            cache_write: 1000,
        };
        let cost = calculate_cost(&usage);
        assert!(
            (cost - EXPECTED_COST).abs() < TOLERANCE,
            "Expected cost with cache ~{EXPECTED_COST}, got {cost}"
        );
    }

    #[test]
    fn test_calculate_cost_zero_tokens() {
        const TOLERANCE: f64 = 0.0001;

        let usage = TokenUsage {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
        };
        let cost = calculate_cost(&usage);
        assert!(
            cost.abs() < TOLERANCE,
            "Zero tokens should result in zero cost, got {cost}"
        );
    }

    #[test]
    fn test_calculate_cost_large_values() {
        const LARGE_INPUT: u64 = 1_000_000;
        const LARGE_OUTPUT: u64 = 500_000;
        const EXPECTED_COST: f64 = 10.5;
        const TOLERANCE: f64 = 0.01;

        let usage = TokenUsage {
            input: LARGE_INPUT,
            output: LARGE_OUTPUT,
            cache_read: 0,
            cache_write: 0,
        };
        let cost = calculate_cost(&usage);
        assert!(
            (cost - EXPECTED_COST).abs() < TOLERANCE,
            "Expected large cost ~{EXPECTED_COST}, got {cost}"
        );
    }

    #[test]
    fn test_cleanup_old_tasks_no_directory() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let merlin_dir = temp.path().join(".merlin");
        let result = cleanup_old_tasks(&merlin_dir);
        assert!(
            result.is_ok(),
            "Cleanup should succeed when directory doesn't exist"
        );
    }

    #[test]
    fn test_cleanup_old_tasks_under_limit() {
        const NUM_TASKS: usize = 5;
        let temp = TempDir::new().expect("Failed to create temp dir");
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        fs::create_dir_all(&tasks_dir).expect("Failed to create tasks dir");

        for task_num in 0..NUM_TASKS {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            fs::write(&task_file, b"test").expect("Failed to write task file");
        }

        let result = cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let remaining = fs::read_dir(&tasks_dir)
            .expect("Failed to read tasks dir")
            .count();
        assert_eq!(
            remaining, NUM_TASKS,
            "All tasks should remain when under limit"
        );
    }

    #[test]
    fn test_cleanup_old_tasks_over_limit() {
        const OVER_LIMIT: usize = MAX_TASKS + 10;

        let temp = TempDir::new().expect("Failed to create temp dir");
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        fs::create_dir_all(&tasks_dir).expect("Failed to create tasks dir");

        for task_num in 0..OVER_LIMIT {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            fs::write(&task_file, b"test").expect("Failed to write task file");
            sleep(Duration::from_millis(10));
        }

        let result = cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let remaining = fs::read_dir(&tasks_dir)
            .expect("Failed to read tasks dir")
            .count();
        assert_eq!(remaining, MAX_TASKS, "Should keep exactly MAX_TASKS tasks");
    }

    #[test]
    fn test_cleanup_old_tasks_ignores_non_gz() {
        const NUM_GZ_TASKS: usize = 3;
        const NUM_OTHER_FILES: usize = 2;
        const EXPECTED_TOTAL: usize = NUM_GZ_TASKS + NUM_OTHER_FILES;

        let temp = TempDir::new().expect("Failed to create temp dir");
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        fs::create_dir_all(&tasks_dir).expect("Failed to create tasks dir");

        for task_num in 0..NUM_GZ_TASKS {
            let task_file = tasks_dir.join(format!("task_{task_num}.gz"));
            fs::write(&task_file, b"test").expect("Failed to write task file");
        }

        for other_num in 0..NUM_OTHER_FILES {
            let other_file = tasks_dir.join(format!("other_{other_num}.txt"));
            fs::write(&other_file, b"not gz").expect("Failed to write other file");
        }

        let result = cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let total_files = fs::read_dir(&tasks_dir)
            .expect("Failed to read tasks dir")
            .count();
        assert_eq!(total_files, EXPECTED_TOTAL, "Should preserve non-gz files");
    }

    #[test]
    fn test_init_tui_logging_creates_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let merlin_dir = temp.path().join(".merlin");
        let project_dir = temp.path().join("project");
        fs::create_dir_all(&merlin_dir).expect("Failed to create .merlin dir");
        fs::create_dir_all(&project_dir).expect("Failed to create project dir");

        let result = init_tui_logging(&merlin_dir, &project_dir, false);
        assert!(result.is_ok(), "init_tui_logging should succeed");

        let log_file = merlin_dir.join("debug.log");
        assert!(log_file.exists(), "Log file should be created");
    }

    #[test]
    fn test_init_tui_logging_local_mode() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let merlin_dir = temp.path().join(".merlin");
        let project_dir = temp.path().join("project");
        fs::create_dir_all(&merlin_dir).expect("Failed to create .merlin dir");
        fs::create_dir_all(&project_dir).expect("Failed to create project dir");

        let result = init_tui_logging(&merlin_dir, &project_dir, true);
        assert!(
            result.is_ok(),
            "init_tui_logging should succeed in local mode"
        );

        let log_file = merlin_dir.join("debug.log");
        assert!(
            log_file.exists(),
            "Log file should be created in local mode"
        );
    }

    #[test]
    fn test_init_tui_logging_removes_old_log() {
        const INITIAL_CONTENT: &str = "old content\n";

        let temp = TempDir::new().expect("Failed to create temp dir");
        let merlin_dir = temp.path().join(".merlin");
        let project_dir = temp.path().join("project");
        fs::create_dir_all(&merlin_dir).expect("Failed to create .merlin dir");
        fs::create_dir_all(&project_dir).expect("Failed to create project dir");

        let log_file = merlin_dir.join("debug.log");
        fs::write(&log_file, INITIAL_CONTENT).expect("Failed to write initial content");

        let result = init_tui_logging(&merlin_dir, &project_dir, false);
        assert!(result.is_ok(), "init_tui_logging should succeed");

        let content = fs::read_to_string(&log_file).expect("Failed to read log file");
        assert!(
            !content.contains(INITIAL_CONTENT),
            "Old content should be removed"
        );
    }

    #[test]
    fn test_token_usage_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.input, 0, "Default input should be 0");
        assert_eq!(usage.output, 0, "Default output should be 0");
        assert_eq!(usage.cache_read, 0, "Default cache_read should be 0");
        assert_eq!(usage.cache_write, 0, "Default cache_write should be 0");
    }

    #[test]
    fn test_display_response_metrics() {
        let response = Response {
            text: "Test response".to_owned(),
            confidence: 0.95,
            tokens_used: TokenUsage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
            },
            provider: "test-provider".to_owned(),
            latency_ms: 250,
        };

        display_response_metrics(&response);
    }

    #[test]
    fn test_print_chat_header() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let term = Term::stdout();

        let result = print_chat_header(&term, temp.path());
        assert!(result.is_ok(), "print_chat_header should succeed");
    }

    #[test]
    fn test_cleanup_old_tasks_with_mixed_extensions() {
        const NUM_GZ_FILES: usize = 30;
        const NUM_JSON_FILES: usize = 15;

        let temp = TempDir::new().expect("Failed to create temp dir");
        let tasks_dir = temp.path().join(".merlin").join("tasks");
        fs::create_dir_all(&tasks_dir).expect("Failed to create tasks dir");

        for file_num in 0..NUM_GZ_FILES {
            let file_path = tasks_dir.join(format!("task_{file_num}.gz"));
            fs::write(&file_path, b"gz data").expect("Failed to write gz file");
            sleep(Duration::from_millis(5));
        }

        for file_num in 0..NUM_JSON_FILES {
            let file_path = tasks_dir.join(format!("data_{file_num}.json"));
            fs::write(&file_path, b"{}").expect("Failed to write json file");
        }

        let result = cleanup_old_tasks(temp.path().join(".merlin").as_path());
        assert!(result.is_ok(), "Cleanup should succeed");

        let gz_count = fs::read_dir(&tasks_dir)
            .expect("Failed to read dir")
            .filter_map(StdResult::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "gz")
            })
            .count();

        assert_eq!(
            gz_count, NUM_GZ_FILES,
            "Should keep all gz files under limit"
        );

        let json_count = fs::read_dir(&tasks_dir)
            .expect("Failed to read dir")
            .filter_map(StdResult::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "json")
            })
            .count();

        assert_eq!(json_count, NUM_JSON_FILES, "Should preserve all json files");
    }
}
