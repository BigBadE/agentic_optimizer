//! Score collection, computation, and reciprocal rank fusion.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::embedding::SearchResult;

use super::content_scoring::{
    apply_exact_match_bonus, calculate_chunk_quality, calculate_pattern_boost,
    calculate_query_file_alignment,
};
use super::file_scoring::calculate_file_boost;
use super::query_analysis::calculate_adaptive_weights;

/// Helper struct to hold vector score data
pub struct VectorScoreData {
    pub scores: HashMap<PathBuf, f32>,
    pub previews: HashMap<PathBuf, String>,
    pub max_score: f32,
}

/// Parameters for computing combined scores
pub struct ScoreComputationParams<'score> {
    pub bm25_scores: &'score HashMap<PathBuf, f32>,
    pub vector_scores: &'score HashMap<PathBuf, f32>,
    pub previews: &'score HashMap<PathBuf, String>,
    pub max_bm25: f32,
    pub max_vector: f32,
    pub bm25_weight: f32,
    pub vector_weight: f32,
}

/// Collect BM25 scores into a map and find max score
pub fn collect_bm25_scores(
    bm25_results: &[(PathBuf, f32)],
    paths: &mut HashSet<PathBuf>,
) -> (HashMap<PathBuf, f32>, f32) {
    let mut bm25_scores = HashMap::default();
    let mut max_bm25 = 0.0f32;

    for (path, score) in bm25_results {
        if *score > 0.0 {
            bm25_scores.insert(path.clone(), *score);
            max_bm25 = max_bm25.max(*score);
            paths.insert(path.clone());
        }
    }
    (bm25_scores, max_bm25)
}

/// Collect vector scores and previews into maps and find max score
pub fn collect_vector_scores(
    vector_results: &[SearchResult],
    paths: &mut HashSet<PathBuf>,
) -> VectorScoreData {
    let mut vector_scores = HashMap::default();
    let mut previews = HashMap::default();
    let mut max_vector = 0.0f32;

    for result in vector_results {
        if result.score > 0.0 {
            vector_scores.insert(result.file_path.clone(), result.score);
            max_vector = max_vector.max(result.score);
            paths.insert(result.file_path.clone());
        }
        previews.insert(result.file_path.clone(), result.preview.clone());
    }
    VectorScoreData {
        scores: vector_scores,
        previews,
        max_score: max_vector,
    }
}

/// Compute the final combined score for a search result
pub fn compute_combined_score(
    path: &PathBuf,
    query: &str,
    score_params: &ScoreComputationParams<'_>,
) -> SearchResult {
    let bm25_scores = score_params.bm25_scores;
    let vector_scores = score_params.vector_scores;
    let previews = score_params.previews;
    let max_bm25 = score_params.max_bm25;
    let max_vector = score_params.max_vector;
    let bm25_weight = score_params.bm25_weight;
    let vector_weight = score_params.vector_weight;
    let bm25_raw = bm25_scores.get(path).copied().unwrap_or(0.0);
    let vector_raw = vector_scores.get(path).copied().unwrap_or(0.0);

    let bm25_normalized = if max_bm25 > 0.0 {
        bm25_raw / max_bm25
    } else {
        0.0
    };
    let vector_normalized = if max_vector > 0.0 {
        vector_raw / max_vector
    } else {
        0.0
    };

    // Apply minimum BM25 threshold - weak matches don't contribute (tuned: 0.75)
    let mut bm25_contribution = if bm25_raw >= 0.75 {
        bm25_normalized * bm25_weight
    } else {
        0.0
    };

    bm25_contribution = apply_exact_match_bonus(bm25_contribution, query, previews.get(path));
    let vector_contribution = vector_normalized * vector_weight;

    let preview = previews.get(path).cloned().unwrap_or_default();
    let file_boost = calculate_file_boost(path);
    let query_alignment = calculate_query_file_alignment(query, path, &preview);
    let pattern_boost = calculate_pattern_boost(&preview);
    let chunk_quality = calculate_chunk_quality(&preview);
    let combined_score = (bm25_contribution + vector_contribution)
        * file_boost
        * query_alignment
        * pattern_boost
        * chunk_quality;

    SearchResult {
        file_path: path.clone(),
        score: combined_score,
        preview,
        bm25_score: (bm25_contribution > 0.0).then_some(bm25_contribution),
        vector_score: (vector_contribution > 0.0).then_some(vector_contribution),
    }
}

/// Combine BM25 keyword scores with vector semantic scores using weighted normalization
pub fn reciprocal_rank_fusion(
    query: &str,
    bm25_results: &[(PathBuf, f32)],
    vector_results: &[SearchResult],
    top_k: usize,
) -> Vec<SearchResult> {
    let (bm25_weight, vector_weight) = calculate_adaptive_weights(query);
    let mut paths = HashSet::default();

    let (bm25_scores, max_bm25) = collect_bm25_scores(bm25_results, &mut paths);
    let vector_data = collect_vector_scores(vector_results, &mut paths);

    let score_params = ScoreComputationParams {
        bm25_scores: &bm25_scores,
        vector_scores: &vector_data.scores,
        previews: &vector_data.previews,
        max_bm25,
        max_vector: vector_data.max_score,
        bm25_weight,
        vector_weight,
    };

    let mut combined: Vec<SearchResult> = paths
        .into_iter()
        .map(|path| compute_combined_score(&path, query, &score_params))
        .collect();

    combined.sort_by(|result_a, result_b| {
        result_b
            .score
            .partial_cmp(&result_a.score)
            .unwrap_or(Ordering::Equal)
    });

    if let Some(max_score) = combined.first().map(|result| result.score)
        && max_score > 0.0
    {
        for result in &mut combined {
            result.score /= max_score;
            if let Some(bm25_score) = result.bm25_score.as_mut() {
                *bm25_score /= max_score;
            }
            if let Some(vector_score) = result.vector_score.as_mut() {
                *vector_score /= max_score;
            }
        }
    }

    combined.truncate(top_k);

    combined
}
