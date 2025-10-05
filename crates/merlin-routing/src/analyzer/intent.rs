use crate::{Complexity, Priority};

/// Extracted intent from user request
#[derive(Debug, Clone)]
pub struct Intent {
    pub action: Action,
    pub scope: Scope,
    pub priority: Priority,
    pub complexity_hint: Option<Complexity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Create,
    Modify,
    Delete,
    Refactor,
    Fix,
    Test,
    Document,
    Analyze,
    Optimize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    File(String),
    Function(String),
    Module(String),
    Project,
    Multiple(Vec<String>),
}

/// Extract intent from user request using keyword matching
pub struct IntentExtractor;

impl IntentExtractor {
    #[must_use] 
    pub fn new() -> Self {
        Self
    }
    
    #[must_use] 
    pub fn extract(&self, request: &str) -> Intent {
        let request_lower = request.to_lowercase();
        
        let action = self.detect_action(&request_lower);
        let scope = self.detect_scope(request);
        let priority = self.detect_priority(&request_lower);
        let complexity_hint = self.estimate_complexity(&request_lower, &action);
        
        Intent {
            action,
            scope,
            priority,
            complexity_hint,
        }
    }
    
    fn detect_action(&self, request: &str) -> Action {
        if request.contains("create") || request.contains("add") || request.contains("new") {
            Action::Create
        } else if request.contains("modify") || request.contains("update") || request.contains("change") {
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
    
    fn detect_scope(&self, request: &str) -> Scope {
        if request.contains(".rs") || request.contains(".toml") {
            let files: Vec<String> = request
                .split_whitespace()
                .filter(|word| word.contains('.'))
                .map(String::from)
                .collect();
            
            if files.len() == 1 {
                Scope::File(files[0].clone())
            } else if files.len() > 1 {
                Scope::Multiple(files)
            } else {
                Scope::Project
            }
        } else if request.contains("function") || request.contains("fn ") {
            Scope::Function(String::new())
        } else if request.contains("module") || request.contains("mod ") {
            Scope::Module(String::new())
        } else if request.contains("project") || request.contains("crate") {
            Scope::Project
        } else {
            Scope::Project
        }
    }
    
    fn detect_priority(&self, request: &str) -> Priority {
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
    
    fn estimate_complexity(&self, request: &str, action: &Action) -> Option<Complexity> {
        let word_count = request.split_whitespace().count();
        
        let base_complexity = match action {
            Action::Create | Action::Delete => Complexity::Simple,
            Action::Modify | Action::Document => Complexity::Simple,
            Action::Fix | Action::Test => Complexity::Medium,
            Action::Refactor | Action::Optimize => Complexity::Complex,
            Action::Analyze => Complexity::Medium,
        };
        
        let adjusted = if word_count > 50 {
            match base_complexity {
                Complexity::Trivial => Complexity::Simple,
                Complexity::Simple => Complexity::Medium,
                Complexity::Medium => Complexity::Complex,
                Complexity::Complex => Complexity::Complex,
            }
        } else {
            base_complexity
        };
        
        Some(adjusted)
    }
}

impl Default for IntentExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_action() {
        let extractor = IntentExtractor::new();
        let intent = extractor.extract("Create a new file test.rs");
        assert_eq!(intent.action, Action::Create);
    }
    
    #[test]
    fn test_fix_action() {
        let extractor = IntentExtractor::new();
        let intent = extractor.extract("Fix the bug in parser.rs");
        assert_eq!(intent.action, Action::Fix);
    }
    
    #[test]
    fn test_refactor_action() {
        let extractor = IntentExtractor::new();
        let intent = extractor.extract("Refactor the entire module");
        assert_eq!(intent.action, Action::Refactor);
    }
    
    #[test]
    fn test_file_scope() {
        let extractor = IntentExtractor::new();
        let intent = extractor.extract("Modify test.rs");
        assert!(matches!(intent.scope, Scope::File(_)));
    }
    
    #[test]
    fn test_critical_priority() {
        let extractor = IntentExtractor::new();
        let intent = extractor.extract("Critical bug fix needed");
        assert_eq!(intent.priority, Priority::Critical);
    }
    
    #[test]
    fn test_complexity_estimation() {
        let extractor = IntentExtractor::new();
        
        let simple = extractor.extract("Add a comment");
        assert_eq!(simple.complexity_hint, Some(Complexity::Simple));
        
        let complex = extractor.extract("Refactor the entire codebase");
        assert_eq!(complex.complexity_hint, Some(Complexity::Complex));
    }
}
