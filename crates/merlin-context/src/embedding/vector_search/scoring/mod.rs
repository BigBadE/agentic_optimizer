//! Search result scoring and ranking logic.

mod content_scoring;
mod file_scoring;
mod fusion;
mod graph;
mod query_analysis;

// Re-export public types
pub use fusion::{ScoreComputationParams, VectorScoreData};

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::embedding::SearchResult;

/// Scoring utilities
pub struct ScoringUtils;

#[allow(
    dead_code,
    reason = "Utility methods used selectively by scoring algorithms"
)]
impl ScoringUtils {
    /// Detect query intent from keywords
    pub fn detect_query_intent(query: &str) -> &'static str {
        query_analysis::detect_query_intent(query)
    }

    /// Calculate adaptive weights based on query characteristics
    pub fn calculate_adaptive_weights(query: &str) -> (f32, f32) {
        query_analysis::calculate_adaptive_weights(query)
    }

    /// Calculate file type and location boost
    pub fn calculate_file_boost(path: &Path) -> f32 {
        file_scoring::calculate_file_boost(path)
    }

    /// Calculate query-file alignment based on keyword matching
    pub fn calculate_query_file_alignment(query: &str, file_path: &Path, preview: &str) -> f32 {
        content_scoring::calculate_query_file_alignment(query, file_path, preview)
    }

    /// Calculate pattern-based importance boost for code structure
    pub fn calculate_pattern_boost(preview: &str) -> f32 {
        content_scoring::calculate_pattern_boost(preview)
    }

    /// Calculate chunk quality boost based on content
    pub fn calculate_chunk_quality(preview: &str) -> f32 {
        content_scoring::calculate_chunk_quality(preview)
    }

    /// Check if file content has imports matching query terms
    pub fn boost_by_imports(content: &str, query: &str) -> f32 {
        content_scoring::boost_by_imports(content, query)
    }

    /// Apply exact match bonus if preview contains special tokens from query
    pub fn apply_exact_match_bonus(
        bm25_contribution: f32,
        query: &str,
        preview: Option<&String>,
    ) -> f32 {
        content_scoring::apply_exact_match_bonus(bm25_contribution, query, preview)
    }

    /// Collect BM25 scores into a map and find max score
    pub fn collect_bm25_scores(
        bm25_results: &[(PathBuf, f32)],
        paths: &mut HashSet<PathBuf>,
    ) -> (HashMap<PathBuf, f32>, f32) {
        fusion::collect_bm25_scores(bm25_results, paths)
    }

    /// Collect vector scores and previews into maps and find max score
    pub fn collect_vector_scores(
        vector_results: &[SearchResult],
        paths: &mut HashSet<PathBuf>,
    ) -> VectorScoreData {
        fusion::collect_vector_scores(vector_results, paths)
    }

    /// Compute the final combined score for a search result
    pub fn compute_combined_score(
        path: &PathBuf,
        query: &str,
        score_params: &ScoreComputationParams<'_>,
    ) -> SearchResult {
        fusion::compute_combined_score(path, query, score_params)
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
