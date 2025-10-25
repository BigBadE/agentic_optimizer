//! Smart chunking for code and text files with token-based optimization.

mod config;
mod generic;
mod markdown;
mod rust;
mod text;

use std::path::Path;

pub use config::chunk_config;
pub use generic::chunk_generic_code;
pub use markdown::chunk_markdown;
pub use rust::chunk_rust;
pub use text::chunk_text;

/// Optimal token range for chunks
pub const MIN_CHUNK_TOKENS: usize = 100; // ~25 lines
/// Optimal minimum token count for a chunk (~50 lines).
pub const OPTIMAL_MIN_TOKENS: usize = 200; // ~50 lines
/// Optimal maximum token count for a chunk (~125 lines).
pub const OPTIMAL_MAX_TOKENS: usize = 500; // ~125 lines
/// Maximum allowed token count for a chunk (~200 lines).
pub const MAX_CHUNK_TOKENS: usize = 800; // ~200 lines

/// Estimate tokens from text (rough: ~4 chars per token)
pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.len();
    let words = text.split_whitespace().count();
    // Average of character-based and word-based estimates
    let char_estimate = chars / 4;
    let word_estimate = (words * 10) / 13;
    usize::midpoint(char_estimate, word_estimate)
}

/// A chunk of a file with metadata
#[derive(Debug, Clone)]
pub struct FileChunk {
    /// Original file path
    pub file_path: String,
    /// Chunk content
    pub content: String,
    /// Chunk identifier (e.g., "fn main", "## Overview")
    pub identifier: String,
    /// Start line number (1-indexed)
    pub start_line: usize,
    /// End line number (1-indexed)
    pub end_line: usize,
}

impl FileChunk {
    /// Create a new chunk
    pub fn new(
        file_path: String,
        content: String,
        identifier: String,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        Self {
            file_path,
            content,
            identifier,
            start_line,
            end_line,
        }
    }
}

/// Chunk a file based on its extension
pub fn chunk_file(file_path: &Path, content: &str) -> Vec<FileChunk> {
    let path_str = file_path.display().to_string();

    let Some(extension) = file_path.extension() else {
        return chunk_generic_code(path_str, content);
    };
    if let Some(ext) = extension.to_str() {
        match ext {
            "rs" => chunk_rust(path_str, content),
            "md" | "markdown" => chunk_markdown(&path_str, content),
            "txt" | "log" => chunk_text(path_str, content),
            "toml" | "yaml" | "yml" | "json" => chunk_config(path_str, content),
            _ => chunk_generic_code(path_str, content),
        }
    } else {
        chunk_generic_code(path_str, content)
    }
}
