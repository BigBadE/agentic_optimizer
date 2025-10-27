//! Content-based scoring utilities.

use std::path::Path;

/// Calculate query-file alignment based on keyword matching
pub fn calculate_query_file_alignment(query: &str, file_path: &Path, preview: &str) -> f32 {
    let mut alignment = 1.0;
    let query_lower = query.to_lowercase();

    // Extract query keywords (words longer than 3 chars)
    let keywords: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|word| {
            word.len() > 3
                && !matches!(
                    *word,
                    "the" | "and" | "for" | "with" | "from" | "that" | "this"
                )
        })
        .collect();

    if keywords.is_empty() {
        return alignment;
    }

    // Check if filename contains query keywords
    let filename = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_lowercase();

    for keyword in &keywords {
        if filename.contains(keyword) {
            alignment *= 1.4; // Filename match is strong signal
        }
    }

    // Check parent directory names
    if let Some(parent) = file_path.parent() {
        let parent_str = parent.to_str().unwrap_or("").to_lowercase();
        for keyword in &keywords {
            if parent_str.contains(keyword) {
                alignment *= 1.2; // Directory match is good signal
            }
        }
    }

    // Keyword density in preview
    let preview_lower = preview.to_lowercase();
    let keyword_count = keywords
        .iter()
        .filter(|keyword| preview_lower.contains(*keyword))
        .count();

    if keyword_count > 0 {
        let density_boost = (keyword_count as f32).mul_add(0.1, 1.0);
        alignment *= density_boost.min(1.5); // Cap at 1.5x
    }

    alignment
}

/// Calculate pattern-based importance boost for code structure
pub fn calculate_pattern_boost(preview: &str) -> f32 {
    let mut boost = 1.0;

    // Implementation pattern detection
    let has_impl = preview.contains("impl ") || preview.contains("impl<");
    let has_trait = preview.contains("trait ");
    let has_struct = preview.contains("pub struct") || preview.contains("pub enum");
    let has_main_fn = preview.contains("fn main(") || preview.contains("pub fn new(");

    if has_impl && has_struct {
        boost *= 1.3; // Core implementation file
    }

    if has_trait {
        boost *= 1.2; // Trait definitions are important
    }

    if has_main_fn {
        boost *= 1.25; // Entry point functions
    }

    // Count pub items (public API)
    let pub_count = preview.matches("pub fn").count()
        + preview.matches("pub struct").count()
        + preview.matches("pub enum").count();

    if pub_count > 5 {
        boost *= 1.2; // Rich public API
    }

    // Module-level documentation at start
    if preview.trim_start().starts_with("//!") {
        boost *= 1.15; // Module docs indicate important file
    }

    boost
}

/// Calculate chunk quality boost based on content
pub fn calculate_chunk_quality(preview: &str) -> f32 {
    let mut boost = 1.0;

    // Boost chunks with definitions
    if preview.contains("pub struct")
        || preview.contains("pub enum")
        || preview.contains("pub trait")
    {
        boost *= 1.4;
    }

    if preview.contains("pub fn") || preview.contains("pub async fn") {
        boost *= 1.3;
    }

    // Boost module-level documentation
    if preview.trim_start().starts_with("///") || preview.trim_start().starts_with("//!") {
        boost *= 1.2;
    }

    // Penalize chunks that are mostly comments or whitespace
    let non_whitespace_lines = preview
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*")
        })
        .count();

    if non_whitespace_lines < 3 {
        boost *= 0.5; // Mostly empty or comments
    }

    boost
}

/// Check if file content has imports matching query terms
pub fn boost_by_imports(content: &str, query: &str) -> f32 {
    let mut boost = 1.0;
    let query_terms: Vec<&str> = query
        .split_whitespace()
        .filter(|term| term.len() > 3)
        .collect();

    if query_terms.is_empty() {
        return boost;
    }

    // Extract import lines
    let imports: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("use ")
                || trimmed.starts_with("import ")
                || trimmed.starts_with("from ")
                || trimmed.starts_with("require(")
        })
        .collect();

    // Check if imports match query terms
    for term in &query_terms {
        let term_lower = term.to_lowercase();
        if imports
            .iter()
            .any(|import_line| import_line.to_lowercase().contains(&term_lower))
        {
            boost += 0.2;
        }
    }

    boost.min(2.0)
}

/// Apply exact match bonus if preview contains special tokens from query
pub fn apply_exact_match_bonus(
    bm25_contribution: f32,
    query: &str,
    preview: Option<&String>,
) -> f32 {
    if bm25_contribution <= 0.0 {
        return bm25_contribution;
    }

    let Some(preview) = preview else {
        return bm25_contribution;
    };

    let preview_lower = preview.to_lowercase();
    let query_lower = query.to_lowercase();

    // Check for special tokens (--flags, ::paths, #[attributes])
    let special_tokens: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|token| token.contains("--") || token.contains("::") || token.contains("#["))
        .collect();

    for token in special_tokens {
        if preview_lower.contains(token) {
            return bm25_contribution * 1.5; // Exact match bonus
        }
    }

    bm25_contribution
}
