use crate::{Result, RoutingError};
use merlin_core::{Context, FileContext};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_CONTEXT_TOKENS: usize = 100_000;

/// Minimum relevance score to include a file
const MIN_RELEVANCE_SCORE: f32 = 0.3;

/// Manages dynamic context expansion and pruning
#[derive(Debug, Clone)]
pub struct ContextManager {
    included_files: HashMap<PathBuf, FileContextEntry>,
    excluded_files: HashSet<PathBuf>,
    token_budget: usize,
    current_tokens: usize,
}

#[derive(Debug, Clone)]
struct FileContextEntry {
    file: FileContext,
    relevance_score: f32,
    last_accessed: u64,
    access_count: usize,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new() -> Self {
        Self::with_token_budget(MAX_CONTEXT_TOKENS)
    }

    /// Create a context manager with a specific token budget
    pub fn with_token_budget(budget: usize) -> Self {
        Self {
            included_files: HashMap::new(),
            excluded_files: HashSet::new(),
            token_budget: budget,
            current_tokens: 0,
        }
    }

    /// Add a file to the context with a relevance score
    ///
    /// # Errors
    /// Returns an error if the file cannot be added or pruning fails
    pub fn add_file(&mut self, file: FileContext, relevance_score: f32) -> Result<()> {
        if relevance_score < MIN_RELEVANCE_SCORE {
            self.excluded_files.insert(file.path);
            return Ok(());
        }

        let file_tokens = Self::estimate_file_tokens(&file);

        if self.current_tokens + file_tokens > self.token_budget {
            self.prune_least_relevant(file_tokens)?;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        let entry = FileContextEntry {
            file: file.clone(),
            relevance_score,
            last_accessed: timestamp,
            access_count: 1,
        };

        self.current_tokens += file_tokens;
        self.included_files.insert(file.path, entry);

        Ok(())
    }

    /// Request a specific file to be added to context
    /// Request a specific file to be included in context
    ///
    /// # Errors
    /// Returns an error if the file cannot be added
    pub fn request_file(&mut self, path: PathBuf, content: String) -> Result<()> {
        let file = FileContext { path, content };
        self.add_file(file, 1.0)
    }

    /// Mark a file as accessed (increases importance)
    pub fn access_file(&mut self, path: &PathBuf) {
        if let Some(entry) = self.included_files.get_mut(path) {
            entry.access_count += 1;
            entry.last_accessed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs());

            entry.relevance_score = entry.relevance_score.mul_add(0.9, 0.1).min(1.0);
        }
    }

    /// Remove a file from context
    /// Remove a file from context
    ///
    /// # Errors
    /// Returns an error if the file cannot be removed
    pub fn remove_file(&mut self, path: &PathBuf) -> Result<()> {
        if let Some(entry) = self.included_files.remove(path) {
            let file_tokens = Self::estimate_file_tokens(&entry.file);
            self.current_tokens = self.current_tokens.saturating_sub(file_tokens);
            self.excluded_files.insert(path.clone());
        }
        Ok(())
    }

    /// Prune least relevant files to make room for new ones
    /// Prune least relevant files to free up tokens
    ///
    /// # Errors
    /// Returns an error if pruning fails
    fn prune_least_relevant(&mut self, needed_tokens: usize) -> Result<()> {
        let mut entries: Vec<_> = self.included_files.iter().collect();

        entries.sort_by(|entry_a, entry_b| {
            let score_a = Self::calculate_importance_score(entry_a.1);
            let score_b = Self::calculate_importance_score(entry_b.1);
            score_a.partial_cmp(&score_b).unwrap_or(Ordering::Equal)
        });

        let mut freed_tokens = 0;
        let mut to_remove = Vec::new();

        for (path, entry) in entries {
            if freed_tokens >= needed_tokens {
                break;
            }

            let file_tokens = Self::estimate_file_tokens(&entry.file);
            freed_tokens += file_tokens;
            to_remove.push(path.clone());
        }

        for path in to_remove {
            self.remove_file(&path)?;
        }

        if freed_tokens < needed_tokens {
            return Err(RoutingError::Other(
                "Cannot free enough tokens for new file".to_owned(),
            ));
        }

        Ok(())
    }

    /// Calculate importance score for a file entry
    fn calculate_importance_score(entry: &FileContextEntry) -> f32 {
        let recency_weight = 0.3;
        let access_weight = 0.3;
        let relevance_weight = 0.4;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        let age_seconds = now.saturating_sub(entry.last_accessed);
        let recency_score = 1.0 / (1.0 + (age_seconds as f32 / 3600.0));

        let access_score = (entry.access_count as f32).min(10.0) / 10.0;

        recency_weight * recency_score
            + access_weight * access_score
            + relevance_weight * entry.relevance_score
    }

    /// Estimate token count for a file
    fn estimate_file_tokens(file: &FileContext) -> usize {
        (file.content.len() + file.path.to_string_lossy().len()) / 4
    }

