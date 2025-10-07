use anyhow::Result;
use console::{style, Term};
use merlin_agent::{Agent, AgentConfig, AgentRequest, AgentExecutor};
use merlin_context::ContextBuilder;
use merlin_core::{ModelProvider, Query, TokenUsage};
use merlin_languages::{create_backend, Language};
use merlin_providers::OpenRouterProvider;
use merlin_routing::{
    MessageLevel, RoutingConfig, RoutingOrchestrator, TaskResult, TuiApp, UiChannel, UiEvent, Task,
};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use tokio::spawn;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
    EnvFilter,
    Registry,
};
use dialoguer::Input;
use toml::to_string_pretty;

mod cli;
mod config;

use clap::Parser as _;
use cli::{Cli, Commands, UiMode, Validation};
use config::Config;

const MAX_TASKS: usize = 50;

#[tokio::main]
/// # Errors
/// Returns an error if initialization or command handling fails.
///
/// # Panics
/// May panic if tracing subscriber initialization fails unexpectedly.
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
async fn handle_chat(
    project: PathBuf,
    model: Option<String>,
) -> Result<()> {
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

    let agent_config = AgentConfig::new()
        .with_system_prompt(
            "You are a helpful AI coding assistant. Analyze the provided code context and help the user with their requests. \
             Be concise but thorough. When making code changes, provide complete, working code."
        )
        .with_max_context_tokens(100_000)
        .with_top_k_context_files(15);

    let agent = Agent::with_config(provider, agent_config);
    let mut executor = agent.executor().with_language_backend(backend);

    term.write_line(&format!("{}", style("\u{2713} Agent ready!").green().bold()))?;
    term.write_line("")?;
    term.write_line(&format!("{}", style("Type your message (or 'exit' to quit):").cyan()))?;
    term.write_line("")?;

    chat_loop(&term, &mut executor, &project).await
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

    let config = Config::load_from_project(&project);

    let mut provider = OpenRouterProvider::from_config_or_env(config.providers.openrouter_key)?;
    if let Some(model_name) = config.providers.high_model {
        provider = provider.with_model(model_name);
    }

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

    let estimated_cost = provider.estimate_cost(&context);
    tracing::info!("Estimated cost: ${:.4}", estimated_cost);

    tracing::info!("Sending request to {}...", provider.name());
    let response = provider.generate(&query, &context).await?;

    tracing::info!("\n{sep}\n", sep = "=".repeat(80));
    tracing::info!("{text}", text = response.text);
    tracing::info!("{sep}", sep = "=".repeat(80));

    tracing::info!("\nMetrics:");
    tracing::info!("  Provider: {provider}", provider = response.provider);
    tracing::info!("  Confidence: {confidence:.2}", confidence = response.confidence);
    tracing::info!("  Latency: {latency}ms", latency = response.latency_ms);
    tracing::info!("  Tokens:");
    tracing::info!("    Input: {input}", input = response.tokens_used.input);
    tracing::info!("    Output: {output}", output = response.tokens_used.output);
    tracing::info!("    Cache Read: {cache_read}", cache_read = response.tokens_used.cache_read);
    tracing::info!("    Cache Write: {cache_write}", cache_write = response.tokens_used.cache_write);
    tracing::info!("    Total: {total}", total = response.tokens_used.total());

    let actual_cost = calculate_cost(&response.tokens_used);
    tracing::info!("  Cost: ${actual_cost:.4}");

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
    
    term.write_line(&format!("{} {}", 
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
            term.write_line(&format!("{} {}", 
                style("Error:").red().bold(),
                style(error.to_string()).red()
            ))?;
            return Err(error.into());
        }
    };
    
    term.write_line("")?;
    term.write_line(&format!("{} {}", 
        style("\u{2713} Context ready:").green().bold(),
        style(format!("{} sections, ~{} tokens", context.files.len(), context.token_estimate())).yellow()
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
            status = if config.providers.openrouter_key.is_some() { "Set" } else { "Not set" }
        );
        tracing::info!(
            "  High Model: {model}",
            model = config.providers.high_model.as_deref().unwrap_or("default")
        );
        tracing::info!(
            "  Medium Model: {model}",
            model = config.providers.medium_model.as_deref().unwrap_or("default")
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

async fn handle_interactive_agent(
    project: PathBuf,
    flags: InteractiveFlags,
) -> Result<()> {
    // Create routing configuration
    let mut config = RoutingConfig::default();
    config.validation.enabled = !matches!(flags.validation, Validation::Disabled);
    config.workspace.root_path.clone_from(&project);
    
    if flags.local_only {
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;
    }

    let orchestrator = RoutingOrchestrator::new(config);

    if !matches!(flags.ui, UiMode::Tui) {
        // Plain console mode
        let term = Term::stdout();

        term.write_line(&format!("{}", style("=== Merlin - Interactive AI Coding Assistant ===").cyan().bold()))?;
        term.write_line(&format!("Project: {}", project.display()))?;
        term.write_line(&format!("Mode: {}", if flags.local_only { "Local Only" } else { "Multi-Model Routing" }))?;
        term.write_line("")?;
        term.write_line("\u{2713} Agent ready!")?;
        term.write_line("")?;
        term.write_line("Type your request (or 'exit' to quit):")?;
        term.write_line("")?;

        loop {
            term.write_line("You:")?;

            let input = Input::<String>::new()
                .with_prompt(">")
                .interact_text()?;

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
                Ok(results) => print_results_plain(&term, &results, matches!(flags.ui, UiMode::PlainVerbose))?,
                Err(error) => {
                    term.write_line(&format!("Error: {error}"))?;
                    term.write_line("")?;
                }
            }
        }
    } else {
        // TUI mode (DEFAULT) - fully self-contained
        run_tui_interactive(orchestrator, project, flags.local_only, matches!(flags.ui, UiMode::PlainVerbose)).await?;
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
            entry.path().extension()
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

/// Run fully self-contained TUI interactive session
///
/// # Errors
/// Returns an error if filesystem, TUI, or async operations fail.
async fn run_tui_interactive(
    orchestrator: RoutingOrchestrator,
    project: PathBuf,
    local_only: bool,
    verbose: bool,
) -> Result<()> {
    // Create .merlin directory for logs and task storage
    let merlin_dir = project.join(".merlin");
    fs::create_dir_all(&merlin_dir)?;
    
    // Clean up old debug logs (delete and recreate)
    let debug_log = merlin_dir.join("debug.log");
    if debug_log.exists() {
        fs::remove_file(&debug_log)?;
    }
    let mut log_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&debug_log)?;
    
    writeln!(log_file, "=== Session started at {:?} ===", SystemTime::now())?;
    writeln!(log_file, "Project: {}", project.display())?;
    writeln!(log_file, "Mode: {}", if local_only { "Local Only" } else { "Multi-Model" })?;
    
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
            
            let orchestrator_clone = orchestrator.clone();
            let ui_channel_clone = ui_channel.clone();
            let verbose_clone = verbose;
            let mut log_clone = log_file.try_clone()?;
            
            // Get selected task as parent for new task
            let parent_task_id = tui_app.get_selected_task_id();
            
            // TODO: Context gathering should be handled by self-determination (GATHER action)
            // For now, disable automatic context injection to prevent infinite loops
            
            spawn(async move {
                // Create a single task from user input - let self-determination handle decomposition
                // Use original user_input for task description, enhanced_input for execution
                let task = Task::new(user_input.clone());
                let task_id = task.id;
                
                if let Err(error) = writeln!(log_clone, "Created task: {user_input}") {
                    let () = ui_channel_clone.send(UiEvent::SystemMessage {
                        level: MessageLevel::Warning,
                        message: format!("Failed to write to log: {error}"),
                    });
                }
                
                // Notify UI of task start (parent is the selected task, if any)
                ui_channel_clone.task_started_with_parent(task_id, user_input.clone(), parent_task_id);
                
                // Add prompt header to output
                ui_channel_clone.send(UiEvent::TaskOutput {
                    task_id,
                    output: format!("Prompt: {user_input}\n"),
                });
                
                // Execute with self-determination (task will assess itself)
                match orchestrator_clone.execute_task_streaming(task, ui_channel_clone.clone()).await {
                    Ok(result) => {
                        ui_channel_clone.completed(result.task_id, result.clone());
                        
                        try_write_log(&ui_channel_clone, &mut log_clone, &format!("Response: {}", result.response.text));
                        try_write_log(&ui_channel_clone, &mut log_clone, &format!(
                            "Tier: {} | Duration: {}ms | Tokens: {}",
                            result.tier_used,
                            result.duration_ms,
                            result.response.tokens_used.total()
                        ));
                        
                        maybe_send_verbose(&ui_channel_clone, &result, verbose_clone);
                    }
                    Err(error) => {
                        try_write_log(&ui_channel_clone, &mut log_clone, &format!("Error: {error}"));
                        ui_channel_clone.send(UiEvent::SystemMessage {
                            level: MessageLevel::Error,
                            message: format!("Error: {error}"),
                        });
                        ui_channel_clone.failed(task_id, error.to_string());
                    }
                }
            });
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

    (usage.cache_write as f64)
        .mul_add(
            CACHE_WRITE_COST,
            (usage.cache_read as f64)
                .mul_add(CACHE_READ_COST, (usage.output as f64).mul_add(OUTPUT_COST, usage.input as f64 * INPUT_COST)),
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
    term.write_line(&format!("{}", style("=== Agentic Optimizer - Interactive Chat ===").cyan().bold()))?;
    term.write_line(&format!("{} {}", 
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
        
        let input = Input::<String>::new()
            .with_prompt(">")
            .interact_text()?;

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
                term.write_line(&format!("{} {} | {} {}ms | {} {} tokens", 
                    style("Provider:").dim(),
                    style(&result.response.provider_used).dim(),
                    style("Latency:").dim(),
                    style(result.metadata.total_time_ms).dim(),
                    style("Tokens:").dim(),
                    style(result.response.tokens_used.total()).dim()
                ))?;
                
                if result.response.tokens_used.cache_read > 0 {
                    term.write_line(&format!("{} {} tokens ({}% cache hit)", 
                        style("Cache:").dim(),
                        style(result.response.tokens_used.cache_read).dim(),
                        style(format!("{:.1}", 
                            (result.response.tokens_used.cache_read as f64 / 
                             result.response.tokens_used.total() as f64) * 100.0
                        )).dim()
                    ))?;
                }
                
                term.write_line(&format!("{}", style("---").dim()))?;
                term.write_line("")?;
            }
            Err(error) => {
                term.write_line(&format!("{} {}", 
                    style("Error:").red().bold(),
                    style(error.to_string()).red()
                ))?;
                term.write_line("")?;
            }
        }
    }
    Ok(())
}
