//! Agent executor with streaming task execution and tool calling

mod context;
mod logging;
mod step_executor;
pub(crate) mod typescript;

#[cfg(test)]
mod tests;

use context::{ContextBuilder, ConversationHistory};
use logging::ContextLogger;
pub use step_executor::{StepExecutionParams, StepExecutor, StepResult};

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::sync::Mutex;

use crate::Validator;
use merlin_context::ContextFetcher;
use merlin_core::AgentResponse;
use merlin_core::{
    Context, Response, Result, RoutingConfig, RoutingError, Task, TaskId, TaskResult, TokenUsage,
    ValidationResult,
    ui::{UiChannel, UiEvent},
};
use merlin_routing::{ModelRouter, ProviderRegistry};
use merlin_tooling::{ToolRegistry, generate_typescript_signatures};

/// Parameters for creating an `AgentExecutor` with a custom provider registry
pub struct AgentExecutorParams {
    /// Model router
    pub router: Arc<dyn ModelRouter>,
    /// Validator
    pub validator: Arc<dyn Validator>,
    /// Tool registry
    pub tool_registry: Arc<ToolRegistry>,
    /// Context fetcher
    pub context_fetcher: ContextFetcher,
    /// Routing configuration
    pub config: RoutingConfig,
    /// Provider registry
    pub provider_registry: Arc<ProviderRegistry>,
}

/// Agent executor that streams task execution with tool calling
#[derive(Clone)]
pub struct AgentExecutor {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    tool_registry: Arc<ToolRegistry>,
    context_builder: ContextBuilder,
    context_dump_enabled: Arc<AtomicBool>,
    /// Provider registry for accessing model providers
    provider_registry: Arc<ProviderRegistry>,
}

impl AgentExecutor {
    /// Create a new agent executor
    ///
    /// # Errors
    /// Returns an error if provider registry initialization fails.
    pub fn new(
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        tool_registry: Arc<ToolRegistry>,
        context_fetcher: ContextFetcher,
        config: &RoutingConfig,
    ) -> Result<Self> {
        let provider_registry = Arc::new(ProviderRegistry::new(config.clone())?);
        let context_fetcher_arc = Arc::new(Mutex::new(context_fetcher));
        let conversation_history = Arc::new(Mutex::new(Vec::new()));
        let context_builder = ContextBuilder::new(context_fetcher_arc, conversation_history);

        Ok(Self {
            router,
            validator,
            tool_registry,
            context_builder,
            context_dump_enabled: Arc::new(AtomicBool::new(false)),
            provider_registry,
        })
    }

    /// Create a new agent executor with a custom provider registry (for testing).
    ///
    /// # Errors
    /// Returns an error if initialization fails.
    pub fn with_provider_registry(params: AgentExecutorParams) -> Result<Self> {
        let context_fetcher_arc = Arc::new(Mutex::new(params.context_fetcher));
        let conversation_history = Arc::new(Mutex::new(Vec::new()));
        let context_builder = ContextBuilder::new(context_fetcher_arc, conversation_history);

        Ok(Self {
            router: params.router,
            validator: params.validator,
            tool_registry: params.tool_registry,
            context_builder,
            context_dump_enabled: Arc::new(AtomicBool::new(false)),
            provider_registry: params.provider_registry,
        })
    }

    /// Enable context dumping to debug.log
    pub fn enable_context_dump(&mut self) {
        self.context_dump_enabled.store(true, Ordering::Relaxed);
    }

    /// Disable context dumping
    pub fn disable_context_dump(&mut self) {
        self.context_dump_enabled.store(false, Ordering::Relaxed);
    }

    /// Set conversation history for context building
    pub async fn set_conversation_history(&mut self, history: ConversationHistory) {
        let mut conv_history = self.context_builder.conversation_history.lock().await;
        *conv_history = history;
    }

    /// Add a message to conversation history
    pub async fn add_to_conversation(&mut self, role: String, content: String) {
        let mut conv_history = self.context_builder.conversation_history.lock().await;
        conv_history.push((role, content));
    }

