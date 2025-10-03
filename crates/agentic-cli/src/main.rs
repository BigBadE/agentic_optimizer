use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
use console::{style, Term};

use agentic_context::ContextBuilder;
use agentic_core::{ModelProvider as _, Query, TokenUsage};
use agentic_providers::OpenRouterProvider;
use agentic_languages::{Language, create_backend};
use agentic_agent::{Agent, AgentConfig, AgentRequest};

mod cli;
mod config;
mod ollama;

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
        Commands::Chat {
            project,
            model,
        } => {
            handle_chat(project, model).await?;
        }
        Commands::Query {
            query,
            project,
            files,
            max_files,
        } => {
            handle_query(query, project, files, max_files).await?;
        }
        Commands::Prompt {
            query,
            project,
            files,
            max_files,
        } => {
            handle_prompt(query, project, files, max_files).await?;
        }
        Commands::Config { full } => {
            handle_config(full)?;
        }
        Commands::Metrics { daily } => {
            handle_metrics(daily);
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
    let provider: std::sync::Arc<dyn agentic_core::ModelProvider> = std::sync::Arc::new(provider);

    let backend = create_backend(Language::Rust)?;
    
    ollama::ensure_available().await?;

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
    term.write_line(&format!("{}", style("Enabling Rust semantic analysis...").cyan()))?;
    let backend = create_backend(Language::Rust)?;
    builder = builder.with_language_backend(backend);

    // Ensure Ollama is available for intelligent context fetching (required)
    term.write_line(&format!("{}", style("Checking Ollama availability...").cyan()))?;
    ollama::ensure_available().await?;

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
