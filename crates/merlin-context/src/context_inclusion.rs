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

    /// Try to add a file to the context
    /// Returns true if added, false if would exceed token limit
    pub fn try_add_file(&mut self, file: FileContext) -> bool {
        let file_tokens = Self::estimate_tokens(&file.content);

        if self.token_count + file_tokens > self.max_tokens {
            return false;
        }

        self.token_count += file_tokens;
        self.files.push(file);
        true
    }

    /// Add a file, truncating if necessary to fit within token limit
    pub fn add_file_truncated(&mut self, mut file: FileContext) -> bool {
        let file_tokens = Self::estimate_tokens(&file.content);

        if self.token_count >= self.max_tokens {
            return false; // Already at limit
        }

        let available_tokens = self.max_tokens - self.token_count;

        if file_tokens <= available_tokens {
            // Fits completely
            self.token_count += file_tokens;
        } else {
            // Truncate to fit
            let chars_to_keep = (available_tokens * 4).min(file.content.len());
            file.content.truncate(chars_to_keep);
            file.content.push_str("\n... [truncated]");

            let actual_tokens = Self::estimate_tokens(&file.content);
            self.token_count += actual_tokens;
        }
        self.files.push(file);
        true
    }

    /// Get current token count
    pub fn token_count(&self) -> usize {
        self.token_count
    }

    /// Get remaining tokens
    pub fn remaining_tokens(&self) -> usize {
        self.max_tokens - self.token_count
    }

    /// Get number of files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Check if context is full
    pub fn is_full(&self) -> bool {
        self.token_count >= self.max_tokens
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
        if manager.try_add_file(prioritized_file.file) {
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

    #[test]
    fn test_context_manager_new() {
        let manager = ContextManager::new(1000);
        assert_eq!(manager.token_count(), 0);
        assert_eq!(manager.file_count(), 0);
        assert_eq!(manager.remaining_tokens(), 1000);
        assert!(!manager.is_full());
    }

    #[test]
    fn test_context_manager_default() {
        let manager = ContextManager::default();
        assert_eq!(manager.max_tokens, MAX_CONTEXT_TOKENS);
    }

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

    #[test]
    fn test_try_add_file_success() {
        let mut manager = ContextManager::new(1000);
        let file = create_test_file("test.rs", "fn main() {}");

        assert!(manager.try_add_file(file));
        assert_eq!(manager.file_count(), 1);
        assert!(manager.token_count() > 0);
    }

    #[test]
    fn test_try_add_file_exceeds_limit() {
        let mut manager = ContextManager::new(10); // Very small limit
        let large_content = &"x".repeat(1000);
        let file = create_test_file("test.rs", large_content);

        assert!(!manager.try_add_file(file));
        assert_eq!(manager.file_count(), 0);
    }

    #[test]
    fn test_add_file_truncated_fits() {
        let mut manager = ContextManager::new(1000);
        let file = create_test_file("test.rs", "small content");

        assert!(manager.add_file_truncated(file));
        assert_eq!(manager.file_count(), 1);
    }

    #[test]
    fn test_add_file_truncated_needs_truncation() {
        let mut manager = ContextManager::new(50);
        let large_content = &"x".repeat(500);
        let file = create_test_file("test.rs", large_content);

        assert!(manager.add_file_truncated(file));
        assert_eq!(manager.file_count(), 1);
        // Content should be truncated
        assert!(manager.files()[0].content.contains("[truncated]"));
    }

    #[test]
    fn test_add_file_truncated_already_full() {
        // Create manager with very small limit
        let mut manager = ContextManager::new(1);
        let large_content = &"test content that is definitely more than one token ".repeat(10);
        let file1 = create_test_file("test1.rs", large_content);

        // Add first file with truncation - should work
        let added1 = manager.add_file_truncated(file1);
        assert!(added1, "Should be able to add first file with truncation");

        // Now try to add a second file when at/near capacity
        let file2 = create_test_file("test2.rs", large_content);
        let added2 = manager.add_file_truncated(file2);

        // Check that behavior is consistent: either both files added or limit reached
        if added2 {
            assert_eq!(
                manager.file_count(),
                2,
                "Should have 2 files if second was added"
            );
        } else {
            assert_eq!(
                manager.file_count(),
                1,
                "Should have 1 file if second was rejected"
            );
            assert!(
                manager.is_full(),
                "Manager should be full when rejecting files"
            );
        }
    }

    #[test]
    fn test_remaining_tokens() {
        let mut manager = ContextManager::new(1000);
        // Use content that's long enough to produce tokens
        let content = "fn main() { println!(\"Hello, world!\"); }";
        let file = create_test_file("test.rs", content);
        let initial_tokens = manager.token_count();
        manager.try_add_file(file);

        let remaining = manager.remaining_tokens();
        assert!(
            manager.token_count() > initial_tokens,
            "Should have added tokens"
        );
        assert_eq!(remaining, 1000 - manager.token_count());
    }

    #[test]
    fn test_is_full() {
        // Create a manager with very small limit
        let mut manager = ContextManager::new(1);
        assert!(!manager.is_full());

        // Add a file that will definitely exceed the limit
        let large_content = &"x ".repeat(100); // 200 chars, 100 words
        let file = create_test_file("test.rs", large_content);

        // This should fail because file is too large
        let added = manager.try_add_file(file);
        assert!(!added, "Should not be able to add file exceeding limit");
        assert!(
            !manager.is_full(),
            "Manager should not be full when add failed"
        );

        // Now with truncation it should work
        let file2 = create_test_file("test2.rs", large_content);
        let added2 = manager.add_file_truncated(file2);
        // Either it added with truncation, or it's already full
        assert!(added2 || manager.is_full());
    }

    #[test]
    fn test_into_files() {
        let mut manager = ContextManager::new(1000);
        let file = create_test_file("test.rs", "content");
        manager.try_add_file(file);

        let files = manager.into_files();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_files_reference() {
        let mut manager = ContextManager::new(1000);
        let file = create_test_file("test.rs", "content");
        manager.try_add_file(file);

        assert_eq!(manager.files().len(), 1);
        assert_eq!(manager.file_count(), 1);
    }

    #[test]
    fn test_file_priority_ordering() {
        assert!(FilePriority::Critical > FilePriority::High);
        assert!(FilePriority::High > FilePriority::Medium);
        assert!(FilePriority::Medium > FilePriority::Low);
    }

    #[test]
    fn test_prioritized_file_new() {
        let file = create_test_file("test.rs", "content");
        let prio = PrioritizedFile::new(file, FilePriority::High);

        assert_eq!(prio.priority, FilePriority::High);
        assert!(prio.score.is_none());
    }

    #[test]
    fn test_prioritized_file_with_score() {
        let file = create_test_file("test.rs", "content");
        let prio = PrioritizedFile::with_score(file, FilePriority::Medium, 0.8);

        assert_eq!(prio.priority, FilePriority::Medium);
        assert_eq!(prio.score, Some(0.8));
    }

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

    #[test]
    fn test_constants() {
        assert_eq!(MAX_CONTEXT_TOKENS, 10_000);
        assert!((MIN_SIMILARITY_SCORE - 0.5).abs() < 0.001);
    }
}
