//! Context inclusion logic with token counting and limits.

use std::cmp::Ordering;

use merlin_core::FileContext;

/// Maximum tokens allowed in context
pub const MAX_CONTEXT_TOKENS: usize = 10_000;

/// Minimum similarity score for semantic search results
pub const MIN_SIMILARITY_SCORE: f32 = 0.4;

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
    #[must_use]
    pub const fn new(max_tokens: usize) -> Self {
        Self {
            files: Vec::new(),
            token_count: 0,
            max_tokens,
        }
    }

    /// Estimate tokens in text (rough approximation: ~4 chars per token)
    #[must_use]
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
    #[must_use]
    pub const fn token_count(&self) -> usize {
        self.token_count
    }

    /// Get remaining tokens
    #[must_use]
    pub const fn remaining_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.token_count)
    }

    /// Get number of files
    #[must_use]
    pub const fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Check if context is full
    #[must_use]
    pub const fn is_full(&self) -> bool {
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
    #[must_use]
    pub const fn new(file: FileContext, priority: FilePriority) -> Self {
        Self {
            file,
            priority,
            score: None,
        }
    }

    /// Create with score
    #[must_use]
    pub const fn with_score(file: FileContext, priority: FilePriority, score: f32) -> Self {
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
        file_b.priority
            .cmp(&file_a.priority)
            .then_with(|| {
                match (file_b.score, file_a.score) {
                    (Some(score_b), Some(score_a)) => score_b.partial_cmp(&score_a).unwrap_or(Ordering::Equal),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            })
    });

    let mut added = 0;
    for pf in files {
        if manager.try_add_file(pf.file) {
            added += 1;
        } else {
            break; // Context is full
        }
    }

    added
}

