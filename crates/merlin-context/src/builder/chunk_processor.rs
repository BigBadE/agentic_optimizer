//! Chunk processing utilities for extracting and merging code chunks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use merlin_core::{Error, FileContext, Result};

use crate::context_inclusion::{FilePriority, MIN_SIMILARITY_SCORE, PrioritizedFile};
use crate::embedding::SearchResult;

/// Type alias for file chunks map
pub type FileChunksMap = HashMap<PathBuf, Vec<(usize, usize, f32)>>;

/// Type alias for file score information
pub type FileScoreInfo = (PathBuf, f32, Option<f32>, Option<f32>);

/// Merge overlapping chunks considering context expansion
pub fn merge_overlapping_chunks(chunks: Vec<(usize, usize, f32)>) -> Vec<(usize, usize, f32)> {
    const CONTEXT_LINES: usize = 50;

    if chunks.is_empty() {
        return Vec::new();
    }

    let mut merged = Vec::new();
    let mut current_start = chunks[0].0;
    let mut current_end = chunks[0].1;
    let mut max_score = chunks[0].2;

    for (start, end, score) in chunks.into_iter().skip(1) {
        // Check if chunks overlap when considering context expansion
        // Two chunks overlap if: start - CONTEXT <= current_end + CONTEXT
        let expanded_current_end = current_end + CONTEXT_LINES;
        let expanded_start = start.saturating_sub(CONTEXT_LINES);

        if expanded_start <= expanded_current_end {
            // Merge: extend current chunk
            current_end = current_end.max(end);
            max_score = max_score.max(score);
        } else {
            // No overlap: save current and start new
            merged.push((current_start, current_end, max_score));
            current_start = start;
            current_end = end;
            max_score = score;
        }
    }

    // Add the last chunk
    merged.push((current_start, current_end, max_score));

    merged
}

/// Extract a chunk with surrounding context (only for code files)
///
/// # Errors
/// Returns an error if file cannot be read
pub fn extract_chunk_with_context(
    file_path: &PathBuf,
    start_line: usize,
    end_line: usize,
    include_context: bool,
) -> Result<FileContext> {
    use std::fs;

    let content = fs::read_to_string(file_path)
        .map_err(|read_error| Error::Other(format!("Failed to read file: {read_error}")))?;

    let lines: Vec<&str> = content.lines().collect();

    // Calculate context window (±50 lines for code, exact chunk for text)
    let (context_start, context_end) = if include_context {
        const CONTEXT_LINES: usize = 50;
        (
            (start_line.saturating_sub(CONTEXT_LINES)).max(1),
            (end_line + CONTEXT_LINES).min(lines.len()),
        )
    } else {
        // Text files: exact chunk only
        (start_line, end_line)
    };

    // Extract lines with context
    let chunk_lines: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(line_index, _)| *line_index + 1 >= context_start && *line_index < context_end)
        .map(|(_, line)| *line)
        .collect();

    let chunk_content = chunk_lines.join("\n");

    // Create a marker to show the actual matched chunk (only if we added context)
    let marker = if include_context && (context_start < start_line || context_end > end_line) {
        format!("\n\n--- Matched chunk: lines {start_line}-{end_line} ---\n")
    } else {
        String::default()
    };

    let final_content = if !marker.is_empty() {
        format!("--- Context: lines {context_start}-{context_end} ---\n{chunk_content}{marker}")
    } else if include_context {
        format!("--- Context: lines {context_start}-{context_end} ---\n{chunk_content}")
    } else {
        // Text files without context - still show line range
        format!("--- Lines {context_start}-{context_end} ---\n{chunk_content}")
    };

    Ok(FileContext {
        path: file_path.clone(),
        content: final_content,
    })
}

/// Check if a chunk should be included based on size and score
pub fn should_include_chunk(tokens: usize, score: f32) -> bool {
    if tokens < 50 {
        return false; // Always filter tiny chunks
    }
    if tokens < 100 && score < 0.7 {
        return false; // Filter small low-score chunks
    }
    true
}

