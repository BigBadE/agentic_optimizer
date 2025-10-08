//! Benchmark system for evaluating context fetching quality.

use merlin_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use toml::from_str as toml_from_str;

/// Priority level for expected files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Must appear in top 3 results.
    Critical,
    /// Should appear in top 5 results.
    High,
    /// Should appear in top 10 results.
    Medium,
    /// Should appear in top 20 results.
    Low,
}

impl Priority {
    /// Get weight for NDCG calculation
    pub fn weight(self) -> f32 {
        match self {
            Self::Critical => 1.0,
            Self::High => 0.8,
            Self::Medium => 0.5,
            Self::Low => 0.2,
        }
    }

    /// Get expected rank range
    pub fn expected_rank(self) -> usize {
        match self {
            Self::Critical => 3,
            Self::High => 5,
            Self::Medium => 10,
            Self::Low => 20,
        }
    }
}

/// Expected file in benchmark
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedFile {
    /// File path relative to project root.
    pub path: String,
    /// Priority level for this file.
    pub priority: Priority,
    /// Explanation of why this file is expected.
    pub reason: String,
}

/// Excluded file that should not appear
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExcludedFile {
    /// File path that should not be in results.
    pub path: String,
    /// Explanation of why this file should be excluded.
    pub reason: String,
}

/// Test case definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestCase {
    /// Name of the test case.
    pub name: String,
    /// Description of what this test evaluates.
    pub description: String,
    /// User query to test.
    pub query: String,
    /// Optional project root override.
    #[serde(default)]
    pub project_root: Option<String>,
    /// Files expected to appear in results.
    pub expected: Vec<ExpectedFile>,
    /// Files that should not appear in results.
    pub excluded: Vec<ExcludedFile>,
}

/// Result of a single file in the ranking
#[derive(Debug, Clone)]
pub struct RankedFile {
    /// Path to the file.
    pub path: PathBuf,
    /// Position in the ranking (1-indexed).
    pub rank: usize,
    /// Relevance score.
    pub score: f32,
}

/// Benchmark metrics
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkMetrics {
    /// Precision at rank 3 (fraction of top 3 that are expected).
    pub precision_at_3: f32,
    /// Precision at rank 5 (fraction of top 5 that are expected).
    pub precision_at_5: f32,
    /// Precision at rank 10 (fraction of top 10 that are expected).
    pub precision_at_10: f32,
    /// Recall at rank 10 (fraction of expected files in top 10).
    pub recall_at_10: f32,
    /// Recall at rank 20 (fraction of expected files in top 20).
    pub recall_at_20: f32,
    /// Mean Reciprocal Rank.
    pub mrr: f32,
    /// Normalized Discounted Cumulative Gain at rank 10.
    pub ndcg_at_10: f32,
    /// Fraction of excluded files that appeared in results.
    pub exclusion_rate: f32,
    /// Fraction of critical files in top 3.
    pub critical_in_top_3: f32,
    /// Fraction of high-priority files in top 5.
    pub high_in_top_5: f32,
}

/// Detailed result for a test case
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// The test case that was executed
    pub test_case: TestCase,
    /// Performance metrics for this test case
    pub metrics: BenchmarkMetrics,
    /// All ranked files returned by the search
    pub ranked_files: Vec<RankedFile>,
    /// Expected files that were found (rank, file)
    pub found_expected: Vec<(usize, ExpectedFile)>,
    /// Expected files that were not found
    pub missing_expected: Vec<ExpectedFile>,
    /// Excluded files that were incorrectly found (rank, file)
    pub found_excluded: Vec<(usize, ExcludedFile)>,
}

impl BenchmarkResult {
    /// Create result from test case and ranked files
    pub fn new(test_case: TestCase, ranked_files: Vec<RankedFile>) -> Self {
        let metrics = Self::calculate_metrics(&test_case, &ranked_files);

        let mut found_expected = Vec::default();
        let mut missing_expected = Vec::default();

        for expected in &test_case.expected {
            if let Some(rank) = Self::find_file_rank(&ranked_files, &expected.path) {
                found_expected.push((rank, expected.clone()));
            } else {
                missing_expected.push(expected.clone());
            }
        }

        let mut found_excluded = Vec::default();
        for excluded in &test_case.excluded {
            if let Some(rank) = Self::find_file_rank(&ranked_files, &excluded.path)
                && rank <= 20
            {
                found_excluded.push((rank, excluded.clone()));
            }
        }

        Self {
            test_case,
            metrics,
            ranked_files,
            found_expected,
            missing_expected,
            found_excluded,
        }
    }

