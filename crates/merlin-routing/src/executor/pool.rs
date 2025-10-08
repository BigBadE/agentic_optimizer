use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use merlin_core::{Context, FileContext, ModelProvider, Query, TokenUsage};
use merlin_local::LocalModelProvider;
use merlin_providers::{AnthropicProvider, GroqProvider, OpenRouterProvider};

use super::graph::TaskGraph;
use super::state::WorkspaceState;
use crate::{ModelRouter, ModelTier, Result, RoutingError, Task, TaskResult, Validator};

/// Parallel task executor with concurrency limits
pub struct ExecutorPool {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    max_concurrent: usize,
    workspace: Arc<WorkspaceState>,
}

impl ExecutorPool {
    const ENV_OPENROUTER_API_KEY: &'static str = "OPENROUTER_API_KEY";
    const ENV_ANTHROPIC_API_KEY: &'static str = "ANTHROPIC_API_KEY";
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

    /// Create a concrete `ModelProvider` for a specific `ModelTier`.
    ///
    /// # Errors
    /// Returns an error if provider initialization fails or required API keys are missing.
    fn create_provider_for_tier(tier: &ModelTier) -> Result<Arc<dyn ModelProvider>> {
        use std::env;

        match tier {
            ModelTier::Local { model_name } => {
                Ok(Arc::new(LocalModelProvider::new(model_name.clone())))
            }
            ModelTier::Groq { model_name } => {
                let provider = GroqProvider::new()
                    .map_err(|err| RoutingError::Other(err.to_string()))?
                    .with_model(model_name.clone());
                Ok(Arc::new(provider))
            }
            ModelTier::Premium {
                provider: provider_name,
                model_name,
            } => match provider_name.as_str() {
                "openrouter" => {
                    let api_key = env::var(Self::ENV_OPENROUTER_API_KEY).map_err(|_| {
                        RoutingError::Other(format!("{} not set", Self::ENV_OPENROUTER_API_KEY))
                    })?;
                    let provider = OpenRouterProvider::new(api_key)?.with_model(model_name.clone());
                    Ok(Arc::new(provider))
                }
                "anthropic" => {
                    let api_key = env::var(Self::ENV_ANTHROPIC_API_KEY).map_err(|_| {
                        RoutingError::Other(format!("{} not set", Self::ENV_ANTHROPIC_API_KEY))
                    })?;
                    let provider = AnthropicProvider::new(api_key)?;
                    Ok(Arc::new(provider))
                }
                _ => Err(RoutingError::Other(format!(
                    "Unknown provider: {provider_name}"
                ))),
            },
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
                    drop(permit);
                    result
                });
            }

            if let Some(result) = join_set.join_next().await {
                let task_result =
                    result.map_err(|err| RoutingError::ExecutionFailed(err.to_string()))??;
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
        let start = Instant::now();

        let routing_decision = router.route(&task).await?;

        // Create provider based on tier
        let provider = Self::create_provider_for_tier(&routing_decision.tier)?;

        // Build context from task with agent-aware system prompt
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
            workspace.root_path().display(),
            task.description
        );

        let mut context = Context::new(&system_prompt);

        // Add files from workspace if specified
        for file_path in &task.context_needs.required_files {
            if let Some(content) = workspace.read_file(file_path).await {
                context = context.with_files(vec![FileContext::new(file_path.clone(), content)]);
            }
        }

        // Create query
        let query = Query::new(task.description.clone());

        // Execute with provider
        let response = provider
            .generate(&query, &context)
            .await
            .map_err(|err| RoutingError::Other(err.to_string()))?;

        let validation = validator.validate(&response, &task).await?;

        if !validation.passed {
            return Err(RoutingError::ValidationFailed(validation));
        }

        Ok(TaskResult {
            task_id: task.id,
            response,
            tier_used: routing_decision.tier.to_string(),
            tokens_used: TokenUsage::default(),
            validation,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "Test code is allowed to use expect")]
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
    #[ignore = "Requires actual Ollama instance"]
    /// # Panics
    /// Panics if executing the graph returns an error in the test harness.
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
