#![cfg(test)]
//! Integration tests for chunking validation across the entire codebase.

use std::fs;
use std::env::current_dir;
use std::path::{Path, PathBuf};
use tracing::info;
use merlin_context::embedding::chunking::{
    chunk_file, estimate_tokens, 
    MIN_CHUNK_TOKENS, MAX_CHUNK_TOKENS
};

/// Walk the codebase and collect all source files
fn collect_all_source_files() -> Vec<PathBuf> {
    use ignore::WalkBuilder;
    
    let project_root: PathBuf = current_dir()
        .ok()
        .and_then(|path| path.ancestors().nth(2).map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    
    let mut files = Vec::new();
    
    let walker = WalkBuilder::new(&project_root)
        .max_depth(None)
        .hidden(true)
        .git_ignore(true)
        .build();
    
    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        
        if entry.file_type().is_some_and(|file_type| file_type.is_file())
            && let Some(ext) = path.extension().and_then(|ext_str| ext_str.to_str()) {
                match ext {
                    "rs" | "md" | "toml" | "txt" | "yaml" | "yml" | "json" => {
                        files.push(path.to_path_buf());
                    }
                    _ => {}
                }
            }
    }
    
    files
}

#[cfg(test)]
#[test]
/// # Panics
///
/// Panics if any chunk violates the minimum token constraint.
fn test_all_chunks_respect_min_tokens() {
    let files = collect_all_source_files();
    
    info!("Testing {} files for MIN_CHUNK_TOKENS compliance...", files.len());
    
    let mut violations = Vec::new();
    let mut total_chunks = 0;
    let mut files_tested = 0;
    
    for file_path in files {
        let Ok(content) = fs::read_to_string(&file_path) else { continue };
        
        if content.trim().is_empty() {
            continue;
        }
        
        let chunks = chunk_file(&file_path, &content);
        
        if chunks.is_empty() {
            continue;
        }
        
        files_tested += 1;
        
        for chunk in &chunks {
            total_chunks += 1;
            let tokens = estimate_tokens(&chunk.content);
            
            // Allow first chunk to be below minimum if it's the only chunk
            if chunks.len() == 1 && tokens < MIN_CHUNK_TOKENS {
                // This is acceptable - single small file
                continue;
            }
            
            // Check for zero-line chunks (CRITICAL - must never happen)
            if chunk.content.trim().is_empty() {
                violations.push(format!(
                    "EMPTY CHUNK: {}:{}-{} [{}] - content is empty!",
                    file_path.display(),
                    chunk.start_line,
                    chunk.end_line,
                    chunk.identifier
                ));
            }
            
            // Check minimum token requirement (strict enforcement)
            if tokens < MIN_CHUNK_TOKENS {
                violations.push(format!(
                    "BELOW MIN: {}:{}-{} [{}] - {} tokens (min: {})",
                    file_path.display(),
                    chunk.start_line,
                    chunk.end_line,
                    chunk.identifier,
                    tokens,
                    MIN_CHUNK_TOKENS
                ));
            }
        }
    }
    
    info!("Tested {files_tested} files, {total_chunks} total chunks");
    
    if !violations.is_empty() {
        info!("\n❌ Found {} violations:", violations.len());
        for (index, violation) in violations.iter().enumerate().take(20) {
            info!("  {}. {}", index + 1, violation);
        }
        if violations.len() > 20 {
            info!("  ... and {} more", violations.len() - 20);
        }
        panic!("❌ MIN_CHUNK_TOKENS validation failed with {} violations", violations.len());
    }
    
    info!("✅ All chunks respect MIN_CHUNK_TOKENS");
}

#[cfg(test)]
#[test]
/// # Panics
///
/// Panics if any chunk exceeds the maximum token constraint.
fn test_all_chunks_respect_max_tokens() {
    let files = collect_all_source_files();
    
    info!("Testing {} files for MAX_CHUNK_TOKENS compliance...", files.len());
    
    let mut violations = Vec::new();
    let mut total_chunks = 0;
    let mut files_tested = 0;
    
    for file_path in files {
        let Ok(content) = fs::read_to_string(&file_path) else { continue };
        
        if content.trim().is_empty() {
            continue;
        }
        
        let chunks = chunk_file(&file_path, &content);
        
        if chunks.is_empty() {
            continue;
        }
        
        files_tested += 1;
        
        for chunk in chunks {
            total_chunks += 1;
            let tokens = estimate_tokens(&chunk.content);
            
            // Strict MAX enforcement (no tolerance)
            if tokens > MAX_CHUNK_TOKENS {
                violations.push(format!(
                    "ABOVE MAX: {}:{}-{} [{}] - {} tokens (max: {})",
                    file_path.display(),
                    chunk.start_line,
                    chunk.end_line,
                    chunk.identifier,
                    tokens,
                    MAX_CHUNK_TOKENS
                ));
            }
        }
    }
    
    info!("Tested {files_tested} files, {total_chunks} total chunks");
    
    if !violations.is_empty() {
        info!("\n⚠️  Found {} violations:", violations.len());
        for (index, violation) in violations.iter().enumerate().take(20) {
            info!("  {}. {}", index + 1, violation);
        }
        if violations.len() > 20 {
            info!("  ... and {} more", violations.len() - 20);
        }
        panic!("❌ MAX_CHUNK_TOKENS validation failed with {} violations", violations.len());
    }
    
    info!("✅ All chunks respect MAX_CHUNK_TOKENS");
}

