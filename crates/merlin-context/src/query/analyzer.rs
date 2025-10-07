//! Query analysis to extract intent and keywords.

use super::types::{QueryIntent, Action, Scope, Complexity};

/// Analyzes user queries to extract intent and keywords
pub struct QueryAnalyzer;

impl QueryAnalyzer {
    /// Create a new query analyzer
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Analyze a query string and extract intent
    #[must_use]
    pub fn analyze(&self, query: &str) -> QueryIntent {
        let query_lower = query.to_lowercase();
        
        let action = Self::detect_action(&query_lower);
        let scope = Self::detect_scope(&query_lower);
        let complexity = Self::estimate_complexity(&query_lower, action, scope);
        let keywords = Self::extract_keywords(&query_lower);
        let entities = Self::extract_entities(query);
        
        QueryIntent {
            action,
            keywords,
            entities,
            scope,
            complexity,
        }
    }

    /// Detect the action type from the query
    fn detect_action(query: &str) -> Action {
        const CREATE_KEYWORDS: &[&str] = &[
            "create", "add", "implement", "build", "make", "generate", "new"
        ];
        const MODIFY_KEYWORDS: &[&str] = &[
            "modify", "change", "update", "edit", "alter"
        ];
        const DEBUG_KEYWORDS: &[&str] = &[
            "fix", "debug", "bug", "error", "issue", "problem", "broken"
        ];
        const EXPLAIN_KEYWORDS: &[&str] = &[
            "explain", "what", "how", "why", "show", "describe", "tell"
        ];
        const REFACTOR_KEYWORDS: &[&str] = &[
            "refactor", "reorganize", "restructure", "optimize", "improve"
        ];
        const SEARCH_KEYWORDS: &[&str] = &[
            "find", "search", "locate", "where"
        ];

        if CREATE_KEYWORDS.iter().any(|keyword| query.contains(keyword)) {
            Action::Create
        } else if DEBUG_KEYWORDS.iter().any(|keyword| query.contains(keyword)) {
            Action::Debug
        } else if REFACTOR_KEYWORDS.iter().any(|keyword| query.contains(keyword)) {
            Action::Refactor
        } else if EXPLAIN_KEYWORDS.iter().any(|keyword| query.contains(keyword)) {
            Action::Explain
        } else if SEARCH_KEYWORDS.iter().any(|keyword| query.contains(keyword)) {
            Action::Search
        } else if MODIFY_KEYWORDS.iter().any(|keyword| query.contains(keyword)) {
            Action::Modify
        } else {
            // Default to modify for ambiguous queries
            Action::Modify
        }
    }

    /// Detect the scope of the change
    fn detect_scope(query: &str) -> Scope {
        const CODEBASE_KEYWORDS: &[&str] = &[
            "all", "everywhere", "codebase", "project", "entire", "whole"
        ];
        const MODULE_KEYWORDS: &[&str] = &[
            "module", "package", "folder", "directory", "related"
        ];

        if CODEBASE_KEYWORDS.iter().any(|keyword| query.contains(keyword)) {
            Scope::Codebase
        } else if MODULE_KEYWORDS.iter().any(|keyword| query.contains(keyword)) {
            Scope::Module
        } else {
            Scope::Focused
        }
    }

    /// Estimate query complexity
    fn estimate_complexity(query: &str, action: Action, scope: Scope) -> Complexity {
        let word_count = query.split_whitespace().count();
        
        // Complex if codebase-wide
        if matches!(scope, Scope::Codebase) {
            return Complexity::Complex;
        }
        
        // Complex if creating or refactoring
        if matches!(action, Action::Create | Action::Refactor) {
            return Complexity::Complex;
        }
        
        // Simple if searching or explaining
        if matches!(action, Action::Search | Action::Explain) {
            return Complexity::Simple;
        }
        
        // Use word count as heuristic
        if word_count > 20 {
            Complexity::Complex
        } else if word_count > 10 {
            Complexity::Medium
        } else {
            Complexity::Simple
        }
    }

    /// Extract keywords from the query
    fn extract_keywords(query: &str) -> Vec<String> {
        const STOP_WORDS: &[&str] = &[
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "as", "is", "was", "are", "were", "be",
            "been", "being", "have", "has", "had", "do", "does", "did", "will",
            "would", "should", "could", "may", "might", "must", "can", "this",
            "that", "these", "those", "i", "you", "he", "she", "it", "we", "they"
        ];

        query
            .split_whitespace()
            .filter(|word| {
                word.len() > 2 && !STOP_WORDS.contains(word)
            })
            .map(|word| word.trim_matches(|character: char| !character.is_alphanumeric()).to_string())
            .filter(|word| !word.is_empty())
            .collect()
    }

    /// Extract entities (capitalized words, likely types/functions)
    fn extract_entities(query: &str) -> Vec<String> {
        query
            .split_whitespace()
            .filter(|word| {
                // Check if word starts with uppercase or contains :: (Rust path)
                let has_uppercase = word.chars().next().is_some_and(char::is_uppercase);
                let has_path = word.contains("::");
                (has_uppercase || has_path) && word.len() > 1  // Skip single chars like "I"
            })
            .map(|word| {
                // Clean up punctuation from end
                word.trim_end_matches(|character: char| !character.is_alphanumeric() && character != ':' && character != '_')
                    .to_string()
            })
            .filter(|word| !word.is_empty() && word.len() > 1)
            .collect()
    }
}

impl Default for QueryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// # Panics
    /// Panics if create action is not detected.
    fn test_detect_create_action() {
        let analyzer = QueryAnalyzer::new();
        let intent = analyzer.analyze("Create a new authentication module");
        assert!(matches!(intent.action, Action::Create));
    }

    #[test]
    /// # Panics
    /// Panics if debug action is not detected.
    fn test_detect_debug_action() {
        let analyzer = QueryAnalyzer::new();
        let intent = analyzer.analyze("Fix the bug in UserService");
        assert!(matches!(intent.action, Action::Debug));
    }

    #[test]
    /// # Panics
    /// Panics if expected keywords are not extracted.
    fn test_extract_keywords() {
        let analyzer = QueryAnalyzer::new();
        let intent = analyzer.analyze("Implement authentication for the user service");
        assert!(intent.keywords.contains(&"authentication".to_string()));
        assert!(intent.keywords.contains(&"user".to_string()));
        assert!(intent.keywords.contains(&"service".to_string()));
    }

    #[test]
    /// # Panics
    /// Panics if expected entities are not extracted.
    fn test_extract_entities() {
        let analyzer = QueryAnalyzer::new();
        let intent = analyzer.analyze("Fix UserService::find_by_email method");
        assert!(intent.entities.contains(&"UserService::find_by_email".to_string()));
    }

    #[test]
    /// # Panics
    /// Panics if complexity estimation fails to categorize as expected.
    fn test_complexity_estimation() {
        let analyzer = QueryAnalyzer::new();
        
        let simple = analyzer.analyze("Find the User struct");
        assert!(matches!(simple.complexity, Complexity::Simple));
        
        let complex = analyzer.analyze("Refactor the entire authentication system to use OAuth2");
        assert!(matches!(complex.complexity, Complexity::Complex));
    }
}
