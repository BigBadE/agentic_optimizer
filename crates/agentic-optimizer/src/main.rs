use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agentic_optimizer::{
    cli::{Cli, Commands},
    config::Config,
    context::ContextBuilder,
    core::{ModelProvider, Query},
    providers::AnthropicProvider,
};

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
        Commands::Config { full } => {
            handle_config(full)?;
        }
        Commands::Metrics { daily } => {
            handle_metrics(daily)?;
        }
    }

    Ok(())
}

async fn handle_query(
    query_text: String,
    project: std::path::PathBuf,
    files: Vec<std::path::PathBuf>,
    max_files: Option<usize>,
) -> Result<()> {
    tracing::info!("Processing query: {}", query_text);

    let config = Config::from_env();

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

    println!("\n{}\n", "=".repeat(80));
    println!("{}", response.text);
    println!("{}", "=".repeat(80));

    println!("\nMetrics:");
    println!("  Provider: {}", response.provider);
    println!("  Confidence: {:.2}", response.confidence);
    println!("  Latency: {}ms", response.latency_ms);
    println!("  Tokens:");
    println!("    Input: {}", response.tokens_used.input);
    println!("    Output: {}", response.tokens_used.output);
    println!("    Cache Read: {}", response.tokens_used.cache_read);
    println!("    Cache Write: {}", response.tokens_used.cache_write);
    println!("    Total: {}", response.tokens_used.total());

    let actual_cost = calculate_cost(&response.tokens_used);
    println!("  Cost: ${:.4}", actual_cost);

    Ok(())
}

fn handle_config(full: bool) -> Result<()> {
    let config = Config::from_env();

    if full {
        let toml = toml::to_string_pretty(&config)?;
        println!("{}", toml);
    } else {
        println!("Configuration:");
        println!("  Anthropic API Key: {}", 
            if config.providers.anthropic_api_key.is_some() {
                "Set"
            } else {
                "Not set"
            }
        );
        println!("  Max Files: {}", config.context.max_files);
        println!("  Max File Size: {} bytes", config.context.max_file_size);
    }

    Ok(())
}

fn handle_metrics(_daily: bool) -> Result<()> {
    println!("Metrics tracking not yet implemented in MVP.");
    println!("This will be added in Phase 5 (Advanced Optimizations).");
    Ok(())
}

fn calculate_cost(usage: &agentic_optimizer::TokenUsage) -> f64 {
    const INPUT_COST: f64 = 3.0 / 1_000_000.0;
    const OUTPUT_COST: f64 = 15.0 / 1_000_000.0;
    const CACHE_READ_COST: f64 = 0.3 / 1_000_000.0;
    const CACHE_WRITE_COST: f64 = 3.75 / 1_000_000.0;

    (usage.input as f64 * INPUT_COST)
        + (usage.output as f64 * OUTPUT_COST)
        + (usage.cache_read as f64 * CACHE_READ_COST)
        + (usage.cache_write as f64 * CACHE_WRITE_COST)
}
