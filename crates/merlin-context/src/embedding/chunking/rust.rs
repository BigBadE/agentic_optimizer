//! Rust code chunking - prioritizes innermost items (functions over impls).

use std::mem;
use super::{FileChunk, estimate_tokens, MIN_CHUNK_TOKENS, MAX_CHUNK_TOKENS};

/// Chunk Rust code - prioritizes innermost items (functions over impls)
pub fn chunk_rust(file_path: String, content: &str) -> Vec<FileChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut i = 0;
    let mut buffer = String::new();

    while i < lines.len() {
        let trimmed = lines[i].trim();
        
        // Detect item starts
        if is_rust_item_start(trimmed) {
            let (start_idx, end_idx, chunk_id, end_line) = extract_rust_item_indices(&lines, i);
            
            // Build string using buffer to avoid allocations
            buffer.clear();
            for (line_idx, line) in lines.iter().enumerate().take(end_idx + 1).skip(start_idx) {
                if line_idx > start_idx {
                    buffer.push('\n');
                }
                buffer.push_str(line);
            }
            
            let tokens = estimate_tokens(&buffer);
            
            // If chunk is too large, try to split it
            if tokens > MAX_CHUNK_TOKENS {
                if chunk_id.starts_with("impl ") {
                    // Try to break impl into functions
                    let sub_chunks = chunk_impl_into_functions(&file_path, &lines, start_idx, end_idx, i);
                    if !sub_chunks.is_empty() {
                        chunks.extend(sub_chunks);
                        i = end_line + 1;
                        continue;
                    }
                }
                
                // Force split on empty lines
                let sub_chunks = force_split_large_chunk(&file_path, &lines, start_idx, end_idx, &chunk_id);
                if !sub_chunks.is_empty() {
                    // Verify all sub-chunks are within limits
                    let all_valid = sub_chunks.iter().all(|chunk| {
                        let token_count = estimate_tokens(&chunk.content);
                        token_count <= MAX_CHUNK_TOKENS
                    });
                    
                    if all_valid {
                        chunks.extend(sub_chunks);
                        i = end_line + 1;
                        continue;
                    }
                }
                
                // If we can't split it properly, skip it (too large and no good split points)
                tracing::warn!("Skipping large chunk {chunk_id} ({tokens} tokens) - no good split points");
                i = end_line + 1;
                continue;
            }
            
            // Only add if meets minimum token requirement
            if !buffer.trim().is_empty() && tokens >= MIN_CHUNK_TOKENS {
                chunks.push(FileChunk::new(
                    file_path.clone(),
                    mem::take(&mut buffer),
                    chunk_id,
                    i + 1,
                    end_line + 1,
                ));
            }
            // If below minimum, just skip it (will be part of surrounding context)
            
            i = end_line + 1;
        } else {
            i += 1;
        }
    }
    
    // If no chunks or very few, return whole file
    if chunks.is_empty() {
        chunks.push(FileChunk::new(
            file_path,
            content.to_owned(),
            String::from("file"),
            1,
            lines.len(),
        ));
    }
    
    chunks
}

/// Check if line starts a Rust item
fn is_rust_item_start(line: &str) -> bool {
    line.starts_with("pub fn ") || line.starts_with("fn ") ||
    line.starts_with("async fn ") || line.starts_with("pub async fn ") ||
    line.starts_with("pub struct ") || line.starts_with("struct ") ||
    line.starts_with("pub enum ") || line.starts_with("enum ") ||
    line.starts_with("pub trait ") || line.starts_with("trait ") ||
    line.starts_with("impl ") || line.starts_with("impl<") ||
    line.starts_with("pub mod ") || line.starts_with("mod ") ||
    line.starts_with("pub const ") || line.starts_with("const ") ||
    line.starts_with("pub static ") || line.starts_with("static ")
}

