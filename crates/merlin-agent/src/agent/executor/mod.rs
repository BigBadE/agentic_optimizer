//! Agent executor with streaming task execution and tool calling

mod context;
mod logging;
mod query_intent;
mod self_determining;
mod typescript;

#[cfg(test)]
mod tests;

use context::{ContextBuilder, ConversationHistory};
use logging::ContextLogger;
use query_intent::QueryIntent;
use self_determining::{
    BoxedTaskFuture, SelfDeterminingExecutor, StreamingParams, execute_with_streaming,
};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tokio::sync::Mutex;

use crate::Validator;
use merlin_context::ContextFetcher;
use merlin_core::{
    Context, ExecutionContext, ModelProvider, Query, Response, Result, RoutingConfig, RoutingError,
    Subtask, Task, TaskAction, TaskId, TaskResult, TaskState, TokenUsage, ValidationResult,
    ui::{TaskProgress, UiChannel, UiEvent},
};
use merlin_routing::{ModelRouter, ProviderRegistry, RoutingDecision};
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

/// Parameters for task execution
struct ExecutionParams<'life> {
    task_id: TaskId,
    #[allow(dead_code, reason = "May be used in future implementations")]
    task: &'life Task,
    provider: &'life Arc<dyn ModelProvider>,
    context: &'life Context,
    ui_channel: &'life UiChannel,
    decision: &'life RoutingDecision,
}

