//! Agent executor with streaming task execution and tool calling

mod context;
mod logging;
mod parallel;
mod response_processing;
mod step_executor;
pub(crate) mod typescript;

#[cfg(test)]
mod tests;

use context::{ContextBuilder, ConversationHistory};
use logging::ContextLogger;
use response_processing::{ResponseProcessingParams, ResponseProcessor};
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
use tokio::sync::RwLock;

use crate::Validator;
use merlin_context::ContextFetcher;
use merlin_core::AgentResponse;
use merlin_core::ModelProvider;
use merlin_core::{
    Context, Result, RoutingConfig, RoutingError, StepType, Task, TaskId, TaskResult, TaskStep,
    ui::{UiChannel, UiEvent},
};
use merlin_routing::{ModelRouter, ProviderRegistry};
use merlin_tooling::{PersistentTypeScriptRuntime, ToolRegistry, generate_typescript_signatures};
use tracing::{Level, span};
use tracing_futures::Instrument as _;

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
/// Parameters for creating an `AgentExecutor` with a custom provider registry
pub struct AgentExecutorParams {
    /// Model router
    pub router: Arc<dyn ModelRouter>,
    /// Validator
    pub validator: Arc<dyn Validator>,
    /// Tool registry
    pub tool_registry: ToolRegistry,
    /// Context fetcher
    pub context_fetcher: ContextFetcher,
    /// Routing configuration
    pub config: RoutingConfig,
    /// Provider registry
    pub provider_registry: ProviderRegistry,
}

/// Agent executor that streams task execution with tool calling
pub struct AgentExecutor {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    tool_registry: ToolRegistry,
    context_builder: ContextBuilder,
    context_dump_enabled: AtomicBool,
    /// Provider registry for accessing model providers
    provider_registry: ProviderRegistry,
    /// Persistent TypeScript runtime for agent code execution
    runtime: PersistentTypeScriptRuntime,
    /// Cached compiled TypeScript agent prompt (computed once at initialization, includes tool signatures)
    compiled_typescript_prompt: String,
}

impl AgentExecutor {
    /// Create a new agent executor
    ///
    /// # Errors
    /// Returns an error if provider registry initialization fails.
    pub fn new(
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        tool_registry: ToolRegistry,
        context_fetcher: ContextFetcher,
        config: &RoutingConfig,
    ) -> Result<Self> {
        let provider_registry = ProviderRegistry::new(config.clone())?;
        let context_fetcher_arc = Arc::new(context_fetcher);
        let conversation_history = Arc::new(RwLock::new(Vec::new()));
        let context_builder = ContextBuilder::new(context_fetcher_arc, conversation_history);

        // Create persistent runtime with tools
        let tools = tool_registry.list_tools();
        let mut tools_map = HashMap::new();
        for tool in &tools {
            if let Some(tool_arc) = tool_registry.get_tool(tool.name()) {
                tools_map.insert(tool.name().to_owned(), tool_arc);
            }
        }
        let runtime = PersistentTypeScriptRuntime::new(&tools_map).map_err(|err| {
            RoutingError::Other(format!("Failed to create TypeScript runtime: {err}"))
        })?;

        // Generate and cache TypeScript signatures once
        let signatures = generate_typescript_signatures(&tools).map_err(|err| {
            RoutingError::Other(format!("Failed to generate TypeScript signatures: {err}"))
        })?;

        // Compile TypeScript agent prompt once (load template + inject signatures)
        let compiled_prompt = Self::compile_typescript_prompt(&signatures)?;

        Ok(Self {
            router,
            validator,
            tool_registry,
            context_builder,
            context_dump_enabled: AtomicBool::new(false),
            provider_registry,
            runtime,
            compiled_typescript_prompt: compiled_prompt,
        })
    }

    /// Create a new agent executor with a custom provider registry (for testing).
    ///
    /// # Errors
    /// Returns an error if initialization fails.
    pub fn with_provider_registry(params: AgentExecutorParams) -> Result<Self> {
        let context_fetcher_arc = Arc::new(params.context_fetcher);
        let conversation_history = Arc::new(RwLock::new(Vec::new()));
        let context_builder = ContextBuilder::new(context_fetcher_arc, conversation_history);

        // Create persistent runtime with tools
        let tools = params.tool_registry.list_tools();
        let mut tools_map = HashMap::new();
        for tool in &tools {
            if let Some(tool_arc) = params.tool_registry.get_tool(tool.name()) {
                tools_map.insert(tool.name().to_owned(), tool_arc);
            }
        }
        let runtime = PersistentTypeScriptRuntime::new(&tools_map).map_err(|err| {
            RoutingError::Other(format!("Failed to create TypeScript runtime: {err}"))
        })?;

        // Generate and cache TypeScript signatures once
        let signatures = generate_typescript_signatures(&tools).map_err(|err| {
            RoutingError::Other(format!("Failed to generate TypeScript signatures: {err}"))
        })?;

        // Compile TypeScript agent prompt once (load template + inject signatures)
        let compiled_prompt = Self::compile_typescript_prompt(&signatures)?;

        Ok(Self {
            router: params.router,
            validator: params.validator,
            tool_registry: params.tool_registry,
            context_builder,
            context_dump_enabled: AtomicBool::new(false),
            provider_registry: params.provider_registry,
            runtime,
            compiled_typescript_prompt: compiled_prompt,
        })
    }

