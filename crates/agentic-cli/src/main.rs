use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

use agentic_context::ContextBuilder;
use agentic_core::{ModelProvider as _, Query, TokenUsage};
use agentic_providers::AnthropicProvider;
use agentic_languages::{Language, create_backend};

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
            handle_prompt(query, project, files, max_files)?;
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
    let context = builder.build_context(&query)?;
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
fn handle_prompt(
    query_text: String,
    project: PathBuf,
    files: Vec<PathBuf>,
    max_files: Option<usize>,
) -> Result<()> {
    tracing::info!("Analyzing prompt: {}", query_text);

    let mut builder = ContextBuilder::new(project);
    if let Some(max) = max_files {
        builder = builder.with_max_files(max);
    }

    // Always enable Rust semantic analysis
    tracing::info!("Enabling Rust semantic analysis...");
    let backend = create_backend(Language::Rust)?;
    builder = builder.with_language_backend(backend);

    let query = Query::new(query_text).with_files(files);

    tracing::info!("Building context...");
    let context = builder.build_context(&query)?;
    
    tracing::info!("\n{sep}\n", sep = "=".repeat(80));
    tracing::info!("RELEVANT FILES FOR PROMPT");
    tracing::info!("{sep}\n", sep = "=".repeat(80));
    
    for (index, file) in context.files.iter().enumerate() {
        tracing::info!("{}. {}", index + 1, file.path.display());
    }
    
    tracing::info!("\n{sep}", sep = "=".repeat(80));
    tracing::info!("Total files: {}", context.files.len());
    tracing::info!("Estimated tokens: ~{}", context.token_estimate());
    tracing::info!("{sep}\n", sep = "=".repeat(80));

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
