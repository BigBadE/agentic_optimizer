//! Rust code chunking - prioritizes innermost items (functions over impls).

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

/// Check if line starts a Rust item
fn is_rust_item_start(line: &str) -> bool {
    line.starts_with("pub fn ")
        || line.starts_with("fn ")
        || line.starts_with("async fn ")
        || line.starts_with("pub async fn ")
        || line.starts_with("pub struct ")
        || line.starts_with("struct ")
        || line.starts_with("pub enum ")
        || line.starts_with("enum ")
        || line.starts_with("pub trait ")
        || line.starts_with("trait ")
        || line.starts_with("impl ")
        || line.starts_with("impl<")
        || line.starts_with("pub mod ")
        || line.starts_with("mod ")
        || line.starts_with("pub const ")
        || line.starts_with("const ")
        || line.starts_with("pub static ")
        || line.starts_with("static ")
}

/// Extract a complete Rust item indices (handles brace matching)
fn extract_rust_item_indices(lines: &[&str], start: usize) -> (usize, usize, String, usize) {
    let identifier = extract_rust_identifier(lines[start].trim());
    let mut brace_depth = 0;
    let mut found_opening_brace = false;

    for (offset, line) in lines[start..].iter().enumerate() {
        for character in line.chars() {
            match character {
                '{' => {
                    brace_depth += 1;
                    found_opening_brace = true;
                }
                '}' => {
                    brace_depth -= 1;
                    if found_opening_brace && brace_depth == 0 {
                        let end_line = start + offset;
                        return (start, end_line, identifier, end_line);
                    }
                }
                _ => {}
            }
        }

        // Handle items without braces (like type aliases, consts)
        if !found_opening_brace && line.trim().ends_with(';') {
            let end_line = start + offset;
            return (start, end_line, identifier, end_line);
        }
    }

    (start, lines.len() - 1, identifier, lines.len() - 1)
}

/// Break an impl block into individual function chunks
fn chunk_impl_into_functions(
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
fn force_split_large_chunk(
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
fn force_split_by_line_count(
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
            if current_end > current_start {
                buffer.push('\n');
            }
            buffer.push_str(lines[current_end]);

            let tokens = estimate_tokens(&buffer);

            // If adding this line would exceed MAX, back up and emit the chunk
            if tokens > MAX_CHUNK_TOKENS && current_end > current_start {
                // Remove the last line we added
                if let Some(pos) = buffer.rfind('\n') {
                    buffer.truncate(pos);
                }
                current_end -= 1;
                break;
            }

            // If we're at MAX or close to it, emit the chunk
            if tokens >= MAX_CHUNK_TOKENS * 9 / 10 {
                break;
            }

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

/// Extract identifier from Rust item declaration
fn extract_rust_identifier(line: &str) -> String {
    let line = line.trim();

    // Remove pub/async/const/unsafe modifiers
    let line = line
        .trim_start_matches("pub ")
        .trim_start_matches("async ")
        .trim_start_matches("const ")
        .trim_start_matches("unsafe ");

    // Extract based on keyword
    if let Some(rest) = line.strip_prefix("fn ") {
        return format!(
            "fn {}",
            rest.split(&['(', '<', ' '][..]).next().unwrap_or("unknown")
        );
    }
    if let Some(rest) = line.strip_prefix("struct ") {
        return format!(
            "struct {}",
            rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown")
        );
    }
    if let Some(rest) = line.strip_prefix("enum ") {
        return format!(
            "enum {}",
            rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown")
        );
    }
    if let Some(rest) = line.strip_prefix("trait ") {
        return format!(
            "trait {}",
            rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown")
        );
    }
    if line.starts_with("impl ") || line.starts_with("impl<") {
        let impl_part = line.split('{').next().unwrap_or(line).trim();
        return format!(
            "impl {}",
            impl_part.strip_prefix("impl ").unwrap_or("").trim()
        );
    }
    if let Some(rest) = line.strip_prefix("mod ") {
        return format!(
            "mod {}",
            rest.split(&[' ', '{'][..]).next().unwrap_or("unknown")
        );
    }

    String::from("item")
}
