//! Search result scoring and ranking logic.

mod content_scoring;
mod file_scoring;
mod fusion;
mod graph;
mod query_analysis;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::embedding::SearchResult;

/// Scoring utilities
pub struct ScoringUtils;

impl ScoringUtils {
    /// Check if file content has imports matching query terms
    pub fn boost_by_imports(content: &str, query: &str) -> f32 {
        content_scoring::boost_by_imports(content, query)
    }

    /// Combine BM25 keyword scores with vector semantic scores using weighted normalization
    pub fn reciprocal_rank_fusion(
        query: &str,
        bm25_results: &[(PathBuf, f32)],
        vector_results: &[SearchResult],
        top_k: usize,
    ) -> Vec<SearchResult> {
        fusion::reciprocal_rank_fusion(query, bm25_results, vector_results, top_k)
    }

    /// Apply graph-based boost to results
    pub fn apply_graph_boost(results: &mut [SearchResult], graph: &HashMap<PathBuf, Vec<PathBuf>>) {
        graph::apply_graph_boost(results, graph);
    }

    /// Filter results by minimum similarity score
    pub fn filter_by_min_score(results: Vec<SearchResult>) -> Vec<SearchResult> {
        graph::filter_by_min_score(results)
    }

    /// Build import graph from Rust source files.
    /// Currently returns an empty graph when rust-analyzer backend is not available.
    pub fn build_import_graph(files: &[PathBuf]) -> HashMap<PathBuf, Vec<PathBuf>> {
        graph::build_import_graph(files)
    }
}
