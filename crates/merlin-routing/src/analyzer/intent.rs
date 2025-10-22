use crate::Priority;
use std::cmp::Ordering;

/// Extracted intent from user request.
#[derive(Debug, Clone)]
pub struct Intent {
    /// Primary action to perform
    pub action: Action,
    /// Scope of the operation
    pub scope: Scope,
    /// Priority level
    pub priority: Priority,
    /// Optional difficulty hint from analysis (1-10)
    pub difficulty_hint: Option<u8>,
    /// Description of the task
    pub description: String,
}

/// Action type for the task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Create new code/files
    Create,
    /// Modify existing code/files
    Modify,
    /// Delete code/files
    Delete,
    /// Refactor/restructure code
    Refactor,
    /// Fix bugs or errors
    Fix,
    /// Add or run tests
    Test,
    /// Add documentation
    Document,
    /// Analyze or review code
    Analyze,
    /// Optimize performance or code quality
    Optimize,
}

/// Scope of the operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Single file
    File(String),
    /// Function within a file
    Function(String),
    /// Module
    Module(String),
    /// Entire project
    Project,
    /// Multiple files
    Multiple(Vec<String>),
}

/// Extracts intent (action, scope, priority) from user requests.
#[derive(Default)]
pub struct IntentExtractor;

impl IntentExtractor {
    // No-arg constructor unnecessary; use `Default` instead

    /// Extracts intent from a user request string.
    pub fn extract(&self, request: &str) -> Intent {
        let request_lower = request.to_lowercase();
        let action = Self::detect_action(&request_lower);
        let scope = Self::detect_scope(request);
        let priority = Self::detect_priority(&request_lower);
        let difficulty_hint = Some(Self::estimate_difficulty(&request_lower, &action));

        Intent {
            action,
            scope,
            priority,
            difficulty_hint,
            description: request.to_owned(),
        }
    }

    fn detect_action(request: &str) -> Action {
        if request.contains("create") || request.contains("add") || request.contains("new") {
            Action::Create
        } else if request.contains("modify")
            || request.contains("update")
            || request.contains("change")
        {
            Action::Modify
        } else if request.contains("delete") || request.contains("remove") {
            Action::Delete
        } else if request.contains("refactor") || request.contains("restructure") {
            Action::Refactor
        } else if request.contains("fix") || request.contains("bug") || request.contains("error") {
            Action::Fix
        } else if request.contains("test") {
            Action::Test
        } else if request.contains("document") || request.contains("comment") {
            Action::Document
        } else if request.contains("analyze") || request.contains("review") {
            Action::Analyze
        } else if request.contains("optimize") || request.contains("improve") {
            Action::Optimize
        } else {
            Action::Modify
        }
    }

    fn detect_scope(request: &str) -> Scope {
        if request.contains(".rs") || request.contains(".toml") {
            let files: Vec<String> = request
                .split_whitespace()
                .filter(|word| word.contains('.'))
                .map(String::from)
                .collect();
            match files.len().cmp(&1) {
                Ordering::Equal => Scope::File(files[0].clone()),
                Ordering::Greater => Scope::Multiple(files),
                Ordering::Less => Scope::Project,
            }
        } else if request.contains("function") || request.contains("fn ") {
            Scope::Function(String::default())
        } else if request.contains("module") || request.contains("mod ") {
            Scope::Module(String::default())
        } else {
            Scope::Project
        }
    }

    fn detect_priority(request: &str) -> Priority {
        if request.contains("critical") || request.contains("urgent") || request.contains("asap") {
            Priority::Critical
        } else if request.contains("important") || request.contains("high priority") {
            Priority::High
        } else if request.contains("low priority") || request.contains("when you can") {
            Priority::Low
        } else {
            Priority::Medium
        }
    }

    fn estimate_difficulty(request: &str, action: &Action) -> u8 {
        let word_count = request.split_whitespace().count();
        let base_difficulty = match action {
            Action::Create | Action::Delete | Action::Modify | Action::Document => 4,
            Action::Fix | Action::Test | Action::Analyze => 5,
            Action::Refactor | Action::Optimize => 7,
        };

        // Increase difficulty for longer/more complex requests
        if word_count > 50 {
            (base_difficulty + 2).min(10)
        } else {
            base_difficulty
        }
    }
}

// Default derived above

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_action() {
        let extractor = IntentExtractor;
        let intent = extractor.extract("Create a new file test.rs");
        assert_eq!(intent.action, Action::Create);
    }

    #[test]
    fn test_fix_action() {
        let extractor = IntentExtractor;
        let intent = extractor.extract("Fix the bug in parser.rs");
        assert_eq!(intent.action, Action::Fix);
    }

    #[test]
    fn test_refactor_action() {
        let extractor = IntentExtractor;
        let intent = extractor.extract("Refactor the entire module");
        assert_eq!(intent.action, Action::Refactor);
    }

    #[test]
    fn test_file_scope() {
        let extractor = IntentExtractor;
        let intent = extractor.extract("Modify test.rs");
        assert!(matches!(intent.scope, Scope::File(_)));
    }

    #[test]
    fn test_critical_priority() {
        let extractor = IntentExtractor;
        let intent = extractor.extract("Critical bug fix needed");
        assert_eq!(intent.priority, Priority::Critical);
    }

    #[test]
    fn test_complexity_estimation() {
        let extractor = IntentExtractor;

        let simple = extractor.extract("Add a comment");
        let simple_difficulty = simple.difficulty_hint.unwrap_or(5);
        assert!(
            simple_difficulty <= 5,
            "Simple tasks should have low to medium difficulty, got {simple_difficulty}"
        );

        let complex = extractor.extract("Refactor the entire codebase");
        let complex_difficulty = complex.difficulty_hint.unwrap_or(5);
        eprintln!("Complex task difficulty: {complex_difficulty}");
        assert!(
            complex_difficulty >= 7,
            "Complex tasks should have high difficulty, got {complex_difficulty}"
        );
    }
}
