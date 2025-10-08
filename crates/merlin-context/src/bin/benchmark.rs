//! Benchmark runner for context fetching evaluation.

use chrono::Local;
use clap::Parser;
use merlin_context::ContextBuilder;
use merlin_context::benchmark::{BenchmarkResult, RankedFile, TestCase, load_test_cases};
use merlin_core::{Error, Result};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;
use tracing::info;

#[derive(Parser)]
#[command(name = "benchmark")]
#[command(about = "Run context fetching benchmarks")]
struct Args {
    /// Project to benchmark (directory name in `benchmarks/test_cases`/)
    #[arg(short, long)]
    project: String,

    /// Specific test case to run (without .toml extension)
    #[arg(short, long)]
    test: Option<String>,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Generate markdown report to file
    #[arg(short, long)]
    report: Option<PathBuf>,

    /// Project root directory
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[tokio::main]
/// # Errors
/// Returns an error if loading test cases, running benchmarks, or writing reports fails.
///
/// # Panics
/// May panic if terminal output fails or in unexpected runtime conditions.
async fn main() -> Result<()> {
    let args = Args::parse();

    let test_cases_dir = PathBuf::from("benchmarks")
        .join("test_cases")
        .join(&args.project);

    if !test_cases_dir.exists() {
        tracing::error!(
            "Test cases directory not found: {}",
            test_cases_dir.display()
        );
        tracing::info!("Available projects:");
        if let Ok(entries) = fs::read_dir("benchmarks/test_cases") {
            for entry in entries.flatten().filter(|entry| entry.path().is_dir()) {
                tracing::info!("  - {}", entry.file_name().to_string_lossy());
            }
        }
        exit(1);
    }

    let test_cases = load_test_cases(&test_cases_dir)?;

    if test_cases.is_empty() {
        tracing::error!("No test cases found in {}", test_cases_dir.display());
        exit(1);
    }

    let filtered_cases: Vec<_> = if let Some(test_name) = &args.test {
        test_cases
            .into_iter()
            .filter(|(path, _)| {
                let Some(stem) = path.file_stem() else {
                    return false;
                };
                stem.to_str().is_some_and(|name| name == test_name)
            })
            .collect()
    } else {
        test_cases
    };

    if filtered_cases.is_empty() {
        tracing::error!("No matching test cases found");
        exit(1);
    }

    info!(
        "Running {} benchmark(s) for project: {}\n",
        filtered_cases.len(),
        args.project
    );

    let mut all_results = Vec::default();

    for (_test_path, test_case) in filtered_cases {
        info!("{banner}", banner = "\u{2550}".repeat(59));
        info!("Running: {}", test_case.name);
        info!("{banner}\n", banner = "\u{2550}".repeat(59));

        let result = run_benchmark(&args.root, &test_case, args.verbose).await?;

        info!("{}", result.format_report());
        info!("");

        all_results.push(result);
    }

    if all_results.len() > 1 {
        print_summary(&all_results);
    }

    if let Some(report_path) = args.report {
        generate_report(&all_results, &report_path)?;
        info!("Report saved to: {}", report_path.display());
    }

    Ok(())
}

/// Run a single benchmark and return the result.
///
/// # Errors
/// Returns an error if context building or searching fails.
async fn run_benchmark(
    default_root: &Path,
    test_case: &TestCase,
    verbose: bool,
) -> Result<BenchmarkResult> {
    if verbose {
        info!("Query: \"{}\"\n", test_case.query);
    }

    let project_root = test_case
        .project_root
        .as_ref()
        .map_or_else(|| default_root.to_path_buf(), PathBuf::from);

    if verbose {
        info!("Project root: {}\n", project_root.display());
    }

    let mut builder = ContextBuilder::new(project_root);

    let search_results = builder.search_context(&test_case.query).await?;

    if verbose {
        info!("Found {} search results\n", search_results.len());
    }

    let ranked_files: Vec<RankedFile> = search_results
        .iter()
        .enumerate()
        .map(|(index, result)| RankedFile {
            path: result.file_path.clone(),
            rank: index + 1,
            score: result.score,
        })
        .collect();

    Ok(BenchmarkResult::new(test_case.clone(), ranked_files))
}

fn print_summary(results: &[BenchmarkResult]) {
    info!("{banner}", banner = "\u{2550}".repeat(59));
    info!("SUMMARY");
    info!("{banner}\n", banner = "\u{2550}".repeat(59));

    let avg_precision_3 = results
        .iter()
        .map(|res| res.metrics.precision_at_3)
        .sum::<f32>()
        / results.len() as f32;
    let avg_precision_5 = results
        .iter()
        .map(|res| res.metrics.precision_at_5)
        .sum::<f32>()
        / results.len() as f32;
    let avg_precision_10 = results
        .iter()
        .map(|res| res.metrics.precision_at_10)
        .sum::<f32>()
        / results.len() as f32;
    let avg_recall_10 = results
        .iter()
        .map(|res| res.metrics.recall_at_10)
        .sum::<f32>()
        / results.len() as f32;
    let avg_mrr = results.iter().map(|res| res.metrics.mrr).sum::<f32>() / results.len() as f32;
    let avg_ndcg = results
        .iter()
        .map(|res| res.metrics.ndcg_at_10)
        .sum::<f32>()
        / results.len() as f32;
    let avg_exclusion = results
        .iter()
        .map(|res| res.metrics.exclusion_rate)
        .sum::<f32>()
        / results.len() as f32;
    let avg_critical = results
        .iter()
        .map(|res| res.metrics.critical_in_top_3)
        .sum::<f32>()
        / results.len() as f32;
    let avg_high = results
        .iter()
        .map(|res| res.metrics.high_in_top_5)
        .sum::<f32>()
        / results.len() as f32;

    info!("Average Metrics ({} test cases):", results.len());
    info!("  Precision@3:        {:.1}%", avg_precision_3 * 100.0);
    info!("  Precision@5:        {:.1}%", avg_precision_5 * 100.0);
    info!("  Precision@10:       {:.1}%", avg_precision_10 * 100.0);
    info!("  Recall@10:          {:.1}%", avg_recall_10 * 100.0);
    info!("  MRR:                {avg_mrr:.3}");
    info!("  NDCG@10:            {avg_ndcg:.3}");
    info!("  Exclusion Rate:     {:.1}%", avg_exclusion * 100.0);
    info!("  Critical in Top-3:  {:.1}%", avg_critical * 100.0);
    info!("  High in Top-5:      {:.1}%", avg_high * 100.0);
    info!("");
}

/// Generate a Markdown report summarizing benchmark results.
///
/// # Errors
/// Returns an error if writing the report fails.
#[allow(
    clippy::too_many_lines,
    reason = "Report generation requires comprehensive formatting"
)]
fn generate_report(results: &[BenchmarkResult], path: &PathBuf) -> Result<()> {
    let mut report = String::default();

    report.push_str("# Context Fetching Benchmark Report\n\n");
    writeln!(
        report,
        "**Date**: {}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    )
    .map_err(|error| Error::Other(error.to_string()))?;
    writeln!(report, "**Test Cases**: {}\n", results.len())
        .map_err(|error| Error::Other(error.to_string()))?;

    report.push_str("## Summary\n\n");

    let avg_precision_3 = results
        .iter()
        .map(|res| res.metrics.precision_at_3)
        .sum::<f32>()
        / results.len() as f32;
    let avg_precision_5 = results
        .iter()
        .map(|res| res.metrics.precision_at_5)
        .sum::<f32>()
        / results.len() as f32;
    let avg_precision_10 = results
        .iter()
        .map(|res| res.metrics.precision_at_10)
        .sum::<f32>()
        / results.len() as f32;
    let avg_recall_10 = results
        .iter()
        .map(|res| res.metrics.recall_at_10)
        .sum::<f32>()
        / results.len() as f32;
    let avg_mrr = results.iter().map(|res| res.metrics.mrr).sum::<f32>() / results.len() as f32;
    let avg_ndcg = results
        .iter()
        .map(|res| res.metrics.ndcg_at_10)
        .sum::<f32>()
        / results.len() as f32;
    let avg_exclusion = results
        .iter()
        .map(|res| res.metrics.exclusion_rate)
        .sum::<f32>()
        / results.len() as f32;
    let avg_critical = results
        .iter()
        .map(|res| res.metrics.critical_in_top_3)
        .sum::<f32>()
        / results.len() as f32;
    let avg_high = results
        .iter()
        .map(|res| res.metrics.high_in_top_5)
        .sum::<f32>()
        / results.len() as f32;

    report.push_str("| Metric | Value |\n");
    report.push_str("|--------|-------|\n");
    writeln!(report, "| Precision@3 | {:.1}% |", avg_precision_3 * 100.0)
        .map_err(|error| Error::Other(error.to_string()))?;
    writeln!(report, "| Precision@5 | {:.1}% |", avg_precision_5 * 100.0)
        .map_err(|error| Error::Other(error.to_string()))?;
    writeln!(
        report,
        "| Precision@10 | {:.1}% |",
        avg_precision_10 * 100.0
    )
    .map_err(|error| Error::Other(error.to_string()))?;
    writeln!(report, "| Recall@10 | {:.1}% |", avg_recall_10 * 100.0)
        .map_err(|error| Error::Other(error.to_string()))?;
    writeln!(report, "| MRR | {avg_mrr:.3} |").map_err(|error| Error::Other(error.to_string()))?;
    writeln!(report, "| NDCG@10 | {avg_ndcg:.3} |")
        .map_err(|error| Error::Other(error.to_string()))?;
    writeln!(report, "| Exclusion Rate | {:.1}% |", avg_exclusion * 100.0)
        .map_err(|error| Error::Other(error.to_string()))?;
    writeln!(
        report,
        "| Critical in Top-3 | {:.1}% |",
        avg_critical * 100.0
    )
    .map_err(|error| Error::Other(error.to_string()))?;
    writeln!(report, "| High in Top-5 | {:.1}% |\n", avg_high * 100.0)
        .map_err(|error| Error::Other(error.to_string()))?;

    report.push_str("## Individual Test Cases\n\n");

    for result in results {
        report.push_str("---\n\n");
        report.push_str(&result.format_report());
        report.push('\n');
    }

    fs::write(path, report)?;

    Ok(())
}
