use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
use console::{style, Term};

use merlin_context::ContextBuilder;
use merlin_core::{ModelProvider as _, Query, TokenUsage};
use merlin_providers::OpenRouterProvider;
use merlin_languages::{Language, create_backend};
use merlin_agent::{Agent, AgentConfig, AgentRequest};

mod cli;
mod config;

use clap::Parser as _;
use cli::{Cli, Commands};
use config::Config;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agentic_optimizer=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
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
            handle_interactive_agent(
                cli.project,
                cli.no_validate,
                cli.verbose,
                cli.no_tui,
                cli.local,
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_chat(
    project: PathBuf,
    model: Option<String>,
) -> Result<()> {
    let term = Term::stdout();
    
    term.write_line(&format!("{}", style("=== Agentic Optimizer - Interactive Chat ===").cyan().bold()))?;
    term.write_line(&format!("{} {}", 
        style("Project:").cyan(), 
        style(project.display()).yellow()
    ))?;
    term.write_line("")?;

    let config = Config::load_from_project(&project);

    term.write_line(&format!("{}", style("Initializing agent...").cyan()))?;
    
    let mut provider = OpenRouterProvider::from_config_or_env(config.providers.openrouter_key)?;
    let model_to_use = model.or(config.providers.high_model.clone());
    if let Some(model_name) = model_to_use {
        provider = provider.with_model(model_name);
    }
    let provider: std::sync::Arc<dyn merlin_core::ModelProvider> = std::sync::Arc::new(provider);

    let backend = create_backend(Language::Rust)?;

    let config = AgentConfig::new()
        .with_system_prompt(
            "You are a helpful AI coding assistant. Analyze the provided code context and help the user with their requests. \
             Be concise but thorough. When making code changes, provide complete, working code."
        )
        .with_max_context_tokens(100_000)
        .with_top_k_context_files(15);

    let agent = Agent::with_config(provider, config);
    let mut executor = agent.executor().with_language_backend(backend);

    term.write_line(&format!("{}", style("\u{2713} Agent ready!").green().bold()))?;
    term.write_line("")?;
    term.write_line(&format!("{}", style("Type your message (or 'exit' to quit):").cyan()))?;
    term.write_line("")?;

    loop {
        term.write_line(&format!("{}", style("You:").green().bold()))?;
        
        let input = dialoguer::Input::<String>::new()
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
        
        let request = AgentRequest::new(trimmed.to_owned(), project.clone());
        
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
        Err(e) => {
            term.write_line(&format!("{} {}", 
                style("Error:").red().bold(),
                style(e.to_string()).red()
            ))?;
            return Err(e.into());
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
fn handle_config(full: bool) -> Result<()> {
    let config = Config::from_env();

    if full {
        let toml = toml::to_string_pretty(&config)?;
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
async fn handle_interactive_agent(
    project: PathBuf,
    no_validate: bool,
    verbose: bool,
    no_tui: bool,
    local_only: bool,
) -> Result<()> {
    // Create routing configuration
    let mut config = merlin_routing::RoutingConfig::default();
    config.validation.enabled = !no_validate;
    config.workspace.root_path = project.clone();
    
    if local_only {
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;
    }

    let orchestrator = merlin_routing::RoutingOrchestrator::new(config);

    if no_tui {
        // Plain console mode
        use console::{style, Term};
        let term = Term::stdout();

        term.write_line(&format!("{}", style("=== Merlin - Interactive AI Coding Assistant ===").cyan().bold()))?;
        term.write_line(&format!("Project: {}", project.display()))?;
        term.write_line(&format!("Mode: {}", if local_only { "Local Only" } else { "Multi-Model Routing" }))?;
        term.write_line("")?;
        term.write_line("\u{2713} Agent ready!")?;
        term.write_line("")?;
        term.write_line("Type your request (or 'exit' to quit):")?;
        term.write_line("")?;

        loop {
            term.write_line("You:")?;

            let input = dialoguer::Input::<String>::new()
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
                Ok(results) => {
                    term.write_line("Merlin:")?;
                    term.write_line("")?;

                    for result in &results {
                        term.write_line(&result.response.text)?;
                        term.write_line("")?;

                        if verbose {
                            term.write_line(&format!("Tier: {} | Duration: {}ms | Tokens: {}",
                                                     result.tier_used,
                                                     result.duration_ms,
                                                     result.response.tokens_used.total()
                            ))?;
                        }
                    }
                }
                Err(error) => {
                    term.write_line(&format!("Error: {error}"))?;
                    term.write_line("")?;
                }
            }
        }
    } else {
        // TUI mode (DEFAULT) - fully self-contained
        run_tui_interactive(orchestrator, project, local_only, verbose).await?;
    }

    Ok(())
}

/// Clean up old task files to prevent disk space waste
fn cleanup_old_tasks(merlin_dir: &std::path::Path) -> Result<()> {
    use std::fs;
    
    let tasks_dir = merlin_dir.join("tasks");
    if !tasks_dir.exists() {
        return Ok(());
    }
    
    // Get all task files sorted by modification time
    let mut task_files: Vec<_> = fs::read_dir(&tasks_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "gz")
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            entry.metadata().ok().and_then(|meta| {
                meta.modified().ok().map(|time| (entry.path(), time))
            })
        })
        .collect();
    
    // Sort by modification time (newest first)
    task_files.sort_by(|a, b| b.1.cmp(&a.1));
    
    // Keep only the 50 most recent, delete the rest
    const MAX_TASKS: usize = 50;
    for (path, _) in task_files.iter().skip(MAX_TASKS) {
        drop(fs::remove_file(path));
    }
    
    Ok(())
}

/// Run fully self-contained TUI interactive session
async fn run_tui_interactive(
    orchestrator: merlin_routing::RoutingOrchestrator,
    project: PathBuf,
    local_only: bool,
    verbose: bool,
) -> Result<()> {
    use merlin_routing::{TuiApp, UiEvent, MessageLevel};
    use std::fs;
    use std::io::Write as _;
    
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
        .open(&debug_log)?;
    
    writeln!(log_file, "=== Session started at {:?} ===", std::time::SystemTime::now())?;
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
        let should_quit = tui_app.tick().await?;
        if should_quit {
            break;
        }
        
        // Check if user submitted input
        if let Some(user_input) = tui_app.take_pending_input() {
            writeln!(log_file, "User: {user_input}")?;
            
            ui_channel.send(UiEvent::SystemMessage {
                level: MessageLevel::Info,
                message: format!("Processing: {user_input}"),
            });
            
            let orchestrator_clone = orchestrator.clone();
            let ui_channel_clone = ui_channel.clone();
            let verbose_clone = verbose;
            let mut log_clone = log_file.try_clone()?;
            // If selected task is a child, use its parent; otherwise use the selected task itself
            let parent_task_id = tui_app.get_selected_task_parent()
                .or_else(|| tui_app.get_selected_task_id());
            
            // Get thread context for conversation continuity
            let thread_context = tui_app.get_thread_context();
            let context_str = if !thread_context.is_empty() {
                let mut ctx = String::from("\n\n=== Previous Conversation Context ===\n");
                for (idx, (_task_id, description, output)) in thread_context.iter().enumerate() {
                    ctx.push_str(&format!("\n[Task {}] {}\n", idx + 1, description));
                    if !output.is_empty() {
                        ctx.push_str(&format!("Output:\n{}\n", output));
                    }
                }
                ctx.push_str("\n=== End Context ===\n\n");
                ctx
            } else {
                String::new()
            };
            
            let enhanced_input = if context_str.is_empty() {
                user_input.clone()
            } else {
                format!("{}{}", context_str, user_input)
            };
            
            tokio::spawn(async move {
                match orchestrator_clone.analyze_request(&enhanced_input).await {
                    Ok(analysis) => {
                        writeln!(log_clone, "Analysis: {} tasks", analysis.tasks.len()).ok();
                        
                        ui_channel_clone.send(UiEvent::SystemMessage {
                            level: MessageLevel::Success,
                            message: format!("Generated {} task(s)", analysis.tasks.len()),
                        });
                        
                        for task in &analysis.tasks {
                            ui_channel_clone.task_started_with_parent(task.id, task.description.clone(), parent_task_id);
                            // Send the prompt as the first output
                            let first_line = task.description.lines().next().unwrap_or(&task.description);
                            ui_channel_clone.output(task.id, first_line.to_string());
                        }
                        
                        match orchestrator_clone.execute_tasks(analysis.tasks).await {
                            Ok(results) => {
                                for result in &results {
                                    ui_channel_clone.completed(result.task_id, result.clone());
                                    
                                    writeln!(log_clone, "Response: {}", result.response.text).ok();
                                    writeln!(log_clone, "Tier: {} | Duration: {}ms | Tokens: {}",
                                        result.tier_used,
                                        result.duration_ms,
                                        result.response.tokens_used.total()
                                    ).ok();
                                    
                                    if verbose_clone {
                                        ui_channel_clone.send(UiEvent::SystemMessage {
                                            level: MessageLevel::Info,
                                            message: format!("[Tier: {} | {}ms | {} tokens]",
                                                result.tier_used,
                                                result.duration_ms,
                                                result.response.tokens_used.total()
                                            ),
                                        });
                                    }
                                }
                            }
                            Err(error) => {
                                writeln!(log_clone, "Error: {error}").ok();
                                ui_channel_clone.send(UiEvent::SystemMessage {
                                    level: MessageLevel::Error,
                                    message: format!("Error: {error}"),
                                });
                            }
                        }
                    }
                    Err(error) => {
                        writeln!(log_clone, "Analysis error: {error}").ok();
                        ui_channel_clone.send(UiEvent::SystemMessage {
                            level: MessageLevel::Error,
                            message: format!("Analysis failed: {error}"),
                        });
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

#[allow(dead_code)]
/// Handle the route command - use multi-model routing system
async fn handle_route(
    request: String,
    project: PathBuf,
    validate: bool,
    verbose: bool,
    no_tui: bool,
) -> Result<()> {
    // TUI is default, use plain mode only if --no-tui is specified
    if !no_tui {
        return handle_route_tui(request, project, validate).await;
    }
    
    let term = Term::stdout();
    
    term.write_line(&format!("{}", style("=== Multi-Model Routing ===").cyan().bold()))?;
    term.write_line(&format!("{} {}", 
        style("Request:").cyan(), 
        style(&request).yellow()
    ))?;
    term.write_line(&format!("{} {}", 
        style("Project:").cyan(), 
        style(project.display()).yellow()
    ))?;
    term.write_line("")?;

    // Create routing configuration
    let mut config = merlin_routing::RoutingConfig::default();
    config.validation.enabled = validate;
    config.workspace.root_path = project;
    
    if verbose {
        term.write_line(&format!("{}", style("Configuration:").cyan()))?;
        term.write_line(&format!("  Local enabled: {}", config.tiers.local_enabled))?;
        term.write_line(&format!("  Groq enabled: {}", config.tiers.groq_enabled))?;
        term.write_line(&format!("  Validation enabled: {}", config.validation.enabled))?;
        term.write_line(&format!("  Max concurrent: {}", config.execution.max_concurrent_tasks))?;
        term.write_line("")?;
    }

    // Create orchestrator
    term.write_line(&format!("{}", style("Initializing orchestrator...").cyan()))?;
    let orchestrator = merlin_routing::RoutingOrchestrator::new(config);
    
    // Analyze request
    term.write_line(&format!("{}", style("Analyzing request...").cyan()))?;
    let analysis = orchestrator.analyze_request(&request).await?;
    
    term.write_line(&format!("{} {}", 
        style("\u{2713} Analysis complete:").green().bold(),
        style(format!("{} task(s) generated", analysis.tasks.len())).yellow()
    ))?;
    term.write_line("")?;

    if verbose {
        term.write_line(&format!("{}", style("Tasks:").cyan()))?;
        for (i, task) in analysis.tasks.iter().enumerate() {
            term.write_line(&format!("  {}. {} (complexity: {:?}, priority: {:?})", 
                i + 1, 
                task.description, 
                task.complexity,
                task.priority
            ))?;
            if !task.dependencies.is_empty() {
                term.write_line(&format!("     Dependencies: {} task(s)", task.dependencies.len()))?;
            }
        }
        term.write_line("")?;
        term.write_line(&format!("{} {:?}", 
            style("Execution strategy:").cyan(),
            analysis.execution_strategy
        ))?;
        term.write_line("")?;
    }

    // Execute tasks
    term.write_line(&format!("{}", style("Executing tasks...").cyan()))?;
    let start = std::time::Instant::now();
    
    let results = orchestrator.process_request(&request).await?;
    
    let duration = start.elapsed();
    term.write_line(&format!("{} {} task(s) in {:.0}s",
        style("\u{2713} Completed:").green().bold(),
        style(results.len()).yellow(),
        duration.as_secs_f64()
    ))?;
    term.write_line("")?;

    // Show results
    term.write_line(&format!("{}", style("Results:").cyan().bold()))?;
    for (i, result) in results.iter().enumerate() {
        term.write_line(&format!("  {}. Task {:?}", i + 1, result.task_id))?;
        term.write_line(&format!("     Tier: {}", result.tier_used))?;
        term.write_line(&format!("     Duration: {}ms", result.duration_ms))?;
        term.write_line(&format!("     Tokens: {}", result.response.tokens_used.total()))?;
        
        if validate && verbose {
            term.write_line(&format!("     Validation: {} (score: {:.2})", 
                if result.validation.passed { "\u{2713} PASSED" } else { "\u{2717} FAILED" },
                result.validation.score
            ))?;
        }
        
        if verbose {
            term.write_line(&format!("     Response preview: {}", 
                result.response.text.chars().take(100).collect::<String>()
            ))?;
        }
        term.write_line("")?;
    }

    // Summary
    let total_tokens: u64 = results.iter().map(|r| r.response.tokens_used.total()).sum();
    let total_duration: u64 = results.iter().map(|r| r.duration_ms).sum();
    
    term.write_line(&format!("{}", style("Summary:").cyan().bold()))?;
    term.write_line(&format!("  Total tokens: {total_tokens}"))?;
    term.write_line(&format!("  Total duration: {total_duration}ms"))?;
    term.write_line(&format!("  Average per task: {}ms", total_duration / results.len() as u64))?;

    Ok(())
}

#[allow(dead_code)]
/// Handle route command with TUI mode
async fn handle_route_tui(
    request: String,
    project: PathBuf,
    validate: bool,
) -> Result<()> {
    use merlin_routing::{TuiApp, UiEvent};
    
    // Create TUI app and get channel
    let (mut tui_app, ui_channel) = TuiApp::new()?;
    
    // Create configuration
    let mut config = merlin_routing::RoutingConfig::default();
    config.validation.enabled = validate;
    config.workspace.root_path = project.clone();
    
    // Create orchestrator
    let orchestrator = merlin_routing::RoutingOrchestrator::new(config);
    
    // Spawn execution task
    let exec_request = request.clone();
    let exec_handle = tokio::spawn(async move {
        // Send initial message
        let () = ui_channel.send(UiEvent::SystemMessage {
            level: merlin_routing::MessageLevel::Info,
            message: format!("Analyzing request: {exec_request}"),
        });
        
        // Analyze request
        match orchestrator.analyze_request(&exec_request).await {
            Ok(analysis) => {
                let () = ui_channel.send(UiEvent::SystemMessage {
                    level: merlin_routing::MessageLevel::Success,
                    message: format!("Generated {} task(s)", analysis.tasks.len()),
                });
                
                // Start tasks
                for task in &analysis.tasks {
                    let () = ui_channel.send(UiEvent::TaskStarted {
                        task_id: task.id,
                        parent_id: None,
                        description: task.description.clone(),
                    });
                }
                
                // Execute tasks
                match orchestrator.process_request(&exec_request).await {
                    Ok(results) => {
                        for result in results {
                            let task_id = result.task_id;
                            let () = ui_channel.send(UiEvent::TaskCompleted {
                                task_id,
                                result,
                            });
                        }
                        
                        let () = ui_channel.send(UiEvent::SystemMessage {
                            level: merlin_routing::MessageLevel::Success,
                            message: "All tasks completed successfully!".to_owned(),
                        });
                    }
                    Err(e) => {
                        let () = ui_channel.send(UiEvent::SystemMessage {
                            level: merlin_routing::MessageLevel::Error,
                            message: format!("Execution failed: {e}"),
                        });
                    }
                }
            }
            Err(e) => {
                let () = ui_channel.send(UiEvent::SystemMessage {
                    level: merlin_routing::MessageLevel::Error,
                    message: format!("Analysis failed: {e}"),
                });
            }
        }
    });
    
    // Run TUI with tick loop
    tui_app.enable_raw_mode()?;
    
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(16));
    loop {
        interval.tick().await;
        let should_quit = tui_app.tick().await?;
        if should_quit {
            break;
        }
    }
    
    tui_app.disable_raw_mode()?;
    
    // Wait for execution to complete
    exec_handle.await?;
    
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

