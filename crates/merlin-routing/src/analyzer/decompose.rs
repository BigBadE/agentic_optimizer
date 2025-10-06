use crate::Task;
use super::intent::{Action, Intent};

/// Decomposes complex requests into smaller tasks
pub struct TaskDecomposer;

impl TaskDecomposer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
    
    #[must_use]
    pub fn decompose(&self, intent: &Intent, request: &str) -> Vec<Task> {
        match &intent.action {
            Action::Refactor => self.decompose_refactor(intent, request),
            Action::Create if self.is_complex_creation(request) => {
                self.decompose_creation(intent, request)
            }
            Action::Fix if self.requires_analysis(request) => {
                self.decompose_fix(intent, request)
            }
            _ => vec![self.create_single_task(intent, request)],
        }
    }
    
    fn decompose_refactor(&self, intent: &Intent, request: &str) -> Vec<Task> {
        let mut tasks = Vec::new();
        
        let analyze_task = Task::new(format!("Analyze current structure: {request}"))
            .with_complexity(crate::Complexity::Medium)
            .with_priority(intent.priority);
        tasks.push(analyze_task.clone());
        
        let refactor_task = Task::new(format!("Refactor: {request}"))
            .with_complexity(crate::Complexity::Complex)
            .with_priority(intent.priority)
            .with_dependencies(vec![analyze_task.id]);
        tasks.push(refactor_task.clone());
        
        let test_task = Task::new(format!("Test refactored code: {request}"))
            .with_complexity(crate::Complexity::Medium)
            .with_priority(intent.priority)
            .with_dependencies(vec![refactor_task.id]);
        tasks.push(test_task);
        
        tasks
    }
    
    fn decompose_creation(&self, intent: &Intent, request: &str) -> Vec<Task> {
        let mut tasks = Vec::new();
        
        let design_task = Task::new(format!("Design structure: {request}"))
            .with_complexity(crate::Complexity::Simple)
            .with_priority(intent.priority);
        tasks.push(design_task.clone());
        
        let implement_task = Task::new(format!("Implement: {request}"))
            .with_complexity(crate::Complexity::Medium)
            .with_priority(intent.priority)
            .with_dependencies(vec![design_task.id]);
        tasks.push(implement_task.clone());
        
        let test_task = Task::new(format!("Add tests: {request}"))
            .with_complexity(crate::Complexity::Simple)
            .with_priority(intent.priority)
            .with_dependencies(vec![implement_task.id]);
        tasks.push(test_task);
        
        tasks
    }
    
    fn decompose_fix(&self, intent: &Intent, request: &str) -> Vec<Task> {
        let mut tasks = Vec::new();
        
        let diagnose_task = Task::new(format!("Diagnose issue: {request}"))
            .with_complexity(crate::Complexity::Medium)
            .with_priority(intent.priority);
        tasks.push(diagnose_task.clone());
        
        let fix_task = Task::new(format!("Fix: {request}"))
            .with_complexity(crate::Complexity::Medium)
            .with_priority(intent.priority)
            .with_dependencies(vec![diagnose_task.id]);
        tasks.push(fix_task.clone());
        
        let verify_task = Task::new(format!("Verify fix: {request}"))
            .with_complexity(crate::Complexity::Simple)
            .with_priority(intent.priority)
            .with_dependencies(vec![fix_task.id]);
        tasks.push(verify_task);
        
        tasks
    }
    
    fn create_single_task(&self, intent: &Intent, request: &str) -> Task {
        let complexity = intent.complexity_hint.unwrap_or(crate::Complexity::Medium);
        
        Task::new(request.to_string())
            .with_complexity(complexity)
            .with_priority(intent.priority)
    }
    
    fn is_complex_creation(&self, request: &str) -> bool {
        let request_lower = request.to_lowercase();
        request_lower.contains("new module")
            || request_lower.contains("new crate")
            || request_lower.contains("entire")
            || request_lower.split_whitespace().count() > 20
    }
    
    fn requires_analysis(&self, request: &str) -> bool {
        let request_lower = request.to_lowercase();
        request_lower.contains("complex")
            || request_lower.contains("investigate")
            || request_lower.contains("not sure")
            || request_lower.contains("figure out")
    }
}

impl Default for TaskDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::intent::IntentExtractor;

    #[test]
    fn test_simple_task_no_decomposition() {
        let decomposer = TaskDecomposer::new();
        let extractor = IntentExtractor::new();
        
        let intent = extractor.extract("Add a comment to main.rs");
        let tasks = decomposer.decompose(&intent, "Add a comment to main.rs");
        
        assert_eq!(tasks.len(), 1);
    }
    
    #[test]
    fn test_refactor_decomposition() {
        let decomposer = TaskDecomposer::new();
        let extractor = IntentExtractor::new();
        
        let intent = extractor.extract("Refactor the parser module");
        let tasks = decomposer.decompose(&intent, "Refactor the parser module");
        
        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].description.contains("Analyze"));
        assert!(tasks[1].description.contains("Refactor"));
        assert!(tasks[2].description.contains("Test"));
        
        assert_eq!(tasks[1].dependencies.len(), 1);
        assert_eq!(tasks[2].dependencies.len(), 1);
    }
    
    #[test]
    fn test_complex_creation_decomposition() {
        let decomposer = TaskDecomposer::new();
        let extractor = IntentExtractor::new();
        
        let intent = extractor.extract("Create a new module for handling authentication");
        let tasks = decomposer.decompose(&intent, "Create a new module for handling authentication");
        
        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].description.contains("Design"));
        assert!(tasks[1].description.contains("Implement"));
        assert!(tasks[2].description.contains("tests"));
    }
    
    #[test]
    fn test_fix_with_analysis() {
        let decomposer = TaskDecomposer::new();
        let extractor = IntentExtractor::new();
        
        let intent = extractor.extract("Fix the complex bug in the parser");
        let tasks = decomposer.decompose(&intent, "Fix the complex bug in the parser");
        
        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].description.contains("Diagnose"));
        assert!(tasks[1].description.contains("Fix"));
        assert!(tasks[2].description.contains("Verify"));
    }
}
