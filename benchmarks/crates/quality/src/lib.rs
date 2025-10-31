//! Quality benchmarking for context retrieval system.

pub mod metrics;
pub mod test_case;

use anyhow::{Context as _, Result, anyhow, bail};
use merlin_context::ContextBuilder;
use merlin_core::{FileContext, Query};
use metrics::{AggregateMetrics, BenchmarkMetrics};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use test_case::TestCase;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::warn;
use walkdir::WalkDir;

/// Run all benchmarks in a directory
///
/// # Errors
/// Returns error if test case files cannot be read or parsed
pub async fn run_benchmarks_async(test_cases_dir: &Path) -> Result<Vec<BenchmarkResult>> {
    let mut test_cases = Vec::new();

    for entry in WalkDir::new(test_cases_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "toml")
        {
            let test_case = TestCase::from_file(entry.path())
                .with_context(|| format!("Failed to load test case: {}", entry.path().display()))?;

            // Setup repository if needed
            if let Some(repo_config) = &test_case.repository {
                setup_repository(&test_case.project_root, repo_config).with_context(|| {
                    format!("Failed to setup repository for test: {}", test_case.name)
                })?;
            }

            test_cases.push(test_case);
        }
    }

    // Group test cases by project root to share ContextBuilder/VectorSearchManager instances
    let mut grouped_cases: HashMap<String, Vec<TestCase>> = HashMap::new();
    for test_case in test_cases {
        grouped_cases
            .entry(test_case.project_root.clone())
            .or_default()
            .push(test_case);
    }

    // Spawn each project group as a parallel task
    let mut group_tasks = JoinSet::new();

    for (project_root_str, project_cases) in grouped_cases {
        group_tasks.spawn(async move {
            run_benchmarks_for_project(&project_root_str, project_cases).await
        });
    }

    let mut all_results = Vec::new();
    while let Some(result) = group_tasks.join_next().await {
        match result {
            Ok(group_results) => all_results.extend(group_results),
            Err(error) => warn!("Project group failed: {error}"),
        }
    }

    Ok(all_results)
}

/// Run benchmarks for a single project with a shared `ContextBuilder`
async fn run_benchmarks_for_project(
    project_root_str: &str,
    test_cases: Vec<TestCase>,
) -> Vec<BenchmarkResult> {
    let project_root = Path::new(project_root_str).to_path_buf();

    // Create builder with increased max_files for benchmarks
    let builder = Arc::new(Mutex::new(
        ContextBuilder::new(project_root.clone()).with_max_files(20),
    ));

    // Warm up the builder by running a dummy query to initialize all systems
    // This ensures embeddings are ready before parallel execution
    {
        let warmup_query = Query::new("initialization");
        let _ignored = builder.lock().await.build_context(&warmup_query).await;
    }

    // Run test cases in parallel, sharing the builder
    let mut tasks = JoinSet::new();
    for test_case in test_cases {
        let builder_clone = Arc::clone(&builder);
        tasks.spawn(
            async move { run_single_benchmark_with_builder(&test_case, builder_clone).await },
        );
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(benchmark_result) => results.push(benchmark_result),
            Err(error) => warn!("Benchmark task failed: {error}"),
        }
    }

    results
}

/// Run a single benchmark test case with a shared `ContextBuilder`
async fn run_single_benchmark_with_builder(
    test_case: &TestCase,
    builder: Arc<Mutex<ContextBuilder>>,
) -> BenchmarkResult {
    let project_path = Path::new(&test_case.project_root);
    let mut logs = Vec::new();

    logs.push(format!("Running test: {}", test_case.name));
    logs.push(format!("Query: {}", test_case.query));
    logs.push(format!("Project: {}", project_path.display()));

    let results = perform_search_with_builder(&test_case.query, project_path, builder)
        .await
        .unwrap_or_else(|err| {
            logs.push(format!("❌ Search failed: {err}"));
            Vec::new()
        });

    let num_results = results.len();
    logs.push(format!("✓ Found {num_results} results"));

    if !results.is_empty() {
        logs.push("Retrieved files:".to_owned());
        for (index, result) in results.iter().take(10).enumerate() {
            logs.push(format!("  {}. {result}", index + 1));
        }
    }

    let metrics = BenchmarkMetrics::calculate(&results, &test_case.expected);

    BenchmarkResult {
        name: test_case.name.clone(),
        query: test_case.query.clone(),
        results,
        metrics,
        logs,
    }
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
    /// Execution logs
    pub logs: Vec<String>,
}

/// Perform actual context search using merlin-context with a shared builder
///
/// # Errors
/// Returns error if context building fails
async fn perform_search_with_builder(
    query: &str,
    project_root: &Path,
    builder: Arc<Mutex<ContextBuilder>>,
) -> Result<Vec<String>> {
    if !project_root.exists() {
        return Err(anyhow!("Failed to find project {}", project_root.display()));
    }

    let query_obj = Query::new(query);

    // Lock the builder for this search operation and build context
    let context = {
        let mut builder_guard = builder.lock().await;
        builder_guard.build_context(&query_obj).await?
    };

    let paths: Vec<String> = context
        .files
        .iter()
        .map(|file: &FileContext| {
            file.path
                .strip_prefix(project_root)
                .unwrap_or(&file.path)
                .to_string_lossy()
                .to_string()
                .replace('\\', "/") // Normalize to forward slashes for cross-platform consistency
        })
        .collect();

    Ok(paths)
}

