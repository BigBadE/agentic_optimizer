//! Benchmark system for evaluating context fetching quality.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use agentic_core::Result;

/// Priority level for expected files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl Priority {
    /// Get weight for NDCG calculation
    pub fn weight(self) -> f32 {
        match self {
            Priority::Critical => 1.0,
            Priority::High => 0.8,
            Priority::Medium => 0.5,
            Priority::Low => 0.2,
        }
    }

    /// Get expected rank range
    pub fn expected_rank(self) -> usize {
        match self {
            Priority::Critical => 3,
            Priority::High => 5,
            Priority::Medium => 10,
            Priority::Low => 20,
        }
    }
}

/// Expected file in benchmark
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpectedFile {
    pub path: String,
    pub priority: Priority,
    pub reason: String,
}

/// Excluded file that should not appear
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExcludedFile {
    pub path: String,
    pub reason: String,
}

/// Test case definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub query: String,
    #[serde(default)]
    pub project_root: Option<String>,
    pub expected: Vec<ExpectedFile>,
    pub excluded: Vec<ExcludedFile>,
}

/// Result of a single file in the ranking
#[derive(Debug, Clone)]
pub struct RankedFile {
    pub path: PathBuf,
    pub rank: usize,
    pub score: f32,
}

/// Benchmark metrics
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkMetrics {
    pub precision_at_3: f32,
    pub precision_at_5: f32,
    pub precision_at_10: f32,
    pub recall_at_10: f32,
    pub recall_at_20: f32,
    pub mrr: f32,
    pub ndcg_at_10: f32,
    pub exclusion_rate: f32,
    pub critical_in_top_3: f32,
    pub high_in_top_5: f32,
}

/// Detailed result for a test case
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub test_case: TestCase,
    pub metrics: BenchmarkMetrics,
    pub ranked_files: Vec<RankedFile>,
    pub found_expected: Vec<(usize, ExpectedFile)>,
    pub missing_expected: Vec<ExpectedFile>,
    pub found_excluded: Vec<(usize, ExcludedFile)>,
}

