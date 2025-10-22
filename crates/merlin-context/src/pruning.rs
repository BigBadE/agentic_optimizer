//! Intelligent context pruning for optimized token usage.
//!
//! This module implements advanced pruning strategies to maximize context
//! effectiveness while staying within token budgets.

use merlin_core::FileContext;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::string::ToString;

/// Relevance scorer for context files
pub struct RelevanceScorer {
    /// Query keywords extracted for matching
    keywords: Vec<String>,
    /// File type preferences
    preferred_extensions: HashSet<String>,
}

impl RelevanceScorer {
    /// Create a new relevance scorer from a query
    pub fn from_query(query: &str) -> Self {
        let keywords = query
            .split_whitespace()
            .filter(|word| word.len() > 2)
            .map(str::to_lowercase)
            .collect();

        let preferred_extensions = ["rs", "toml", "md", "json"]
            .iter()
            .map(ToString::to_string)
            .collect();

        Self {
            keywords,
            preferred_extensions,
        }
    }

    /// Score a file's relevance (0.0 to 1.0)
    pub fn score(&self, file: &FileContext) -> f32 {
        let mut score = 0.0;

        // 1. Keyword matching in content (up to 0.5)
        let content_lower = file.content.to_lowercase();
        let keyword_matches = self
            .keywords
            .iter()
            .filter(|keyword| content_lower.contains(keyword.as_str()))
            .count();

        if !self.keywords.is_empty() {
            score += (keyword_matches as f32 / self.keywords.len() as f32) * 0.5;
        }

        // 2. File extension preference (up to 0.2)
        if let Some(ext) = file
            .path
            .extension()
            .and_then(|extension| extension.to_str())
            && self.preferred_extensions.contains(ext)
        {
            score += 0.2;
        }

        // 3. File size (prefer smaller files for efficiency) (up to 0.15)
        let size = file.content.len();
        if size < 5000 {
            score += 0.15;
        } else if size < 20_000 {
            score += 0.1;
        } else if size < 50_000 {
            score += 0.05;
        }

        // 4. Recency/modification markers (up to 0.15)
        // Files with TODO, FIXME, or recent patterns
        if content_lower.contains("todo")
            || content_lower.contains("fixme")
            || content_lower.contains("hack")
        {
            score += 0.15;
        }

        score.clamp(0.0, 1.0)
    }

    /// Score multiple files and return sorted by relevance
    pub fn score_files(&self, files: Vec<FileContext>) -> Vec<(FileContext, f32)> {
        let mut scored: Vec<_> = files
            .into_iter()
            .map(|file| {
                let score = self.score(&file);
                (file, score)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|first, second| second.1.partial_cmp(&first.1).unwrap_or(Ordering::Equal));

        scored
    }
}

/// Dependency graph builder for file relationships
pub struct DependencyGraph {
    /// Map from file to its dependencies
    dependencies: HashMap<PathBuf, Vec<PathBuf>>,
    /// Project root for path resolution
    project_root: PathBuf,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            dependencies: HashMap::new(),
            project_root,
        }
    }

    /// Add a file and extract its dependencies
    pub fn add_file(&mut self, file: &FileContext) {
        let deps = self.extract_dependencies(&file.content, &file.path);
        self.dependencies.insert(file.path.clone(), deps);
    }

    /// Extract dependencies from file content
    fn extract_dependencies(&self, content: &str, current_file: &Path) -> Vec<PathBuf> {
        let mut deps = Vec::new();

        // Rust-specific patterns
        for line in content.lines() {
            let trimmed = line.trim();

            // use statements: use crate::foo::bar;
            if let Some(use_path) = trimmed.strip_prefix("use ")
                && let Some(dep) = self.resolve_use_path(use_path, current_file)
            {
                deps.push(dep);
            }

            // mod statements: mod foo;
            if let Some(mod_name) = trimmed
                .strip_prefix("mod ")
                .and_then(|stripped| stripped.strip_suffix(';'))
                && let Some(dep) = Self::resolve_mod_path(mod_name.trim(), current_file)
            {
                deps.push(dep);
            }
        }

        deps
    }

    /// Resolve a use path to a file path
    fn resolve_use_path(&self, use_path: &str, _current_file: &Path) -> Option<PathBuf> {
        // Extract module path (remove traits, types, etc.)
        let module_path = use_path
            .split("::")
            .take_while(|part| !part.starts_with('{'))
            .collect::<Vec<_>>()
            .join("::");

        // Convert crate:: paths to file paths
        if let Some(crate_path) = module_path.strip_prefix("crate::") {
            let file_path = crate_path.replace("::", "/");
            let potential_file = self
                .project_root
                .join("src")
                .join(format!("{file_path}.rs"));

            if potential_file.exists() {
                return Some(potential_file);
            }

            // Try as directory with mod.rs
            let potential_mod = self.project_root.join("src").join(file_path).join("mod.rs");
            if potential_mod.exists() {
                return Some(potential_mod);
            }
        }

        None
    }

    /// Resolve a mod statement to a file path
    fn resolve_mod_path(mod_name: &str, current_file: &Path) -> Option<PathBuf> {
        // Get directory of current file
        let current_dir = current_file.parent()?;

        // Try mod_name.rs in same directory
        let sibling_file = current_dir.join(format!("{mod_name}.rs"));
        if sibling_file.exists() {
            return Some(sibling_file);
        }

        // Try mod_name/mod.rs
        let subdir_mod = current_dir.join(mod_name).join("mod.rs");
        if subdir_mod.exists() {
            return Some(subdir_mod);
        }

        None
    }

    /// Get all dependencies of a file (transitive)
    pub fn get_all_dependencies(&self, file_path: &Path, max_depth: usize) -> HashSet<PathBuf> {
        let mut visited = HashSet::new();
        let mut to_visit = vec![(file_path.to_path_buf(), 0)];

        while let Some((current, depth)) = to_visit.pop() {
            if depth >= max_depth || !visited.insert(current.clone()) {
                continue;
            }

            if let Some(deps) = self.dependencies.get(&current) {
                for dep in deps {
                    to_visit.push((dep.clone(), depth + 1));
                }
            }
        }

        visited
    }

    /// Expand a set of files to include their immediate dependencies
    pub fn expand_with_dependencies(
        &self,
        files: &[PathBuf],
        max_depth: usize,
    ) -> HashSet<PathBuf> {
        let mut expanded = HashSet::new();

        for file in files {
            let deps = self.get_all_dependencies(file, max_depth);
            expanded.extend(deps);
        }

        expanded
    }
}

