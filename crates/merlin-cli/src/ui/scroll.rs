//! Scrolling utilities for UI components
//!
//! This module provides helper functions for consistent text line counting
//! to avoid off-by-one errors in scroll calculations.

/// Counts the number of lines in text content
///
/// Returns 0 for completely empty strings, otherwise returns the actual line count
/// including empty lines.
///
/// # Arguments
/// * `text` - Text content to count lines in
///
/// # Returns
/// Number of lines (0 only if text is completely empty)
pub fn count_text_lines(text: &str) -> u16 {
    if text.is_empty() {
        return 0;
    }
    text.lines().count() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_text_lines() {
        // Empty text
        assert_eq!(count_text_lines(""), 0);

        // Whitespace and newlines are counted
        assert_eq!(count_text_lines("   "), 1);
        assert_eq!(count_text_lines("\n\n"), 2);

        // Single line
        assert_eq!(count_text_lines("hello"), 1);

        // Multiple lines
        assert_eq!(count_text_lines("hello\nworld"), 2);
        assert_eq!(count_text_lines("a\nb\nc\n"), 3);
    }
}