#[cfg(test)]
#[test]
/// # Panics
///
/// Panics if any chunk has invalid line numbers.
fn test_chunk_line_numbers_are_valid() {
    let files = collect_all_source_files();
    
    info!("Testing {} files for valid line numbers...", files.len());
    
    let mut violations = Vec::new();
    let mut total_chunks = 0;
    let mut files_tested = 0;
    
    for file_path in files {
        let Ok(content) = fs::read_to_string(&file_path) else { continue };
        
        if content.trim().is_empty() {
            continue;
        }
        
        let line_count = content.lines().count();
        let chunks = chunk_file(&file_path, &content);
        
        if chunks.is_empty() {
            continue;
        }
        
        files_tested += 1;
        
        for chunk in chunks {
            total_chunks += 1;
            
            // Check start line is valid
            if chunk.start_line == 0 {
                violations.push(format!(
                    "INVALID START: {}:{}-{} [{}] - start_line is 0 (should be 1-indexed)",
                    file_path.display(),
                    chunk.start_line,
                    chunk.end_line,
                    chunk.identifier
                ));
            }
            
            // Check end line is valid
            if chunk.end_line > line_count {
                violations.push(format!(
                    "INVALID END: {}:{}-{} [{}] - end_line {} exceeds file length {}",
                    file_path.display(),
                    chunk.start_line,
                    chunk.end_line,
                    chunk.identifier,
                    chunk.end_line,
                    line_count
                ));
            }
            
            // Check start <= end
            if chunk.start_line > chunk.end_line {
                violations.push(format!(
                    "INVALID RANGE: {}:{}-{} [{}] - start_line > end_line",
                    file_path.display(),
                    chunk.start_line,
                    chunk.end_line,
                    chunk.identifier
                ));
            }
        }
    }
    
    info!("Tested {files_tested} files, {total_chunks} total chunks");
    
    if !violations.is_empty() {
        info!("\n❌ Found {} violations:", violations.len());
        for (index, violation) in violations.iter().enumerate().take(20) {
            info!("  {}. {}", index + 1, violation);
        }
        if violations.len() > 20 {
            info!("  ... and {} more", violations.len() - 20);
        }
        panic!("❌ Line number validation failed with {} violations", violations.len());
    }
    
    info!("✅ All chunks have valid line numbers");
}

#[cfg(test)]
#[test]
/// # Panics
///
/// Panics if statistics calculations encounter unexpected empty state.
fn test_chunk_statistics() {
    let files = collect_all_source_files();
    
    info!("Gathering chunk statistics...");
    
    let mut total_files = 0;
    let mut total_chunks = 0;
    let mut token_counts = Vec::new();
    
    
    for file_path in files {
        let Ok(content) = fs::read_to_string(&file_path) else { continue };
        
        if content.trim().is_empty() {
            continue;
        }
        
        let chunks = chunk_file(&file_path, &content);
        
        if chunks.is_empty() {
            continue;
        }
        
        total_files += 1;
        total_chunks += chunks.len();
        
        // Only include multi-chunk files in statistics
        if chunks.len() > 1 {
            for chunk in chunks {
                let tokens = estimate_tokens(&chunk.content);
                token_counts.push(tokens);
            }
        }
    }
    
    if token_counts.is_empty() {
        info!("No chunks found");
        return;
    }
    
    token_counts.sort_unstable();
    
    let min_tokens = token_counts[0];
    let max_tokens = token_counts[token_counts.len() - 1];
    let median_tokens = token_counts[token_counts.len() / 2];
    let avg_tokens: usize = token_counts.iter().sum::<usize>() / token_counts.len();
    
    let avg_chunks_per_file: f32 = total_chunks as f32 / total_files as f32;
    
    info!("\n📊 Chunk Statistics (multi-chunk files only):");
    info!("  Files processed: {total_files}");
    info!("  Total chunks: {total_chunks}");
    info!("  Chunks analyzed: {} (excluding single-chunk files)", token_counts.len());
    info!("  Avg chunks/file: {avg_chunks_per_file:.1}");
    info!("\n  Token distribution:");
    info!("    Min: {min_tokens} tokens");
    info!("    Median: {median_tokens} tokens");
    info!("    Average: {avg_tokens} tokens");
    info!("    Max: {max_tokens} tokens");
    info!("\n  Target range: {MIN_CHUNK_TOKENS}-{MAX_CHUNK_TOKENS} tokens");
    
    let in_range = token_counts.iter()
        .filter(|&tokens_ref| (MIN_CHUNK_TOKENS..=MAX_CHUNK_TOKENS).contains(tokens_ref))
        .count();
    let percentage = (in_range as f32 / token_counts.len() as f32) * 100.0;
    
    info!("  In target range: {in_range} ({percentage:.1}%)");
}