    /// Compile TypeScript agent prompt with tool signatures
    ///
    /// # Errors
    /// Returns an error if prompt loading or compilation fails
    fn compile_typescript_prompt(tool_signatures: &str) -> Result<String> {
        use merlin_core::prompts::load_prompt;

        const TOOL_SIGNATURES_PLACEHOLDER: &str = "{tool_signatures}";

        // Load TypeScript agent prompt template
        let prompt_template = load_prompt("typescript_agent").map_err(|err| {
            RoutingError::Other(format!("Failed to load typescript_agent prompt: {err}"))
        })?;

        // Replace placeholder with actual signatures
        Ok(prompt_template.replace(TOOL_SIGNATURES_PLACEHOLDER, tool_signatures))
    }

    /// Enable context dumping to debug.log
    pub fn enable_context_dump(&mut self) {
        self.context_dump_enabled.store(true, Ordering::Relaxed);
    }
    /// Disable context dumping to debug.log
    pub fn disable_context_dump(&mut self) {
        self.context_dump_enabled.store(false, Ordering::Relaxed);
    }

    /// Set conversation history for context building
    pub async fn set_conversation_history(&mut self, history: ConversationHistory) {
        let mut conv_history = self.context_builder.conversation_history.write().await;
        *conv_history = history;
    }
    /// Add to conversation history for context building
    pub async fn add_to_conversation(&mut self, role: String, content: String) {
        let mut conv_history = self.context_builder.conversation_history.write().await;
        conv_history.push((role, content));
    }

    /// Execute a task using the task list execution model
    ///
    /// # Errors
    /// Returns an error if routing, provider creation, execution, or validation fails
    pub async fn execute_task(&mut self, task: Task, ui_channel: UiChannel) -> Result<TaskResult> {
        let span = span!(Level::INFO, "execute_task", task_id = ?task.id);

        async move {
            let start = Instant::now();
            let task_id = task.id;

            // Route and get provider
            let decision = self.router.route(&task).await?;
            let provider = self
                .provider_registry
                .get_provider_for_task(task.difficulty, decision.model)?;

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
            let mut processor =
                ResponseProcessor::new(&self.validator, &self.tool_registry, &mut self.runtime);
            processor
                .process_response(ResponseProcessingParams {
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
        .instrument(span)
        .await
    }

    /// Execute agent with step executor
    ///
    /// # Errors
    /// Returns an error if agent execution fails
    async fn execute_with_step_executor(
        &mut self,
        params: ExecutorParams<'_>,
    ) -> Result<AgentResponse> {
        let span = span!(Level::INFO, "execute_with_step_executor", task_id = ?params.task_id);

        async move {
            let temp_step = TaskStep {
                title: params.task.description.clone(),
                description: params.task.description.clone(),
                step_type: StepType::Implementation,
                exit_requirement: None, // No validation
                context: None,
                dependencies: Vec::new(),
            };

            let response = StepExecutor::execute_with_agent(AgentExecutionParams {
                step: &temp_step,
                context: params.context,
                provider: params.provider,
                tool_registry: &self.tool_registry,
                runtime: &mut self.runtime,
                task_id: params.task_id,
                ui_channel: params.ui_channel,
                retry_attempt: 0,
                previous_result: None,
            })
            .await?;

            Ok(response)
        }
        .instrument(span)
        .await
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
        let span = span!(Level::INFO, "build_context_and_log", task_id = ?task_id);

        async move {
            ui_channel.send(UiEvent::TaskStepStarted {
                task_id,
                step_id: "context_analysis".to_owned(),
                step_type: "thinking".to_owned(),
                content: "Analyzing query intent".to_owned(),
            });

            // Use cached compiled prompt (already has signatures injected)
            let context = self
                .context_builder
                .build_context_for_typescript(task, ui_channel, &self.compiled_typescript_prompt)
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
        .instrument(span)
        .await
    }
}