    /// Build a context from the currently included files
    pub fn build_context(&self, system_prompt: String) -> Context {
        let files: Vec<FileContext> = self
            .included_files
            .values()
            .map(|entry| entry.file.clone())
            .collect();

        Context {
            system_prompt,
            files,
        }
    }

    /// Get statistics about the current context
    pub fn get_stats(&self) -> ContextStats {
        ContextStats {
            included_files: self.included_files.len(),
            excluded_files: self.excluded_files.len(),
            current_tokens: self.current_tokens,
            token_budget: self.token_budget,
            utilization: (self.current_tokens as f32 / self.token_budget as f32) * 100.0,
        }
    }

    /// Get the list of included file paths
    pub fn get_included_files(&self) -> Vec<PathBuf> {
        self.included_files.keys().cloned().collect()
    }

    /// Check if a file is included
    pub fn is_included(&self, path: &PathBuf) -> bool {
        self.included_files.contains_key(path)
    }

    /// Get remaining token budget
    pub fn remaining_budget(&self) -> usize {
        self.token_budget.saturating_sub(self.current_tokens)
    }

    /// Optimize context by removing low-value files
    ///
    /// # Errors
    /// Returns an error if optimization fails
    pub fn optimize(&mut self) -> Result<usize> {
        let mut removed = 0;
        let mut to_remove = Vec::new();

        for (path, entry) in &self.included_files {
            let importance = Self::calculate_importance_score(entry);
            if importance < MIN_RELEVANCE_SCORE {
                to_remove.push(path.clone());
            }
        }

        for path in to_remove {
            self.remove_file(&path)?;
            removed += 1;
        }

        Ok(removed)
    }

    /// Clear all context
    pub fn clear(&mut self) {
        self.included_files.clear();
        self.excluded_files.clear();
        self.current_tokens = 0;
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about context usage
#[derive(Debug, Clone)]
pub struct ContextStats {
    /// Number of files included in context
    pub included_files: usize,
    /// Number of files excluded from context
    pub excluded_files: usize,
    /// Current token count
    pub current_tokens: usize,
    /// Maximum token budget
    pub token_budget: usize,
    /// Percentage of budget utilized
    pub utilization: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_file(path: &str, size: usize) -> FileContext {
        FileContext {
            path: PathBuf::from(path),
            content: "x".repeat(size),
        }
    }

    #[test]
    fn test_context_manager_creation() {
        let manager = ContextManager::new();
        assert_eq!(manager.get_stats().included_files, 0);
    }

    #[test]
    fn test_add_file() {
        let mut manager = ContextManager::new();
        let file = create_test_file("test.rs", 100);

        manager.add_file(file, 0.8).unwrap();
        assert_eq!(manager.get_stats().included_files, 1);
    }

    #[test]
    fn test_low_relevance_excluded() {
        let mut manager = ContextManager::new();
        let file = create_test_file("test.rs", 100);

        manager.add_file(file, 0.1).unwrap();
        assert_eq!(manager.get_stats().included_files, 0);
        assert_eq!(manager.get_stats().excluded_files, 1);
    }

    #[test]
    fn test_token_budget_enforcement() {
        let mut manager = ContextManager::with_token_budget(100);

        let file1 = create_test_file("file1.rs", 200);
        let file2 = create_test_file("file2.rs", 200);

        manager.add_file(file1, 0.9).unwrap();
        manager.add_file(file2, 0.8).unwrap();

        assert!(manager.get_stats().current_tokens <= 100);
    }

    #[test]
    fn test_file_access_tracking() {
        let mut manager = ContextManager::new();
        let file = create_test_file("test.rs", 100);
        let path = file.path.clone();

        manager.add_file(file, 0.5).unwrap();
        manager.access_file(&path);

        let entry = manager.included_files.get(&path).unwrap();
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_remove_file() {
        let mut manager = ContextManager::new();
        let file = create_test_file("test.rs", 100);
        let path = file.path.clone();

        manager.add_file(file, 0.8).unwrap();
        assert!(manager.is_included(&path));

        manager.remove_file(&path).unwrap();
        assert!(!manager.is_included(&path));
    }

    #[test]
    fn test_optimize() {
        let mut manager = ContextManager::new();

        let file1 = create_test_file("file1.rs", 100);
        let file2 = create_test_file("file2.rs", 100);

        manager.add_file(file1, 0.5).unwrap();
        manager.add_file(file2, 0.9).unwrap();

        let stats_before = manager.get_stats();
        assert_eq!(stats_before.included_files, 2);

        let removed = manager.optimize().unwrap();
        assert_eq!(removed, 0);

        let stats_after = manager.get_stats();
        assert_eq!(stats_after.included_files, 2);
    }

    #[test]
    fn test_clear() {
        let mut manager = ContextManager::new();
        let file = create_test_file("test.rs", 100);

        manager.add_file(file, 0.8).unwrap();
        manager.clear();

        assert_eq!(manager.get_stats().included_files, 0);
        assert_eq!(manager.get_stats().current_tokens, 0);
    }
}
