//! Context inclusion logic with token counting and limits.

use std::cmp::Ordering;

use merlin_core::FileContext;

/// Maximum tokens allowed in context
pub const MAX_CONTEXT_TOKENS: usize = 10_000;

/// Minimum similarity score for semantic search results
pub const MIN_SIMILARITY_SCORE: f32 = 0.5;

/// Context manager that tracks token usage
pub struct ContextManager {
    /// Files included in context
    files: Vec<FileContext>,
    /// Current token count
    token_count: usize,
    /// Maximum tokens allowed
    max_tokens: usize,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(max_tokens: usize) -> Self {
        Self {
            files: Vec::default(),
            token_count: 0,
            max_tokens,
        }
    }

    /// Estimate tokens in text (rough approximation: ~4 chars per token)
    pub fn estimate_tokens(text: &str) -> usize {
        // More accurate: count words and punctuation
        let chars = text.len();
        let words = text.split_whitespace().count();

        // Average of character-based and word-based estimates
        // Characters: ~4 chars per token
        // Words: ~1.3 words per token
        let char_estimate = chars / 4;
        let word_estimate = (words * 10) / 13;

        usize::midpoint(char_estimate, word_estimate)
    }

    /// Get current token count
    pub fn token_count(&self) -> usize {
        self.token_count
    }

    /// Get number of files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Consume and return the files
    pub fn into_files(self) -> Vec<FileContext> {
        self.files
    }

    /// Get files reference
    pub fn files(&self) -> &[FileContext] {
        &self.files
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new(MAX_CONTEXT_TOKENS)
    }
}

/// Priority for file inclusion
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FilePriority {
    /// Low priority (semantic search with low score)
    Low = 0,
    /// Medium priority (pattern matches)
    Medium = 1,
    /// High priority (symbol matches, entry points)
    High = 2,
    /// Critical priority (explicitly requested files)
    Critical = 3,
}

/// File with priority for sorting
#[derive(Debug, Clone)]
pub struct PrioritizedFile {
    /// The file context
    pub file: FileContext,
    /// Priority level
    pub priority: FilePriority,
    /// Optional score (for semantic search)
    pub score: Option<f32>,
}

impl PrioritizedFile {
    /// Create a new prioritized file
    pub fn new(file: FileContext, priority: FilePriority) -> Self {
        Self {
            file,
            priority,
            score: None,
        }
    }

    /// Create with score
    pub fn with_score(file: FileContext, priority: FilePriority, score: f32) -> Self {
        Self {
            file,
            priority,
            score: Some(score),
        }
    }
}

/// Sort files by priority and add to context manager
pub fn add_prioritized_files(
    manager: &mut ContextManager,
    mut files: Vec<PrioritizedFile>,
) -> usize {
    // Sort by priority (high to low), then by score (high to low)
    files.sort_by(|file_a, file_b| {
        file_b
            .priority
            .cmp(&file_a.priority)
            .then_with(|| match (file_b.score, file_a.score) {
                (Some(score_b), Some(score_a)) => {
                    score_b.partial_cmp(&score_a).unwrap_or(Ordering::Equal)
                }
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
    });

    let mut added = 0;
    for prioritized_file in files {
        let file_tokens = ContextManager::estimate_tokens(&prioritized_file.file.content);
        if manager.token_count + file_tokens <= manager.max_tokens {
            manager.token_count += file_tokens;
            manager.files.push(prioritized_file.file);
            added += 1;
        } else {
            break; // Context is full
        }
    }

    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_file(path: &str, content: &str) -> FileContext {
        FileContext {
            path: PathBuf::from(path),
            content: content.to_owned(),
        }
    }

    /// Tests context manager initialization.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_context_manager_new() {
        let manager = ContextManager::new(1000);
        assert_eq!(manager.token_count(), 0);
        assert_eq!(manager.file_count(), 0);
    }

    /// Tests token estimation from text.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_estimate_tokens() {
        let text = "Hello world";
        let tokens = ContextManager::estimate_tokens(text);
        // "Hello world" = 11 chars, 2 words
        // char_estimate = 11/4 = 2
        // word_estimate = (2*10)/13 = 1
        // midpoint(2, 1) = 1
        assert!(tokens > 0);
    }

    /// Tests that prioritized files are sorted by priority level.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_add_prioritized_files_sorts_by_priority() {
        let mut manager = ContextManager::new(10000);

        let files = vec![
            PrioritizedFile::new(create_test_file("low.rs", "content"), FilePriority::Low),
            PrioritizedFile::new(
                create_test_file("critical.rs", "content"),
                FilePriority::Critical,
            ),
            PrioritizedFile::new(
                create_test_file("medium.rs", "content"),
                FilePriority::Medium,
            ),
            PrioritizedFile::new(create_test_file("high.rs", "content"), FilePriority::High),
        ];

        let added = add_prioritized_files(&mut manager, files);

        assert_eq!(added, 4);
        assert_eq!(manager.file_count(), 4);

        // Critical should be first
        assert!(manager.files()[0].path.ends_with("critical.rs"));
    }

    /// Tests that files with same priority are sorted by relevance score.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_add_prioritized_files_sorts_by_score() {
        let mut manager = ContextManager::new(10000);

        let files = vec![
            PrioritizedFile::with_score(
                create_test_file("file1.rs", "content"),
                FilePriority::High,
                0.5,
            ),
            PrioritizedFile::with_score(
                create_test_file("file2.rs", "content"),
                FilePriority::High,
                0.9,
            ),
            PrioritizedFile::with_score(
                create_test_file("file3.rs", "content"),
                FilePriority::High,
                0.7,
            ),
        ];

        let added = add_prioritized_files(&mut manager, files);

        assert_eq!(added, 3);
        // Highest score should be first
        assert!(manager.files()[0].path.ends_with("file2.rs"));
    }

    /// Tests that adding files stops when context manager reaches capacity.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_add_prioritized_files_stops_when_full() {
        let mut manager = ContextManager::new(50); // Small limit
        let large_content = &"x".repeat(1000);

        let files = vec![
            PrioritizedFile::new(
                create_test_file("file1.rs", "content"),
                FilePriority::Critical,
            ),
            PrioritizedFile::new(
                create_test_file("file2.rs", large_content),
                FilePriority::High,
            ),
            PrioritizedFile::new(
                create_test_file("file3.rs", "content"),
                FilePriority::Medium,
            ),
        ];

        let added = add_prioritized_files(&mut manager, files);

        // Should stop adding when full
        assert!(added < 3);
    }
}
