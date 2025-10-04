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

    term.write_line(&format!("{}", style("✓ Agent ready!").green().bold()))?;
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
        style("✓ Context ready:").green().bold(),
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
    _no_tui: bool,
    local_only: bool,
) -> Result<()> {
    let term = Term::stdout();
    
    term.write_line(&format!("{}", style("=== Merlin - Interactive AI Coding Assistant ===").cyan().bold()))?;
    term.write_line(&format!("{} {}", 
        style("Project:").cyan(), 
        style(project.display()).yellow()
    ))?;
    term.write_line(&format!("{} {}", 
        style("Mode:").cyan(), 
        if local_only { style("Local Only").yellow() } else { style("Multi-Model Routing").yellow() }
    ))?;
    term.write_line("")?;

    // Create routing configuration
    let mut config = merlin_routing::RoutingConfig::default();
    config.validation.enabled = !no_validate;
    config.workspace.root_path = project.clone();
    
    if local_only {
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;
    }

    let orchestrator = merlin_routing::RoutingOrchestrator::new(config);

    term.write_line(&format!("{}", style("✓ Agent ready!").green().bold()))?;
    term.write_line("")?;
    term.write_line(&format!("{}", style("Type your request (or 'exit' to quit):").cyan()))?;
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

        // Execute request through routing system
        match orchestrator.process_request(trimmed).await {
            Ok(results) => {
                term.write_line(&format!("{}", style("Merlin:").blue().bold()))?;
                term.write_line("")?;
                
                for result in &results {
                    term.write_line(&result.response.text)?;
                    term.write_line("")?;
                    
                    if verbose {
                        term.write_line(&format!("{}", style("---").dim()))?;
                        term.write_line(&format!("{} {} | {} {}ms | {} {} tokens", 
                            style("Tier:").dim(),
                            style(&result.tier_used).dim(),
                            style("Duration:").dim(),
                            style(result.duration_ms).dim(),
                            style("Tokens:").dim(),
                            style(result.response.tokens_used.total()).dim()
                        ))?;
                        term.write_line(&format!("{}", style("---").dim()))?;
                    }
                }
                term.write_line("")?;
            }
            Err(e) => {
                term.write_line(&format!("{} {}", 
                    style("Error:").red().bold(),
                    style(e.to_string()).red()
                ))?;
                term.write_line("")?;
            }
        }
    }

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
        style("✓ Analysis complete:").green().bold(),
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
    term.write_line(&format!("{} {} task(s) in {:.2}s", 
        style("✓ Completed:").green().bold(),
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
                if result.validation.passed { "✓ PASSED" } else { "✗ FAILED" },
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
    term.write_line(&format!("  Total tokens: {}", total_tokens))?;
    term.write_line(&format!("  Total duration: {}ms", total_duration))?;
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
    let (tui_app, ui_channel) = TuiApp::new()?;
    
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
        let _ = ui_channel.send(UiEvent::SystemMessage {
            level: merlin_routing::MessageLevel::Info,
            message: format!("Analyzing request: {}", exec_request),
        });
        
        // Analyze request
        match orchestrator.analyze_request(&exec_request).await {
            Ok(analysis) => {
                let _ = ui_channel.send(UiEvent::SystemMessage {
                    level: merlin_routing::MessageLevel::Success,
                    message: format!("Generated {} task(s)", analysis.tasks.len()),
                });
                
                // Start tasks
                for task in &analysis.tasks {
                    let _ = ui_channel.send(UiEvent::TaskStarted {
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
                            let _ = ui_channel.send(UiEvent::TaskCompleted {
                                task_id,
                                result,
                            });
                        }
                        
                        let _ = ui_channel.send(UiEvent::SystemMessage {
                            level: merlin_routing::MessageLevel::Success,
                            message: "All tasks completed successfully!".to_string(),
                        });
                    }
                    Err(e) => {
                        let _ = ui_channel.send(UiEvent::SystemMessage {
                            level: merlin_routing::MessageLevel::Error,
                            message: format!("Execution failed: {}", e),
                        });
                    }
                }
            }
            Err(e) => {
                let _ = ui_channel.send(UiEvent::SystemMessage {
                    level: merlin_routing::MessageLevel::Error,
                    message: format!("Analysis failed: {}", e),
                });
            }
        }
    });
    
    // Run TUI
    tui_app.run().await?;
    
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

    // Chain mul_add for better numerical behavior and to satisfy clippy.
    (usage.cache_write as f64)
        .mul_add(
            CACHE_WRITE_COST,
            (usage.cache_read as f64)
                .mul_add(CACHE_READ_COST, (usage.output as f64).mul_add(OUTPUT_COST, usage.input as f64 * INPUT_COST)),
        )
}

