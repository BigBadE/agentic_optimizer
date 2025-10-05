use async_trait::async_trait;
use crate::{Complexity, ExecutionStrategy, Result, TaskAnalysis, TaskAnalyzer};
use super::complexity::ComplexityEstimator;
use super::decompose::TaskDecomposer;
use super::intent::IntentExtractor;

/// Local task analyzer using heuristics (no LLM required)
pub struct LocalTaskAnalyzer {
    intent_extractor: IntentExtractor,
    complexity_estimator: ComplexityEstimator,
    task_decomposer: TaskDecomposer,
    max_parallel_tasks: usize,
}

impl LocalTaskAnalyzer {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            intent_extractor: IntentExtractor::new(),
            complexity_estimator: ComplexityEstimator::new(),
            task_decomposer: TaskDecomposer::new(),
            max_parallel_tasks: 4,
        }
    }
    
    #[must_use] 
    pub fn with_max_parallel(mut self, max: usize) -> Self {
        self.max_parallel_tasks = max;
        self
    }
}

impl Default for LocalTaskAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskAnalyzer for LocalTaskAnalyzer {
    async fn analyze(&self, request: &str) -> Result<TaskAnalysis> {
        let intent = self.intent_extractor.extract(request);
        
        let complexity = self.complexity_estimator.estimate(&intent, request);
        
        let mut tasks = self.task_decomposer.decompose(&intent, request);
        
        for task in &mut tasks {
            let context_needs = self.complexity_estimator.estimate_context_needs(&intent, request);
            *task = task.clone().with_context(context_needs);
        }
        
        let execution_strategy = if tasks.len() == 1 {
            ExecutionStrategy::Sequential
        } else if tasks.iter().all(|t| t.dependencies.is_empty()) {
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
    
    fn estimate_complexity(&self, request: &str) -> Complexity {
        let intent = self.intent_extractor.extract(request);
        self.complexity_estimator.estimate(&intent, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_request() {
        let analyzer = LocalTaskAnalyzer::new();
        let analysis = analyzer.analyze("Add a comment to main.rs").await.unwrap();
        
        assert_eq!(analysis.tasks.len(), 1);
        assert!(matches!(analysis.execution_strategy, ExecutionStrategy::Sequential));
    }
    
    #[tokio::test]
    async fn test_refactor_request() {
        let analyzer = LocalTaskAnalyzer::new();
        let analysis = analyzer.analyze("Refactor the parser module").await.unwrap();
        
        assert_eq!(analysis.tasks.len(), 3);
        assert!(matches!(analysis.execution_strategy, ExecutionStrategy::Pipeline));
    }
    
    #[tokio::test]
    async fn test_complexity_estimation() {
        let analyzer = LocalTaskAnalyzer::new();
        
        let simple = analyzer.estimate_complexity("Add a comment");
        assert!(matches!(simple, Complexity::Trivial | Complexity::Simple));
        
        let complex = analyzer.estimate_complexity("Refactor the entire architecture");
        assert!(matches!(complex, Complexity::Complex));
    }
    
    #[tokio::test]
    async fn test_context_needs() {
        let analyzer = LocalTaskAnalyzer::new();
        let analysis = analyzer.analyze("Modify test.rs and main.rs").await.unwrap();
        
        assert!(!analysis.tasks.is_empty());
        let task = &analysis.tasks[0];
        assert!(!task.context_needs.required_files.is_empty());
    }
}
