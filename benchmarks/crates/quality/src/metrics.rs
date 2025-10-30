//! Information retrieval metrics for context quality benchmarking.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Priority level for expected files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// Critical file - must be in top results
    Critical,
    /// High priority file
    High,
    /// Medium priority file
    Medium,
    /// Low priority file
    Low,
}

impl Priority {
    /// Convert priority to relevance score for NDCG calculation
    pub const fn to_relevance_score(self) -> f64 {
        match self {
            Self::Critical => 3.0,
            Self::High => 2.0,
            Self::Medium => 1.0,
            Self::Low => 0.5,
        }
    }
}

/// Expected file with priority
#[derive(Debug, Clone)]
pub struct ExpectedFile {
    /// File path
    pub path: String,
    /// Priority level
    pub priority: Priority,
    /// Reason for relevance
    pub reason: String,
}

/// Benchmark metrics for a single test case
#[derive(Debug, Clone)]
pub struct BenchmarkMetrics {
    /// Precision at 3 (% of top 3 results that are relevant)
    pub precision_at_3: f64,
    /// Precision at 10 (% of top 10 results that are relevant)
    pub precision_at_10: f64,
    /// Recall at 10 (% of relevant files found in top 10)
    pub recall_at_10: f64,
    /// Mean Reciprocal Rank
    pub mrr: f64,
    /// Normalized Discounted Cumulative Gain at 10
    pub ndcg_at_10: f64,
    /// Percentage of critical files in top 3
    pub critical_in_top_3: f64,
}

impl BenchmarkMetrics {
    /// Normalize path separators to forward slashes for consistent comparison
    fn normalize_path(path: &str) -> String {
        path.replace('\\', "/")
    }

    /// Calculate metrics from results and expected files
    pub fn calculate(results: &[String], expected: &[ExpectedFile]) -> Self {
        // Normalize all paths to use forward slashes for consistent comparison
        let expected_paths: HashSet<_> = expected
            .iter()
            .map(|exp| Self::normalize_path(&exp.path))
            .collect();
        let critical_paths: HashSet<_> = expected
            .iter()
            .filter(|exp| exp.priority == Priority::Critical)
            .map(|exp| Self::normalize_path(&exp.path))
            .collect();

        // Normalize result paths as well
        let normalized_results: Vec<String> = results
            .iter()
            .map(|res| Self::normalize_path(res))
            .collect();

        let precision_at_3 = Self::precision_at_k(&normalized_results, &expected_paths, 3);
        let precision_at_10 = Self::precision_at_k(&normalized_results, &expected_paths, 10);
        let recall_at_10 = Self::recall_at_k(&normalized_results, &expected_paths, 10);
        let mrr = Self::mean_reciprocal_rank(&normalized_results, &expected_paths);
        let ndcg_at_10 = Self::ndcg_at_k(&normalized_results, expected, 10);
        let critical_in_top_3 = Self::critical_in_top_k(&normalized_results, &critical_paths, 3);

        Self {
            precision_at_3,
            precision_at_10,
            recall_at_10,
            mrr,
            ndcg_at_10,
            critical_in_top_3,
        }
    }

    /// Calculate precision at k
    fn precision_at_k(results: &[String], expected: &HashSet<String>, cutoff: usize) -> f64 {
        let top_k = results.iter().take(cutoff);
        let relevant_count = top_k.filter(|res| expected.contains(res.as_str())).count();
        (relevant_count as f64 / cutoff.min(results.len()) as f64) * 100.0
    }

    /// Calculate recall at k
    fn recall_at_k(results: &[String], expected: &HashSet<String>, cutoff: usize) -> f64 {
        if expected.is_empty() {
            return 0.0;
        }
        let top_k = results.iter().take(cutoff);
        let found_count = top_k.filter(|res| expected.contains(res.as_str())).count();
        (found_count as f64 / expected.len() as f64) * 100.0
    }

    /// Calculate Mean Reciprocal Rank
    fn mean_reciprocal_rank(results: &[String], expected: &HashSet<String>) -> f64 {
        for (index, result) in results.iter().enumerate() {
            if expected.contains(result.as_str()) {
                return 1.0 / (index + 1) as f64;
            }
        }
        0.0
    }

    /// Calculate Normalized Discounted Cumulative Gain at k
    fn ndcg_at_k(results: &[String], expected: &[ExpectedFile], cutoff: usize) -> f64 {
        let dcg = Self::dcg_at_k(results, expected, cutoff);
        let idcg = Self::ideal_dcg_at_k(expected, cutoff);

        if idcg.abs() < f64::EPSILON {
            0.0
        } else {
            dcg / idcg
        }
    }

    /// Calculate Discounted Cumulative Gain at k
    fn dcg_at_k(results: &[String], expected: &[ExpectedFile], cutoff: usize) -> f64 {
        // Normalize expected paths for comparison
        let expected_map: HashMap<_, _> = expected
            .iter()
            .map(|exp| (Self::normalize_path(&exp.path), exp))
            .collect();

        results
            .iter()
            .take(cutoff)
            .enumerate()
            .map(|(index, result)| {
                let relevance = expected_map
                    .get(result.as_str())
                    .map_or(0.0, |exp| exp.priority.to_relevance_score());
                relevance / ((index + 2) as f64).log2()
            })
            .sum()
    }

