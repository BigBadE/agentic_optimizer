//! Rust code chunking - prioritizes innermost items (functions over impls).

#[path = "rust/detection.rs"]
mod detection;
#[path = "rust/identifier.rs"]
mod identifier;
#[path = "rust/splitting.rs"]
mod splitting;

use self::detection::{extract_rust_item_indices, is_rust_item_start};
use self::identifier::extract_rust_identifier;
use self::splitting::{
    chunk_impl_into_functions, force_split_by_line_count, force_split_large_chunk,
};
use super::{FileChunk, MAX_CHUNK_TOKENS, MIN_CHUNK_TOKENS, estimate_tokens};
use std::mem;

/// Chunk Rust code - prioritizes innermost items (functions over impls)
pub fn chunk_rust(file_path: String, content: &str) -> Vec<FileChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::default();
    let mut index = 0;
    let mut buffer = String::default();

    while index < lines.len() {
        let trimmed = lines[index].trim();

        // Detect item starts
        if is_rust_item_start(trimmed) {
            let (start_idx, end_idx, chunk_id, end_line) = extract_rust_item_indices(&lines, index);
            let refs = ItemRefs {
                file_path: &file_path,
                lines: &lines,
                chunk_id: &chunk_id,
            };
            let bounds = ItemBounds {
                start_idx,
                end_idx,
                end_line,
                current_line: index,
            };
            index = process_rust_item(&refs, &bounds, &mut buffer, &mut chunks);
        } else {
            index += 1;
        }
    }

    // If no chunks, fallback to generic chunking to respect MAX_CHUNK_TOKENS
    if chunks.is_empty() {
        return super::chunk_generic_code(file_path, content);
    }

    chunks
}

struct ItemBounds {
    start_idx: usize,
    end_idx: usize,
    end_line: usize,
    current_line: usize,
}

struct ItemRefs<'life> {
    file_path: &'life str,
    lines: &'life [&'life str],
    chunk_id: &'life str,
}

/// Process a single Rust item (function, struct, impl, etc.)
fn process_rust_item(
    refs: &ItemRefs,
    bounds: &ItemBounds,
    buffer: &mut String,
    chunks: &mut Vec<FileChunk>,
) -> usize {
    let file_path = refs.file_path;
    let lines = refs.lines;
    let chunk_id = refs.chunk_id;
    let start_idx = bounds.start_idx;
    let end_idx = bounds.end_idx;
    let end_line = bounds.end_line;
    let current_line = bounds.current_line;
    // Build string using buffer to avoid allocations
    buffer.clear();
    for (line_idx, line) in lines.iter().enumerate().take(end_idx + 1).skip(start_idx) {
        if line_idx > start_idx {
            buffer.push('\n');
        }
        buffer.push_str(line);
    }

    let tokens = estimate_tokens(buffer);

    // If chunk is too large, try to split it
    if tokens > MAX_CHUNK_TOKENS {
        if let Some(sub_chunks) = try_split_large_item(&TrySplitCtx {
            file_path,
            lines,
            start_idx,
            end_idx,
            chunk_id,
            base_line: current_line,
        }) {
            chunks.extend(sub_chunks);
            return end_line + 1;
        }

        // Force split by line count if no good split points found
        tracing::warn!(
            "Force-splitting large chunk {chunk_id} ({tokens} tokens) by line count - no optimal split points"
        );
        let forced_chunks =
            force_split_by_line_count(file_path, lines, start_idx, end_idx, chunk_id);
        chunks.extend(forced_chunks);
        return end_line + 1;
    }

    // Only add if meets minimum token requirement
    if !buffer.trim().is_empty() && tokens >= MIN_CHUNK_TOKENS {
        let total_lines = lines.len();
        let start_line = current_line + 1;
        let end_line_1 = (end_line + 1).min(total_lines);
        let start_line_clamped = start_line.max(1).min(end_line_1);
        chunks.push(FileChunk::new(
            file_path.to_owned(),
            mem::take(buffer),
            chunk_id.to_owned(),
            start_line_clamped,
            end_line_1,
        ));
    }
    // If below minimum, just skip it (will be part of surrounding context)

    end_line + 1
}

/// Try to split a large item into smaller chunks
struct TrySplitCtx<'life> {
    file_path: &'life str,
    lines: &'life [&'life str],
    start_idx: usize,
    end_idx: usize,
    chunk_id: &'life str,
    base_line: usize,
}

fn try_split_large_item(ctx: &TrySplitCtx) -> Option<Vec<FileChunk>> {
    let file_path = ctx.file_path;
    let lines = ctx.lines;
    let start_idx = ctx.start_idx;
    let end_idx = ctx.end_idx;
    let chunk_id = ctx.chunk_id;
    let base_line = ctx.base_line;
    if chunk_id.starts_with("impl ") {
        // Try to break impl into functions
        let sub_chunks = chunk_impl_into_functions(file_path, lines, start_idx, end_idx, base_line);
        if !sub_chunks.is_empty() {
            return Some(sub_chunks);
        }
    }

    // Force split on empty lines
    let sub_chunks = force_split_large_chunk(file_path, lines, start_idx, end_idx, chunk_id);
    if !sub_chunks.is_empty() {
        // Verify all sub-chunks are within limits
        let all_valid = sub_chunks.iter().all(|chunk| {
            let token_count = estimate_tokens(&chunk.content);
            token_count <= MAX_CHUNK_TOKENS
        });

        if all_valid {
            return Some(sub_chunks);
        }
    }

    None
}
