//! Graph-based scoring and filtering utilities.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::context_inclusion::MIN_SIMILARITY_SCORE;
use crate::embedding::SearchResult;

/// Apply graph-based boost to results
pub fn apply_graph_boost(results: &mut [SearchResult], graph: &HashMap<PathBuf, Vec<PathBuf>>) {
    // Build reverse graph (who imports this file)
    let mut reverse_graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::default();
    for (file, imports) in graph {
        for imported in imports {
            reverse_graph
                .entry(imported.clone())
                .or_default()
                .push(file.clone());
        }
    }

    // Boost files based on graph relationships
    for result in &mut *results {
        let mut graph_boost = 1.0;

        // Boost if many files import this (central/important)
        if let Some(importers) = reverse_graph.get(&result.file_path) {
            let import_count = importers.len();
            if import_count > 5 {
                graph_boost *= 1.3; // Heavily imported = important
            } else if import_count > 2 {
                graph_boost *= 1.15; // Moderately imported
            }
        }

        // Boost if this file imports many others (coordinator/orchestrator)
        if let Some(imports) = graph.get(&result.file_path) {
            let import_count = imports.len();
            if import_count > 10 {
                graph_boost *= 1.2; // Orchestrator file
            }
        }

        result.score *= graph_boost;
    }
}

/// Filter results by minimum similarity score
pub fn filter_by_min_score(results: Vec<SearchResult>) -> Vec<SearchResult> {
    results
        .into_iter()
        .filter(|result| result.score >= MIN_SIMILARITY_SCORE)
        .collect()
}

/// Build import graph from Rust source files.
/// Currently returns an empty graph when rust-analyzer backend is not available.
pub fn build_import_graph(_files: &[PathBuf]) -> HashMap<PathBuf, Vec<PathBuf>> {
    // Graph ranking is an enhancement; safe to return empty graph.
    HashMap::default()
}