    fn find_file_rank(ranked_files: &[RankedFile], path: &str) -> Option<usize> {
        // Normalize path separators for comparison
        let normalized_expected = path.replace('/', "\\");

        for file in ranked_files {
            let file_path_str = file.path.to_str().unwrap_or("");
            // Check both original and normalized versions
            if file_path_str.contains(path) || file_path_str.contains(&normalized_expected) {
                return Some(file.rank);
            }
        }
        None
    }

    fn calculate_metrics(test_case: &TestCase, ranked_files: &[RankedFile]) -> BenchmarkMetrics {
        let expected_paths: HashSet<String> = test_case
            .expected
            .iter()
            .map(|expected_file| expected_file.path.clone())
            .collect();

        let excluded_paths: HashSet<String> = test_case
            .excluded
            .iter()
            .map(|excluded_file| excluded_file.path.clone())
            .collect();

        let precision_at_3 = Self::calculate_precision_at_k(ranked_files, &expected_paths, 3);
        let precision_at_5 = Self::calculate_precision_at_k(ranked_files, &expected_paths, 5);
        let precision_at_10 = Self::calculate_precision_at_k(ranked_files, &expected_paths, 10);
        let recall_at_10 = Self::calculate_recall_at_k(ranked_files, &expected_paths, 10);
        let recall_at_20 = Self::calculate_recall_at_k(ranked_files, &expected_paths, 20);
        let mrr = Self::calculate_mrr(ranked_files, &expected_paths);
        let ndcg_at_10 = Self::calculate_ndcg(&test_case.expected, ranked_files, 10);
        let exclusion_rate = Self::calculate_exclusion_rate(ranked_files, &excluded_paths);
        let critical_in_top_3 = Self::calculate_priority_in_top_k(
            &test_case.expected,
            ranked_files,
            Priority::Critical,
            3,
        );
        let high_in_top_5 =
            Self::calculate_high_priority_in_top_k(&test_case.expected, ranked_files, 5);

        BenchmarkMetrics {
            precision_at_3,
            precision_at_5,
            precision_at_10,
            recall_at_10,
            recall_at_20,
            mrr,
            ndcg_at_10,
            exclusion_rate,
            critical_in_top_3,
            high_in_top_5,
        }
    }

    fn calculate_precision_at_k(
        ranked_files: &[RankedFile],
        expected_paths: &HashSet<String>,
        limit: usize,
    ) -> f32 {
        let top_k: Vec<_> = ranked_files.iter().take(limit).collect();
        let relevant = top_k
            .iter()
            .filter(|ranked_file| {
                let path_str = ranked_file.path.to_str().unwrap_or("");
                expected_paths.iter().any(|expected_path| {
                    let normalized = expected_path.replace('/', "\\");
                    path_str.contains(expected_path) || path_str.contains(&normalized)
                })
            })
            .count();
        relevant as f32 / limit as f32
    }

    fn calculate_recall_at_k(
        ranked_files: &[RankedFile],
        expected_paths: &HashSet<String>,
        limit: usize,
    ) -> f32 {
        let top_k: Vec<_> = ranked_files.iter().take(limit).collect();
        let found = top_k
            .iter()
            .filter(|ranked_file| {
                let path_str = ranked_file.path.to_str().unwrap_or("");
                expected_paths.iter().any(|expected_path| {
                    let normalized = expected_path.replace('/', "\\");
                    path_str.contains(expected_path) || path_str.contains(&normalized)
                })
            })
            .count();
        found as f32 / expected_paths.len() as f32
    }

    fn calculate_mrr(ranked_files: &[RankedFile], expected_paths: &HashSet<String>) -> f32 {
        let first_relevant = ranked_files.iter().position(|ranked_file| {
            let path_str = ranked_file.path.to_str().unwrap_or("");
            expected_paths.iter().any(|expected_path| {
                let normalized = expected_path.replace('/', "\\");
                path_str.contains(expected_path) || path_str.contains(&normalized)
            })
        });

        first_relevant.map_or(0.0, |rank| 1.0 / (rank + 1) as f32)
    }

