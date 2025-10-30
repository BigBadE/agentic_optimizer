//! Quality benchmark CLI for context retrieval system.
#![cfg_attr(
    test,
    allow(
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        reason = "Allow for tests"
    )
)]

use std::fs::write;
use std::path::PathBuf;
use std::process::exit;

use anyhow::{Context as _, Result};
use merlin_benchmarks_quality::{generate_report, run_benchmarks_async};
use pico_args::Arguments;
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt};

struct Args {
    test_cases: PathBuf,
    output: Option<PathBuf>,
    name: Option<String>,
    verbose: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut pargs = Arguments::from_env();

        if pargs.contains(["-h", "--help"]) {
            print_help();
            exit(0);
        }

        let args = Self {
            test_cases: pargs
                .opt_value_from_str(["-t", "--test-cases"])?
                .unwrap_or_else(|| PathBuf::from("benchmarks/crates/quality/test_cases")),
            output: pargs.opt_value_from_str(["-o", "--output"])?,
            name: pargs.opt_value_from_str(["-n", "--name"])?,
            verbose: pargs.contains(["-v", "--verbose"]),
        };

        let remaining = pargs.finish();
        if !remaining.is_empty() {
            tracing::warn!("Unexpected arguments: {remaining:?}");
        }

        Ok(args)
    }
}

fn print_help() {
    println!("quality-bench - Run context quality benchmarks");
    println!();
    println!("USAGE:");
    println!("    quality-bench [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -t, --test-cases <PATH>      Directory containing test case TOML files");
    println!("                                 [default: benchmarks/crates/quality/test_cases]");
    println!("    -o, --output <PATH>          Output file for results (markdown format)");
    println!("    -n, --name <NAME>            Run specific test case by name");
    println!("    -v, --verbose                Show verbose output");
    println!("    -h, --help                   Print help information");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber to see debug logs
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args = Args::parse()?;

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
