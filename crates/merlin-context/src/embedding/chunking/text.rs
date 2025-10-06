//! Plain text chunking by paragraphs with token limits.

use std::mem::take;
use super::{FileChunk, estimate_tokens, MIN_CHUNK_TOKENS, OPTIMAL_MIN_TOKENS, MAX_CHUNK_TOKENS};

/// Chunk plain text by paragraphs with token limits
#[must_use] 
pub fn chunk_text(file_path: String, content: &str) -> Vec<FileChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk_start = 0;
    let mut buffer = String::new();
    let mut chunk_num = 1;
    let mut line_count = 0;

    for (i, line) in lines.iter().enumerate() {
        if line_count > 0 {
            buffer.push('\n');
        }
        buffer.push_str(line);
        line_count += 1;
        
        let tokens = estimate_tokens(&buffer);
        
        // Split on empty lines if we're in optimal range or over max
        if line.trim().is_empty() && (tokens >= OPTIMAL_MIN_TOKENS || tokens >= MAX_CHUNK_TOKENS) {
            if tokens >= MIN_CHUNK_TOKENS && !buffer.trim().is_empty() {
                chunks.push(FileChunk::new(
                    file_path.clone(),
                    take(&mut buffer),
                    format!("paragraph {chunk_num}"),
                    current_chunk_start + 1,
                    i + 1,
                ));
                chunk_num += 1;
            } else {
                buffer.clear();
            }
            line_count = 0;
            current_chunk_start = i + 1;
        }
    }
    
    // Add remaining - only if meets minimum OR if we have no chunks yet
    if line_count > 0 {
        let tokens = estimate_tokens(&buffer);
        if !buffer.trim().is_empty() && (tokens >= MIN_CHUNK_TOKENS || chunks.is_empty()) {
            chunks.push(FileChunk::new(
                file_path.clone(),
                buffer,
                format!("paragraph {chunk_num}"),
                current_chunk_start + 1,
                lines.len(),
            ));
        }
    }
    
    if chunks.is_empty() {
        chunks.push(FileChunk::new(
            file_path,
            content.to_owned(),
            String::from("text"),
            1,
            lines.len(),
        ));
    }
    
    chunks
}
