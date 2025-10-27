//! Chunk splitting logic for large Rust items.

use super::extract_rust_item_indices;
use crate::embedding::chunking::{FileChunk, MAX_CHUNK_TOKENS, MIN_CHUNK_TOKENS, estimate_tokens};
use std::mem;

/// Break an impl block into individual function chunks
pub fn chunk_impl_into_functions(
    file_path: &str,
    lines: &[&str],
    start_idx: usize,
    end_idx: usize,
    _base_line: usize,
) -> Vec<FileChunk> {
    let mut chunks = Vec::default();
    let mut index = start_idx;
    let mut buffer = String::default();

    // Skip the impl line itself
    while index <= end_idx
        && !lines[index].trim().starts_with("fn ")
        && !lines[index].trim().starts_with("pub fn ")
    {
        index += 1;
    }

    while index <= end_idx {
        let trimmed = lines[index].trim();

        if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
            let (fn_start, fn_end, fn_id, _) = extract_rust_item_indices(lines, index);

            // Build string using buffer
            buffer.clear();
            for (line_idx, line) in lines.iter().enumerate().take(fn_end + 1).skip(fn_start) {
                if line_idx > fn_start {
                    buffer.push('\n');
                }
                buffer.push_str(line);
            }

            let tokens = estimate_tokens(&buffer);

            if tokens >= MIN_CHUNK_TOKENS {
                let total_lines = lines.len();
                let start_line = fn_start + 1;
                let end_line_1 = (fn_end + 1).min(total_lines);
                let start_line_clamped = start_line.max(1).min(end_line_1);
                chunks.push(FileChunk::new(
                    file_path.to_owned(),
                    mem::take(&mut buffer),
                    fn_id,
                    start_line_clamped,
                    end_line_1,
                ));
            }
            // Skip functions below minimum

            index = fn_end + 1;
        } else {
            index += 1;
        }
    }

    chunks
}

/// Force split a large chunk on empty lines
pub fn force_split_large_chunk(
    file_path: &str,
    lines: &[&str],
    start_idx: usize,
    end_idx: usize,
    base_id: &str,
) -> Vec<FileChunk> {
    let mut chunks = Vec::default();
    let mut buffer = String::default();
    let mut chunk_start = start_idx;
    let mut line_count = 0;
    let mut part_num = 1;
    let mut last_empty_line = None;

    for line_idx in start_idx..=end_idx {
        if line_count > 0 {
            buffer.push('\n');
        }
        buffer.push_str(lines[line_idx]);
        line_count += 1;

        let tokens = estimate_tokens(&buffer);

        if lines[line_idx].trim().is_empty() {
            last_empty_line = Some((line_idx, tokens));
        }

        if let Some(empty_idx) = should_force_split(tokens, last_empty_line) {
            let meta = EmitMeta {
                file_path,
                base_id,
                part_num,
                total_lines: lines.len(),
            };
            emit_chunk_from_range(&meta, lines, chunk_start, empty_idx, &mut chunks);
            rebuild_buffer_after_split(lines, empty_idx, line_idx, &mut buffer, &mut line_count);
            part_num += 1;
            chunk_start = empty_idx + 1;
            last_empty_line = None;
        }
    }

    let meta = EmitMeta {
        file_path,
        base_id,
        part_num,
        total_lines: lines.len(),
    };
    flush_remaining(&meta, buffer, chunk_start, end_idx, &mut chunks);
    chunks
}

fn should_force_split(tokens: usize, last_empty: Option<(usize, usize)>) -> Option<usize> {
    if let Some((empty_idx, empty_tokens)) = last_empty
        && tokens > MAX_CHUNK_TOKENS
        && empty_tokens >= MIN_CHUNK_TOKENS
    {
        return Some(empty_idx);
    }
    None
}

struct EmitMeta<'life> {
    file_path: &'life str,
    base_id: &'life str,
    part_num: usize,
    total_lines: usize,
}

