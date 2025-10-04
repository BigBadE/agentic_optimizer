use std::sync::Arc;
use crate::{
    ConflictAwareTaskGraph, ExecutorPool, FileLockManager, LocalTaskAnalyzer, ModelRouter,
    Result, RoutingConfig, StrategyRouter, Task, TaskAnalysis, TaskAnalyzer, TaskResult,
    ValidationPipeline, Validator, WorkspaceState,
};

/// High-level orchestrator that coordinates all routing components
pub struct RoutingOrchestrator {
    config: RoutingConfig,
    analyzer: Arc<dyn TaskAnalyzer>,
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    workspace: Arc<WorkspaceState>,
    lock_manager: Arc<FileLockManager>,
}

impl RoutingOrchestrator {
    pub fn new(config: RoutingConfig) -> Self {
        let lock_manager = FileLockManager::new();
        
        let analyzer = Arc::new(LocalTaskAnalyzer::new()
            .with_max_parallel(config.execution.max_concurrent_tasks));
        
        let router = Arc::new(StrategyRouter::with_default_strategies());
        
        let validator = Arc::new(ValidationPipeline::with_default_stages()
            .with_early_exit(config.validation.early_exit));
        
        let workspace = WorkspaceState::new(config.workspace.root_path.clone());
        
        Self {
            config,
            analyzer,
            router,
            validator,
            workspace,
            lock_manager,
        }
    }
    
    pub fn with_analyzer(mut self, analyzer: Arc<dyn TaskAnalyzer>) -> Self {
        self.analyzer = analyzer;
        self
    }
    
    pub fn with_router(mut self, router: Arc<dyn ModelRouter>) -> Self {
        self.router = router;
        self
    }
    
    pub fn with_validator(mut self, validator: Arc<dyn Validator>) -> Self {
        self.validator = validator;
        self
    }
    
    /// Analyze a user request and decompose into tasks
    pub async fn analyze_request(&self, request: &str) -> Result<TaskAnalysis> {
        self.analyzer.analyze(request).await
    }
    
    /// Execute a single task with routing and validation
    pub async fn execute_task(&self, task: Task) -> Result<TaskResult> {
        let decision = self.router.route(&task).await?;
        
        // TODO: Use decision.tier to select appropriate provider
        // For now, return a mock result
        Ok(TaskResult {
            task_id: task.id,
            response: agentic_core::Response {
                text: "Task executed successfully".to_string(),
                confidence: 0.9,
                tokens_used: agentic_core::TokenUsage::default(),
                provider: decision.tier.to_string(),
                latency_ms: decision.estimated_latency_ms,
            },
            tier_used: decision.tier.to_string(),
            validation: crate::ValidationResult::default(),
            duration_ms: decision.estimated_latency_ms,
        })
    }
    
    /// Execute multiple tasks with dependency management
    pub async fn execute_tasks(&self, tasks: Vec<Task>) -> Result<Vec<TaskResult>> {
        if self.config.execution.enable_conflict_detection {
            let graph = ConflictAwareTaskGraph::from_tasks(tasks);
            
            if graph.has_cycles() {
                return Err(crate::RoutingError::CyclicDependency);
            }
            
            // TODO: Implement conflict-aware execution
            // For now, use basic executor
            let executor = ExecutorPool::new(
                self.router.clone(),
                self.validator.clone(),
                self.config.execution.max_concurrent_tasks,
                self.workspace.clone(),
            );
            
            let basic_graph = crate::TaskGraph::from_tasks(
                graph.ready_non_conflicting_tasks(&Default::default(), &Default::default())
            );
            
            executor.execute_graph(basic_graph).await
        } else {
            let graph = crate::TaskGraph::from_tasks(tasks);
            
            let executor = ExecutorPool::new(
                self.router.clone(),
                self.validator.clone(),
                self.config.execution.max_concurrent_tasks,
                self.workspace.clone(),
            );
            
            executor.execute_graph(graph).await
        }
    }
    
    /// Complete workflow: analyze request → execute tasks → return results
    pub async fn process_request(&self, request: &str) -> Result<Vec<TaskResult>> {
        let analysis = self.analyze_request(request).await?;
        
        self.execute_tasks(analysis.tasks).await
    }
    
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }
    
    pub fn workspace(&self) -> Arc<WorkspaceState> {
        self.workspace.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        
        assert!(orchestrator.config().tiers.local_enabled);
    }
    
    #[tokio::test]
    async fn test_analyze_request() {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        
        let analysis = orchestrator.analyze_request("Add a comment to main.rs").await.unwrap();
        assert!(!analysis.tasks.is_empty());
    }
    
    #[tokio::test]
    async fn test_process_simple_request() {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        
        let results = orchestrator.process_request("Add a comment").await.unwrap();
        assert!(!results.is_empty());
    }
}
