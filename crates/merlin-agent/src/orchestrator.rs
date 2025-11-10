use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{AgentExecutor, ContextFetcher, ThreadStore, ValidationPipeline, Validator};
use merlin_core::{
    Result, RoutingConfig, RoutingError, Task, TaskResult, ThreadId, TokenUsage, UiChannel,
    ValidationResult,
};
use merlin_routing::{
    CacheStats, DailyReport, MetricsCollector, MetricsReport, ModelRouter, ProviderRegistry,
    RequestMetrics, RequestMetricsParams, ResponseCache, StrategyRouter,
};
use merlin_tooling::{
    BashTool, ContextRequestTool, DeleteFileTool, EditFileTool, ListFilesTool, ReadFileTool,
    ToolRegistry, WriteFileTool,
};

/// Type alias for conversation history (role, content) tuples
type ConversationHistory = Vec<(String, String)>;

/// Parameters for task execution (internal)
#[derive(Clone)]
struct TaskExecutionParams {
    task: Task,
    ui_channel: UiChannel,
    conversation_history: ConversationHistory,
}

/// High-level orchestrator that coordinates all routing components
pub struct RoutingOrchestrator {
    config: RoutingConfig,
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    workspace_root: PathBuf,
    /// Provider registry (for testing, allows injecting mock providers)
    provider_registry: Option<ProviderRegistry>,
    /// Thread storage for conversation management
    thread_store: Option<Arc<Mutex<ThreadStore>>>,
    /// Whether to enable embedding/vector search initialization
    enable_embeddings: bool,
    /// Response cache for reducing API costs and latency
    cache: Arc<Mutex<ResponseCache>>,
    /// Metrics collector for tracking task execution statistics
    metrics: Arc<Mutex<MetricsCollector>>,
}

impl RoutingOrchestrator {
    /// Creates a new routing orchestrator with the given configuration.
    ///
    /// # Errors
    /// Returns error if provider registry initialization fails.
    pub fn new(config: RoutingConfig) -> Result<Self> {
        // Create provider registry and router
        let provider_registry = ProviderRegistry::new(config.clone())?;
        let router = Arc::new(StrategyRouter::new(provider_registry));

        // Validation with default stages, early exit disabled
        let validator = Arc::new(ValidationPipeline::with_default_stages());

        Ok(Self {
            config,
            router,
            validator,
            workspace_root: PathBuf::from("."),
            provider_registry: None,
            enable_embeddings: true,
            thread_store: None,
            cache: Arc::new(Mutex::new(ResponseCache::new())),
            metrics: Arc::new(Mutex::new(MetricsCollector::new())),
        })
    }

    /// Creates a new orchestrator for testing with a custom router.
    ///
    /// # Errors
    /// Returns error if configuration is invalid.
    pub fn new_with_router(
        config: RoutingConfig,
        router: Arc<dyn ModelRouter>,
        provider_registry: ProviderRegistry,
    ) -> Result<Self> {
        // Validation with default stages, early exit disabled
        let validator = Arc::new(ValidationPipeline::with_default_stages());

        Ok(Self {
            config,
            router,
            validator,
            workspace_root: PathBuf::from("."),
            provider_registry: Some(provider_registry),
            thread_store: None,
            enable_embeddings: true,
            cache: Arc::new(Mutex::new(ResponseCache::new())),
            metrics: Arc::new(Mutex::new(MetricsCollector::new())),
        })
    }

    /// Attaches thread storage for conversation management.
    #[must_use]
    pub fn with_thread_store(mut self, thread_store: Arc<Mutex<ThreadStore>>) -> Self {
        self.thread_store = Some(thread_store);
        self
    }

    /// Sets the workspace directory for file operations.
    #[must_use]
    pub fn with_workspace(mut self, workspace_path: PathBuf) -> Self {
        self.workspace_root = workspace_path;
        self
    }

    /// Sets whether to enable embedding/vector search initialization.
    #[must_use]
    pub fn with_embeddings(mut self, enable: bool) -> Self {
        self.enable_embeddings = enable;
        self
    }

    /// Gets the thread store if available
    pub fn thread_store(&self) -> Option<Arc<Mutex<ThreadStore>>> {
        self.thread_store.clone()
    }

