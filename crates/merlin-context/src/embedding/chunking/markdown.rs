//! Markdown chunking by headers with token-based limits.

use std::mem::take;
use super::{FileChunk, estimate_tokens, MIN_CHUNK_TOKENS, OPTIMAL_MIN_TOKENS};

/// Chunk Markdown by headers with token-based limits
#[must_use]
pub fn chunk_markdown(file_path: String, content: &str) -> Vec<FileChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks: Vec<FileChunk> = Vec::new();
    let mut current_chunk_start: usize = 0;
    let mut buffer = String::new();
    let mut current_header = String::from("preamble");
    let mut line_count: usize = 0;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            let header_level = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            let header_text = trimmed.trim_start_matches('#').trim();
            let tokens = estimate_tokens(&buffer);

            if line_count > 0 && tokens >= MIN_CHUNK_TOKENS {
                if !buffer.trim().is_empty() {
                    chunks.push(FileChunk::new(
                        file_path.clone(),
                        take(&mut buffer),
                        current_header.clone(),
                        current_chunk_start + 1,
                        index,
                    ));
                }
                line_count = 0;
                current_chunk_start = index;
            }

            current_header = format!("{} {}", "#".repeat(header_level), header_text);
        }

        if line_count > 0 { buffer.push('\n'); }
        buffer.push_str(line);
        line_count += 1;

        let tokens = estimate_tokens(&buffer);
        if trimmed.is_empty() && tokens >= OPTIMAL_MIN_TOKENS {
            if !buffer.trim().is_empty() {
                chunks.push(FileChunk::new(
                    file_path.clone(),
                    take(&mut buffer),
                    current_header.clone(),
                    current_chunk_start + 1,
                    index + 1,
                ));
            }
            line_count = 0;
            current_chunk_start = index + 1;
        }
    }

    if line_count > 0 {
        let tokens = estimate_tokens(&buffer);
        if !buffer.trim().is_empty() && (tokens >= MIN_CHUNK_TOKENS || chunks.is_empty()) {
            chunks.push(FileChunk::new(
                file_path.clone(),
                buffer,
                current_header,
                current_chunk_start + 1,
                lines.len(),
            ));
        }
    }

    if chunks.is_empty() {
        chunks.push(FileChunk::new(
            file_path,
            content.to_owned(),
            String::from("document"),
            1,
            lines.len(),
        ));
    }

    chunks
}
