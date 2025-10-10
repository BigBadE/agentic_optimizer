//! Config file chunking by top-level sections with token limits.

use super::{FileChunk, MAX_CHUNK_TOKENS, MIN_CHUNK_TOKENS, estimate_tokens};
use std::mem::take;

/// Chunk config files by top-level sections with token limits
pub fn chunk_config(file_path: String, content: &str) -> Vec<FileChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::default();
    let mut current_chunk_start = 0;
    let mut buffer = String::default();
    let mut current_section = String::from("root");
    let mut line_count = 0;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect section headers (e.g., [section] in TOML)
        let is_section_header = trimmed.starts_with('[') && trimmed.ends_with(']');

        // Check if adding this line would exceed MAX_CHUNK_TOKENS
        let mut test_buffer = buffer.clone();
        if line_count > 0 {
            test_buffer.push('\n');
        }
        test_buffer.push_str(line);
        let tokens_with_line = estimate_tokens(&test_buffer);

        // Force chunk if we'd exceed MAX_CHUNK_TOKENS, or split on section header if we're past OPTIMAL_MAX_TOKENS
        let should_chunk = if tokens_with_line > MAX_CHUNK_TOKENS {
            true
        } else if is_section_header {
            let tokens = estimate_tokens(&buffer);
            line_count > 0 && tokens >= MIN_CHUNK_TOKENS
        } else {
            false
        };

        if should_chunk {
            let tokens = estimate_tokens(&buffer);
            if !buffer.trim().is_empty() && (tokens >= MIN_CHUNK_TOKENS || chunks.is_empty()) {
                chunks.push(FileChunk::new(
                    file_path.clone(),
                    take(&mut buffer),
                    current_section.clone(),
                    current_chunk_start + 1,
                    index,
                ));
                line_count = 0;
            }
            current_chunk_start = index;
        }

        if is_section_header {
            current_section = trimmed.to_string();
            if should_chunk {
                current_chunk_start = index;
            }
        }

        if line_count > 0 {
            buffer.push('\n');
        }
        buffer.push_str(line);
        line_count += 1;
    }

    // Add remaining - only if meets minimum OR if we have no chunks yet
    if line_count > 0 {
        let tokens = estimate_tokens(&buffer);
        if !buffer.trim().is_empty() && (tokens >= MIN_CHUNK_TOKENS || chunks.is_empty()) {
            chunks.push(FileChunk::new(
                file_path.clone(),
                buffer,
                current_section,
                current_chunk_start + 1,
                lines.len(),
            ));
        }
    }

    if chunks.is_empty() {
        chunks.push(FileChunk::new(
            file_path,
            content.to_string(),
            String::from("config"),
            1,
            lines.len(),
        ));
    }

    chunks
}