/// Check if a file is a code file (not documentation/text)
pub fn is_code_file(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    ext.to_str().is_some_and(|ext| {
        matches!(
            ext,
            "rs" | "py"
                | "js"
                | "ts"
                | "jsx"
                | "tsx"
                | "java"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "go"
                | "rb"
                | "php"
                | "cs"
                | "swift"
                | "kt"
                | "scala"
                | "toml"
                | "yaml"
                | "yml"
                | "json"
                | "xml"
        )
    })
}

/// Helper to process merged chunks for a single file.
pub fn process_merged_chunks(
    file_path: &PathBuf,
    merged: Vec<(usize, usize, f32)>,
    is_code: bool,
    search_prioritized: &mut Vec<PrioritizedFile>,
) {
    for (start, end, score) in merged {
        match extract_chunk_with_context(file_path, start, end, is_code) {
            Ok(chunk_ctx) => {
                let priority = if is_code {
                    FilePriority::High
                } else {
                    FilePriority::Medium
                };

                search_prioritized.push(PrioritizedFile::with_score(chunk_ctx, priority, score));
            }
            Err(extract_error) => {
                tracing::warn!(
                    "Failed to extract chunk from {}: {extract_error}",
                    file_path.display()
                );
            }
        }
    }
}

/// Processes search results into prioritized file chunks.
pub fn process_search_results(
    project_root: &Path,
    semantic_matches: &[SearchResult],
) -> (Vec<PrioritizedFile>, Vec<FileScoreInfo>) {
    // Filter out low-quality small chunks
    let filtered_matches: Vec<_> = semantic_matches
        .iter()
        .filter(|result| {
            if let Some(path_str) = result.file_path.to_str()
                && let Some((_, range_part)) = path_str.rsplit_once(':')
                && let Some((start_str, end_str)) = range_part.split_once('-')
                && let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>())
            {
                let line_count = end - start;
                let estimated_tokens = line_count * 10;
                return should_include_chunk(estimated_tokens, result.score);
            }
            true
        })
        .collect();

    tracing::info!(
        "After quality filtering: {} chunks (removed {} low-quality)",
        filtered_matches.len(),
        semantic_matches.len() - filtered_matches.len()
    );

    // Group chunks by file
    let mut file_chunks: FileChunksMap = HashMap::new();

    for result in &filtered_matches {
        if let Some(path_str) = result.file_path.to_str()
            && let Some((file_part, range_part)) = path_str.rsplit_once(':')
        {
            // Convert relative path to absolute by joining with project root
            let relative_path = PathBuf::from(file_part);
            let absolute_path = project_root.join(relative_path);
            if let Some((start_str, end_str)) = range_part.split_once('-')
                && let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>())
            {
                file_chunks
                    .entry(absolute_path)
                    .or_default()
                    .push((start, end, result.score));
            }
        }
    }

    // Track scores for display and apply penalties (convert to absolute paths)
    // Apply 0.5x penalty to non-source code files
    let file_scores: Vec<FileScoreInfo> = filtered_matches
        .iter()
        .filter_map(|result| {
            let path_str = result.file_path.to_str()?;
            let (file_part, _) = path_str.rsplit_once(':')?;
            let relative_path = PathBuf::from(file_part);
            let absolute_path = project_root.join(relative_path);

            // Apply penalty to non-source files
            let is_source = is_code_file(&absolute_path);
            let score_multiplier = if is_source { 1.0 } else { 0.5 };
            let adjusted_score = result.score * score_multiplier;

            // Filter out files that fall below threshold after penalty
            if adjusted_score < MIN_SIMILARITY_SCORE {
                return None;
            }

            Some((
                absolute_path,
                adjusted_score,
                result.bm25_score.map(|score| score * score_multiplier),
                result.vector_score.map(|score| score * score_multiplier),
            ))
        })
        .collect();

    // Merge overlapping chunks and extract (only for files that passed score threshold)
    let mut search_prioritized = Vec::new();

    for (file_path, mut chunks) in file_chunks {
        // Check if this file passed the score threshold
        let file_passed_threshold = file_scores.iter().any(|(path, _, _, _)| path == &file_path);
        if !file_passed_threshold {
            continue;
        }

        chunks.sort_by_key(|(start, _, _)| *start);
        let merged = merge_overlapping_chunks(chunks);
        let is_code = is_code_file(&file_path);

        process_merged_chunks(&file_path, merged, is_code, &mut search_prioritized);
    }

    (search_prioritized, file_scores)
}
