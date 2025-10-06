//! Markdown chunking by headers with token-based limits.

use std::mem::take;
use super::{FileChunk, estimate_tokens, MIN_CHUNK_TOKENS, OPTIMAL_MIN_TOKENS};

/// Chunk Markdown by headers with token-based limits
#[must_use] 
pub fn chunk_markdown(file_path: String, content: &str) -> Vec<FileChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk_start = 0;
    let mut buffer = String::new();
    let mut current_header = String::from("preamble");
    let mut line_count = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Detect headers
        if trimmed.starts_with('#') {
            let header_level = trimmed.chars().take_while(|ch| *ch == '#').count();
            let header_text = trimmed.trim_start_matches('#').trim();
            
            // Check if current chunk meets minimum
            let tokens = estimate_tokens(&buffer);
            
            // Only split if we have enough content - always merge small sections
            if line_count > 0 && tokens >= MIN_CHUNK_TOKENS {
                if !buffer.trim().is_empty() {
                    chunks.push(FileChunk::new(
                        file_path.clone(),
                        take(&mut buffer),
                        current_header.clone(),
                        current_chunk_start + 1,
                        i,
                    ));
                }
                line_count = 0;
                current_chunk_start = i;
            }
            
            // Update header (but keep accumulating content if below minimum)
            current_header = format!("{} {}", "#".repeat(header_level), header_text);
        }
        
        if line_count > 0 {
            buffer.push('\n');
        }
        buffer.push_str(line);
        line_count += 1;
        
        // Force split large chunks on empty lines
        let tokens = estimate_tokens(&buffer);
        if trimmed.is_empty() && tokens >= OPTIMAL_MIN_TOKENS {
            if !buffer.trim().is_empty() {
                chunks.push(FileChunk::new(
                    file_path.clone(),
                    take(&mut buffer),
                    current_header.clone(),
                    current_chunk_start + 1,
                    i + 1,
                ));
            }
            line_count = 0;
            current_chunk_start = i + 1;
        }
    }
    
    // Add remaining content - only if meets minimum OR if we have no chunks yet
    if line_count > 0 {
        let tokens = estimate_tokens(&buffer);
        if !buffer.trim().is_empty() {
            if tokens >= MIN_CHUNK_TOKENS {
                chunks.push(FileChunk::new(
                    file_path.clone(),
                    buffer,
                    current_header,
                    current_chunk_start + 1,
                    lines.len(),
                ));
            } else if chunks.is_empty() {
                // If this is the only content, include it even if below minimum
                chunks.push(FileChunk::new(
                    file_path.clone(),
                    buffer,
                    current_header,
                    current_chunk_start + 1,
                    lines.len(),
                ));
            }
            // Otherwise discard - too small and we have other chunks
        }
    }
    
    // If still no chunks, return whole file
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
