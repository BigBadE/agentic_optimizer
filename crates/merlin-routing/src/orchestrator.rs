use std::env;
use std::path::{Path, PathBuf};
use std::slice::from_ref;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::to_string_pretty;
use tokio::fs;
use tracing::{info, warn};

use crate::user_interface::events::{MessageLevel, UiEvent};
use crate::{
    AgentExecutor, ConflictAwareTaskGraph, ContextFetcher, ExecutorPool, ListFilesTool,
    LocalTaskAnalyzer, ModelRouter, ReadFileTool, Result, RoutingConfig, RoutingError,
    RunCommandTool, StrategyRouter, SubagentTool, Task, TaskAnalysis, TaskAnalyzer, TaskGraph,
    TaskResult, Tool, ToolRegistry, TypeScriptTool, UiChannel, ValidationPipeline, Validator,
    WorkspaceState, WriteFileTool,
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
    /// Get the Merlin folder path, respecting `MERLIN_FOLDER` environment variable
    fn get_merlin_folder(project_root: &Path) -> PathBuf {
        env::var("MERLIN_FOLDER").map_or_else(|_| project_root.join(".merlin"), PathBuf::from)
    }

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

    /// Execute a task with streaming, tool support, and conversation history
    ///
    /// # Errors
    /// Returns an error if task execution or validation fails, or if a provider interaction returns an error
    pub async fn execute_task_streaming_with_history(
        &self,
        task: Task,
        ui_channel: UiChannel,
        conversation_history: Vec<(String, String)>,
    ) -> Result<TaskResult> {
        // Create tool registry with workspace tools
        let workspace_root = self.workspace.root_path().clone();

        // First, create the basic tools
        let basic_tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(ReadFileTool::new(workspace_root.clone())),
            Arc::new(WriteFileTool::new(workspace_root.clone())),
            Arc::new(ListFilesTool::new(workspace_root.clone())),
            Arc::new(RunCommandTool::new(workspace_root)),
        ];

        // Create advanced tools
        let ts_tool = Arc::new(TypeScriptTool::new(basic_tools.clone()));
        let subagent_tool = Arc::new(SubagentTool::new());

        // Build the complete registry
        let mut tool_registry = ToolRegistry::default();
        for tool in basic_tools {
            tool_registry = tool_registry.with_tool(tool);
        }
        tool_registry = tool_registry.with_tool(ts_tool);
        tool_registry = tool_registry.with_tool(subagent_tool);
        let tool_registry = Arc::new(tool_registry);

        // Create context fetcher for building context
        let context_fetcher = ContextFetcher::new(self.workspace.root_path().clone());

        // Create agent executor
        let mut executor = AgentExecutor::new(
            Arc::clone(&self.router),
            Arc::clone(&self.validator),
            tool_registry,
            context_fetcher,
        );

        // Enable context dump if configured
        if self.config.execution.context_dump {
            executor.enable_context_dump();
        }

        // Set conversation history if provided
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

        // Execute with streaming
        let task_clone = task.clone();
        let result = executor.execute_streaming(task, ui_channel.clone()).await?;

        // Best-effort: snapshot the submitted task on completion (non-fatal)
        match self.write_single_task_snapshot(&task_clone).await {
            Ok(path) => {
                info!("Saved task snapshot: {}", path.display());
                ui_channel.send(UiEvent::SystemMessage {
                    level: MessageLevel::Info,
                    message: format!("Saved task snapshot: {}", path.display()),
                });
            }
            Err(error) => {
                warn!("Failed to write task snapshot: {}", error);
            }
        }

        Ok(result)
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

        // Persist analyzed tasks to .merlin/tasks for recovery across restarts (non-fatal)
        if self.write_tasks_snapshot(&analysis.tasks).await.is_err() {
            // Ignore snapshot persistence failures to avoid breaking request processing
        }

        let results = self.execute_tasks(analysis.tasks.clone()).await?;

        // After successful completion, persist the tasks snapshot again (non-fatal)
        if self.write_tasks_snapshot(&analysis.tasks).await.is_err() {
            // Ignore snapshot persistence failures
        }

        Ok(results)
    }

    /// Write the current analyzed tasks to `<workspace>/.merlin/tasks/<timestamp>.json`.
    ///
    /// # Errors
    /// Returns an error if directory creation or file write fails.
    async fn write_tasks_snapshot(&self, tasks: &[Task]) -> Result<PathBuf> {
        let root = self.workspace.root_path().clone();
        let merlin_dir = Self::get_merlin_folder(&root);
        let dir = merlin_dir.join("tasks");
        fs::create_dir_all(&dir).await?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| RoutingError::Other(err.to_string()))?;
        let filename = format!("{}-tasks.json", now.as_millis());
        let path = dir.join(filename);

        let json = to_string_pretty(tasks)?;
        fs::write(&path, json).await?;
        Ok(path)
    }

    /// Write a single task snapshot to `<workspace>/.merlin/tasks/<timestamp>.json`.
    ///
    /// This avoids requiring `Task: Clone` when only a reference is available.
    ///
    /// # Errors
    /// Returns an error if directory creation or file write fails.
    async fn write_single_task_snapshot(&self, task: &Task) -> Result<PathBuf> {
        self.write_tasks_snapshot(from_ref(task)).await
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
