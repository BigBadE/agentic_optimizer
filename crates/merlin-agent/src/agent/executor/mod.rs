//! Agent executor with streaming task execution and tool calling

mod context;
mod logging;
mod parallel;
mod step_executor;
pub(crate) mod typescript;

#[cfg(test)]
mod tests;

use context::{ContextBuilder, ConversationHistory};
use logging::ContextLogger;
pub use step_executor::{
    AgentExecutionParams, StepExecutionParams, StepExecutor, StepResult, TaskListExecutionParams,
};

use std::{
    collections::HashMap,
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
use merlin_core::ModelProvider;
use merlin_core::{
    Context, Response, Result, RoutingConfig, RoutingError, StepType, Task, TaskId, TaskResult,
    TaskStep, TokenUsage, ValidationResult,
    ui::{UiChannel, UiEvent},
};
use merlin_routing::{ModelRouter, ProviderRegistry, RoutingDecision};
use merlin_tooling::{PersistentTypeScriptRuntime, ToolRegistry, generate_typescript_signatures};

/// Parameters for executing with step executor
struct ExecutorParams<'exec> {
    /// Task to execute
    task: &'exec Task,
    /// Context for execution
    context: &'exec Context,
    /// Provider for execution
    provider: &'exec Arc<dyn ModelProvider>,
    /// Task ID
    task_id: TaskId,
    /// UI channel for events
    ui_channel: &'exec UiChannel,
}

/// Parameters for processing agent response
struct ResponseProcessingParams<'resp> {
    /// Agent response to process
    agent_response: AgentResponse,
    /// Task ID
    task_id: TaskId,
    /// Task being executed
    task: &'resp Task,
    /// Routing decision
    decision: &'resp RoutingDecision,
    /// Context used
    context: &'resp Context,
    /// Provider used
    provider: &'resp Arc<dyn ModelProvider>,
    /// Duration in milliseconds
    duration_ms: u64,
    /// UI channel for events
    ui_channel: &'resp UiChannel,
}

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
    /// Persistent TypeScript runtime for agent code execution
    runtime: Arc<PersistentTypeScriptRuntime>,
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

        // Create persistent runtime with tools
        let tools = tool_registry.list_tools();
        let mut tools_map = HashMap::new();
        for tool in tools {
            if let Some(tool_arc) = tool_registry.get_tool(tool.name()) {
                tools_map.insert(tool.name().to_owned(), tool_arc);
            }
        }
        let runtime = Arc::new(PersistentTypeScriptRuntime::new(tools_map));

        Ok(Self {
            router,
            validator,
            tool_registry,
            context_builder,
            context_dump_enabled: Arc::new(AtomicBool::new(false)),
            provider_registry,
            runtime,
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

        // Create persistent runtime with tools
        let tools = params.tool_registry.list_tools();
        let mut tools_map = HashMap::new();
        for tool in tools {
            if let Some(tool_arc) = params.tool_registry.get_tool(tool.name()) {
                tools_map.insert(tool.name().to_owned(), tool_arc);
            }
        }
        let runtime = Arc::new(PersistentTypeScriptRuntime::new(tools_map));

        Ok(Self {
            router: params.router,
            validator: params.validator,
            tool_registry: params.tool_registry,
            context_builder,
            context_dump_enabled: Arc::new(AtomicBool::new(false)),
            provider_registry: params.provider_registry,
            runtime,
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
        let agent_response = self
            .execute_with_step_executor(ExecutorParams {
                task: &task,
                context: &context,
                provider: &provider,
                task_id,
                ui_channel: &ui_channel,
            })
            .await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Handle response type
        self.process_agent_response(ResponseProcessingParams {
            agent_response,
            task_id,
            task: &task,
            decision: &decision,
            context: &context,
            provider: &provider,
            duration_ms,
            ui_channel: &ui_channel,
        })
        .await
    }

    /// Execute agent with step executor
    ///
    /// # Errors
    /// Returns an error if agent execution fails
    async fn execute_with_step_executor(
        &self,
        params: ExecutorParams<'_>,
    ) -> Result<AgentResponse> {
        let temp_step = TaskStep {
            title: params.task.description.clone(),
            description: params.task.description.clone(),
            step_type: StepType::Implementation,
            exit_requirement: None, // No validation
            context: None,
            dependencies: Vec::new(),
        };

        StepExecutor::execute_with_agent(AgentExecutionParams {
            step: &temp_step,
            context: params.context,
            provider: params.provider,
            tool_registry: &self.tool_registry,
            runtime: &self.runtime,
            task_id: params.task_id,
            ui_channel: params.ui_channel,
        })
        .await
    }

    /// Process agent response and create task result
    ///
    /// # Errors
    /// Returns an error if validation or task list execution fails
    async fn process_agent_response(
        &self,
        params: ResponseProcessingParams<'_>,
    ) -> Result<TaskResult> {
        match params.agent_response {
            AgentResponse::DirectResult(result) => {
                merlin_deps::tracing::debug!("Agent returned DirectResult");
                let response = Response {
                    text: result,
                    confidence: 1.0,
                    tokens_used: TokenUsage::default(),
                    provider: params.decision.model.to_string(),
                    latency_ms: params.duration_ms,
                };

                let validation = self.validate_response(&response, params.task).await?;

                Ok(TaskResult {
                    task_id: params.task_id,
                    response,
                    tier_used: params.decision.model.to_string(),
                    tokens_used: TokenUsage::default(),
                    validation,
                    duration_ms: params.duration_ms,
                    work_unit: None,
                })
            }
            AgentResponse::TaskList(task_list) => {
                merlin_deps::tracing::debug!(
                    "Agent returned TaskList with {} steps",
                    task_list.steps.len()
                );
                let step_result = StepExecutor::execute_task_list(TaskListExecutionParams {
                    task_list: &task_list,
                    base_context: params.context,
                    provider: params.provider,
                    tool_registry: &self.tool_registry,
                    runtime: &self.runtime,
                    task_id: params.task_id,
                    ui_channel: params.ui_channel,
                    recursion_depth: 0,
                })
                .await?;

                let response = Response {
                    text: step_result.text,
                    confidence: 1.0,
                    tokens_used: TokenUsage::default(),
                    provider: params.decision.model.to_string(),
                    latency_ms: step_result.duration_ms,
                };

                let validation = self.validate_response(&response, params.task).await?;

                Ok(TaskResult {
                    task_id: params.task_id,
                    response,
                    tier_used: params.decision.model.to_string(),
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
