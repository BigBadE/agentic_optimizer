use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
use console::{style, Term};

use agentic_context::ContextBuilder;
use agentic_core::{ModelProvider as _, Query, TokenUsage};
use agentic_providers::AnthropicProvider;
use agentic_languages::{Language, create_backend};

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

async fn handle_query(
    query_text: String,
    project: PathBuf,
    files: Vec<PathBuf>,
    max_files: Option<usize>,
) -> Result<()> {
    tracing::info!("Processing query: {}", query_text);

    let _config = Config::from_env();

    let provider = AnthropicProvider::from_env()?;

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
    term.write_line(&style("=".repeat(80)).dim().to_string())?;
    term.write_line(&format!("{}", style("RELEVANT FILES FOR PROMPT").green().bold()))?;
    term.write_line(&style("=".repeat(80)).dim().to_string())?;
    term.write_line("")?;
    
    for (index, file) in context.files.iter().enumerate() {
        term.write_line(&format!("{}. {}", 
            style(format!("{:3}", index + 1)).cyan(),
            style(file.path.display()).white()
        ))?;
    }
    
    term.write_line("")?;
    term.write_line(&style("=".repeat(80)).dim().to_string())?;
    term.write_line(&format!("{} {}", 
        style("Total files:").bold(),
        style(context.files.len()).yellow()
    ))?;
    term.write_line(&format!("{} {}", 
        style("Estimated tokens:").bold(),
        style(format!("~{}", context.token_estimate())).yellow()
    ))?;
    term.write_line(&style("=".repeat(80)).dim().to_string())?;
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
            "  Anthropic API Key: {status}",
            status = if config.providers.anthropic_api_key.is_some() { "Set" } else { "Not set" }
        );
        tracing::info!("  Max Files: {max}", max = config.context.max_files);
        tracing::info!("  Max File Size: {size} bytes", size = config.context.max_file_size);
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
