//! Benchmark runner for context fetching evaluation.

use std::path::PathBuf;
use clap::Parser;
use agentic_context::benchmark::{load_test_cases, BenchmarkResult, RankedFile};
use agentic_context::ContextBuilder;
use agentic_core::Result;

#[derive(Parser)]
#[command(name = "benchmark")]
#[command(about = "Run context fetching benchmarks")]
struct Args {
    /// Project to benchmark (directory name in benchmarks/test_cases/)
    #[arg(short, long, default_value = "valor")]
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
async fn main() -> Result<()> {
    let args = Args::parse();

    let test_cases_dir = PathBuf::from("benchmarks")
        .join("test_cases")
        .join(&args.project);

    if !test_cases_dir.exists() {
        eprintln!("Error: Test cases directory not found: {}", test_cases_dir.display());
        eprintln!("Available projects:");
        if let Ok(entries) = std::fs::read_dir("benchmarks/test_cases") {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    eprintln!("  - {}", entry.file_name().to_string_lossy());
                }
            }
        }
        std::process::exit(1);
    }

    let test_cases = load_test_cases(&test_cases_dir)?;

    if test_cases.is_empty() {
        eprintln!("No test cases found in {}", test_cases_dir.display());
        std::process::exit(1);
    }

    let filtered_cases: Vec<_> = if let Some(test_name) = &args.test {
        test_cases
            .into_iter()
            .filter(|(path, _)| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s == test_name)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        test_cases
    };

    if filtered_cases.is_empty() {
        eprintln!("No matching test cases found");
        std::process::exit(1);
    }

    println!("Running {} benchmark(s) for project: {}\n", filtered_cases.len(), args.project);

    let mut all_results = Vec::new();

    for (_test_path, test_case) in filtered_cases {
        println!("═══════════════════════════════════════════════════════════");
        println!("Running: {}", test_case.name);
        println!("═══════════════════════════════════════════════════════════\n");

        let result = run_benchmark(&args.root, &test_case, args.verbose).await?;

        println!("{}", result.format_report());
        println!();

        all_results.push(result);
    }

    if all_results.len() > 1 {
        print_summary(&all_results);
    }

    if let Some(report_path) = args.report {
        generate_report(&all_results, &report_path)?;
        println!("Report saved to: {}", report_path.display());
    }

    Ok(())
}

async fn run_benchmark(
    default_root: &PathBuf,
    test_case: &agentic_context::benchmark::TestCase,
    verbose: bool,
) -> Result<BenchmarkResult> {
    if verbose {
        println!("Query: \"{}\"\n", test_case.query);
    }

    let project_root = if let Some(ref custom_root) = test_case.project_root {
        PathBuf::from(custom_root)
    } else {
        default_root.clone()
    };

    if verbose {
        println!("Project root: {}\n", project_root.display());
    }

    let mut builder = ContextBuilder::new(project_root);

    let search_results = builder
        .search_context(&test_case.query)
        .await?;

    if verbose {
        println!("Found {} search results\n", search_results.len());
    }

    let ranked_files: Vec<RankedFile> = search_results
        .iter()
        .enumerate()
        .map(|(i, result)| RankedFile {
            path: result.file_path.clone(),
            rank: i + 1,
            score: result.score,
        })
        .collect();

    Ok(BenchmarkResult::new(test_case.clone(), ranked_files))
}

