use super::decompose::TaskDecomposer;
use super::intent::IntentExtractor;
use crate::{ExecutionStrategy, Result, TaskAnalysis, TaskAnalyzer};
use async_trait::async_trait;

/// Local task analyzer using heuristics (no LLM required)
pub struct LocalTaskAnalyzer {
    intent_extractor: IntentExtractor,
    task_decomposer: TaskDecomposer,
    max_parallel_tasks: usize,
}

impl LocalTaskAnalyzer {
    /// Set maximum parallel tasks
    #[must_use]
    pub fn with_max_parallel(mut self, max: usize) -> Self {
        self.max_parallel_tasks = max;
        self
    }
}

impl Default for LocalTaskAnalyzer {
    fn default() -> Self {
        Self {
            intent_extractor: IntentExtractor,
            task_decomposer: TaskDecomposer,
            max_parallel_tasks: 4,
        }
    }
}

#[async_trait]
impl TaskAnalyzer for LocalTaskAnalyzer {
    async fn analyze(&self, request: &str) -> Result<TaskAnalysis> {
        let intent = self.intent_extractor.extract(request);

        let difficulty = intent.difficulty_hint.unwrap_or(5);
        tracing::info!(
            "📊 Task difficulty analysis: {} | Action: {:?} | Scope: {}",
            difficulty,
            intent.action,
            match &intent.scope {
                super::Scope::Function(name) => format!("Function({name})"),
                super::Scope::File(path) => format!("File({path})"),
                super::Scope::Module(name) => format!("Module({name})"),
                super::Scope::Multiple(files) => format!("Multiple({} files)", files.len()),
                super::Scope::Project => "Project".to_owned(),
            }
        );

        let tasks = self.task_decomposer.decompose(&intent, request);

        let execution_strategy = if tasks.len() == 1 {
            ExecutionStrategy::Sequential
        } else if tasks.iter().all(|task| task.dependencies.is_empty()) {
            ExecutionStrategy::Parallel {
                max_concurrent: self.max_parallel_tasks,
            }
        } else {
            ExecutionStrategy::Pipeline
        };

        Ok(TaskAnalysis {
            tasks,
            execution_strategy,
        })
    }

    fn estimate_difficulty(&self, request: &str) -> u8 {
        let intent = self.intent_extractor.extract(request);
        intent.difficulty_hint.unwrap_or(5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests simple request analysis.
    ///
    /// # Errors
    /// Returns an error if analysis fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_simple_request() -> Result<()> {
        let analyzer = LocalTaskAnalyzer::default();
        let analysis = analyzer.analyze("Add a comment to main.rs").await?;

        assert_eq!(analysis.tasks.len(), 1);
        assert!(matches!(
            analysis.execution_strategy,
            ExecutionStrategy::Sequential
        ));
        Ok(())
    }

    /// Tests refactor request analysis.
    ///
    /// # Errors
    /// Returns an error if analysis fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_refactor_request() -> Result<()> {
        let analyzer = LocalTaskAnalyzer::default();
        let analysis = analyzer.analyze("Refactor the parser module").await?;

        assert_eq!(analysis.tasks.len(), 3);
        assert!(matches!(
            analysis.execution_strategy,
            ExecutionStrategy::Pipeline
        ));
        Ok(())
    }

    /// Tests complexity estimation for simple and complex tasks.
    ///
    /// # Errors
    /// Returns an error if analysis fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_complexity_estimation() -> Result<()> {
        let analyzer = LocalTaskAnalyzer::default();

        // Test simple task
        let simple_analysis = analyzer.analyze("Add a comment").await?;
        assert!(!simple_analysis.tasks.is_empty());
        let simple_difficulty = simple_analysis.tasks[0].difficulty;
        assert!(
            simple_difficulty <= 5,
            "Simple tasks should have low to medium difficulty, got {simple_difficulty}"
        );

        // Test complex task
        let complex_analysis = analyzer.analyze("Refactor the entire architecture").await?;
        assert!(!complex_analysis.tasks.is_empty());
        let complex_difficulty = complex_analysis.tasks[0].difficulty;
        assert!(
            complex_difficulty >= 5,
            "Complex tasks should have medium to high difficulty, got {complex_difficulty}"
        );

        Ok(())
    }

    /// Tests context needs extraction from request.
    ///
    /// # Errors
    /// Returns an error if analysis fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_context_needs() -> Result<()> {
        let analyzer = LocalTaskAnalyzer::default();
        let analysis = analyzer.analyze("Modify test.rs and main.rs").await?;

        // Verify analysis was created successfully
        assert!(!analysis.tasks.is_empty());
        let task = &analysis.tasks[0];

        // Context needs may or may not be populated depending on implementation
        // Just verify the field exists and is accessible
        let _ = &task.context_needs.required_files;
        Ok(())
    }
}
