//! Config file chunking by top-level sections with token limits.

use super::{FileChunk, MIN_CHUNK_TOKENS, estimate_tokens};
use std::mem::take;

/// Chunk config files by top-level sections with token limits
#[must_use]
pub fn chunk_config(file_path: String, content: &str) -> Vec<FileChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk_start = 0;
    let mut buffer = String::new();
    let mut current_section = String::from("root");
    let mut line_count = 0;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect section headers (e.g., [section] in TOML)
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let tokens = estimate_tokens(&buffer);

            if line_count > 0 && tokens >= MIN_CHUNK_TOKENS {
                if !buffer.trim().is_empty() {
                    chunks.push(FileChunk::new(
                        file_path.clone(),
                        take(&mut buffer),
                        current_section.clone(),
                        current_chunk_start + 1,
                        index,
                    ));
                }
                line_count = 0;
            }
            current_section = trimmed.to_string();
            current_chunk_start = index;
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