impl BenchmarkResult {
    /// Create result from test case and ranked files
    pub fn new(test_case: TestCase, ranked_files: Vec<RankedFile>) -> Self {
        let metrics = Self::calculate_metrics(&test_case, &ranked_files);
        
        let mut found_expected = Vec::new();
        let mut missing_expected = Vec::new();
        
        for expected in &test_case.expected {
            if let Some(rank) = Self::find_file_rank(&ranked_files, &expected.path) {
                found_expected.push((rank, expected.clone()));
            } else {
                missing_expected.push(expected.clone());
            }
        }
        
        let mut found_excluded = Vec::new();
        for excluded in &test_case.excluded {
            if let Some(rank) = Self::find_file_rank(&ranked_files, &excluded.path) {
                if rank <= 20 {
                    found_excluded.push((rank, excluded.clone()));
                }
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
        let expected_paths: HashSet<String> = test_case.expected.iter()
            .map(|e| e.path.clone())
            .collect();
        
        let excluded_paths: HashSet<String> = test_case.excluded.iter()
            .map(|e| e.path.clone())
            .collect();
        
        let precision_at_k = |k: usize| -> f32 {
            let top_k: Vec<_> = ranked_files.iter().take(k).collect();
            let relevant = top_k.iter()
                .filter(|f| {
                    let path_str = f.path.to_str().unwrap_or("");
                    expected_paths.iter().any(|e| {
                        let normalized = e.replace('/', "\\");
                        path_str.contains(e) || path_str.contains(&normalized)
                    })
                })
                .count();
            relevant as f32 / k as f32
        };
        
        let recall_at_k = |k: usize| -> f32 {
            let top_k: Vec<_> = ranked_files.iter().take(k).collect();
            let found = top_k.iter()
                .filter(|f| {
                    let path_str = f.path.to_str().unwrap_or("");
                    expected_paths.iter().any(|e| {
                        let normalized = e.replace('/', "\\");
                        path_str.contains(e) || path_str.contains(&normalized)
                    })
                })
                .count();
            found as f32 / expected_paths.len() as f32
        };
        
        let mrr = {
            let first_relevant = ranked_files.iter()
                .position(|f| {
                    let path_str = f.path.to_str().unwrap_or("");
                    expected_paths.iter().any(|e| {
                        let normalized = e.replace('/', "\\");
                        path_str.contains(e) || path_str.contains(&normalized)
                    })
                });
            
            first_relevant.map(|rank| 1.0 / (rank + 1) as f32).unwrap_or(0.0)
        };
        
        let ndcg_at_10 = Self::calculate_ndcg(&test_case.expected, ranked_files, 10);
        
        let exclusion_rate = {
            let top_20: Vec<_> = ranked_files.iter().take(20).collect();
            let excluded_found = top_20.iter()
                .filter(|f| {
                    let path_str = f.path.to_str().unwrap_or("");
                    excluded_paths.iter().any(|e| {
                        let normalized = e.replace('/', "\\");
                        path_str.contains(e) || path_str.contains(&normalized)
                    })
                })
                .count();
            
            1.0 - (excluded_found as f32 / excluded_paths.len().max(1) as f32)
        };
        
        let critical_in_top_3 = {
            let critical: Vec<_> = test_case.expected.iter()
                .filter(|e| e.priority == Priority::Critical)
                .collect();
            
            if critical.is_empty() {
                1.0
            } else {
                let found = critical.iter()
                    .filter(|e| {
                        let normalized = e.path.replace('/', "\\");
                        ranked_files.iter().take(3).any(|f| {
                            let path_str = f.path.to_str().unwrap_or("");
                            path_str.contains(&e.path) || path_str.contains(&normalized)
                        })
                    })
                    .count();
                found as f32 / critical.len() as f32
            }
        };
        
        let high_in_top_5 = {
            let high: Vec<_> = test_case.expected.iter()
                .filter(|e| matches!(e.priority, Priority::Critical | Priority::High))
                .collect();
            
            if high.is_empty() {
                1.0
            } else {
                let found = high.iter()
                    .filter(|e| {
                        let normalized = e.path.replace('/', "\\");
                        ranked_files.iter().take(5).any(|f| {
                            let path_str = f.path.to_str().unwrap_or("");
                            path_str.contains(&e.path) || path_str.contains(&normalized)
                        })
                    })
                    .count();
                found as f32 / high.len() as f32
            }
        };
        
        BenchmarkMetrics {
            precision_at_3: precision_at_k(3),
            precision_at_5: precision_at_k(5),
            precision_at_10: precision_at_k(10),
            recall_at_10: recall_at_k(10),
            recall_at_20: recall_at_k(20),
            mrr,
            ndcg_at_10,
            exclusion_rate,
            critical_in_top_3,
            high_in_top_5,
        }
    }
    
    fn calculate_ndcg(expected: &[ExpectedFile], ranked_files: &[RankedFile], k: usize) -> f32 {
        let expected_map: HashMap<String, f32> = expected.iter()
            .map(|e| (e.path.clone(), e.priority.weight()))
            .collect();
        
        let dcg: f32 = ranked_files.iter()
            .take(k)
            .enumerate()
            .map(|(i, f)| {
                let path_str = f.path.to_str().unwrap_or("");
                let relevance = expected_map.iter()
                    .find(|(path, _)| {
                        let normalized = path.replace('/', "\\");
                        path_str.contains(path.as_str()) || path_str.contains(&normalized)
                    })
                    .map(|(_, weight)| *weight)
                    .unwrap_or(0.0);
                
                relevance / (i as f32 + 2.0).log2()
            })
            .sum();
        
        let mut ideal_relevances: Vec<f32> = expected.iter()
            .map(|e| e.priority.weight())
            .collect();
        ideal_relevances.sort_by(|a, b| b.partial_cmp(a).unwrap());
        
        let idcg: f32 = ideal_relevances.iter()
            .take(k)
            .enumerate()
            .map(|(i, rel)| rel / (i as f32 + 2.0).log2())
            .sum();
        
        if idcg > 0.0 {
            dcg / idcg
        } else {
            0.0
        }
    }
    
    /// Format result as human-readable text
    pub fn format_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("# Benchmark: {}\n\n", self.test_case.name));
        report.push_str(&format!("**Query**: \"{}\"\n\n", self.test_case.query));
        report.push_str(&format!("**Description**: {}\n\n", self.test_case.description));
        
        report.push_str("## Metrics\n\n");
        report.push_str(&format!("- **Precision@3**:  {:.1}%\n", self.metrics.precision_at_3 * 100.0));
        report.push_str(&format!("- **Precision@5**:  {:.1}%\n", self.metrics.precision_at_5 * 100.0));
        report.push_str(&format!("- **Precision@10**: {:.1}%\n", self.metrics.precision_at_10 * 100.0));
        report.push_str(&format!("- **Recall@10**:    {:.1}%\n", self.metrics.recall_at_10 * 100.0));
        report.push_str(&format!("- **Recall@20**:    {:.1}%\n", self.metrics.recall_at_20 * 100.0));
        report.push_str(&format!("- **MRR**:          {:.3}\n", self.metrics.mrr));
        report.push_str(&format!("- **NDCG@10**:      {:.3}\n", self.metrics.ndcg_at_10));
        report.push_str(&format!("- **Exclusion**:    {:.1}%\n", self.metrics.exclusion_rate * 100.0));
        report.push_str(&format!("- **Critical in Top-3**: {:.1}%\n", self.metrics.critical_in_top_3 * 100.0));
        report.push_str(&format!("- **High in Top-5**:     {:.1}%\n\n", self.metrics.high_in_top_5 * 100.0));
        
        report.push_str("## Top 10 Results\n\n");
        for (i, file) in self.ranked_files.iter().take(10).enumerate() {
            let rank = i + 1;
            let path_str = file.path.to_str().unwrap_or("?");
            
            let status = if let Some((_, expected)) = self.found_expected.iter().find(|(r, _)| *r == rank) {
                format!("✅ (expected: {:?})", expected.priority)
            } else if self.found_excluded.iter().any(|(r, _)| *r == rank) {
                "❌ (excluded)".to_string()
            } else {
                "❌ (not expected)".to_string()
            };
            
            report.push_str(&format!("{}. {} {} (score: {:.3})\n", rank, path_str, status, file.score));
        }
        
        if !self.missing_expected.is_empty() {
            report.push_str("\n## Missing Expected Files\n\n");
            for expected in &self.missing_expected {
                report.push_str(&format!("- **{}** ({:?}): {}\n", expected.path, expected.priority, expected.reason));
            }
        }
        
        report
    }
}

/// Load test case from TOML file
pub fn load_test_case(path: &Path) -> Result<TestCase> {
    let content = std::fs::read_to_string(path)?;
    let test_case: TestCase = toml::from_str(&content)?;
    Ok(test_case)
}

/// Load all test cases from a directory
pub fn load_test_cases(dir: &Path) -> Result<Vec<(PathBuf, TestCase)>> {
    let mut test_cases = Vec::new();
    
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            match load_test_case(&path) {
                Ok(test_case) => test_cases.push((path, test_case)),
                Err(e) => eprintln!("Warning: Failed to load {}: {}", path.display(), e),
            }
        }
    }
    
    Ok(test_cases)
}
