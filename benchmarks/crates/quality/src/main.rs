//! Quality benchmark CLI for context retrieval system.

use std::fs::write;
use std::path::PathBuf;
use std::process::exit;

use anyhow::{Context as _, Result};
use merlin_benchmarks_quality::{generate_report, run_benchmarks_async};
use pico_args::Arguments;
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt};

struct Args {
    test_cases: PathBuf,
    output: Option<PathBuf>,
    name: Option<String>,
    verbose: bool,
}

impl Args {
    /// Parses command-line arguments into the `Args` structure.
    ///
    /// # Errors
    /// Returns an error if required arguments are invalid or cannot be parsed.
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
    info!("quality-bench - Run context quality benchmarks");
    info!("");
    info!("USAGE:");
    info!("    quality-bench [OPTIONS]");
    info!("");
    info!("OPTIONS:");
    info!("    -t, --test-cases <PATH>      Directory containing test case TOML files");
    info!("                                 [default: benchmarks/crates/quality/test_cases]");
    info!("    -o, --output <PATH>          Output file for results (markdown format)");
    info!("    -n, --name <NAME>            Run specific test case by name");
    info!("    -v, --verbose                Show verbose output");
    info!("    -h, --help                   Print help information");
}

/// Main entry point for quality benchmark CLI.
///
/// # Errors
/// Returns an error if benchmark execution or report generation fails.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber to see debug logs
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args = Args::parse()?;

    info!("Running context quality benchmarks...");
    info!("Test cases directory: {}", args.test_cases.display());
    info!("");

    let results = run_benchmarks_async(&args.test_cases)
        .await
        .context("Failed to run benchmarks")?;

    if results.is_empty() {
        info!("No test cases found in {}", args.test_cases.display());
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
        info!("No test cases matched the filter");
        return Ok(());
    }

    let report = generate_report(&filtered_results);

    if let Some(output_path) = &args.output {
        write(output_path, &report)
            .with_context(|| format!("Failed to write report to {}", output_path.display()))?;
        info!("Report written to: {}", output_path.display());
    } else {
        info!("{report}");
    }

    if args.verbose {
        info!("\nDetailed Results:");
        for result in &filtered_results {
            info!("\n{}", "=".repeat(60));
            info!("Test: {}", result.name);
            info!("Query: {}", result.query);
            info!("Results count: {}", result.results.len());
            info!("Metrics:");
            info!("  P@3:  {:.1}%", result.metrics.precision_at_3);
            info!("  P@10: {:.1}%", result.metrics.precision_at_10);
            info!("  R@10: {:.1}%", result.metrics.recall_at_10);
            info!("  MRR:  {:.3}", result.metrics.mrr);
            info!("  NDCG: {:.3}", result.metrics.ndcg_at_10);
            info!("  Crit: {:.1}%", result.metrics.critical_in_top_3);
        }
    }

    Ok(())
}
