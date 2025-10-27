//! Rust item detection and extraction.

use crate::embedding::chunking::{MAX_CHUNK_TOKENS, estimate_tokens};

/// Check if line starts a Rust item
pub fn is_rust_item_start(line: &str) -> bool {
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

/// Extract a complete Rust item indices (handles brace matching with token limit)
pub fn extract_rust_item_indices(lines: &[&str], start: usize) -> (usize, usize, String, usize) {
    let identifier = super::extract_rust_identifier(lines[start].trim());
    let mut brace_depth = 0;
    let mut found_opening_brace = false;
    let mut buffer = String::default();
    let mut last_balanced_line = start;

    for (offset, line) in lines[start..].iter().enumerate() {
        // Add line to buffer to check tokens
        if offset > 0 {
            buffer.push('\n');
        }
        buffer.push_str(line);

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

        // Track the last line where braces were balanced
        if found_opening_brace && brace_depth == 0 {
            last_balanced_line = start + offset;
        }

        // Check if we're exceeding MAX_CHUNK_TOKENS
        let tokens = estimate_tokens(&buffer);
        if tokens > MAX_CHUNK_TOKENS {
            if offset > 0 && brace_depth == 0 {
                // We're at a balanced point, stop here
                let end_line = start + offset;
                return (start, end_line, identifier, end_line);
            }
            if offset > 0 && last_balanced_line > start {
                // Stop at the last balanced point we saw
                return (start, last_balanced_line, identifier, last_balanced_line);
            }
            if offset > 10 {
                // Force stop after accumulating some lines, even if not balanced
                let end_line = start + offset;
                tracing::warn!(
                    "Force-stopping {identifier} at line {} due to MAX_CHUNK_TOKENS (depth={})",
                    end_line + 1,
                    brace_depth
                );
                return (start, end_line, identifier, end_line);
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
