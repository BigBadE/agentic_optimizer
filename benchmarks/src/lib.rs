//! Quality benchmarking for context retrieval system.

pub mod metrics;
pub mod test_case;

use anyhow::{Context as _, Result};
use metrics::{AggregateMetrics, BenchmarkMetrics};
use std::path::Path;
use test_case::TestCase;
use walkdir::WalkDir;

/// Run all benchmarks in a directory
///
/// # Errors
/// Returns error if test case files cannot be read or parsed
pub fn run_benchmarks(test_cases_dir: &Path) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();

    for entry in WalkDir::new(test_cases_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "toml")
        {
            let test_case = TestCase::from_file(entry.path())
                .with_context(|| format!("Failed to load test case: {}", entry.path().display()))?;

            let result = run_single_benchmark(&test_case);
            results.push(result);
        }
    }

    Ok(results)
}

/// Result of running a single benchmark
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Test case name
    pub name: String,
    /// Query used
    pub query: String,
    /// Retrieved results (file paths)
    pub results: Vec<String>,
    /// Calculated metrics
    pub metrics: BenchmarkMetrics,
}

/// Run a single benchmark test case
fn run_single_benchmark(test_case: &TestCase) -> BenchmarkResult {
    // TODO: Integrate with actual context retrieval system
    // For now, return mock results
    let results = mock_search(&test_case.query);

    let metrics = BenchmarkMetrics::calculate(&results, &test_case.expected);

    BenchmarkResult {
        name: test_case.name.clone(),
        query: test_case.query.clone(),
        results,
        metrics,
    }
}

/// Mock search function (replace with actual context retrieval)
fn mock_search(_query: &str) -> Vec<String> {
    vec![
        "crates/css/modules/cascade/src/lib.rs".to_owned(),
        "crates/css/modules/core/src/lib.rs".to_owned(),
        "crates/css/orchestrator/src/lib.rs".to_owned(),
    ]
}

/// Generate markdown report from benchmark results
#[allow(
    clippy::let_underscore_must_use,
    reason = "writeln! to String never fails"
)]
pub fn generate_report(results: &[BenchmarkResult]) -> String {
    use std::fmt::Write as _;

    let mut report = String::default();

    report.push_str("# Context Quality Benchmark Results\n\n");

    let metrics: Vec<_> = results
        .iter()
        .map(|result| result.metrics.clone())
        .collect();
    let aggregate = AggregateMetrics::from_metrics(&metrics);

    report.push_str("## Summary\n\n");
    let _ = writeln!(report, "**Test Cases**: {}\n", aggregate.test_count);
    report.push_str("| Metric | Value | Target |\n");
    report.push_str("|--------|-------|--------|\n");
    let _ = writeln!(
        report,
        "| Precision@3 | {:.1}% | 60% |",
        aggregate.avg_precision_at_3
    );
    let _ = writeln!(
        report,
        "| Precision@10 | {:.1}% | 55% |",
        aggregate.avg_precision_at_10
    );
    let _ = writeln!(
        report,
        "| Recall@10 | {:.1}% | 70% |",
        aggregate.avg_recall_at_10
    );
    let _ = writeln!(report, "| MRR | {:.3} | 0.700 |", aggregate.avg_mrr);
    let _ = writeln!(
        report,
        "| NDCG@10 | {:.3} | 0.750 |",
        aggregate.avg_ndcg_at_10
    );
    let _ = writeln!(
        report,
        "| Critical in Top-3 | {:.1}% | 65% |\n",
        aggregate.avg_critical_in_top_3
    );

    report.push_str("## Individual Test Results\n\n");

    for result in results {
        let _ = writeln!(report, "### {}\n", result.name);
        let _ = writeln!(report, "**Query**: \"{}\"\n", result.query);
        report.push_str("| Metric | Value |\n");
        report.push_str("|--------|-------|\n");
        let _ = writeln!(
            report,
            "| Precision@3 | {:.1}% |",
            result.metrics.precision_at_3
        );
        let _ = writeln!(
            report,
            "| Precision@10 | {:.1}% |",
            result.metrics.precision_at_10
        );
        let _ = writeln!(
            report,
            "| Recall@10 | {:.1}% |",
            result.metrics.recall_at_10
        );
        let _ = writeln!(report, "| MRR | {:.3} |", result.metrics.mrr);
        let _ = writeln!(report, "| NDCG@10 | {:.3} |", result.metrics.ndcg_at_10);
        let _ = writeln!(
            report,
            "| Critical in Top-3 | {:.1}% |\n",
            result.metrics.critical_in_top_3
        );

        report.push_str("**Top 10 Results**:\n");
        for (index, path) in result.results.iter().take(10).enumerate() {
            let _ = writeln!(report, "{}. `{}`", index + 1, path);
        }
        report.push('\n');
    }

    report
}
