use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use crate::{ModelRouter, Result, RoutingError, Task, TaskResult, Validator};
use super::graph::TaskGraph;
use super::state::WorkspaceState;

/// Parallel task executor with concurrency limits
pub struct ExecutorPool {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    max_concurrent: usize,
    workspace: Arc<WorkspaceState>,
}

impl ExecutorPool {
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
    
    fn create_provider_for_tier(tier: &crate::ModelTier) -> Result<Arc<dyn merlin_core::ModelProvider>> {
        use std::env;
        
        match tier {
            crate::ModelTier::Local { model_name } => {
                Ok(Arc::new(merlin_local::LocalModelProvider::new(model_name.clone())))
            }
            crate::ModelTier::Groq { model_name } => {
                let provider = merlin_providers::GroqProvider::new()
                    .map_err(|e| RoutingError::Other(e.to_string()))?
                    .with_model(model_name.clone());
                Ok(Arc::new(provider))
            }
            crate::ModelTier::Premium { provider: provider_name, model_name } => {
                match provider_name.as_str() {
                    "openrouter" => {
                        let api_key = env::var("OPENROUTER_API_KEY")
                            .map_err(|_| RoutingError::Other("OPENROUTER_API_KEY not set".to_string()))?;
                        let provider = merlin_providers::OpenRouterProvider::new(api_key)?
                            .with_model(model_name.clone());
                        Ok(Arc::new(provider))
                    }
                    "anthropic" => {
                        let api_key = env::var("ANTHROPIC_API_KEY")
                            .map_err(|_| RoutingError::Other("ANTHROPIC_API_KEY not set".to_string()))?;
                        let provider = merlin_providers::AnthropicProvider::new(api_key)?;
                        Ok(Arc::new(provider))
                    }
                    _ => Err(RoutingError::Other(format!("Unknown provider: {provider_name}")))
                }
            }
        }
    }
    
    /// Execute task graph with parallel execution
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
                
                let router = self.router.clone();
                let validator = self.validator.clone();
                let workspace = self.workspace.clone();
                let permit = semaphore.clone().acquire_owned().await
                    .map_err(|e| RoutingError::Other(e.to_string()))?;
                
                join_set.spawn(async move {
                    let result = Self::execute_task(
                        task,
                        router,
                        validator,
                        workspace,
                    ).await;
                    drop(permit);
                    result
                });
            }
            
            if let Some(result) = join_set.join_next().await {
                let task_result = result
                    .map_err(|e| RoutingError::ExecutionFailed(e.to_string()))??;
                running.remove(&task_result.task_id);
                completed.insert(task_result.task_id);
                results.push(task_result);
            }
        }
        
        Ok(results)
    }
    
    async fn execute_task(
        task: Task,
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        workspace: Arc<WorkspaceState>,
    ) -> Result<TaskResult> {
        let start = std::time::Instant::now();
        
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
        
        let mut context = merlin_core::Context::new(&system_prompt);
        
        // Add files from workspace if specified
        for file_path in &task.context_needs.required_files {
            if let Some(content) = workspace.read_file(file_path).await {
                context = context.with_files(vec![merlin_core::FileContext::new(
                    file_path.clone(),
                    content,
                )]);
            }
        }
        
        // Create query
        let query = merlin_core::Query::new(task.description.clone());
        
        // Execute with provider
        let response = provider.generate(&query, &context).await
            .map_err(|e| RoutingError::Other(e.to_string()))?;
        
        let validation = validator.validate(&response, &task).await?;
        
        if !validation.passed {
            return Err(RoutingError::ValidationFailed(validation));
        }
        
        Ok(TaskResult {
            task_id: task.id,
            response,
            tier_used: routing_decision.tier.to_string(),
            validation,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Task, ValidationResult};
    use async_trait::async_trait;
    use std::path::PathBuf;

    struct MockRouter;
    
    #[async_trait]
    impl ModelRouter for MockRouter {
        async fn route(&self, _task: &Task) -> Result<crate::RoutingDecision> {
            Ok(crate::RoutingDecision {
                tier: crate::ModelTier::Local {
                    model_name: "test".to_string(),
                },
                estimated_cost: 0.0,
                estimated_latency_ms: 0,
                reasoning: "test".to_string(),
            })
        }
        
        async fn is_available(&self, _tier: &crate::ModelTier) -> bool {
            true
        }
    }
    
    struct MockValidator;
    
    #[async_trait]
    impl Validator for MockValidator {
        async fn validate(
            &self,
            _response: &merlin_core::Response,
            _task: &Task,
        ) -> Result<ValidationResult> {
            Ok(ValidationResult::default())
        }
        
        async fn quick_validate(&self, _response: &merlin_core::Response) -> Result<bool> {
            Ok(true)
        }
    }
    
    #[tokio::test]
    #[ignore] // Requires actual Ollama instance
    async fn test_executor_pool_basic() {
        let router = Arc::new(MockRouter);
        let validator = Arc::new(MockValidator);
        let workspace = WorkspaceState::new(PathBuf::from("/tmp"));
        
        let executor = ExecutorPool::new(router, validator, 2, workspace);
        
        let task_a = Task::new("Task A".to_string());
        let task_b = Task::new("Task B".to_string());
        
        let graph = TaskGraph::from_tasks(vec![task_a, task_b]);
        let results = executor.execute_graph(graph).await.unwrap();
        
        assert_eq!(results.len(), 2);
    }
}