    /// Calculate ideal DCG at k (best possible ordering)
    fn ideal_dcg_at_k(expected: &[ExpectedFile], cutoff: usize) -> f64 {
        let mut relevances: Vec<_> = expected
            .iter()
            .map(|exp| exp.priority.to_relevance_score())
            .collect();
        relevances.sort_by(|left, right| right.partial_cmp(left).unwrap_or(Ordering::Equal));

        relevances
            .iter()
            .take(cutoff)
            .enumerate()
            .map(|(index, &relevance)| relevance / ((index + 2) as f64).log2())
            .sum()
    }

    /// Calculate percentage of critical files in top k
    fn critical_in_top_k(results: &[String], critical: &HashSet<String>, cutoff: usize) -> f64 {
        if critical.is_empty() {
            return 0.0;
        }
        let top_k = results.iter().take(cutoff);
        let critical_count = top_k.filter(|res| critical.contains(res.as_str())).count();
        (critical_count as f64 / critical.len() as f64) * 100.0
    }
}

/// Aggregate metrics across multiple test cases
#[derive(Debug, Clone)]
pub struct AggregateMetrics {
    /// Average precision at 3
    pub avg_precision_at_3: f64,
    /// Average precision at 10
    pub avg_precision_at_10: f64,
    /// Average recall at 10
    pub avg_recall_at_10: f64,
    /// Average MRR
    pub avg_mrr: f64,
    /// Average NDCG at 10
    pub avg_ndcg_at_10: f64,
    /// Average critical in top 3
    pub avg_critical_in_top_3: f64,
    /// Number of test cases
    pub test_count: usize,
}

impl AggregateMetrics {
    /// Calculate aggregate metrics from individual metrics
    pub fn from_metrics(metrics: &[BenchmarkMetrics]) -> Self {
        let test_count = metrics.len();
        if test_count == 0 {
            return Self {
                avg_precision_at_3: 0.0,
                avg_precision_at_10: 0.0,
                avg_recall_at_10: 0.0,
                avg_mrr: 0.0,
                avg_ndcg_at_10: 0.0,
                avg_critical_in_top_3: 0.0,
                test_count: 0,
            };
        }

        let sum_precision_3: f64 = metrics.iter().map(|metric| metric.precision_at_3).sum();
        let sum_precision_10: f64 = metrics.iter().map(|metric| metric.precision_at_10).sum();
        let sum_recall_10: f64 = metrics.iter().map(|metric| metric.recall_at_10).sum();
        let sum_mrr: f64 = metrics.iter().map(|metric| metric.mrr).sum();
        let sum_ndcg: f64 = metrics.iter().map(|metric| metric.ndcg_at_10).sum();
        let sum_critical: f64 = metrics.iter().map(|metric| metric.critical_in_top_3).sum();

        let count = test_count as f64;

        Self {
            avg_precision_at_3: sum_precision_3 / count,
            avg_precision_at_10: sum_precision_10 / count,
            avg_recall_at_10: sum_recall_10 / count,
            avg_mrr: sum_mrr / count,
            avg_ndcg_at_10: sum_ndcg / count,
            avg_critical_in_top_3: sum_critical / count,
            test_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]

    fn test_precision_calculation() {
        let results = vec![
            "file1.rs".to_owned(),
            "file2.rs".to_owned(),
            "file3.rs".to_owned(),
        ];
        let expected = vec![
            ExpectedFile {
                path: "file1.rs".to_owned(),
                priority: Priority::Critical,
                reason: "test".to_owned(),
            },
            ExpectedFile {
                path: "file3.rs".to_owned(),
                priority: Priority::High,
                reason: "test".to_owned(),
            },
        ];

        let metrics = BenchmarkMetrics::calculate(&results, &expected);
        assert!((metrics.precision_at_3 - 66.67).abs() < 0.1);
    }

    #[test]

    fn test_recall_calculation() {
        let results = vec![
            "file1.rs".to_owned(),
            "file2.rs".to_owned(),
            "file3.rs".to_owned(),
        ];
        let expected = vec![
            ExpectedFile {
                path: "file1.rs".to_owned(),
                priority: Priority::Critical,
                reason: "test".to_owned(),
            },
            ExpectedFile {
                path: "file4.rs".to_owned(),
                priority: Priority::High,
                reason: "test".to_owned(),
            },
        ];

        let metrics = BenchmarkMetrics::calculate(&results, &expected);
        assert!((metrics.recall_at_10 - 50.0).abs() < f64::EPSILON);
    }

    #[test]

    fn test_mrr_calculation() {
        let results = vec![
            "file1.rs".to_owned(),
            "file2.rs".to_owned(),
            "file3.rs".to_owned(),
        ];
        let expected = vec![ExpectedFile {
            path: "file2.rs".to_owned(),
            priority: Priority::Critical,
            reason: "test".to_owned(),
        }];

        let metrics = BenchmarkMetrics::calculate(&results, &expected);
        assert!((metrics.mrr - 0.5).abs() < f64::EPSILON);
    }
}