    /// Execute a task using the task list execution model
    ///
    /// # Errors
    ///
    /// Returns an error if routing, provider creation, execution, or validation fails
    pub async fn execute_task(&mut self, task: Task, ui_channel: UiChannel) -> Result<TaskResult> {
        let start = Instant::now();
        let task_id = task.id;

        // Route and get provider
        let decision = self.router.route(&task).await?;
        let provider = self.provider_registry.get_provider(decision.model)?;

        // Build context with tool signatures
        let context = self
            .build_context_and_log(&task, &ui_channel, task_id)
            .await?;

        // Execute agent - returns String | TaskList
        let agent_response = StepExecutor::execute_with_agent(
            &task.description,
            &context,
            &provider,
            &self.tool_registry,
            task_id,
            &ui_channel,
        )
        .await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Handle response type
        match agent_response {
            AgentResponse::DirectResult(result) => {
                // Simple string response - create TaskResult
                let response = Response {
                    text: result,
                    confidence: 1.0,
                    tokens_used: TokenUsage::default(),
                    provider: decision.model.to_string(),
                    latency_ms: duration_ms,
                };

                let validation = self.validate_response(&response, &task).await?;

                Ok(TaskResult {
                    task_id,
                    response,
                    tier_used: decision.model.to_string(),
                    tokens_used: TokenUsage::default(),
                    validation,
                    duration_ms,
                    work_unit: None,
                })
            }
            AgentResponse::TaskList(task_list) => {
                // Execute task list with StepExecutor
                let step_result = StepExecutor::execute_task_list(
                    &task_list,
                    &context,
                    &provider,
                    &self.tool_registry,
                    task_id,
                    &ui_channel,
                    0, // recursion_depth
                )
                .await?;

                let response = Response {
                    text: step_result.text,
                    confidence: 1.0,
                    tokens_used: TokenUsage::default(),
                    provider: decision.model.to_string(),
                    latency_ms: step_result.duration_ms,
                };

                let validation = self.validate_response(&response, &task).await?;

                Ok(TaskResult {
                    task_id,
                    response,
                    tier_used: decision.model.to_string(),
                    tokens_used: TokenUsage::default(),
                    validation,
                    duration_ms: step_result.duration_ms,
                    work_unit: None,
                })
            }
        }
    }

    /// Build context and log
    ///
    /// # Errors
    /// Returns an error if context building fails
    async fn build_context_and_log(
        &self,
        task: &Task,
        ui_channel: &UiChannel,
        task_id: TaskId,
    ) -> Result<Context> {
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "context_analysis".to_owned(),
            step_type: "thinking".to_owned(),
            content: "Analyzing query intent".to_owned(),
        });

        // Generate TypeScript signatures for all tools
        let tools = self.tool_registry.list_tools();
        let signatures = generate_typescript_signatures(&tools).map_err(|err| {
            RoutingError::Other(format!("Failed to generate TypeScript signatures: {err}"))
        })?;

        let context = self
            .context_builder
            .build_context_for_typescript(task, ui_channel, &signatures)
            .await?;

        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "context_analysis".to_owned(),
        });

        ContextLogger::log_context_breakdown(&context, &self.context_builder).await;
        if self.context_dump_enabled.load(Ordering::Relaxed) {
            ContextLogger::dump_context_to_log(&context, task, &self.context_builder).await;
        }

        Ok(context)
    }

    /// Validate response and log failures
    ///
    /// # Errors
    /// Returns an error if validation fails
    async fn validate_response(
        &self,
        response: &Response,
        task: &Task,
    ) -> Result<ValidationResult> {
        self.validator
            .validate(response, task)
            .await
            .map_err(|validation_error| {
                merlin_deps::tracing::info!(
                    "Validation failed. Model response was:\n{}\n\nError: {:?}",
                    response.text,
                    validation_error
                );
                validation_error
            })
    }
}