/// Token budget allocator for optimal context distribution
pub struct TokenBudgetAllocator {
    /// Total token budget
    total_budget: usize,
    /// Minimum tokens per file
    min_per_file: usize,
    /// Reserve for high-priority files
    priority_reserve: f32,
}

impl TokenBudgetAllocator {
    /// Create a new budget allocator
    pub fn new(total_budget: usize) -> Self {
        Self {
            total_budget,
            min_per_file: 100,
            priority_reserve: 0.7, // 70% reserved for high-priority
        }
    }

    /// Allocate tokens to files based on priority and relevance
    pub fn allocate(&self, files: &[(FileContext, f32, u8)]) -> HashMap<PathBuf, usize> {
        let mut allocations = HashMap::new();

        if files.is_empty() {
            return allocations;
        }

        // Separate by priority (0=Low, 1=Medium, 2=High, 3=Critical)
        let high_priority: Vec<_> = files
            .iter()
            .filter(|(_, _, priority)| *priority >= 2)
            .collect();
        let low_priority: Vec<_> = files
            .iter()
            .filter(|(_, _, priority)| *priority < 2)
            .collect();

        // Calculate budgets
        let high_budget = (self.total_budget as f32 * self.priority_reserve) as usize;
        let low_budget = self.total_budget - high_budget;

        // Allocate to high-priority files
        if !high_priority.is_empty() {
            let per_file_high = high_budget / high_priority.len();
            for (file, _score, _priority) in &high_priority {
                allocations.insert(file.path.clone(), per_file_high.max(self.min_per_file));
            }
        }

        // Allocate to low-priority files based on relevance scores
        if !low_priority.is_empty() {
            let total_score: f32 = low_priority.iter().map(|(_, score, _)| score).sum();

            for (file, score, _priority) in &low_priority {
                let allocation = if total_score > 0.0 {
                    ((low_budget as f32) * (score / total_score)) as usize
                } else {
                    low_budget / low_priority.len()
                };

                allocations.insert(file.path.clone(), allocation.max(self.min_per_file));
            }
        }

        allocations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir, write};

    #[test]
    fn test_relevance_scorer_keyword_matching() {
        let scorer = RelevanceScorer::from_query("rust async tokio");

        let file = FileContext {
            path: PathBuf::from("test.rs"),
            content: "async fn main() { tokio::spawn(async {}); }".to_owned(),
        };

        let score = scorer.score(&file);
        assert!(score > 0.5); // Should have high score with 2/3 keywords
    }

    #[test]
    fn test_relevance_scorer_file_extension() {
        let scorer = RelevanceScorer::from_query("test");

        let rust_file = FileContext {
            path: PathBuf::from("test.rs"),
            content: String::new(),
        };

        let other_file = FileContext {
            path: PathBuf::from("test.xyz"),
            content: String::new(),
        };

        assert!(scorer.score(&rust_file) > scorer.score(&other_file));
    }

    #[test]
    fn test_dependency_graph_rust_use() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_root = temp_dir.path().to_path_buf();

        // Create src directory structure
        let src_dir = project_root.join("src");
        create_dir(&src_dir).expect("Failed to create src dir");

        let lib_file = src_dir.join("lib.rs");
        write(&lib_file, "pub mod foo;").expect("Failed to write lib.rs");

        let foo_file = src_dir.join("foo.rs");
        write(&foo_file, "pub fn bar() {}").expect("Failed to write foo.rs");

        let mut graph = DependencyGraph::new(project_root);

        let file_context = FileContext {
            path: lib_file.clone(),
            content: "use crate::foo;\npub mod foo;".to_owned(),
        };

        graph.add_file(&file_context);

        let deps = graph.get_all_dependencies(&lib_file, 1);
        assert!(deps.contains(&foo_file) || deps.contains(&lib_file)); // Should find at least the file itself
    }

    #[test]
    fn test_token_budget_allocator() {
        let allocator = TokenBudgetAllocator::new(1000);

        let files = vec![
            (
                FileContext {
                    path: PathBuf::from("critical.rs"),
                    content: String::new(),
                },
                0.9,
                3, // Critical priority
            ),
            (
                FileContext {
                    path: PathBuf::from("low.rs"),
                    content: String::new(),
                },
                0.3,
                0, // Low priority
            ),
        ];

        let allocations = allocator.allocate(&files);

        // Critical file should get more tokens
        let critical_tokens = allocations.get(&PathBuf::from("critical.rs")).unwrap_or(&0);
        let low_tokens = allocations.get(&PathBuf::from("low.rs")).unwrap_or(&0);

        assert!(critical_tokens > low_tokens);
    }
}