fn print_summary(results: &[BenchmarkResult]) {
    println!("═══════════════════════════════════════════════════════════");
    println!("SUMMARY");
    println!("═══════════════════════════════════════════════════════════\n");

    let avg_precision_3 = results.iter().map(|r| r.metrics.precision_at_3).sum::<f32>() / results.len() as f32;
    let avg_precision_5 = results.iter().map(|r| r.metrics.precision_at_5).sum::<f32>() / results.len() as f32;
    let avg_precision_10 = results.iter().map(|r| r.metrics.precision_at_10).sum::<f32>() / results.len() as f32;
    let avg_recall_10 = results.iter().map(|r| r.metrics.recall_at_10).sum::<f32>() / results.len() as f32;
    let avg_mrr = results.iter().map(|r| r.metrics.mrr).sum::<f32>() / results.len() as f32;
    let avg_ndcg = results.iter().map(|r| r.metrics.ndcg_at_10).sum::<f32>() / results.len() as f32;
    let avg_exclusion = results.iter().map(|r| r.metrics.exclusion_rate).sum::<f32>() / results.len() as f32;
    let avg_critical = results.iter().map(|r| r.metrics.critical_in_top_3).sum::<f32>() / results.len() as f32;
    let avg_high = results.iter().map(|r| r.metrics.high_in_top_5).sum::<f32>() / results.len() as f32;

    println!("Average Metrics ({} test cases):", results.len());
    println!("  Precision@3:        {:.1}%", avg_precision_3 * 100.0);
    println!("  Precision@5:        {:.1}%", avg_precision_5 * 100.0);
    println!("  Precision@10:       {:.1}%", avg_precision_10 * 100.0);
    println!("  Recall@10:          {:.1}%", avg_recall_10 * 100.0);
    println!("  MRR:                {:.3}", avg_mrr);
    println!("  NDCG@10:            {:.3}", avg_ndcg);
    println!("  Exclusion Rate:     {:.1}%", avg_exclusion * 100.0);
    println!("  Critical in Top-3:  {:.1}%", avg_critical * 100.0);
    println!("  High in Top-5:      {:.1}%", avg_high * 100.0);
    println!();
}

fn generate_report(results: &[BenchmarkResult], path: &PathBuf) -> Result<()> {
    let mut report = String::new();

    report.push_str("# Context Fetching Benchmark Report\n\n");
    report.push_str(&format!("**Date**: {}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    report.push_str(&format!("**Test Cases**: {}\n\n", results.len()));

    report.push_str("## Summary\n\n");

    let avg_precision_3 = results.iter().map(|r| r.metrics.precision_at_3).sum::<f32>() / results.len() as f32;
    let avg_precision_5 = results.iter().map(|r| r.metrics.precision_at_5).sum::<f32>() / results.len() as f32;
    let avg_precision_10 = results.iter().map(|r| r.metrics.precision_at_10).sum::<f32>() / results.len() as f32;
    let avg_recall_10 = results.iter().map(|r| r.metrics.recall_at_10).sum::<f32>() / results.len() as f32;
    let avg_mrr = results.iter().map(|r| r.metrics.mrr).sum::<f32>() / results.len() as f32;
    let avg_ndcg = results.iter().map(|r| r.metrics.ndcg_at_10).sum::<f32>() / results.len() as f32;
    let avg_exclusion = results.iter().map(|r| r.metrics.exclusion_rate).sum::<f32>() / results.len() as f32;
    let avg_critical = results.iter().map(|r| r.metrics.critical_in_top_3).sum::<f32>() / results.len() as f32;
    let avg_high = results.iter().map(|r| r.metrics.high_in_top_5).sum::<f32>() / results.len() as f32;

    report.push_str("| Metric | Value |\n");
    report.push_str("|--------|-------|\n");
    report.push_str(&format!("| Precision@3 | {:.1}% |\n", avg_precision_3 * 100.0));
    report.push_str(&format!("| Precision@5 | {:.1}% |\n", avg_precision_5 * 100.0));
    report.push_str(&format!("| Precision@10 | {:.1}% |\n", avg_precision_10 * 100.0));
    report.push_str(&format!("| Recall@10 | {:.1}% |\n", avg_recall_10 * 100.0));
    report.push_str(&format!("| MRR | {:.3} |\n", avg_mrr));
    report.push_str(&format!("| NDCG@10 | {:.3} |\n", avg_ndcg));
    report.push_str(&format!("| Exclusion Rate | {:.1}% |\n", avg_exclusion * 100.0));
    report.push_str(&format!("| Critical in Top-3 | {:.1}% |\n", avg_critical * 100.0));
    report.push_str(&format!("| High in Top-5 | {:.1}% |\n\n", avg_high * 100.0));

    report.push_str("## Individual Test Cases\n\n");

    for result in results {
        report.push_str("---\n\n");
        report.push_str(&result.format_report());
        report.push_str("\n");
    }

    std::fs::write(path, report)?;

    Ok(())
}