fn emit_chunk_from_range(
    meta: &EmitMeta,
    lines: &[&str],
    start: usize,
    end: usize,
    chunks: &mut Vec<FileChunk>,
) {
    let mut split_content = String::default();
    for (idx, line) in lines.iter().enumerate().take(end + 1).skip(start) {
        if idx > start {
            split_content.push('\n');
        }
        split_content.push_str(line);
    }
    let total_lines = lines.len();
    let start_line = start + 1;
    let end_line_1 = (end + 1).min(total_lines);
    let start_line_clamped = start_line.max(1).min(end_line_1);
    chunks.push(FileChunk::new(
        meta.file_path.to_owned(),
        split_content,
        format!("{} (part {})", meta.base_id, meta.part_num),
        start_line_clamped,
        end_line_1,
    ));
}

fn rebuild_buffer_after_split(
    lines: &[&str],
    split_end: usize,
    current_idx: usize,
    buffer: &mut String,
    line_count: &mut usize,
) {
    buffer.clear();
    *line_count = 0;
    for line in lines.iter().take(current_idx + 1).skip(split_end + 1) {
        if *line_count > 0 {
            buffer.push('\n');
        }
        buffer.push_str(line);
        *line_count += 1;
    }
}

fn flush_remaining(
    meta: &EmitMeta,
    buffer: String,
    chunk_start: usize,
    end_idx: usize,
    chunks: &mut Vec<FileChunk>,
) {
    if !buffer.is_empty() {
        let tokens = estimate_tokens(&buffer);
        if tokens >= MIN_CHUNK_TOKENS {
            let start_line = chunk_start + 1;
            let end_line_1 = (end_idx + 1).min(meta.total_lines);
            let start_line_clamped = start_line.max(1).min(end_line_1);
            chunks.push(FileChunk::new(
                meta.file_path.to_owned(),
                buffer,
                format!("{} (part {})", meta.base_id, meta.part_num),
                start_line_clamped,
                end_line_1,
            ));
        } else if let Some(last_chunk) = chunks.last_mut() {
            last_chunk.content.push('\n');
            last_chunk.content.push_str(&buffer);
            let new_end = end_idx + 1;
            if new_end > last_chunk.end_line {
                last_chunk.end_line = new_end.min(meta.total_lines);
            }
        }
    }
}

/// Force split a chunk by line count when no good split points exist
/// This ensures we never skip large chunks entirely
pub fn force_split_by_line_count(
    file_path: &str,
    lines: &[&str],
    start_idx: usize,
    end_idx: usize,
    base_id: &str,
) -> Vec<FileChunk> {
    let mut chunks = Vec::default();
    let mut part_num = 1;
    let mut current_start = start_idx;
    let mut buffer = String::default();

    while current_start <= end_idx {
        buffer.clear();
        let mut current_end = current_start;

        // Build chunks dynamically, ensuring they don't exceed MAX_CHUNK_TOKENS
        while current_end <= end_idx {
            // Build a temporary buffer to test if adding this line would exceed MAX
            let mut test_buffer = buffer.clone();
            if current_end > current_start {
                test_buffer.push('\n');
            }
            test_buffer.push_str(lines[current_end]);

            let test_tokens = estimate_tokens(&test_buffer);

            // If adding this line would exceed MAX_CHUNK_TOKENS, stop here
            if test_tokens > MAX_CHUNK_TOKENS {
                // Only stop if we have at least one line in the chunk
                if current_end > current_start {
                    break;
                }
                // For single huge lines, we have to include it (ensure progress)
                buffer = test_buffer;
                break;
            }

            // Accept this line into the buffer
            buffer = test_buffer;
            current_end += 1;
        }

        // Ensure we make progress even with huge single lines
        if current_end == current_start {
            current_end = current_start;
        }

        let final_tokens = estimate_tokens(&buffer);

        // Only add if not empty and meets minimum
        if !buffer.trim().is_empty() && final_tokens >= MIN_CHUNK_TOKENS {
            let total_lines = lines.len();
            let start_line = current_start + 1;
            let end_line_1 = (current_end + 1).min(total_lines);
            let start_line_clamped = start_line.max(1).min(end_line_1);
            chunks.push(FileChunk::new(
                file_path.to_owned(),
                buffer.clone(),
                format!("{base_id} (forced part {part_num})"),
                start_line_clamped,
                end_line_1,
            ));
        }
        part_num += 1;
        current_start = current_end + 1;
    }

    chunks
}