    fn calculate_exclusion_rate(
        ranked_files: &[RankedFile],
        excluded_paths: &HashSet<String>,
    ) -> f32 {
        let top_20: Vec<_> = ranked_files.iter().take(20).collect();
        let excluded_found = top_20
            .iter()
            .filter(|ranked_file| {
                let path_str = ranked_file.path.to_str().unwrap_or("");
                excluded_paths.iter().any(|excluded_path| {
                    let normalized = excluded_path.replace('/', "\\");
                    path_str.contains(excluded_path) || path_str.contains(&normalized)
                })
            })
            .count();

        1.0 - (excluded_found as f32 / excluded_paths.len().max(1) as f32)
    }

    fn calculate_priority_in_top_k(
        expected_files: &[ExpectedFile],
        ranked_files: &[RankedFile],
        priority: Priority,
        limit: usize,
    ) -> f32 {
        let priority_files: Vec<_> = expected_files
            .iter()
            .filter(|expected_file| expected_file.priority == priority)
            .collect();

        if priority_files.is_empty() {
            return 1.0;
        }

        let found = priority_files
            .iter()
            .filter(|expected_file| {
                let normalized = expected_file.path.replace('/', "\\");
                ranked_files.iter().take(limit).any(|ranked_file| {
                    let path_str = ranked_file.path.to_str().unwrap_or("");
                    path_str.contains(&expected_file.path) || path_str.contains(&normalized)
                })
            })
            .count();
        found as f32 / priority_files.len() as f32
    }

    fn calculate_high_priority_in_top_k(
        expected_files: &[ExpectedFile],
        ranked_files: &[RankedFile],
        limit: usize,
    ) -> f32 {
        let high_priority_files: Vec<_> = expected_files
            .iter()
            .filter(|expected_file| {
                matches!(expected_file.priority, Priority::Critical | Priority::High)
            })
            .collect();

        if high_priority_files.is_empty() {
            return 1.0;
        }

        let found = high_priority_files
            .iter()
            .filter(|expected_file| {
                let normalized = expected_file.path.replace('/', "\\");
                ranked_files.iter().take(limit).any(|ranked_file| {
                    let path_str = ranked_file.path.to_str().unwrap_or("");
                    path_str.contains(&expected_file.path) || path_str.contains(&normalized)
                })
            })
            .count();
        found as f32 / high_priority_files.len() as f32
    }

    fn calculate_ndcg(expected: &[ExpectedFile], ranked_files: &[RankedFile], top_k: usize) -> f32 {
        use std::cmp::Ordering;

        let expected_map: HashMap<String, f32> = expected
            .iter()
            .map(|expected_file| (expected_file.path.clone(), expected_file.priority.weight()))
            .collect();

        let dcg: f32 = ranked_files
            .iter()
            .take(top_k)
            .enumerate()
            .map(|(index, ranked_file)| {
                let path_str = ranked_file.path.to_str().unwrap_or("");
                let relevance = expected_map
                    .iter()
                    .find(|(path, _)| {
                        let normalized = path.replace('/', "\\");
                        path_str.contains(path.as_str()) || path_str.contains(&normalized)
                    })
                    .map_or(0.0, |(_, weight)| *weight);

                relevance / (index as f32 + 2.0).log2()
            })
            .sum();

        let mut ideal_relevances: Vec<f32> = expected
            .iter()
            .map(|expected_file| expected_file.priority.weight())
            .collect();
        ideal_relevances
            .sort_by(|first, second| second.partial_cmp(first).unwrap_or(Ordering::Equal));

        let idcg: f32 = ideal_relevances
            .iter()
            .take(top_k)
            .enumerate()
            .map(|(index, relevance)| relevance / (index as f32 + 2.0).log2())
            .sum();

        if idcg > 0.0 { dcg / idcg } else { 0.0 }
    }

    /// Format header section of the report
    fn format_header(&self, report: &mut String) {
        writeln!(report, "# Benchmark: {}\n", self.test_case.name).unwrap_or(());
        writeln!(report, "**Query**: \"{}\"\n", self.test_case.query).unwrap_or(());
        writeln!(report, "**Description**: {}\n", self.test_case.description).unwrap_or(());
    }

