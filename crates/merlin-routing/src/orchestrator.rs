use std::collections::HashSet;
use std::env::var;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use merlin_core::{Context, FileContext, ModelProvider, Query, Response, TokenUsage};
use merlin_local::LocalModelProvider;
use merlin_providers::{AnthropicProvider, GroqProvider, OpenRouterProvider};

use crate::{
    AgentExecutor, ConflictAwareTaskGraph, ExecutorPool, ListFilesTool,
    LocalTaskAnalyzer, ModelRouter, ModelTier, ReadFileTool, Result, RoutingConfig, RunCommandTool,
    StrategyRouter, Task, TaskAnalysis, TaskAnalyzer, TaskGraph, TaskResult, ToolRegistry, UiChannel,
    ValidationPipeline, Validator, ValidationResult, WorkspaceState, WriteFileTool, RoutingError,
};

/// High-level orchestrator that coordinates all routing components
#[derive(Clone)]
pub struct RoutingOrchestrator {
    config: RoutingConfig,
    analyzer: Arc<dyn TaskAnalyzer>,
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    workspace: Arc<WorkspaceState>,
}

impl RoutingOrchestrator {
    #[must_use]
    pub fn new(config: RoutingConfig) -> Self {
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
        }
    }

    /// Attempt to escalate to a higher tier and generate a response
    ///
    /// # Errors
    /// Returns an error if provider interaction fails
    async fn try_escalate(&self, tier: &ModelTier, query: &Query, context: &Context) -> Result<Option<Response>> {
        let Some(higher_tier) = tier.escalate() else {
            return Ok(None);
        };

        let escalated_provider = self.create_provider(&higher_tier)?;
        let result = escalated_provider
            .generate(query, context)
            .await
            .map_err(|error| RoutingError::Other(error.to_string()))?;
        Ok(Some(result))
    }
    
    #[must_use]
    pub fn with_analyzer(mut self, analyzer: Arc<dyn TaskAnalyzer>) -> Self {
        self.analyzer = analyzer;
        self
    }
    
    #[must_use]
    pub fn with_router(mut self, router: Arc<dyn ModelRouter>) -> Self {
        self.router = router;
        self
    }
    
    #[must_use]
    pub fn with_validator(mut self, validator: Arc<dyn Validator>) -> Self {
        self.validator = validator;
        self
    }
    
    /// Analyze a user request and decompose into tasks
    ///
    /// # Errors
    /// Returns an error if analysis fails
    pub async fn analyze_request(&self, request: &str) -> Result<TaskAnalysis> {
        self.analyzer.analyze(request).await
    }
    
    /// Execute a task with streaming and tool support
    ///
    /// # Errors
    /// Returns an error if task execution or validation fails, or if a provider interaction returns an error
    pub async fn execute_task_streaming(
        &self,
        task: Task,
        ui_channel: UiChannel,
    ) -> Result<TaskResult> {
        // Create tool registry with workspace tools
        let tool_registry = Arc::new(
            ToolRegistry::new()
                .with_tool(Arc::new(ReadFileTool::new(self.config.workspace.root_path.clone())))
                .with_tool(Arc::new(WriteFileTool::new(self.config.workspace.root_path.clone())))
                .with_tool(Arc::new(ListFilesTool::new(self.config.workspace.root_path.clone())))
                .with_tool(Arc::new(RunCommandTool::new(self.config.workspace.root_path.clone())))
        );
        
        // Create agent executor
        let mut executor = AgentExecutor::new(
            Arc::clone(&self.router),
            Arc::clone(&self.validator),
            tool_registry,
        );
        
        // Execute with streaming
        executor.execute_streaming(task, ui_channel).await
    }
    
    /// Execute a single task with routing and validation (legacy method)
    ///
    /// # Errors
    /// Returns an error if routing, provider execution, or validation fails
    pub async fn execute_task(&self, task: Task) -> Result<TaskResult> {
        let start = Instant::now();
        let decision = self.router.route(&task).await?;
        
        // Create provider based on tier
        let provider = self.create_provider(&decision.tier)?;
        
        // Build context from task requirements
        let context = self.build_context(&task).await?;
        
        // Create query
        let query = Query::new(task.description.clone());
        
        // Execute with retries and escalation
        let response = self
            .execute_with_retry(&provider, &query, &context, &decision.tier)
            .await?;
        
        // Validate if enabled
        let validation = if self.config.validation.enabled {
            self.validator.validate(&response, &task).await?
        } else {
            ValidationResult::default()
        };
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        Ok(TaskResult {
            task_id: task.id,
            response,
            tier_used: decision.tier.to_string(),
            tokens_used: TokenUsage::default(),
            validation,
            duration_ms,
        })
    }

    /// Create a concrete provider for the selected `ModelTier`
    ///
    /// # Errors
    /// Returns an error when the selected tier is disabled or required API keys are not configured
    fn create_provider(&self, tier: &ModelTier) -> Result<Arc<dyn ModelProvider>> {
        match tier {
            ModelTier::Local { model_name } => {
                if !self.config.tiers.local_enabled {
                    return Err(RoutingError::NoAvailableTier);
                }
                Ok(Arc::new(LocalModelProvider::new(model_name.clone())))
            }
            ModelTier::Groq { model_name } => {
                if !self.config.tiers.groq_enabled {
                    return Err(RoutingError::NoAvailableTier);
                }
                let provider = GroqProvider::new()
                    .map_err(|error| RoutingError::Other(error.to_string()))?
                    .with_model(model_name.clone());
                Ok(Arc::new(provider))
            }
            ModelTier::Premium { provider: provider_name, model_name } => {
                if !self.config.tiers.premium_enabled {
                    return Err(RoutingError::NoAvailableTier);
                }

                match provider_name.as_str() {
                    "openrouter" => {
                        let api_key = var("OPENROUTER_API_KEY")
                            .map_err(|_| RoutingError::Other("OPENROUTER_API_KEY not set".to_owned()))?;
                        let provider = OpenRouterProvider::new(api_key)?
                            .with_model(model_name.clone());
                        Ok(Arc::new(provider))
                    }
                    "anthropic" => {
                        let api_key = var("ANTHROPIC_API_KEY")
                            .map_err(|_| RoutingError::Other("ANTHROPIC_API_KEY not set".to_owned()))?;
                        let provider = AnthropicProvider::new(api_key)?;
                        Ok(Arc::new(provider))
                    }
                    _ => Err(RoutingError::Other(format!("Unknown provider: {provider_name}"))),
                }
            }
        }
    }

    /// Build a `Context` for the given task
    ///
    /// # Errors
    /// Returns an error if reading required files fails
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

        for file_path in &task.context_needs.required_files {
            if let Some(content) = self.workspace.read_file(file_path).await {
                context = context.with_files(vec![FileContext::new(
                    file_path.clone(),
                    content,
                )]);
            }
        }

        Ok(context)
    }

    /// Execute with retry and optional escalation on failure
    ///
    /// # Errors
    /// Returns an error if retries are exhausted and escalation also fails
    pub async fn execute_with_retry(
        &self,
        provider: &Arc<dyn ModelProvider>,
        query: &Query,
        context: &Context,
        tier: &ModelTier,
    ) -> Result<Response> {
        let mut attempts = 0;
        let max_retries = self.config.tiers.max_retries;
        
        loop {
            match provider.generate(query, context).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    attempts += 1;
                    if attempts < max_retries {
                        // Wait before retry and continue the loop
                        sleep(Duration::from_millis(1000 * attempts as u64)).await;
                        continue;
                    }

                    if let Some(response) = self.try_escalate(tier, query, context).await? {
                        return Ok(response);
                    }

                    return Err(RoutingError::Other(format!(
                        "Failed after {max_retries} retries: {error}"
                    )));
                }
            }
        }
    }
    
    /// Execute multiple tasks with dependency management
    ///
    /// # Errors
    /// Returns an error if conflict detection or task execution fails
    pub async fn execute_tasks(&self, tasks: Vec<Task>) -> Result<Vec<TaskResult>> {
        if self.config.execution.enable_conflict_detection {
            let graph = ConflictAwareTaskGraph::from_tasks(&tasks);
            
            if graph.has_cycles() {
                return Err(RoutingError::CyclicDependency);
            }
            
            // TODO: Implement conflict-aware execution
            // For now, use basic executor
            let executor = ExecutorPool::new(
                Arc::clone(&self.router),
                Arc::clone(&self.validator),
                self.config.execution.max_concurrent_tasks,
                Arc::clone(&self.workspace),
            );
            
            let basic_graph = TaskGraph::from_tasks(
                &graph.ready_non_conflicting_tasks(&HashSet::default(), &HashSet::default())
            );
            
            executor.execute_graph(basic_graph).await
        } else {
            let graph = TaskGraph::from_tasks(&tasks);
            
            let executor = ExecutorPool::new(
                Arc::clone(&self.router),
                Arc::clone(&self.validator),
                self.config.execution.max_concurrent_tasks,
                Arc::clone(&self.workspace),
            );
            
            executor.execute_graph(graph).await
        }
    }
    
    /// Complete workflow: analyze request → execute tasks → return results
    ///
    /// # Errors
    /// Returns an error if analysis or execution fails.
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
        Arc::clone(&self.workspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    /// # Panics
    /// Panics if default configuration does not have local tier enabled.
    async fn test_orchestrator_creation() {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        
        assert!(orchestrator.config().tiers.local_enabled);
    }
    
    #[tokio::test]
    /// # Panics
    /// Panics if `analyze_request` returns an error in the test harness.
    async fn test_analyze_request() {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        
        let analysis = match orchestrator.analyze_request("Add a comment to main.rs").await {
            Ok(analysis) => analysis,
            Err(error) => panic!("analyze_request failed: {error}"),
        };
        assert!(!analysis.tasks.is_empty());
    }
    
    #[tokio::test]
    #[ignore = "Requires actual provider instances"]
    /// # Panics
    /// Panics if `process_request` returns an error in the test harness.
    async fn test_process_simple_request() {
        let config = RoutingConfig::default();
        let orchestrator = RoutingOrchestrator::new(config);
        
        let results = match orchestrator.process_request("Add a comment").await {
            Ok(results) => results,
            Err(error) => panic!("process_request failed: {error}"),
        };
        assert!(!results.is_empty());
    }
}

