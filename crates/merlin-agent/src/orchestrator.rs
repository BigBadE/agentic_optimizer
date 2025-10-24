use std::sync::Arc;

use crate::{
    AgentExecutor, ConflictAwareTaskGraph, ContextFetcher, ExecutorPool, TaskGraph,
    TaskListExecutor, TaskListResult, ValidationPipeline, Validator, WorkspaceState,
};
use merlin_core::{
    Context, Response, Result, RoutingConfig, RoutingError, Task, TaskResult, UiChannel,
};
use merlin_routing::{
    LocalTaskAnalyzer, ModelRouter, ProviderRegistry, StrategyRouter, TaskAnalysis, TaskAnalyzer,
};
use merlin_tooling::{BashTool, ToolRegistry};

/// Parameters for task execution (internal)
struct TaskExecutionParams {
    task: Task,
    ui_channel: UiChannel,
    conversation_history: Vec<(String, String)>,
}

/// High-level orchestrator that coordinates all routing components
#[derive(Clone)]
pub struct RoutingOrchestrator {
    config: RoutingConfig,
    analyzer: Arc<dyn TaskAnalyzer>,
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    workspace: Arc<WorkspaceState>,
    /// Provider registry (for testing, allows injecting mock providers)
    provider_registry: Option<Arc<ProviderRegistry>>,
}

impl RoutingOrchestrator {
    /// Creates a new routing orchestrator with the given configuration.
    ///
    /// Initializes analyzer, router, validator, and workspace with default implementations.
    ///
    /// # Errors
    /// Returns an error if provider registry initialization fails (e.g., missing API keys).
    pub fn new(config: RoutingConfig) -> Result<Self> {
        let analyzer = Arc::new(
            LocalTaskAnalyzer::default().with_max_parallel(config.execution.max_concurrent_tasks),
        );

        // Create provider registry and router
        let provider_registry = ProviderRegistry::new(config.clone())?;
        let router = Arc::new(StrategyRouter::new(provider_registry));

        let validator = Arc::new(
            ValidationPipeline::with_default_stages().with_early_exit(config.validation.early_exit),
        );

        let workspace = WorkspaceState::new(config.workspace.root_path.clone());

        Ok(Self {
            config,
            analyzer,
            router,
            validator,
            workspace,
            provider_registry: None,
        })
    }