/// Extract a complete Rust item indices (handles brace matching)
fn extract_rust_item_indices(lines: &[&str], start: usize) -> (usize, usize, String, usize) {
    let identifier = extract_rust_identifier(lines[start].trim());
    let mut brace_depth = 0;
    let mut found_opening_brace = false;
    
    for (offset, line) in lines[start..].iter().enumerate() {
        for ch in line.chars() {
            match ch {
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
fn chunk_impl_into_functions(file_path: &str, lines: &[&str], start_idx: usize, end_idx: usize, _base_line: usize) -> Vec<FileChunk> {
    let mut chunks = Vec::new();
    let mut i = start_idx;
    let mut buffer = String::new();
    
    // Skip the impl line itself
    while i <= end_idx && !lines[i].trim().starts_with("fn ") && !lines[i].trim().starts_with("pub fn ") {
        i += 1;
    }
    
    while i <= end_idx {
        let trimmed = lines[i].trim();
        
        if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
            let (fn_start, fn_end, fn_id, _) = extract_rust_item_indices(lines, i);
            
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
                chunks.push(FileChunk::new(
                    file_path.to_owned(),
                    mem::take(&mut buffer),
                    fn_id,
                    fn_start + 1,
                    fn_end + 1,
                ));
            }
            // Skip functions below minimum
            
            i = fn_end + 1;
        } else {
            i += 1;
        }
    }
    
    chunks
}

/// Force split a large chunk on empty lines
fn force_split_large_chunk(file_path: &str, lines: &[&str], start_idx: usize, end_idx: usize, base_id: &str) -> Vec<FileChunk> {
    let mut chunks = Vec::new();
    let mut buffer = String::new();
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
        
        // Track empty lines as potential split points
        if lines[line_idx].trim().is_empty() {
            last_empty_line = Some((line_idx, tokens));
        }
        
        // Force split if we're over MAX and have a recent empty line
        if tokens > MAX_CHUNK_TOKENS
            && let Some((empty_idx, empty_tokens)) = last_empty_line
            && empty_tokens >= MIN_CHUNK_TOKENS
        {
            // Build split content from original lines
            let mut split_content = String::new();
            for (idx, line) in lines.iter().enumerate().take(empty_idx + 1).skip(chunk_start) {
                if idx > chunk_start {
                    split_content.push('\n');
                }
                split_content.push_str(line);
            }

            chunks.push(FileChunk::new(
                file_path.to_owned(),
                split_content,
                format!("{base_id} (part {part_num})"),
                chunk_start + 1,
                empty_idx + 1,
            ));

            // Rebuild buffer with remaining content
            buffer.clear();
            line_count = 0;
            for line in lines.iter().take(line_idx + 1).skip(empty_idx + 1) {
                if line_count > 0 {
                    buffer.push('\n');
                }
                buffer.push_str(line);
                line_count += 1;
            }

            part_num += 1;
            chunk_start = empty_idx + 1;
            last_empty_line = None;
        }
        
        // Also split on empty lines when we have enough content
        if lines[line_idx].trim().is_empty() && (MIN_CHUNK_TOKENS..=MAX_CHUNK_TOKENS).contains(&tokens) {
            chunks.push(FileChunk::new(
                file_path.to_owned(),
                mem::take(&mut buffer),
                format!("{base_id} (part {part_num})"),
                chunk_start + 1,
                line_idx + 1,
            ));
            part_num += 1;
            line_count = 0;
            chunk_start = line_idx + 1;
            last_empty_line = None;
        }
    }

    // Add remaining if meets minimum
    if line_count > 0 {
        let tokens = estimate_tokens(&buffer);
        if tokens >= MIN_CHUNK_TOKENS {
            chunks.push(FileChunk::new(
                file_path.to_owned(),
                buffer,
                format!("{base_id} (part {part_num})"),
                chunk_start + 1,
                end_idx + 1,
            ));
        } else if !chunks.is_empty() && let Some(last_chunk) = chunks.last_mut() {
            // Merge small remainder with last chunk if possible
            last_chunk.content.push('\n');
            last_chunk.content.push_str(&buffer);
            last_chunk.end_line = end_idx + 1;
        }
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

    // Extract based on keyword - using map_or_else to satisfy clippy
    line.strip_prefix("fn ").map_or_else(
        || {
            line.strip_prefix("struct ").map_or_else(
                || {
                    line.strip_prefix("enum ").map_or_else(
                        || {
                            line.strip_prefix("trait ").map_or_else(
                                || {
                                    if line.starts_with("impl ") || line.starts_with("impl<") {
                                        let impl_part = line.split('{').next().unwrap_or(line).trim();
                                        format!("impl {}", impl_part.strip_prefix("impl ").unwrap_or("").trim())
                                    } else {
                                        line.strip_prefix("mod ").map_or_else(
                                            || String::from("item"),
                                            |rest| format!("mod {}", rest.split(&[' ', '{'][..]).next().unwrap_or("unknown"))
                                        )
                                    }
                                },
                                |rest| format!("trait {}", rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown"))
                            )
                        },
                        |rest| format!("enum {}", rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown"))
                    )
                },
                |rest| format!("struct {}", rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown"))
            )
        },
        |rest| format!("fn {}", rest.split(&['(', '<', ' '][..]).next().unwrap_or("unknown"))
    )
}
