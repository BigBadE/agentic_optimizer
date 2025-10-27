//! Search and context building functionality.

use std::path::Path;

use merlin_core::{CoreResult as Result, FileContext};

use crate::context_inclusion::{ContextManager, MAX_CONTEXT_TOKENS, add_prioritized_files};
use crate::embedding::{SearchResult, VectorSearchManager};
use crate::query::QueryIntent;

use super::chunk_processor::{FileScoreInfo, process_search_results};

/// Performs hybrid search (BM25 + vector) for relevant code chunks.
///
/// # Errors
/// Returns an error if hybrid search fails
pub async fn perform_hybrid_search(
    vector_manager: Option<&VectorSearchManager>,
    query_text: &str,
) -> Result<Vec<SearchResult>> {
    merlin_deps::tracing::info!("Running hybrid search (BM25 + Vector)...");
    merlin_deps::tracing::info!("Using hybrid BM25 + Vector search for context");

    let semantic_matches = if let Some(manager) = &vector_manager {
        match manager.search(query_text, 50).await {
            Ok(results) => results,
            Err(search_error) => {
                merlin_deps::tracing::warn!("Hybrid search failed: {search_error}");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    if semantic_matches.is_empty() {
        merlin_deps::tracing::info!("Hybrid search: no results (store may be empty)");
    } else {
        merlin_deps::tracing::info!("Hybrid search found {} matches", semantic_matches.len());
        for (idx, result) in semantic_matches.iter().enumerate().take(10) {
            merlin_deps::tracing::debug!(
                "  {}. {} (score: {:.3})",
                idx + 1,
                result.file_path.display(),
                result.score
            );
        }
        if semantic_matches.len() > 10 {
            merlin_deps::tracing::debug!("  ... and {} more", semantic_matches.len() - 10);
        }
    }

    merlin_deps::tracing::info!("Hybrid search complete");
    Ok(semantic_matches)
}

/// Use hybrid search to intelligently gather context
///
/// # Errors
/// Returns an error if hybrid search fails
pub async fn use_subagent_for_context(
    vector_manager: Option<&VectorSearchManager>,
    project_root: &Path,
    _intent: &QueryIntent,
    query_text: &str,
) -> Result<Vec<FileContext>> {
    // Perform hybrid search
    let semantic_matches = perform_hybrid_search(vector_manager, query_text).await?;

    // Process search results into prioritized chunks
    let (search_prioritized, file_scores) = process_search_results(project_root, &semantic_matches);

    // Use context manager to add hybrid search results
    let mut context_mgr = ContextManager::new(MAX_CONTEXT_TOKENS);

    let added = add_prioritized_files(&mut context_mgr, search_prioritized);
    merlin_deps::tracing::info!(
        "Added {} chunks from hybrid search ({} tokens used)",
        added,
        context_mgr.token_count()
    );

    // List all chunks with their detailed scores
    merlin_deps::tracing::info!(
        "📁 Context files: {} files ({} tokens)",
        context_mgr.file_count(),
        context_mgr.token_count()
    );

    log_context_files(&context_mgr, &file_scores);

    let files = context_mgr.into_files();

    Ok(files)
}

/// Log detailed information about context files
fn log_context_files(context_mgr: &ContextManager, file_scores: &[FileScoreInfo]) {
    for (index, file) in context_mgr.files().iter().enumerate() {
        let tokens = ContextManager::estimate_tokens(&file.content);

        // Find the scores for this file
        let (total_score, bm25, vector) = file_scores
            .iter()
            .find(|(path, _, _, _)| path == &file.path)
            .map_or((0.0, None, None), |(_, total, bm25, vector)| {
                (*total, *bm25, *vector)
            });

        // Extract section info from content
        let section_info = file.content.lines().next().map_or_else(
            || "chunk".to_owned(),
            |first_line| {
                if first_line.starts_with("--- Context: lines") {
                    // Code file with context
                    first_line
                        .trim_start_matches("--- Context: lines ")
                        .trim_end_matches(" ---")
                        .to_owned()
                } else if first_line.starts_with("--- Lines") {
                    // Text file without context
                    first_line
                        .trim_start_matches("--- Lines ")
                        .trim_end_matches(" ---")
                        .to_owned()
                } else if file.content.lines().count() < 100 {
                    // Small content without markers is likely a chunk
                    format!("chunk (~{} lines)", file.content.lines().count())
                } else {
                    "full file".to_owned()
                }
            },
        );

        // Format score display
        let score_display = match (bm25, vector) {
            (Some(bm25_score), Some(vec_score)) => {
                format!("total:{total_score:.3} bm25:{bm25_score:.3} vec:{vec_score:.3}")
            }
            (Some(bm25_score), None) => {
                format!("total:{total_score:.3} bm25:{bm25_score:.3} vec:N/A")
            }
            (None, Some(vec_score)) => {
                format!("total:{total_score:.3} bm25:N/A vec:{vec_score:.3}")
            }
            (None, None) => format!("total:{total_score:.3} bm25:N/A vec:N/A"),
        };

        merlin_deps::tracing::info!(
            "  [{index}] {} | {} | {} tok | {}",
            file.path.display(),
            section_info,
            tokens,
            score_display
        );
    }
}
