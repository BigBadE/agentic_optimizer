//! Quality benchmark CLI for context retrieval system.
#![allow(
    clippy::print_stdout,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    reason = "CLI binary uses stdout for output"
)]

use anyhow::{Context as _, Result};
use clap::Parser;
use merlin_benchmarks::{generate_report, run_benchmarks_async};
use std::fs::write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "quality-bench")]
#[command(about = "Run context quality benchmarks", long_about = None)]
struct Args {
    /// Directory containing test case TOML files
    #[arg(short, long, default_value = "benchmarks/test_cases")]
    test_cases: PathBuf,

    /// Output file for results (markdown format)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Run specific test case by name
    #[arg(short = 'n', long)]
    name: Option<String>,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Running context quality benchmarks...");
    println!("Test cases directory: {}", args.test_cases.display());
    println!();

    let results = run_benchmarks_async(&args.test_cases)
        .await
        .context("Failed to run benchmarks")?;

    if results.is_empty() {
        println!("No test cases found in {}", args.test_cases.display());
        return Ok(());
    }

    let filtered_results: Vec<_> = if let Some(name_filter) = &args.name {
        results
            .into_iter()
            .filter(|result| result.name.contains(name_filter))
            .collect()
    } else {
        results
    };

    if filtered_results.is_empty() {
        println!("No test cases matched the filter");
        return Ok(());
    }

    let report = generate_report(&filtered_results);

    if let Some(output_path) = &args.output {
        write(output_path, &report)
            .with_context(|| format!("Failed to write report to {}", output_path.display()))?;
        println!("Report written to: {}", output_path.display());
    } else {
        println!("{report}");
    }

    if args.verbose {
        println!("\nDetailed Results:");
        for result in &filtered_results {
            println!("\n{}", "=".repeat(60));
            println!("Test: {}", result.name);
            println!("Query: {}", result.query);
            println!("Results count: {}", result.results.len());
            println!("Metrics:");
            println!("  P@3:  {:.1}%", result.metrics.precision_at_3);
            println!("  P@10: {:.1}%", result.metrics.precision_at_10);
            println!("  R@10: {:.1}%", result.metrics.recall_at_10);
            println!("  MRR:  {:.3}", result.metrics.mrr);
            println!("  NDCG: {:.3}", result.metrics.ndcg_at_10);
            println!("  Crit: {:.1}%", result.metrics.critical_in_top_3);
        }
    }

    Ok(())
}
