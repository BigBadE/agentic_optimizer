use std::sync::Arc;

use crate::{
    AgentExecutor, BashTool, ConflictAwareTaskGraph, ContextFetcher, ExecutorPool,
    LocalTaskAnalyzer, MessageLevel, ModelRouter, ModelTier, Result, RoutingConfig, RoutingError,
    StrategyRouter, Task, TaskAnalysis, TaskAnalyzer, TaskGraph, TaskResult, ToolRegistry,
    UiChannel, UiEvent, ValidationPipeline, Validator, WorkspaceState,
};

/// Parameters for task execution (internal)
struct TaskExecutionParams {
    task: Task,
    ui_channel: UiChannel,
    conversation_history: Vec<(String, String)>,
    tier_override: Option<ModelTier>,
    is_retry: bool,
}

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
    /// Creates a new routing orchestrator with the given configuration.
    ///
    /// Initializes analyzer, router, validator, and workspace with default implementations.
    pub fn new(config: RoutingConfig) -> Self {
        let analyzer = Arc::new(
            LocalTaskAnalyzer::default().with_max_parallel(config.execution.max_concurrent_tasks),
        );

        let router = Arc::new(StrategyRouter::with_default_strategies().with_tier_config(
            config.tiers.local_enabled,
            config.tiers.groq_enabled,
            config.tiers.premium_enabled,
        ));

        let validator = Arc::new(
            ValidationPipeline::with_default_stages().with_early_exit(config.validation.early_exit),
        );

        let workspace = WorkspaceState::new(config.workspace.root_path.clone());

        Self {
            config,
            analyzer,
            router,
            validator,
            workspace,
        }
    }

    /// Sets a custom task analyzer.
    #[must_use]
    pub fn with_analyzer(mut self, analyzer: Arc<dyn TaskAnalyzer>) -> Self {
        self.analyzer = analyzer;
        self
    }

    /// Sets a custom model router.
    #[must_use]
    pub fn with_router(mut self, router: Arc<dyn ModelRouter>) -> Self {
        self.router = router;
        self
    }

    /// Sets a custom validator.
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
        self.execute_task_streaming_with_history(task, ui_channel, Vec::new())
            .await
    }

    /// Execute a task with streaming, tool support, and conversation history.
    /// Automatically retries with escalated tiers on failure (up to 3 retries).
    ///
    /// # Errors
    /// Returns an error if task execution or validation fails after all retries
    pub async fn execute_task_streaming_with_history(
        &self,
        task: Task,
        ui_channel: UiChannel,
        conversation_history: Vec<(String, String)>,
    ) -> Result<TaskResult> {
        const MAX_RETRIES: usize = 3;
        let mut tier_override: Option<ModelTier> = None;
        let mut retry_count = 0;

        loop {
            let is_retry = retry_count > 0;

            let result = self
                .execute_task_streaming_once(TaskExecutionParams {
                    task: task.clone(),
                    ui_channel: ui_channel.clone(),
                    conversation_history: conversation_history.clone(),
                    tier_override: tier_override.clone(),
                    is_retry,
                })
                .await;

            match result {
                Ok(task_result) => return Ok(task_result),
                Err(error) => {
                    retry_count += 1;

                    // Log the failure for debugging (error message only)
                    tracing::error!(
                        "Task attempt {} failed with error: {:?}",
                        retry_count,
                        error
                    );

                    if retry_count >= MAX_RETRIES {
                        tracing::error!("Task failed after {} retries: {}", MAX_RETRIES, error);
                        return Err(error);
                    }

                    // Get the tier that was used (or would have been used)
                    let Ok(current_tier) =
                        self.get_current_tier(&task, tier_override.as_ref()).await
                    else {
                        tracing::error!("Failed to route task for escalation");
                        return Err(error);
                    };

                    // Try to escalate tier
                    if let Some(escalated_tier) = current_tier.escalate() {
                        tracing::warn!(
                            "Task failed with tier {}, escalating to {} (attempt {}/{})",
                            current_tier,
                            escalated_tier,
                            retry_count + 1,
                            MAX_RETRIES
                        );

                        // Send task retry event
                        ui_channel.send(UiEvent::TaskRetrying {
                            task_id: task.id,
                            retry_count: retry_count as u32,
                            error: error.to_string(),
                        });

                        ui_channel.send(UiEvent::SystemMessage {
                            level: MessageLevel::Warning,
                            message: format!(
                                "Escalating to {} (attempt {}/{})",
                                escalated_tier,
                                retry_count + 1,
                                MAX_RETRIES
                            ),
                        });

                        tier_override = Some(escalated_tier);
                    } else {
                        tracing::error!("Task failed and cannot escalate further: {}", error);
                        return Err(error);
                    }
                }
            }
        }
    }

    /// Get the current tier for escalation purposes
    ///
    /// # Errors
    /// Returns an error if routing fails when `tier_override` is `None`
    async fn get_current_tier(
        &self,
        task: &Task,
        tier_override: Option<&ModelTier>,
    ) -> Result<ModelTier> {
        if let Some(override_tier) = tier_override {
            Ok(override_tier.clone())
        } else {
            let decision = self.router.route(task).await?;
            Ok(decision.tier)
        }
    }

    /// Execute a task once without retry logic (internal method)
    ///
    /// # Errors
    /// Returns an error if task execution or validation fails, or if a provider interaction returns an error
    async fn execute_task_streaming_once(&self, params: TaskExecutionParams) -> Result<TaskResult> {
        let mut executor = self.create_agent_executor(params.is_retry);
        self.setup_conversation_history(&mut executor, params.conversation_history)
            .await;

        executor
            .execute_streaming_with_tier_override_internal(
                params.task,
                params.ui_channel.clone(),
                params.tier_override,
                params.is_retry,
            )
            .await
    }

    /// Creates an agent executor with tool registry and context fetcher
    fn create_agent_executor(&self, is_retry: bool) -> AgentExecutor {
        let tool_registry = ToolRegistry::default().with_tool(Arc::new(BashTool));
        let tool_registry = Arc::new(tool_registry);
        let context_fetcher = ContextFetcher::new(self.workspace.root_path().clone());

        let mut executor = AgentExecutor::new(
            Arc::clone(&self.router),
            Arc::clone(&self.validator),
            tool_registry,
            context_fetcher,
            self.config.clone(),
        );

        if self.config.execution.context_dump && !is_retry {
            executor.enable_context_dump();
        }

        executor
    }

    /// Sets up conversation history on the executor
    async fn setup_conversation_history(
        &self,
        executor: &mut AgentExecutor,
        conversation_history: Vec<(String, String)>,
    ) {
        if conversation_history.is_empty() {
            tracing::info!("No conversation history to set");
        } else {
            tracing::info!(
                "Setting conversation history with {} messages",
                conversation_history.len()
            );
            executor
                .set_conversation_history(conversation_history)
                .await;
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

            let executor = ExecutorPool::new(
                Arc::clone(&self.router),
                Arc::clone(&self.validator),
                self.config.execution.max_concurrent_tasks,
                Arc::clone(&self.workspace),
            );

            executor.execute_conflict_aware_graph(graph).await
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
        let results = self.execute_tasks(analysis.tasks.clone()).await?;
        Ok(results)
    }

    /// Gets the routing configuration.
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }

    /// Gets a reference to the workspace state.
    pub fn workspace(&self) -> Arc<WorkspaceState> {
        Arc::clone(&self.workspace)
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

        let analysis = match orchestrator
            .analyze_request("Add a comment to main.rs")
            .await
        {
            Ok(analysis) => analysis,
            Err(error) => panic!("analyze_request failed: {error}"),
        };
        assert!(!analysis.tasks.is_empty());
    }

    #[tokio::test]
    #[ignore = "Requires GROQ_API_KEY environment variable"]
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
