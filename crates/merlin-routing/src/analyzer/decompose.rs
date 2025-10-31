use super::intent::{Action, Intent};
use crate::Task;

/// Decomposes complex requests into smaller tasks
pub struct TaskDecomposer;

impl TaskDecomposer {
    /// Decompose a request into tasks
    pub fn decompose(&self, intent: &Intent, request: &str) -> Vec<Task> {
        match &intent.action {
            Action::Refactor => Self::decompose_refactor(intent, request),
            Action::Create if Self::is_complex_creation(request) => {
                Self::decompose_creation(intent, request)
            }
            Action::Fix if Self::requires_analysis(request) => Self::decompose_fix(intent, request),
            _ => vec![Self::create_single_task(intent, request)],
        }
    }

    fn decompose_refactor(intent: &Intent, request: &str) -> Vec<Task> {
        let mut tasks = Vec::default();

        let analyze_task = Task::new(format!("Analyze current structure: {request}"))
            .with_difficulty(5)
            .with_priority(intent.priority);
        tasks.push(analyze_task.clone());

        let refactor_task = Task::new(format!("Refactor: {request}"))
            .with_difficulty(7)
            .with_priority(intent.priority)
            .with_dependencies(vec![analyze_task.id]);
        tasks.push(refactor_task.clone());

        let test_task = Task::new(format!("Test refactored code: {request}"))
            .with_difficulty(5)
            .with_priority(intent.priority)
            .with_dependencies(vec![refactor_task.id]);
        tasks.push(test_task);

        tasks
    }

    fn decompose_creation(intent: &Intent, request: &str) -> Vec<Task> {
        let mut tasks = Vec::default();

        let design_task = Task::new(format!("Design structure: {request}"))
            .with_difficulty(3)
            .with_priority(intent.priority);
        tasks.push(design_task.clone());

        let implement_task = Task::new(format!("Implement: {request}"))
            .with_difficulty(6)
            .with_priority(intent.priority)
            .with_dependencies(vec![design_task.id]);
        tasks.push(implement_task.clone());

        let test_task = Task::new(format!("Add tests: {request}"))
            .with_difficulty(4)
            .with_priority(intent.priority)
            .with_dependencies(vec![implement_task.id]);
        tasks.push(test_task);

        tasks
    }

    fn decompose_fix(intent: &Intent, request: &str) -> Vec<Task> {
        let mut tasks = Vec::default();

        let diagnose_task = Task::new(format!("Diagnose issue: {request}"))
            .with_difficulty(5)
            .with_priority(intent.priority);
        tasks.push(diagnose_task.clone());

        let fix_task = Task::new(format!("Fix: {request}"))
            .with_difficulty(6)
            .with_priority(intent.priority)
            .with_dependencies(vec![diagnose_task.id]);
        tasks.push(fix_task.clone());

        let verify_task = Task::new(format!("Verify fix: {request}"))
            .with_difficulty(3)
            .with_priority(intent.priority)
            .with_dependencies(vec![fix_task.id]);
        tasks.push(verify_task);

        tasks
    }

    fn create_single_task(intent: &Intent, _request: &str) -> Task {
        let difficulty = intent.difficulty_hint.unwrap_or(5);

        Task::new(intent.description.clone())
            .with_difficulty(difficulty)
            .with_priority(intent.priority)
    }

    fn is_complex_creation(request: &str) -> bool {
        let request_lower = request.to_lowercase();
        request_lower.contains("new module")
            || request_lower.contains("new crate")
            || request_lower.contains("entire")
            || request_lower.split_whitespace().count() > 20
    }

    fn requires_analysis(request: &str) -> bool {
        let request_lower = request.to_lowercase();
        request_lower.contains("complex")
            || request_lower.contains("investigate")
            || request_lower.contains("not sure")
            || request_lower.contains("figure out")
    }
}

impl Default for TaskDecomposer {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::super::intent::IntentExtractor;
    use super::*;

    /// Tests that simple tasks are not decomposed into subtasks.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_simple_task_no_decomposition() {
        let decomposer = TaskDecomposer;
        let extractor = IntentExtractor;

        let intent = extractor.extract("Add a comment to main.rs");
        let tasks = decomposer.decompose(&intent, "Add a comment to main.rs");

        assert_eq!(tasks.len(), 1);
    }

    /// Tests that refactor tasks are decomposed into analyze, refactor, and test steps.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_refactor_decomposition() {
        let decomposer = TaskDecomposer;
        let extractor = IntentExtractor;

        let intent = extractor.extract("Refactor the parser module");
        let tasks = decomposer.decompose(&intent, "Refactor the parser module");

        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].description.contains("Analyze"));
        assert!(tasks[1].description.contains("Refactor"));
        assert!(tasks[2].description.contains("Test"));

        assert_eq!(tasks[1].dependencies.len(), 1);
        assert_eq!(tasks[2].dependencies.len(), 1);
    }

    /// Tests that complex creation tasks are decomposed into design, implement, and test steps.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_complex_creation_decomposition() {
        let decomposer = TaskDecomposer;
        let extractor = IntentExtractor;

        let intent = extractor.extract("Create a new module for handling authentication");
        let tasks =
            decomposer.decompose(&intent, "Create a new module for handling authentication");

        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].description.contains("Design"));
        assert!(tasks[1].description.contains("Implement"));
        assert!(tasks[2].description.contains("tests"));
    }

    /// Tests that complex fix tasks are decomposed into diagnose, fix, and verify steps.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_fix_with_analysis() {
        let decomposer = TaskDecomposer;
        let extractor = IntentExtractor;

        let intent = extractor.extract("Fix the complex bug in the parser");
        let tasks = decomposer.decompose(&intent, "Fix the complex bug in the parser");

        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].description.contains("Diagnose"));
        assert!(tasks[1].description.contains("Fix"));
        assert!(tasks[2].description.contains("Verify"));
    }
}