    /// Format metrics section of the report
    fn format_metrics(&self, report: &mut String) {
        report.push_str("## Metrics\n\n");
        writeln!(
            report,
            "- **Precision@3**:  {:.1}%",
            self.metrics.precision_at_3 * 100.0
        )
        .unwrap_or(());
        writeln!(
            report,
            "- **Precision@5**:  {:.1}%",
            self.metrics.precision_at_5 * 100.0
        )
        .unwrap_or(());
        writeln!(
            report,
            "- **Precision@10**: {:.1}%",
            self.metrics.precision_at_10 * 100.0
        )
        .unwrap_or(());
        writeln!(
            report,
            "- **Recall@10**:    {:.1}%",
            self.metrics.recall_at_10 * 100.0
        )
        .unwrap_or(());
        writeln!(
            report,
            "- **Recall@20**:    {:.1}%",
            self.metrics.recall_at_20 * 100.0
        )
        .unwrap_or(());
        writeln!(report, "- **MRR**:          {:.3}", self.metrics.mrr).unwrap_or(());
        writeln!(report, "- **NDCG@10**:      {:.3}", self.metrics.ndcg_at_10).unwrap_or(());
        writeln!(
            report,
            "- **Exclusion**:    {:.1}%",
            self.metrics.exclusion_rate * 100.0
        )
        .unwrap_or(());
        writeln!(
            report,
            "- **Critical in Top-3**: {:.1}%",
            self.metrics.critical_in_top_3 * 100.0
        )
        .unwrap_or(());
        writeln!(
            report,
            "- **High in Top-5**:     {:.1}%\n",
            self.metrics.high_in_top_5 * 100.0
        )
        .unwrap_or(());
    }

    /// Get status string for a ranked file
    fn get_file_status(&self, rank: usize) -> String {
        if let Some((_, expected)) = self
            .found_expected
            .iter()
            .find(|(found_rank, _)| *found_rank == rank)
        {
            let priority = match expected.priority {
                Priority::Critical => "Critical",
                Priority::High => "High",
                Priority::Medium => "Medium",
                Priority::Low => "Low",
            };
            format!("Check mark (expected: {priority})")
        } else if self
            .found_excluded
            .iter()
            .any(|(excluded_rank, _)| *excluded_rank == rank)
        {
            "Cross mark (excluded)".to_string()
        } else {
            "Cross mark (not expected)".to_string()
        }
    }

    /// Format top results section of the report
    fn format_top_results(&self, report: &mut String) {
        report.push_str("## Top 10 Results\n\n");
        for (index, ranked_file) in self.ranked_files.iter().take(10).enumerate() {
            let rank = index + 1;
            let path_str = ranked_file.path.to_str().unwrap_or("?");
            let status = self.get_file_status(rank);
            writeln!(
                report,
                "{rank}. {path_str} {status} (score: {:.3})",
                ranked_file.score
            )
            .unwrap_or(());
        }
    }

    /// Format missing files section of the report
    fn format_missing_files(&self, report: &mut String) {
        if self.missing_expected.is_empty() {
            return;
        }
        report.push_str("\n## Missing Expected Files\n\n");
        for expected in &self.missing_expected {
            let priority_str = match expected.priority {
                Priority::Critical => "Critical",
                Priority::High => "High",
                Priority::Medium => "Medium",
                Priority::Low => "Low",
            };
            writeln!(
                report,
                "- **{}** ({priority_str}): {}",
                expected.path, expected.reason
            )
            .unwrap_or(());
        }
    }

    /// Format result as human-readable text
    pub fn format_report(&self) -> String {
        let mut report = String::default();
        self.format_header(&mut report);
        self.format_metrics(&mut report);
        self.format_top_results(&mut report);
        self.format_missing_files(&mut report);
        report
    }
}

/// Type alias for test case collection
type TestCaseCollection = Vec<(PathBuf, TestCase)>;

/// Load test case from TOML file
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed as TOML
pub fn load_test_case(path: &Path) -> Result<TestCase> {
    use std::fs::read_to_string;
    let content = read_to_string(path)?;
    let test_case: TestCase = toml_from_str(&content)?;
    Ok(test_case)
}

/// Load all test cases from a directory
///
/// # Errors
///
/// Returns an error if the directory cannot be read
pub fn load_test_cases(dir: &Path) -> Result<TestCaseCollection> {
    use std::fs::read_dir;
    let mut test_cases = Vec::default();

    for entry in read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            match load_test_case(&path) {
                Ok(test_case) => test_cases.push((path, test_case)),
                Err(error) => {
                    tracing::warn!("Warning: Failed to load {}: {}", path.display(), error);
                }
            }
        }
    }

    Ok(test_cases)
}