    /// Execute a task with streaming and tool support
    ///
    /// # Errors
    /// Returns error if task execution or validation fails.
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
        conversation_history: ConversationHistory,
    ) -> Result<TaskResult> {
        self.execute_task_with_escalation(TaskExecutionParams {
            task,
            ui_channel,
            conversation_history,
        })
        .await
    }

    /// Execute a task within a thread context, automatically extracting conversation history.
    ///
    /// # Errors
    /// Returns an error if thread not found, or if task execution or validation fails
    pub async fn execute_task_in_thread(
        &self,
        task: Task,
        ui_channel: UiChannel,
        thread_id: ThreadId,
    ) -> Result<TaskResult> {
        let conversation_history = self.extract_thread_history(thread_id)?;

        self.execute_task_streaming_with_history(task, ui_channel, conversation_history)
            .await
    }

    /// Extracts conversation history from a thread
    ///
    /// # Errors
    /// Returns an error if thread store is not available or thread not found
    fn extract_thread_history(&self, thread_id: ThreadId) -> Result<ConversationHistory> {
        let thread_store = self
            .thread_store
            .as_ref()
            .ok_or_else(|| RoutingError::Other("Thread store not initialized".to_string()))?;

        let store = thread_store
            .lock()
            .map_err(|_| RoutingError::Other("Failed to lock thread store".to_string()))?;

        let thread = store
            .get_thread(thread_id)
            .ok_or_else(|| RoutingError::Other(format!("Thread {thread_id} not found")))?
            .clone();

        // Drop the lock before processing
        drop(store);

        let mut history = Vec::new();

        for message in &thread.messages {
            // Add user message
            history.push(("user".to_string(), message.content.clone()));

            // Add assistant response from work unit if available
            if let Some(ref work) = message.work {
                // For now, we don't have the actual response text stored in WorkUnit
                // This will be enhanced when we integrate with the execution flow
                // Placeholder: use subtask descriptions as response
                let response_text = work
                    .subtasks
                    .iter()
                    .map(|subtask| subtask.description.clone())
                    .collect::<Vec<_>>()
                    .join("\n");

                if !response_text.is_empty() {
                    history.push(("assistant".to_string(), response_text));
                }
            }
        }

        Ok(history)
    }

    /// Records metrics for a task execution attempt
    fn record_metrics(&self, params: RequestMetricsParams) {
        if let Ok(mut metrics_guard) = self.metrics.lock() {
            metrics_guard.record(RequestMetrics::new(params));
        }
    }

    /// Execute a task with automatic tier escalation on hard errors (internal method)
    ///
    /// Retries up to 3 times total, escalating difficulty by 2 points on each hard error.
    ///
    /// # Errors
    /// Returns an error if task execution fails after all retry attempts
    async fn execute_task_with_escalation(
        &self,
        mut params: TaskExecutionParams,
    ) -> Result<TaskResult> {
        const MAX_ESCALATION_ATTEMPTS: usize = 3;
        const DIFFICULTY_INCREASE: u8 = 2;
        const MAX_DIFFICULTY: u8 = 10;

        let original_difficulty = params.task.difficulty;
        let mut current_difficulty = original_difficulty;

        for attempt in 0..MAX_ESCALATION_ATTEMPTS {
            params.task.difficulty = current_difficulty;

            if attempt > 0 {
                tracing::warn!(
                    "Escalating difficulty {original_difficulty}->{current_difficulty} (attempt {}/{})",
                    attempt + 1,
                    MAX_ESCALATION_ATTEMPTS
                );
            }

            let start_time = Instant::now();
            match self.execute_task_streaming_once(params.clone()).await {
                Ok(result) => {
                    let latency_ms = start_time
                        .elapsed()
                        .as_millis()
                        .try_into()
                        .unwrap_or(u64::MAX);
                    self.record_metrics(RequestMetricsParams {
                        query: params.task.description.clone(),
                        tier_used: result.tier_used.clone(),
                        latency_ms,
                        tokens_used: result.tokens_used.clone(),
                        success: true,
                        escalated: attempt > 0,
                    });

                    if attempt > 0 {
                        tracing::info!(
                            "Task succeeded after tier escalation (difficulty: {}, attempt: {})",
                            current_difficulty,
                            attempt + 1
                        );
                    }
                    return Ok(result);
                }
                Err(err) => {
                    let latency_ms = start_time
                        .elapsed()
                        .as_millis()
                        .try_into()
                        .unwrap_or(u64::MAX);
                    self.record_metrics(RequestMetricsParams {
                        query: params.task.description.clone(),
                        tier_used: format!("Difficulty-{current_difficulty}"),
                        latency_ms,
                        tokens_used: TokenUsage::default(),
                        success: false,
                        escalated: attempt > 0,
                    });

                    if attempt + 1 >= MAX_ESCALATION_ATTEMPTS {
                        tracing::error!(
                            "Task failed after {} escalation attempts (final difficulty: {})",
                            MAX_ESCALATION_ATTEMPTS,
                            current_difficulty
                        );
                        return Err(err);
                    }

                    tracing::info!("Task execution failed: {}. Will escalate and retry.", err);
                    current_difficulty =
                        (current_difficulty + DIFFICULTY_INCREASE).min(MAX_DIFFICULTY);
                }
            }
        }

        Err(RoutingError::Other(format!(
            "Task failed after {MAX_ESCALATION_ATTEMPTS} attempts"
        )))
    }

    /// Execute a task once without retry logic (internal method)
    ///
    /// # Errors
    /// Returns error if task execution or validation fails.
    async fn execute_task_streaming_once(&self, params: TaskExecutionParams) -> Result<TaskResult> {
        // Check cache before executing
        let cache_key = format!(
            "{}:difficulty:{}",
            params.task.description, params.task.difficulty
        );

        if let Ok(cache_guard) = self.cache.lock()
            && let Some(cached_response) = cache_guard.get(&cache_key)
        {
            tracing::info!(
                "Cache hit for task: {} (difficulty: {})",
                params.task.description,
                params.task.difficulty
            );
            return Ok(TaskResult {
                task_id: params.task.id,
                response: cached_response,
                tier_used: format!("cached-difficulty-{}", params.task.difficulty),
                tokens_used: TokenUsage::default(),
                validation: ValidationResult::default(),
                duration_ms: 0,
                work_unit: None,
            });
        }

        let mut executor = self.create_agent_executor()?;
        self.setup_conversation_history(&mut executor, params.conversation_history)
            .await;

        // Use self-determining execution which includes assessment step
        // For simple tasks, this will skip assessment and execute directly
        let result = executor
            .execute_task(params.task.clone(), params.ui_channel.clone())
            .await?;

        // Cache successful result
        if let Ok(mut cache_guard) = self.cache.lock() {
            cache_guard.put(cache_key, result.response.clone());
            tracing::info!(
                "Cached response for task: {} (difficulty: {})",
                params.task.description,
                params.task.difficulty
            );
        }

        Ok(result)
    }

    /// Creates an agent executor with tool registry and context fetcher.
    ///
    /// # Errors
    /// Returns error if executor creation fails.
    fn create_agent_executor(&self) -> Result<AgentExecutor> {
        let tool_registry = ToolRegistry::with_workspace(self.workspace_root.clone())
            .with_tool(Arc::new(BashTool))
            .with_tool(Arc::new(ReadFileTool::new(self.workspace_root.clone())))
            .with_tool(Arc::new(WriteFileTool::new(self.workspace_root.clone())))
            .with_tool(Arc::new(EditFileTool::new(self.workspace_root.clone())))
            .with_tool(Arc::new(DeleteFileTool::new(self.workspace_root.clone())))
            .with_tool(Arc::new(ListFilesTool::new(self.workspace_root.clone())))
            .with_tool(Arc::new(ContextRequestTool::new(
                self.workspace_root.clone(),
            )));
        let context_fetcher = ContextFetcher::new_with_embeddings(
            self.workspace_root.clone(),
            self.enable_embeddings,
        );

        let executor = if let Some(ref registry) = self.provider_registry {
            // Use injected provider registry (for testing)
            use crate::agent::executor::AgentExecutorParams;
            AgentExecutor::with_provider_registry(AgentExecutorParams {
                router: Arc::clone(&self.router),
                validator: Arc::clone(&self.validator),
                tool_registry,
                context_fetcher,
                config: self.config.clone(),
                provider_registry: registry.clone(),
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

        // Context dump is disabled by default
        Ok(executor)
    }

    /// Sets up conversation history on the executor
    async fn setup_conversation_history(
        &self,
        executor: &mut AgentExecutor,
        conversation_history: ConversationHistory,
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

    /// Gets the routing configuration.
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }

    /// Gets the workspace root path.
    #[must_use]
    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    /// Gets cache statistics (entries, size).
    ///
    /// # Errors
    /// Returns error if cache lock is poisoned.
    pub fn cache_stats(&self) -> Result<CacheStats> {
        self.cache
            .lock()
            .map(|cache| cache.stats())
            .map_err(|_| RoutingError::Other("Failed to lock cache".to_string()))
    }

    /// Gets daily metrics report (success rate, latency, cost, tier distribution).
    ///
    /// # Errors
    /// Returns error if metrics lock is poisoned.
    pub fn metrics_report(&self) -> Result<DailyReport> {
        self.metrics
            .lock()
            .map(|metrics| MetricsReport::daily(&metrics))
            .map_err(|_| RoutingError::Other("Failed to lock metrics".to_string()))
    }

    /// Clears the response cache.
    ///
    /// # Errors
    /// Returns error if cache lock is poisoned.
    pub fn clear_cache(&self) -> Result<()> {
        self.cache
            .lock()
            .map(|mut cache| cache.clear())
            .map_err(|_| RoutingError::Other("Failed to lock cache".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// # Panics
    /// Test function - panics indicate test failure
    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = RoutingConfig::default();
        if let Ok(orchestrator) = RoutingOrchestrator::new(config) {
            assert!(orchestrator.config.tiers.local_enabled);
        }
    }
}
