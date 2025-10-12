use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use super::graph::TaskGraph;
use super::scheduler::ConflictAwareTaskGraph;
use super::state::WorkspaceState;
use crate::user_interface::UiChannel;
use crate::{
    AgentExecutor, ContextFetcher, ListFilesTool, ModelRouter, ReadFileTool, Result, RoutingError,
    RunCommandTool, SubagentTool, Task, TaskResult, Tool, ToolRegistry, TypeScriptTool, Validator,
    WriteFileTool,
};

/// Parallel task executor with concurrency limits
pub struct ExecutorPool {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    max_concurrent: usize,
    workspace: Arc<WorkspaceState>,
}

impl ExecutorPool {
    /// Create a new executor pool
    pub fn new(
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        max_concurrent: usize,
        workspace: Arc<WorkspaceState>,
    ) -> Self {
        Self {
            router,
            validator,
            max_concurrent,
            workspace,
        }
    }

    /// Execute task graph with parallel execution
    ///
    /// # Errors
    /// Returns an error if the graph has cycles, if task execution fails, or if acquiring
    /// a semaphore permit fails.
    pub async fn execute_graph(&self, graph: TaskGraph) -> Result<Vec<TaskResult>> {
        if graph.has_cycles() {
            return Err(RoutingError::CyclicDependency);
        }

        let mut completed = HashSet::new();
        let mut running = HashSet::new();
        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        loop {
            let ready = graph.ready_tasks(&completed);

            if ready.is_empty() && join_set.is_empty() {
                break;
            }

            for task in ready {
                if running.contains(&task.id) {
                    continue;
                }

                if join_set.len() >= self.max_concurrent {
                    break;
                }

                running.insert(task.id);

                let router = Arc::clone(&self.router);
                let validator = Arc::clone(&self.validator);
                let workspace = Arc::clone(&self.workspace);
                let permit = Arc::clone(&semaphore)
                    .acquire_owned()
                    .await
                    .map_err(|err| RoutingError::Other(err.to_string()))?;

                join_set.spawn(async move {
                    let result = Self::execute_task(task, router, validator, workspace).await;
                    (result, permit)
                });
            }

            if let Some(joined) = join_set.join_next().await {
                let (task_result_res, _permit) =
                    joined.map_err(|err| RoutingError::ExecutionFailed(err.to_string()))?;
                let task_result = task_result_res?;
                running.remove(&task_result.task_id);
                completed.insert(task_result.task_id);
                results.push(task_result);
            }
        }

        Ok(results)
    }

    /// Execute conflict-aware task graph with file-level conflict detection
    ///
    /// This method ensures that tasks accessing the same files don't run concurrently,
    /// preventing race conditions and file conflicts.
    ///
    /// # Errors
    /// Returns an error if the graph has cycles, if task execution fails, or if acquiring
    /// a semaphore permit fails.
    pub async fn execute_conflict_aware_graph(
        &self,
        graph: ConflictAwareTaskGraph,
    ) -> Result<Vec<TaskResult>> {
        if graph.has_cycles() {
            return Err(RoutingError::CyclicDependency);
        }

        let mut completed = HashSet::new();
        let mut running = HashSet::new();
        let mut results = Vec::new();
        let mut join_set = JoinSet::new();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        loop {
            let ready = graph.ready_non_conflicting_tasks(&completed, &running);

            if ready.is_empty() && join_set.is_empty() {
                break;
            }

            for task in ready {
                if running.contains(&task.id) {
                    continue;
                }

                if join_set.len() >= self.max_concurrent {
                    break;
                }

                running.insert(task.id);

                let router = Arc::clone(&self.router);
                let validator = Arc::clone(&self.validator);
                let workspace = Arc::clone(&self.workspace);
                let permit = Arc::clone(&semaphore)
                    .acquire_owned()
                    .await
                    .map_err(|err| RoutingError::Other(err.to_string()))?;

                join_set.spawn(async move {
                    let result = Self::execute_task(task, router, validator, workspace).await;
                    (result, permit)
                });
            }

            if let Some(joined) = join_set.join_next().await {
                let (task_result_res, _permit) =
                    joined.map_err(|err| RoutingError::ExecutionFailed(err.to_string()))?;
                let task_result = task_result_res?;
                running.remove(&task_result.task_id);
                completed.insert(task_result.task_id);
                results.push(task_result);
            }
        }

        Ok(results)
    }

    /// Execute a single task with the selected provider and validate the response.
    ///
    /// # Errors
    /// Returns an error if routing, provider execution, or validation fails.
    async fn execute_task(
        task: Task,
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        workspace: Arc<WorkspaceState>,
    ) -> Result<TaskResult> {
        // Build tool registry based on workspace root
        let workspace_root = workspace.root_path().clone();

        // First, create the basic tools
        let basic_tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(ReadFileTool::new(workspace_root.clone())),
            Arc::new(WriteFileTool::new(workspace_root.clone())),
            Arc::new(ListFilesTool::new(workspace_root.clone())),
            Arc::new(RunCommandTool::new(workspace_root.clone())),
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

        // Create context fetcher and AgentExecutor
        let context_fetcher = ContextFetcher::new(workspace_root);
        let mut executor = AgentExecutor::new(router, validator, tool_registry, context_fetcher);
        let (sender, _receiver) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(sender);

        executor.execute_streaming(task, ui_channel).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tempfile::TempDir;

    use merlin_core::Response;

    use crate::{ModelTier, RoutingDecision, Task, ValidationResult};

    struct MockRouter;

    #[async_trait]
    impl ModelRouter for MockRouter {
        async fn route(&self, _task: &Task) -> Result<RoutingDecision> {
            Ok(RoutingDecision {
                tier: ModelTier::Local {
                    model_name: "test".to_owned(),
                },
                estimated_cost: 0.0,
                estimated_latency_ms: 0,
                reasoning: "test".to_owned(),
            })
        }

        async fn is_available(&self, _tier: &ModelTier) -> bool {
            true
        }
    }

    struct MockValidator;

    #[async_trait]
    impl Validator for MockValidator {
        async fn validate(&self, _response: &Response, _task: &Task) -> Result<ValidationResult> {
            Ok(ValidationResult::default())
        }

        async fn quick_validate(&self, _response: &Response) -> Result<bool> {
            Ok(true)
        }
    }

    #[tokio::test]
    #[ignore = "Requires Ollama running locally"]
    async fn test_executor_pool_basic() {
        let router = Arc::new(MockRouter);
        let validator = Arc::new(MockValidator);
        let tmp_dir = TempDir::new().expect("create temp dir");
        let workspace = WorkspaceState::new(tmp_dir.path().to_path_buf());

        let executor = ExecutorPool::new(router, validator, 2, workspace);

        let task_a = Task::new("Task A".to_owned());
        let task_b = Task::new("Task B".to_owned());

        let graph = TaskGraph::from_tasks(&[task_a, task_b]);
        let results = match executor.execute_graph(graph).await {
            Ok(results) => results,
            Err(error) => panic!("execute_graph failed: {error}"),
        };

        assert_eq!(results.len(), 2);
    }
}