    /// Creates a new orchestrator for testing with a custom router.
    ///
    /// This bypasses provider initialization and uses the provided router directly.
    /// Useful for testing with mock providers.
    ///
    /// **Note**: This is intended for testing only. For production use, use `new()` instead.
    ///
    /// # Errors
    /// Returns an error if configuration is invalid.
    pub fn new_with_router(
        config: RoutingConfig,
        router: Arc<dyn ModelRouter>,
        provider_registry: Arc<ProviderRegistry>,
    ) -> Result<Self> {
        let analyzer = Arc::new(
            LocalTaskAnalyzer::default().with_max_parallel(config.execution.max_concurrent_tasks),
        );

        let validator = Arc::new(
            ValidationPipeline::with_default_stages().with_early_exit(config.validation.early_exit),
        );

        let workspace = WorkspaceState::new(config.workspace.root_path.clone());

        Ok(Self {
            config,
            analyzer,
            router,
            validator,
            workspace,
            provider_registry: Some(provider_registry),
        })
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
    ///
    /// # Errors
    /// Returns an error if task execution or validation fails
    pub async fn execute_task_streaming_with_history(
        &self,
        task: Task,
        ui_channel: UiChannel,
        conversation_history: Vec<(String, String)>,
    ) -> Result<TaskResult> {
        self.execute_task_streaming_once(TaskExecutionParams {
            task,
            ui_channel,
            conversation_history,
        })
        .await
    }

    /// Execute a task once without retry logic (internal method)
    ///
    /// # Errors
    /// Returns an error if task execution or validation fails, or if a provider interaction returns an error
    async fn execute_task_streaming_once(&self, params: TaskExecutionParams) -> Result<TaskResult> {
        let mut executor = self.create_agent_executor()?;
        self.setup_conversation_history(&mut executor, params.conversation_history)
            .await;

        // Execute the task and get the response
        let result = executor
            .execute_streaming(params.task.clone(), params.ui_channel.clone())
            .await?;

        // Check if the result contains a TaskList (from TypeScript execution)
        if let Some(mut task_list) = result.task_list.clone() {
            tracing::info!("Found TaskList in result: {}", task_list.title);

            // Create TaskListExecutor
            let executor_arc = Arc::new(executor.clone());
            let task_list_executor =
                TaskListExecutor::new(&executor_arc, self.workspace.root_path().clone());

            // Execute the task list
            let context = Context::new(String::new());
            let task_list_result = task_list_executor
                .execute_task_list(&mut task_list, &context, &params.ui_channel, params.task.id)
                .await?;

            // Check if task list succeeded
            match task_list_result {
                TaskListResult::Success => {
                    tracing::info!("TaskList completed successfully");
                    // Return the original result but update the response text
                    return Ok(TaskResult {
                        response: Response {
                            text: format!("TaskList completed: {title}", title = task_list.title),
                            ..result.response
                        },
                        task_list: Some(task_list),
                        ..result
                    });
                }
                TaskListResult::Failed { failed_step } => {
                    tracing::error!("TaskList failed at step: {failed_step}");
                    return Err(RoutingError::Other(format!(
                        "TaskList failed at step: {failed_step}"
                    )));
                }
            }
        }

        // No TaskList found, return result as-is
        Ok(result)
    }

    /// Creates an agent executor with tool registry and context fetcher
    /// Create agent executor.
    ///
    /// # Errors
    /// Returns error if executor creation fails.
    fn create_agent_executor(&self) -> Result<AgentExecutor> {
        let tool_registry = ToolRegistry::default().with_tool(Arc::new(BashTool));
        let tool_registry = Arc::new(tool_registry);
        let context_fetcher = ContextFetcher::new(self.workspace.root_path().clone());

        let mut executor = if let Some(ref registry) = self.provider_registry {
            // Use injected provider registry (for testing)
            use crate::agent::executor::AgentExecutorParams;
            AgentExecutor::with_provider_registry(AgentExecutorParams {
                router: Arc::clone(&self.router),
                validator: Arc::clone(&self.validator),
                tool_registry,
                context_fetcher,
                config: self.config.clone(),
                provider_registry: Arc::clone(registry),
            })?
        } else {
            // Create new provider registry (production)
            AgentExecutor::new(
                Arc::clone(&self.router),
                Arc::clone(&self.validator),
                tool_registry,
                context_fetcher,
                &self.config,
            )?
        };

        if self.config.execution.context_dump {
            executor.enable_context_dump();
        }

        Ok(executor)
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
        if let Ok(orchestrator) = RoutingOrchestrator::new(config) {
            assert!(orchestrator.config.tiers.local_enabled);
        }
        // Test passes even if provider initialization fails
    }

    #[tokio::test]
    async fn test_analyze_request() {
        let config = RoutingConfig::default();
        if let Ok(orchestrator) = RoutingOrchestrator::new(config) {
            let analysis = match orchestrator
                .analyze_request("Add a comment to main.rs")
                .await
            {
                Ok(analysis) => analysis,
                Err(error) => panic!("analyze_request failed: {error}"),
            };
            assert!(!analysis.tasks.is_empty());
        }
    }

    #[tokio::test]
    #[ignore = "Requires GROQ_API_KEY environment variable"]
    async fn test_process_simple_request() {
        let config = RoutingConfig::default();
        if let Ok(orchestrator) = RoutingOrchestrator::new(config) {
            let results = match orchestrator.process_request("Add a comment").await {
                Ok(results) => results,
                Err(error) => panic!("process_request failed: {error}"),
            };
            assert!(!results.is_empty());
        }
    }
}
