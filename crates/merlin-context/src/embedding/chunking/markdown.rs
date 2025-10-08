//! Markdown chunking by headers with token-based limits.

use super::{FileChunk, MAX_CHUNK_TOKENS, MIN_CHUNK_TOKENS, OPTIMAL_MIN_TOKENS, estimate_tokens};
use std::mem::take;

/// Context for pushing chunks
struct PushContext<'ctx> {
    /// Chunk collection
    chunks: &'ctx mut Vec<FileChunk>,
    /// File path
    file_path: &'ctx str,
    /// Buffer to take content from
    buffer: &'ctx mut String,
    /// Header text
    header: &'ctx str,
    /// Start line
    start: usize,
    /// End line
    end: usize,
}

/// Helper to push a chunk if buffer is not empty
fn push_chunk_if_valid(push_ctx: &mut PushContext<'_>) {
    if !push_ctx.buffer.trim().is_empty() {
        push_ctx.chunks.push(FileChunk::new(
            push_ctx.file_path.to_owned(),
            take(push_ctx.buffer),
            push_ctx.header.to_owned(),
            push_ctx.start,
            push_ctx.end,
        ));
    }
}

/// Extract header information from a markdown line
fn extract_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    trimmed.starts_with('#').then(|| {
        let level = trimmed
            .chars()
            .take_while(|character| *character == '#')
            .count();
        let text = trimmed.trim_start_matches('#').trim();
        format!("{} {}", "#".repeat(level), text)
    })
}

/// Check if adding a line would exceed `MAX_CHUNK_TOKENS`
fn would_exceed_max(buffer: &str, line: &str, line_count: usize) -> bool {
    let mut temp = buffer.to_owned();
    if line_count > 0 {
        temp.push('\n');
    }
    temp.push_str(line);
    estimate_tokens(&temp) > MAX_CHUNK_TOKENS
}

/// Context for finalizing chunks
struct FinalizeContext<'ctx> {
    /// Chunk collection
    chunks: &'ctx mut Vec<FileChunk>,
    /// File path
    file_path: &'ctx str,
    /// Full file content
    file_content: &'ctx str,
    /// Remaining buffer
    buffer: String,
    /// Current header
    header: String,
    /// Start position
    start: usize,
    /// Total lines
    lines_len: usize,
}

/// Add remaining buffer content as a chunk if valid
fn finalize_chunks(finalize_ctx: FinalizeContext<'_>) {
    if !finalize_ctx.buffer.trim().is_empty() {
        let tokens = estimate_tokens(&finalize_ctx.buffer);
        if tokens >= MIN_CHUNK_TOKENS || finalize_ctx.chunks.is_empty() {
            finalize_ctx.chunks.push(FileChunk::new(
                finalize_ctx.file_path.to_owned(),
                finalize_ctx.buffer,
                finalize_ctx.header,
                finalize_ctx.start + 1,
                finalize_ctx.lines_len,
            ));
        }
    }

    if finalize_ctx.chunks.is_empty() {
        finalize_ctx.chunks.push(FileChunk::new(
            finalize_ctx.file_path.to_owned(),
            finalize_ctx.file_content.to_owned(),
            String::from("document"),
            1,
            finalize_ctx.lines_len,
        ));
    }
}

/// Chunk Markdown by headers with token-based limits
pub fn chunk_markdown(file_path: &str, content: &str) -> Vec<FileChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks: Vec<FileChunk> = Vec::default();
    let mut current_chunk_start: usize = 0;
    let mut buffer = String::default();
    let mut current_header = String::from("preamble");
    let mut line_count: usize = 0;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if let Some(header) = extract_header(line) {
            if line_count > 0 && estimate_tokens(&buffer) >= MIN_CHUNK_TOKENS {
                push_chunk_if_valid(&mut PushContext {
                    chunks: &mut chunks,
                    file_path,
                    buffer: &mut buffer,
                    header: &current_header,
                    start: current_chunk_start + 1,
                    end: index,
                });
                line_count = 0;
                current_chunk_start = index;
            }
            current_header = header;
        }

        if would_exceed_max(&buffer, line, line_count) && line_count > 0 {
            push_chunk_if_valid(&mut PushContext {
                chunks: &mut chunks,
                file_path,
                buffer: &mut buffer,
                header: &current_header,
                start: current_chunk_start + 1,
                end: index,
            });
            line_count = 0;
            current_chunk_start = index;
        }

        if line_count > 0 {
            buffer.push('\n');
        }
        buffer.push_str(line);
        line_count += 1;

        if trimmed.is_empty() && estimate_tokens(&buffer) >= OPTIMAL_MIN_TOKENS {
            push_chunk_if_valid(&mut PushContext {
                chunks: &mut chunks,
                file_path,
                buffer: &mut buffer,
                header: &current_header,
                start: current_chunk_start + 1,
                end: index + 1,
            });
            line_count = 0;
            current_chunk_start = index + 1;
        }
    }

    finalize_chunks(FinalizeContext {
        chunks: &mut chunks,
        file_path,
        file_content: content,
        buffer,
        header: current_header,
        start: current_chunk_start,
        lines_len: lines.len(),
    });
    chunks
}