/// Parameters for task decomposition
struct DecompositionParams<'life> {
    task_id: TaskId,
    subtasks: Vec<Subtask>,
    ui_channel: &'life UiChannel,
    model: String,
    start: Instant,
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

    /// Execute a task with streaming updates
    ///
    /// # Errors
    ///
    /// Returns an error if routing, provider creation, execution, or validation fails
    pub async fn execute_streaming(
        &mut self,
        task: Task,
        ui_channel: UiChannel,
    ) -> Result<TaskResult> {
        let start = Instant::now();
        let task_id = task.id;

        // Route and get provider
        let decision = self.router.route(&task).await?;
        let provider = self.provider_registry.get_provider(decision.model)?;

        // Build context and log
        let context = self
            .build_context_and_log(&task, &ui_channel, task_id)
            .await?;

        // Execute with streaming
        let response = self
            .execute_task_with_provider(ExecutionParams {
                task_id,
                task: &task,
                provider: &provider,
                context: &context,
                ui_channel: &ui_channel,
                decision: &decision,
            })
            .await?;

        // Validate response
        let validation = self.validate_response(&response, &task).await?;

        let duration_ms = start.elapsed().as_millis() as u64;

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

    /// Execute task with provider and stream results
    ///
    /// # Errors
    /// Returns an error if task execution fails
    async fn execute_task_with_provider(&self, params: ExecutionParams<'_>) -> Result<Response> {
        params.ui_channel.send(UiEvent::TaskStepStarted {
            task_id: params.task_id,
            step_id: "model_execution".to_owned(),
            step_type: "tool_call".to_owned(),
            content: format!("Executing with {}", params.decision.model),
        });

        let query = Query::new(params.task.description.clone());
        let response = execute_with_streaming(StreamingParams {
            provider: params.provider,
            query: &query,
            context: params.context,
            tool_registry: &self.tool_registry,
            task_id: params.task_id,
            ui_channel: params.ui_channel,
        })
        .await?;

        params.ui_channel.send(UiEvent::TaskStepCompleted {
            task_id: params.task_id,
            step_id: "model_execution".to_owned(),
        });

        Ok(response)
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
                tracing::info!(
                    "Validation failed. Model response was:\n{}\n\nError: {:?}",
                    response.text,
                    validation_error
                );
                validation_error
            })
    }

    /// Handle task decomposition action
    ///
    /// # Errors
    /// Returns an error if subtask execution fails
    async fn handle_decomposition(
        &mut self,
        params: DecompositionParams<'_>,
    ) -> Result<TaskResult> {
        let subtask_results = self
            .execute_subtasks_sequentially(params.task_id, params.subtasks, params.ui_channel)
            .await?;

        let combined_response = subtask_results
            .iter()
            .map(|result| result.response.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let duration_ms = params.start.elapsed().as_millis() as u64;

        Ok(TaskResult {
            task_id: params.task_id,
            response: Response {
                text: combined_response,
                confidence: 1.0,
                tokens_used: TokenUsage::default(),
                provider: params.model,
                latency_ms: duration_ms,
            },
            tier_used: "decomposed".to_owned(),
            tokens_used: TokenUsage::default(),
            validation: ValidationResult::default(),
            duration_ms,
            work_unit: None,
        })
    }

    /// Handle context gathering action
    fn handle_context_gathering(
        task_id: TaskId,
        needs: &[String],
        ui_channel: &UiChannel,
        exec_context: &mut ExecutionContext,
    ) {
        ui_channel.send(UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage: "Gathering Context".to_owned(),
                current: 0,
                total: Some(needs.len() as u64),
                message: format!("Fetching: {}", needs.join(", ")),
            },
        });

        ui_channel.send(UiEvent::TaskOutput {
            task_id,
            output: format!("Gathering context: {}", needs.join(", ")),
        });

        SelfDeterminingExecutor::gather_context(exec_context, needs);
    }

    /// Execute subtasks sequentially with progress tracking
    ///
    /// # Errors
    /// Returns an error if any subtask execution fails
    async fn execute_subtasks_sequentially(
        &mut self,
        task_id: TaskId,
        subtasks: Vec<Subtask>,
        ui_channel: &UiChannel,
    ) -> Result<Vec<TaskResult>> {
        let mut subtask_results = Vec::new();
        let total_subtasks = subtasks.len();

        ui_channel.send(UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage: "Decomposing".to_owned(),
                current: 0,
                total: Some(subtasks.len() as u64),
                message: format!("Breaking into {} subtasks", subtasks.len()),
            },
        });

        for (index, subtask_spec) in subtasks.into_iter().enumerate() {
            let subtask = Task::new(subtask_spec.description.clone())
                .with_difficulty(subtask_spec.difficulty);

            ui_channel.send(UiEvent::TaskProgress {
                task_id,
                progress: TaskProgress {
                    stage: "Executing".to_owned(),
                    current: index as u64,
                    total: Some(total_subtasks as u64),
                    message: format!("Subtask {}/{}", index + 1, total_subtasks),
                },
            });

            let result = self
                .execute_self_determining(subtask, ui_channel.clone())
                .await?;
            subtask_results.push(result);
        }

        Ok(subtask_results)
    }

    /// Execute a task with self-determination (Phase 5.1)
    /// The task assesses itself and decides whether to complete, decompose, or gather context
    ///
    /// # Errors
    ///
    /// Returns an error if routing, provider creation, execution, or validation fails
    pub fn execute_self_determining(
        &mut self,
        mut task: Task,
        ui_channel: UiChannel,
    ) -> BoxedTaskFuture<'_> {
        Box::pin(async move {
            let task_id = task.id;
            let start = Instant::now();
            let mut exec_context = ExecutionContext::new(task.description.clone());

            // Check if this is a simple request that doesn't need assessment
            let is_simple = QueryIntent::is_simple(&task.description);

            if is_simple {
                // Skip assessment for simple requests, execute directly
                task.state = TaskState::Executing;
                return self.execute_streaming(task, ui_channel).await;
            }

            // Self-determination loop
            loop {
                // Update task state
                task.state = TaskState::Assessing;

                // Route and get provider
                let decision_result = self.router.route(&task).await?;
                let provider = self.provider_registry.get_provider(decision_result.model)?;

                // Assess the task
                let Ok(decision) = SelfDeterminingExecutor::assess_task_with_provider(
                    &provider,
                    &task,
                    &ui_channel,
                    task_id,
                )
                .await
                else {
                    // Fallback to streaming execution if assessment fails
                    task.state = TaskState::Executing;
                    return self.execute_streaming(task, ui_channel).await;
                };

                // Record decision
                task.decision_history.push(decision.clone());

                // Execute based on decision
                match decision.action {
                    TaskAction::Complete { result } => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        return Ok(SelfDeterminingExecutor::create_completion_result(
                            task_id,
                            result,
                            decision_result.model.to_string(),
                            duration_ms,
                        ));
                    }

                    TaskAction::Decompose { subtasks, .. } => {
                        return self
                            .handle_decomposition(DecompositionParams {
                                task_id,
                                subtasks,
                                ui_channel: &ui_channel,
                                model: decision_result.model.to_string(),
                                start,
                            })
                            .await;
                    }

                    TaskAction::GatherContext { needs } => {
                        Self::handle_context_gathering(
                            task_id,
                            &needs,
                            &ui_channel,
                            &mut exec_context,
                        );
                        // Continue loop to re-assess with new context
                    }
                }
            }
        })
    }
}
