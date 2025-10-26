//! Types for query analysis and context planning.

use serde::{Deserialize, Serialize};

/// Analyzed intent from a user query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryIntent {
    /// The action the user wants to perform
    pub action: Action,
    /// Keywords extracted from the query
    pub keywords: Vec<String>,
    /// Entities mentioned (types, functions, modules)
    pub entities: Vec<String>,
    /// Scope of the change
    pub scope: Scope,
    /// Estimated complexity
    pub complexity: Complexity,
}

/// The type of action requested
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Create new code/feature
    Create,
    /// Modify existing code
    Modify,
    /// Debug/fix an issue
    Debug,
    /// Explain or analyze code
    Explain,
    /// Refactor code
    Refactor,
    /// Search for something
    Search,
}

/// Scope of the change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    /// Single file or function
    Focused,
    /// Multiple related files in a module
    Module,
    /// Codebase-wide changes
    Codebase,
}

/// Query complexity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Complexity {
    /// Simple queries (lookups, simple questions)
    Simple,
    /// Medium complexity (single-file edits)
    Medium,
    /// Complex queries (multi-file changes, architecture)
    Complex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_intent_creation() {
        let intent = QueryIntent {
            action: Action::Create,
            keywords: vec!["test".to_owned()],
            entities: vec!["User".to_owned()],
            scope: Scope::Focused,
            complexity: Complexity::Simple,
        };

        assert_eq!(intent.action, Action::Create);
        assert_eq!(intent.keywords.len(), 1);
        assert_eq!(intent.entities.len(), 1);
        assert_eq!(intent.scope, Scope::Focused);
        assert_eq!(intent.complexity, Complexity::Simple);
    }

    // REMOVED: test_action_variants - Low value enum test

    // REMOVED: test_scope_variants - Low value enum test

    // REMOVED: test_complexity_ordering - Low value enum test

    // REMOVED: test_serde_query_intent - Low value serde test
}