/// Setup repository by cloning if needed and checking out specific commit
///
/// # Errors
/// Returns error if git commands fail
fn setup_repository(project_root: &str, config: &test_case::RepositoryConfig) -> Result<()> {
    let repo_path = PathBuf::from(project_root);

    // Clone if repository doesn't exist
    if !repo_path.exists() {
        tracing::info!("Cloning repository: {} -> {}", config.url, project_root);

        // Create parent directory if needed
        if let Some(parent) = repo_path.parent() {
            create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory: {}", parent.display())
            })?;
        }

        let clone_output = Command::new("git")
            .args(["clone", &config.url, project_root])
            .output()
            .context("Failed to execute git clone")?;

        if !clone_output.status.success() {
            let stderr = String::from_utf8_lossy(&clone_output.stderr);
            bail!("Git clone failed: {stderr}");
        }
    }

    // Verify it's a git repository
    if !repo_path.join(".git").exists() {
        bail!("Directory exists but is not a git repository: {project_root}");
    }

    // Stash any local changes
    let stash_output = Command::new("git")
        .current_dir(&repo_path)
        .args(["stash", "push", "-u", "-m", "Quality benchmark auto-stash"])
        .output()
        .context("Failed to execute git stash")?;

    if !stash_output.status.success() {
        let stderr = String::from_utf8_lossy(&stash_output.stderr);
        warn!("git stash failed (might be nothing to stash): {stderr}");
    }

    // Checkout the specific commit
    tracing::info!("Checking out commit: {} in {}", config.commit, project_root);
    let checkout_output = Command::new("git")
        .current_dir(&repo_path)
        .args(["checkout", &config.commit])
        .output()
        .context("Failed to execute git checkout")?;

    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
        bail!("Git checkout failed: {stderr}");
    }

    Ok(())
}

/// Generate summary section of the report
fn generate_summary_section(aggregate: &AggregateMetrics) -> String {
    use std::fmt::Write as _;

    let mut section = String::from("## Summary\n\n");
    _ = writeln!(section, "**Test Cases**: {}\n", aggregate.test_count);
    section.push_str("| Metric | Value | Target |\n");
    section.push_str("|--------|-------|--------|\n");
    _ = writeln!(
        section,
        "| Precision@3 | {:.1}% | 60% |",
        aggregate.avg_precision_at_3
    );
    _ = writeln!(
        section,
        "| Precision@10 | {:.1}% | 55% |",
        aggregate.avg_precision_at_10
    );
    _ = writeln!(
        section,
        "| Recall@10 | {:.1}% | 70% |",
        aggregate.avg_recall_at_10
    );
    _ = writeln!(section, "| MRR | {:.3} | 0.700 |", aggregate.avg_mrr);
    _ = writeln!(
        section,
        "| NDCG@10 | {:.3} | 0.750 |",
        aggregate.avg_ndcg_at_10
    );
    _ = writeln!(
        section,
        "| Critical in Top-3 | {:.1}% | 65% |\n",
        aggregate.avg_critical_in_top_3
    );

    section
}

/// Generate individual result section
fn generate_result_section(result: &BenchmarkResult) -> String {
    use std::fmt::Write as _;

    let mut section = String::default();
    _ = writeln!(section, "### {}\n", result.name);
    _ = writeln!(section, "**Query**: \"{}\"\n", result.query);
    section.push_str("| Metric | Value |\n");
    section.push_str("|--------|-------|\n");
    _ = writeln!(
        section,
        "| Precision@3 | {:.1}% |",
        result.metrics.precision_at_3
    );
    _ = writeln!(
        section,
        "| Precision@10 | {:.1}% |",
        result.metrics.precision_at_10
    );
    _ = writeln!(
        section,
        "| Recall@10 | {:.1}% |",
        result.metrics.recall_at_10
    );
    _ = writeln!(section, "| MRR | {:.3} |", result.metrics.mrr);
    _ = writeln!(section, "| NDCG@10 | {:.3} |", result.metrics.ndcg_at_10);
    _ = writeln!(
        section,
        "| Critical in Top-3 | {:.1}% |\n",
        result.metrics.critical_in_top_3
    );

    section.push_str("**Top 10 Results**:\n");
    for (index, path) in result.results.iter().take(10).enumerate() {
        _ = writeln!(section, "{}. `{}`", index + 1, path);
    }
    section.push('\n');

    // Add execution logs
    if !result.logs.is_empty() {
        section.push_str("<details>\n");
        section.push_str("<summary>Execution Logs</summary>\n\n");
        section.push_str("```\n");
        for log in &result.logs {
            _ = writeln!(section, "{log}");
        }
        section.push_str("```\n");
        section.push_str("</details>\n\n");
    }

    section
}

/// Generate markdown report from benchmark results
pub fn generate_report(results: &[BenchmarkResult]) -> String {
    let mut report = String::from("# Context Quality Benchmark Results\n\n");

    let metrics: Vec<_> = results
        .iter()
        .map(|result| result.metrics.clone())
        .collect();
    let aggregate = AggregateMetrics::from_metrics(&metrics);

    report.push_str(&generate_summary_section(&aggregate));
    report.push_str("## Individual Test Results\n\n");

    for result in results {
        report.push_str(&generate_result_section(result));
    }

    report
}
