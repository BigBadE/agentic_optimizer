use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::{
    AgentExecutor, ContextFetcher, ThreadStore, ValidationPipeline, Validator, WorkspaceState,
};
use merlin_core::{Result, RoutingConfig, RoutingError, Task, TaskResult, ThreadId, UiChannel};
use merlin_routing::{ModelRouter, ProviderRegistry, StrategyRouter};
use merlin_tooling::{
    BashTool, ContextRequestTool, DeleteFileTool, EditFileTool, ListFilesTool, ReadFileTool,
    ToolRegistry, WriteFileTool,
};

/// Type alias for conversation history (role, content) tuples
type ConversationHistory = Vec<(String, String)>;

/// Parameters for task execution (internal)
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
    workspace: Arc<WorkspaceState>,
    /// Provider registry (for testing, allows injecting mock providers)
    provider_registry: Option<ProviderRegistry>,
    /// Thread storage for conversation management
    thread_store: Option<Arc<Mutex<ThreadStore>>>,
    /// Whether to enable embedding/vector search initialization
    enable_embeddings: bool,
}

impl RoutingOrchestrator {
    /// Creates a new routing orchestrator with the given configuration.
    ///
    /// Initializes analyzer, router, validator, and workspace with default implementations.
    ///
    /// # Errors
    /// Returns an error if provider registry initialization fails (e.g., missing API keys).
    pub fn new(config: RoutingConfig) -> Result<Self> {
        // Create provider registry and router
        let provider_registry = ProviderRegistry::new(config.clone())?;
        let router = Arc::new(StrategyRouter::new(provider_registry));

        // Validation with default stages, early exit disabled
        let validator = Arc::new(ValidationPipeline::with_default_stages());

        // Workspace uses current directory
        let workspace = WorkspaceState::new(PathBuf::from("."));

        Ok(Self {
            config,
            router,
            validator,
            workspace,
            provider_registry: None,
            enable_embeddings: true,
            thread_store: None,
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
        provider_registry: ProviderRegistry,
    ) -> Result<Self> {
        // Validation with default stages, early exit disabled
        let validator = Arc::new(ValidationPipeline::with_default_stages());

        // Workspace uses current directory
        let workspace = WorkspaceState::new(PathBuf::from("."));

        Ok(Self {
            config,
            router,
            validator,
            workspace,
            provider_registry: Some(provider_registry),
            thread_store: None,
            enable_embeddings: true,
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
        self.workspace = WorkspaceState::new(workspace_path);
        self
    }

    /// Sets whether to enable embedding/vector search initialization.
    ///
    /// When disabled, the context fetcher will skip expensive embedding operations.
    /// This is useful for test fixtures that don't need semantic search.
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
        conversation_history: ConversationHistory,
    ) -> Result<TaskResult> {
        self.execute_task_streaming_once(TaskExecutionParams {
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

    /// Execute a task once without retry logic (internal method)
    ///
    /// # Errors
    /// Returns an error if task execution or validation fails, or if a provider interaction returns an error
    async fn execute_task_streaming_once(&self, params: TaskExecutionParams) -> Result<TaskResult> {
        let mut executor = self.create_agent_executor()?;
        self.setup_conversation_history(&mut executor, params.conversation_history)
            .await;

        // Use self-determining execution which includes assessment step
        // For simple tasks, this will skip assessment and execute directly
        let result = executor
            .execute_task(params.task.clone(), params.ui_channel.clone())
            .await?;

        Ok(result)
    }

    /// Creates an agent executor with tool registry and context fetcher
    /// Create agent executor.
    ///
    /// # Errors
    /// Returns error if executor creation fails.
    fn create_agent_executor(&self) -> Result<AgentExecutor> {
        let workspace_root = self.workspace.root_path();
        let tool_registry = ToolRegistry::with_workspace(workspace_root.clone())
            .with_tool(Arc::new(BashTool))
            .with_tool(Arc::new(ReadFileTool::new(workspace_root)))
            .with_tool(Arc::new(WriteFileTool::new(workspace_root)))
            .with_tool(Arc::new(EditFileTool::new(workspace_root)))
            .with_tool(Arc::new(DeleteFileTool::new(workspace_root)))
            .with_tool(Arc::new(ListFilesTool::new(workspace_root)))
            .with_tool(Arc::new(ContextRequestTool::new(workspace_root.clone())));
        let context_fetcher =
            ContextFetcher::new_with_embeddings(workspace_root.clone(), self.enable_embeddings);

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
            merlin_deps::tracing::info!("No conversation history to set");
        } else {
            merlin_deps::tracing::info!(
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

    /// Gets a reference to the workspace state.
    pub fn workspace(&self) -> Arc<WorkspaceState> {
        Arc::clone(&self.workspace)
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
