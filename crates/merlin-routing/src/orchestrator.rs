use std::sync::Arc;
use std::env;
use crate::{
    ConflictAwareTaskGraph, ExecutorPool, FileLockManager, LocalTaskAnalyzer, ModelRouter, ModelTier,
    Result, RoutingConfig, StrategyRouter, Task, TaskAnalysis, TaskAnalyzer, TaskResult,
    ValidationPipeline, Validator, WorkspaceState,
};
use merlin_core::{Context, ModelProvider, Query};

/// High-level orchestrator that coordinates all routing components
#[derive(Clone)]
pub struct RoutingOrchestrator {
    config: RoutingConfig,
    analyzer: Arc<dyn TaskAnalyzer>,
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    workspace: Arc<WorkspaceState>,
    lock_manager: Arc<FileLockManager>,
}

impl RoutingOrchestrator {
    #[must_use] 
    pub fn new(config: RoutingConfig) -> Self {
        let lock_manager = FileLockManager::new();
        
        let analyzer = Arc::new(LocalTaskAnalyzer::new()
            .with_max_parallel(config.execution.max_concurrent_tasks));
        
        let router = Arc::new(StrategyRouter::with_default_strategies()
            .with_tier_config(
                config.tiers.local_enabled,
                config.tiers.groq_enabled,
                config.tiers.premium_enabled
            ));
        
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
        let start = std::time::Instant::now();
        let decision = self.router.route(&task).await?;
        
        // Create provider based on tier
        let provider = self.create_provider(&decision.tier)?;
        
        // Build context from task requirements
        let context = self.build_context(&task).await?;
        
        // Create query
        let query = Query::new(task.description.clone());
        
        // Execute with retries and escalation
        let response = self.execute_with_retry(&provider, &query, &context, &decision.tier, task.clone()).await?;
        
        // Validate if enabled
        let validation = if self.config.validation.enabled {
            self.validator.validate(&response, &task).await?
        } else {
            crate::ValidationResult::default()
        };
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        Ok(TaskResult {
            task_id: task.id,
            response,
            tier_used: decision.tier.to_string(),
            validation,
            duration_ms,
        })
    }
    
    fn create_provider(&self, tier: &ModelTier) -> Result<Arc<dyn ModelProvider>> {
        match tier {
            ModelTier::Local { model_name } => {
                if !self.config.tiers.local_enabled {
                    return Err(crate::RoutingError::NoAvailableTier);
                }
                Ok(Arc::new(merlin_local::LocalModelProvider::new(model_name.clone())))
            }
            ModelTier::Groq { model_name } => {
                if !self.config.tiers.groq_enabled {
                    return Err(crate::RoutingError::NoAvailableTier);
                }
                let provider = merlin_providers::GroqProvider::new()
                    .map_err(|e| crate::RoutingError::Other(e.to_string()))?
                    .with_model(model_name.clone());
                Ok(Arc::new(provider))
            }
            ModelTier::Premium { provider: provider_name, model_name } => {
                if !self.config.tiers.premium_enabled {
                    return Err(crate::RoutingError::NoAvailableTier);
                }
                
                match provider_name.as_str() {
                    "openrouter" => {
                        let api_key = env::var("OPENROUTER_API_KEY")
                            .map_err(|_| crate::RoutingError::Other("OPENROUTER_API_KEY not set".to_string()))?;
                        let provider = merlin_providers::OpenRouterProvider::new(api_key)?
                            .with_model(model_name.clone());
                        Ok(Arc::new(provider))
                    }
                    "anthropic" => {
                        let api_key = env::var("ANTHROPIC_API_KEY")
                            .map_err(|_| crate::RoutingError::Other("ANTHROPIC_API_KEY not set".to_string()))?;
                        // Note: AnthropicProvider uses a fixed model, ignoring model_name for now
                        let provider = merlin_providers::AnthropicProvider::new(api_key)?;
                        Ok(Arc::new(provider))
                    }
                    _ => Err(crate::RoutingError::Other(format!("Unknown provider: {provider_name}")))
                }
            }
        }
    }
    
    async fn build_context(&self, task: &Task) -> Result<Context> {
        let system_prompt = format!(
            "You are Merlin, an AI coding agent working directly in the user's codebase at '{}'.\n\n\
            Your role:\n\
            - Analyze the existing code structure and patterns\n\
            - Provide code changes that integrate seamlessly with the existing codebase\n\
            - Follow the project's coding style and conventions\n\
            - Give specific, actionable suggestions with file paths and line numbers when relevant\n\
            - Explain your reasoning when making architectural decisions\n\n\
            Task: {}\n\n\
            Provide clear, correct, and contextually appropriate code solutions.",
            self.workspace.root_path().display(),
            task.description
        );
        
        let mut context = Context::new(&system_prompt);
        
        // Add files from workspace if specified
        for file_path in &task.context_needs.required_files {
            if let Some(content) = self.workspace.read_file(file_path).await {
                context = context.with_files(vec![merlin_core::FileContext::new(
                    file_path.clone(),
                    content,
                )]);
            }
        }
        
        Ok(context)
    }
    
    async fn execute_with_retry(
        &self,
        provider: &Arc<dyn ModelProvider>,
        query: &Query,
        context: &Context,
        tier: &ModelTier,
        _task: Task,
    ) -> Result<merlin_core::Response> {
        let mut attempts = 0;
        let max_retries = self.config.tiers.max_retries;
        
        loop {
            match provider.generate(query, context).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    attempts += 1;
                    
                    if attempts >= max_retries {
                        // Try escalation
                        if let Some(higher_tier) = tier.escalate() {
                            let escalated_provider = self.create_provider(&higher_tier)?;
                            return escalated_provider.generate(query, context).await
                                .map_err(|e| crate::RoutingError::Other(e.to_string()));
                        }
                        
                        return Err(crate::RoutingError::Other(format!(
                            "Failed after {max_retries} retries: {e}"
                        )));
                    }
                    
                    // Wait before retry
                    tokio::time::sleep(std::time::Duration::from_millis(1000 * attempts as u64)).await;
                }
            }
        }
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
    
    #[must_use] 
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }
    
    #[must_use] 
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
    #[ignore] // Requires actual provider instances
    async fn test_process_simple_request() {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        
        let results = orchestrator.process_request("Add a comment").await.unwrap();
        assert!(!results.is_empty());
    }
}

