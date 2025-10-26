use serde_json::from_value;
use std::future::Future;
use std::mem::replace;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::Validator;
use crate::agent::AgentExecutionResult;
use merlin_core::{
    Context, ExecutionContext, ExecutionMode, ModelProvider, Query, Response, Result,
    RoutingConfig, RoutingError, Subtask, Task, TaskAction, TaskDecision, TaskId, TaskResult,
    TaskState, TokenUsage, ValidationResult,
    ui::{TaskProgress, UiChannel, UiEvent},
};
use merlin_routing::{ModelRouter, ProviderRegistry, RoutingDecision};
use merlin_tooling::{ToolRegistry, TypeScriptRuntime, generate_typescript_signatures};
use std::sync::atomic::{AtomicBool, Ordering};

use super::SelfAssessor;
use merlin_context::ContextFetcher;

/// Type alias for boxed future returning `TaskResult`
type BoxedTaskFuture<'future> = Pin<Box<dyn Future<Output = Result<TaskResult>> + Send + 'future>>;

/// Type alias for conversation history
type ConversationHistory = Vec<(String, String)>;

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
    context_fetcher: Arc<Mutex<ContextFetcher>>,
    conversation_history: Arc<Mutex<ConversationHistory>>,
    context_dump_enabled: Arc<AtomicBool>,
    /// Provider registry for accessing model providers
    provider_registry: Arc<ProviderRegistry>,
}

struct ExecInputs<'life> {
    provider: &'life Arc<dyn ModelProvider>,
    query: &'life Query,
    context: &'life Context,
    ui_channel: &'life UiChannel,
}

/// Parameters for task execution
struct ExecutionParams<'life> {
    task_id: TaskId,
    task: &'life Task,
    provider: &'life Arc<dyn ModelProvider>,
    context: &'life Context,
    ui_channel: &'life UiChannel,
    decision: &'life RoutingDecision,
}

/// Intent classification for queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryIntent {
    /// Conversational query - no file context needed
    Conversational,
    /// Code query - needs file context but no modification
    CodeQuery,
    /// Code modification - needs file context and write capability
    CodeModification,
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

        Ok(Self {
            router,
            validator,
            tool_registry,
            context_fetcher: Arc::new(Mutex::new(context_fetcher)),
            conversation_history: Arc::new(Mutex::new(Vec::new())),
            context_dump_enabled: Arc::new(AtomicBool::new(false)),
            provider_registry,
        })
    }

    /// Create a new agent executor with a custom provider registry (for testing).
    ///
    /// # Errors
    /// Returns an error if initialization fails.
    pub fn with_provider_registry(params: AgentExecutorParams) -> Result<Self> {
        Ok(Self {
            router: params.router,
            validator: params.validator,
            tool_registry: params.tool_registry,
            context_fetcher: Arc::new(Mutex::new(params.context_fetcher)),
            conversation_history: Arc::new(Mutex::new(Vec::new())),
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
        let mut conv_history = self.conversation_history.lock().await;
        *conv_history = history;
    }

    /// Add a message to conversation history
    pub async fn add_to_conversation(&mut self, role: String, content: String) {
        let mut conv_history = self.conversation_history.lock().await;
        conv_history.push((role, content));
    }

    fn complete_analysis_step(ui_channel: &UiChannel, task_id: TaskId) {
        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "analysis".to_owned(),
        });
    }

    /// Assess a task using the given provider and write assessment output to UI.
    ///
    /// # Errors
    /// Returns an error if the provider generation fails or the assessment cannot be parsed.
    async fn assess_task_with_provider(
        &self,
        provider: &Arc<dyn ModelProvider>,
        task: &Task,
        ui_channel: &UiChannel,
        task_id: TaskId,
    ) -> Result<TaskDecision> {
        // Start analysis step
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "analysis".to_owned(),
            step_type: "thinking".to_owned(),
            content: "Analyzing task complexity and determining execution strategy".to_owned(),
        });

        let assessor = SelfAssessor::new(Arc::clone(provider));
        let query = Query::new(format!(
            "Analyze this task and decide if you can complete it immediately or if it needs decomposition:\n\n\"{}\"",
            task.description
        ));
        let context = Context::new("You are a task assessment system.");
        let assessment_response = provider
            .generate(&query, &context)
            .await
            .map_err(|error| RoutingError::Other(format!("Assessment failed: {error}")))?;

        // Parse first; send to UI and complete step on success
        match assessor.parse_assessment_response(&assessment_response.text, task) {
            Ok(decision) => {
                ui_channel.send(UiEvent::TaskOutput {
                    task_id,
                    output: assessment_response.text,
                });
                Self::complete_analysis_step(ui_channel, task_id);
                Ok(decision)
            }
            Err(error) => {
                Self::complete_analysis_step(ui_channel, task_id);
                Err(error)
            }
        }
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

        let context = self.build_context_for_typescript(task, ui_channel).await?;

        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "context_analysis".to_owned(),
        });

        self.log_context_breakdown(&context).await;
        if self.context_dump_enabled.load(Ordering::Relaxed) {
            self.dump_context_to_log(&context, task).await;
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
        let response = self
            .execute_with_streaming(
                params.task_id,
                ExecInputs {
                    provider: params.provider,
                    query: &query,
                    context: params.context,
                    ui_channel: params.ui_channel,
                },
            )
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

    /// Execute with streaming and TypeScript code execution
    ///
    /// # Errors
    /// Returns an error if provider generation fails or code execution fails.
    async fn execute_with_streaming(
        &self,
        task_id: TaskId,
        inputs: ExecInputs<'_>,
    ) -> Result<Response> {
        let ExecInputs {
            provider,
            query,
            context,
            ui_channel,
        } = inputs;
        // Execute the query directly
        let response = provider
            .generate(query, context)
            .await
            .map_err(|err| RoutingError::Other(format!("Provider error: {err}")))?;

        // Log agent response to debug.log
        tracing::debug!("Agent response text: {}", response.text);

        // Extract TypeScript code from response
        let code = Self::extract_typescript_code(&response.text);

        if let Some(typescript_code) = code {
            tracing::debug!("Found TypeScript code, executing...");

            // Execute TypeScript code and get result
            let execution_result = self
                .execute_typescript_code(task_id, &typescript_code, ui_channel)
                .await?;

            // Handle result based on done status
            if let Some(result_text) = execution_result.get_result() {
                // Task is done - send final result to UI
                ui_channel.send(UiEvent::TaskOutput {
                    task_id,
                    output: result_text.to_owned(),
                });
            } else if let Some(next_task_desc) = execution_result.get_next_task() {
                // Task wants to continue - spawn new task
                tracing::info!("Task requested continuation: {}", next_task_desc);
                ui_channel.send(UiEvent::TaskOutput {
                    task_id,
                    output: format!("Continuing with: {next_task_desc}"),
                });
                // Note: Actual task spawning would happen in the orchestrator
                // For now, we just indicate the continuation request
            }
        } else {
            // No TypeScript code found - send response text as-is
            tracing::debug!("No TypeScript code found in response");
            ui_channel.send(UiEvent::TaskOutput {
                task_id,
                output: response.text.clone(),
            });
        }

        Ok(response)
    }

    /// Extract TypeScript code from markdown code blocks
    ///
    /// Looks for ```typescript or ```ts blocks and returns the concatenated code
    fn extract_typescript_code(text: &str) -> Option<String> {
        let mut code_blocks = Vec::new();
        let mut remaining = text;

        // Find all ```typescript or ```ts blocks
        while let Some(start_idx) = remaining
            .find("```typescript")
            .or_else(|| remaining.find("```ts"))
        {
            let is_typescript = remaining[start_idx..].starts_with("```typescript");
            let prefix_len = if is_typescript { 13 } else { 5 }; // "```typescript" or "```ts"

            let after_start = &remaining[start_idx + prefix_len..];

            // Find the closing ```
            if let Some(end_idx) = after_start.find("```") {
                let code = after_start[..end_idx].trim();
                if !code.is_empty() {
                    code_blocks.push(code.to_owned());
                }
                remaining = &after_start[end_idx + 3..];
            } else {
                break;
            }
        }

        if code_blocks.is_empty() {
            None
        } else {
            Some(code_blocks.join("\n\n"))
        }
    }

    /// Execute TypeScript code using the TypeScript runtime
    ///
    /// # Errors
    /// Returns an error if code execution fails or result parsing fails
    async fn execute_typescript_code(
        &self,
        task_id: TaskId,
        code: &str,
        ui_channel: &UiChannel,
    ) -> Result<AgentExecutionResult> {
        // Create TypeScript runtime and register tools
        let mut runtime = TypeScriptRuntime::new();

        // Get all tool names and then get Arc clones
        let tool_names: Vec<String> = self
            .tool_registry
            .list_tools()
            .iter()
            .map(|tool| tool.name().to_owned())
            .collect();

        for tool_name in tool_names {
            if let Some(tool) = self.tool_registry.get_tool(&tool_name) {
                runtime.register_tool(tool);
            }
        }

        // Send step started event
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "typescript_execution".to_owned(),
            step_type: "tool_call".to_owned(),
            content: "Executing TypeScript code".to_owned(),
        });

        // Execute code
        tracing::debug!("Executing TypeScript code:\n{}", code);
        let result_value = runtime.execute(code).await.map_err(|err| {
            tracing::info!(
                "TypeScript execution failed. Code was:\n{}\n\nError: {}",
                code,
                err
            );

            // Send step failed event
            ui_channel.send(UiEvent::TaskStepFailed {
                task_id,
                step_id: "typescript_execution".to_owned(),
                error: err.to_string(),
            });

            RoutingError::Other(format!("TypeScript execution failed: {err}"))
        })?;

        // Parse result as AgentExecutionResult
        // Handle both structured results and plain strings
        let execution_result: AgentExecutionResult = if result_value.is_string() {
            // Plain string result - treat as "done" with the string as the result
            let result_str = result_value.as_str().unwrap_or("").to_owned();
            AgentExecutionResult::done(result_str)
        } else {
            // Try to parse as structured AgentExecutionResult
            from_value(result_value.clone()).map_err(|err| {
                tracing::info!(
                    "Failed to parse execution result. Code was:\n{}\n\nReturned value: {:?}\n\nError: {}",
                    code,
                    result_value,
                    err
                );

                // Send step failed event
                ui_channel.send(UiEvent::TaskStepFailed {
                    task_id,
                    step_id: "typescript_execution".to_owned(),
                    error: format!("Failed to parse execution result: {err}"),
                });

                RoutingError::Other(format!("Failed to parse execution result: {err}"))
            })?
        };

        // Send step completed event
        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "typescript_execution".to_owned(),
        });

        tracing::debug!("TypeScript execution result: {:?}", execution_result);

        Ok(execution_result)
    }

    /// Build context for a task
    ///
    /// # Errors
    /// Returns an error if context building fails
    async fn build_context(&self, task: &Task, ui_channel: &UiChannel) -> Result<Context> {
        let intent = Self::classify_query_intent(&task.description);
        let query = Query::new(task.description.clone());
        let task_id = task.id;

        // For conversational queries, return empty context (prompt added later)
        if intent == QueryIntent::Conversational {
            return Ok(Context::new(String::new()));
        }

        // For code queries, fetch file context as normal
        let ui_clone = ui_channel.clone();

        let progress_callback = Arc::new(move |stage: &str, current: u64, total: Option<u64>| {
            ui_clone.send(UiEvent::TaskProgress {
                task_id,
                progress: TaskProgress {
                    stage: stage.to_owned(),
                    current,
                    total,
                    message: format!(
                        "{} ({}/{})",
                        stage,
                        current,
                        total.map_or_else(|| "?".to_owned(), |total_val| total_val.to_string())
                    ),
                },
            });
        });

        let mut fetcher = self.context_fetcher.lock().await;
        let project_root = fetcher.project_root().clone();
        *fetcher = replace(&mut *fetcher, ContextFetcher::new(project_root))
            .with_progress_callback(progress_callback);

        // Send substep for file gathering
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "file_gathering".to_owned(),
            step_type: "thinking".to_owned(),
            content: "Searching for relevant files".to_owned(),
        });

        // Check if we have conversation history
        let context = {
            let conv_history = self.conversation_history.lock().await;
            if conv_history.is_empty() {
                drop(conv_history);
                fetcher
                    .build_context_for_query(&query)
                    .await
                    .map_err(|err| RoutingError::Other(format!("Failed to build context: {err}")))?
            } else {
                fetcher
                    .build_context_from_conversation(&conv_history, &query)
                    .await
                    .map_err(|err| RoutingError::Other(format!("Failed to build context: {err}")))?
            }
        };

        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "file_gathering".to_owned(),
        });

        Ok(context)
    }

    /// Build context for TypeScript-based agent execution
    ///
    /// # Errors
    /// Returns an error if context building or prompt loading fails
    async fn build_context_for_typescript(
        &self,
        task: &Task,
        ui_channel: &UiChannel,
    ) -> Result<Context> {
        use merlin_core::prompts::load_prompt;
        use std::fmt::Write as _;

        // Load TypeScript agent prompt template
        let prompt_template = load_prompt("typescript_agent").map_err(|err| {
            RoutingError::Other(format!("Failed to load typescript_agent prompt: {err}"))
        })?;

        // Generate TypeScript signatures for all tools
        let tools = self.tool_registry.list_tools();
        let signatures = generate_typescript_signatures(&tools).map_err(|err| {
            RoutingError::Other(format!("Failed to generate TypeScript signatures: {err}"))
        })?;

        // Replace placeholder with actual signatures
        let system_prompt = prompt_template.replace("{TOOL_SIGNATURES}", &signatures);

        // Build base context (may include file context if relevant)
        let intent = Self::classify_query_intent(&task.description);

        let mut context = if intent == QueryIntent::Conversational {
            Context::new(system_prompt)
        } else {
            // Get file context if needed
            let base_context = self.build_context(task, ui_channel).await?;

            // Combine TypeScript prompt with file context
            let mut combined = Context::new(system_prompt);
            combined.files = base_context.files;
            combined
        };

        // Add conversation history if present
        let conv_history = self.conversation_history.lock().await;
        if !conv_history.is_empty() {
            let _write_result1 = write!(context.system_prompt, "\n\n## Conversation History\n\n");

            for (role, content) in conv_history.iter() {
                let _write_result2 = writeln!(context.system_prompt, "{role}: {content}");
            }
        }

        Ok(context)
    }

    /// Classify the intent of a query to determine context needs
    fn classify_query_intent(description: &str) -> QueryIntent {
        let desc_lower = description.to_lowercase();
        let word_count = description.split_whitespace().count();

        // Conversational patterns - no file context needed
        if desc_lower == "hi"
            || desc_lower == "hello"
            || desc_lower == "hey"
            || desc_lower == "thanks"
            || desc_lower == "thank you"
            || desc_lower.starts_with("say hi")
            || desc_lower.starts_with("say hello")
        {
            return QueryIntent::Conversational;
        }

        // Memory/recall patterns
        if desc_lower.contains("remember")
            || desc_lower.contains("what did i")
            || desc_lower.contains("what was the")
            || desc_lower.contains("recall")
            || (desc_lower.contains("what") && desc_lower.contains("told you"))
            || (desc_lower.contains("what") && desc_lower.contains("said"))
        {
            return QueryIntent::Conversational;
        }

        // Very short requests - likely conversational
        if word_count <= 3 {
            return QueryIntent::Conversational;
        }

        // Code modification keywords
        if desc_lower.contains("add ")
            || desc_lower.contains("create ")
            || desc_lower.contains("implement")
            || desc_lower.contains("write ")
            || desc_lower.contains("modify")
            || desc_lower.contains("change ")
            || desc_lower.contains("fix ")
            || desc_lower.contains("update ")
            || desc_lower.contains("refactor")
        {
            return QueryIntent::CodeModification;
        }

        // Default to code query for anything else
        QueryIntent::CodeQuery
    }

    /// Check if a request is simple enough to skip assessment
    fn is_simple_request(description: &str) -> bool {
        matches!(
            Self::classify_query_intent(description),
            QueryIntent::Conversational
        )
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
            let is_simple = Self::is_simple_request(&task.description);

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
                let Ok(decision) = self
                    .assess_task_with_provider(&provider, &task, &ui_channel, task_id)
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

                        return Ok(TaskResult {
                            task_id,
                            response: Response {
                                text: result,
                                confidence: 1.0,
                                tokens_used: TokenUsage::default(),
                                provider: decision_result.model.to_string(),
                                latency_ms: duration_ms,
                            },
                            tier_used: decision_result.model.to_string(),
                            tokens_used: TokenUsage::default(),
                            validation: ValidationResult::default(),
                            duration_ms,
                            work_unit: None,
                        });
                    }

                    TaskAction::Decompose {
                        subtasks,
                        execution_mode,
                    } => {
                        // Task needs to be decomposed into subtasks
                        task.state = TaskState::AwaitingSubtasks;

                        return self
                            .execute_with_subtasks(task.id, subtasks, execution_mode, ui_channel)
                            .await;
                    }

                    TaskAction::GatherContext { needs } => {
                        // Task needs more context before proceeding
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

                        // Gather the requested context
                        Self::gather_context(&mut exec_context, &needs);

                        // Continue loop to re-assess with new context
                    }
                }
            }
        })
    }

    /// Execute a task with subtasks
    ///
    /// # Errors
    /// Returns an error if subtask execution fails
    fn execute_with_subtasks(
        &mut self,
        task_id: TaskId,
        subtasks: Vec<Subtask>,
        _execution_mode: ExecutionMode,
        ui_channel: UiChannel,
    ) -> BoxedTaskFuture<'_> {
        Box::pin(async move {
            let start = Instant::now();

            ui_channel.send(UiEvent::TaskProgress {
                task_id,
                progress: TaskProgress {
                    stage: "Decomposing".to_owned(),
                    current: 0,
                    total: Some(subtasks.len() as u64),
                    message: format!("Breaking into {} subtasks", subtasks.len()),
                },
            });

            ui_channel.send(UiEvent::TaskOutput {
                task_id,
                output: format!("Decomposing into {} subtasks", subtasks.len()),
            });

            // Convert subtask specs to tasks
            let mut subtask_results = Vec::new();

            // For now, only support sequential execution to avoid Send issues
            // Parallel execution can be added later with proper Send bounds
            let total_subtasks = subtasks.len();
            for (index, subtask_spec) in subtasks.into_iter().enumerate() {
                let subtask = Task::new(subtask_spec.description.clone())
                    .with_difficulty(subtask_spec.difficulty);

                // Update progress
                ui_channel.send(UiEvent::TaskProgress {
                    task_id,
                    progress: TaskProgress {
                        stage: "Executing".to_owned(),
                        current: index as u64,
                        total: Some(total_subtasks as u64),
                        message: format!("Subtask {}/{}", index + 1, total_subtasks),
                    },
                });

                ui_channel.send(UiEvent::TaskOutput {
                    task_id,
                    output: format!("Executing subtask: {}", subtask.description),
                });

                let result = self
                    .execute_self_determining(subtask, ui_channel.clone())
                    .await?;
                subtask_results.push(result);
            }

            // Combine results
            let combined_response = subtask_results
                .iter()
                .map(|result| result.response.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");

            let duration_ms = start.elapsed().as_millis() as u64;

            Ok(TaskResult {
                task_id,
                response: Response {
                    text: combined_response,
                    confidence: 1.0,
                    tokens_used: TokenUsage::default(),
                    provider: "decomposed".to_owned(),
                    latency_ms: duration_ms,
                },
                tier_used: "decomposed".to_owned(),
                tokens_used: TokenUsage::default(),
                validation: ValidationResult::default(),
                duration_ms,
                work_unit: None,
            })
        })
    }

    /// Gather context based on needs
    ///
    /// # Errors
    /// Returns an error if context gathering fails
    fn gather_context(exec_context: &mut ExecutionContext, needs: &[String]) {
        for need in needs {
            // Parse the need and gather appropriate context
            if need.to_lowercase().contains("file") {
                // Would read files and add to context
                exec_context
                    .findings
                    .push(format!("Gathered file context for: {need}"));
            } else if need.to_lowercase().contains("command") {
                // Would execute commands and add results to context
                exec_context
                    .findings
                    .push(format!("Gathered command output for: {need}"));
            } else {
                // Generic context gathering
                exec_context
                    .findings
                    .push(format!("Gathered context for: {need}"));
            }
        }
    }

    /// Log context breakdown to debug.log
    async fn log_context_breakdown(&self, context: &Context) {
        use tracing::info;
        const BAR_WIDTH: usize = 50;

        info!("=====================================");
        info!("CONTEXT USAGE BREAKDOWN");
        info!("=====================================");

        // Calculate token counts
        let conv_history = self.conversation_history.lock().await;
        let conversation_tokens = Self::calculate_conversation_tokens(&conv_history);
        let total_files_tokens = Self::calculate_files_tokens(context);
        let system_prompt_tokens = context.system_prompt.len() / 4;
        let total_tokens = context.token_estimate();

        info!("Total tokens: ~{}", total_tokens);
        info!("");

        // Display bar chart breakdown
        Self::log_token_bars(
            conversation_tokens,
            total_files_tokens,
            system_prompt_tokens,
            total_tokens,
            BAR_WIDTH,
        );

        info!("=====================================");

        // Conversation preview
        Self::log_conversation_preview(&conv_history);
        drop(conv_history);

        // File breakdown
        Self::log_file_breakdown(context);

        info!("=====================================");
    }

    /// Calculate conversation token count
    fn calculate_conversation_tokens(conv_history: &[(String, String)]) -> usize {
        let char_count: usize = conv_history
            .iter()
            .map(|(role, content)| role.len() + content.len() + 10)
            .sum();
        char_count / 4
    }

    /// Calculate files token count
    fn calculate_files_tokens(context: &Context) -> usize {
        let char_count: usize = context.files.iter().map(|f| f.content.len()).sum();
        char_count / 4
    }

    /// Log token distribution bar charts
    fn log_token_bars(
        conversation_tokens: usize,
        files_tokens: usize,
        system_tokens: usize,
        total_tokens: usize,
        bar_width: usize,
    ) {
        use tracing::info;

        if total_tokens == 0 {
            return;
        }

        let conv_bar = if conversation_tokens > 0 {
            (conversation_tokens * bar_width / total_tokens).max(1)
        } else {
            0
        };
        let files_bar = if files_tokens > 0 {
            (files_tokens * bar_width / total_tokens).max(1)
        } else {
            0
        };
        let system_bar = if system_tokens > 0 {
            (system_tokens * bar_width / total_tokens).max(1)
        } else {
            0
        };

        info!(
            "Conversation:  {:>6} tokens ({:>5.1}%) {}",
            conversation_tokens,
            (conversation_tokens as f64 / total_tokens as f64) * 100.0,
            "█".repeat(conv_bar)
        );
        info!(
            "Files:         {:>6} tokens ({:>5.1}%) {}",
            files_tokens,
            (files_tokens as f64 / total_tokens as f64) * 100.0,
            "█".repeat(files_bar)
        );
        info!(
            "System Prompt: {:>6} tokens ({:>5.1}%) {}",
            system_tokens,
            (system_tokens as f64 / total_tokens as f64) * 100.0,
            "█".repeat(system_bar)
        );
    }

    /// Log conversation preview
    fn log_conversation_preview(conv_history: &[(String, String)]) {
        use tracing::info;

        if conv_history.is_empty() {
            return;
        }

        info!("💬 Conversation: {} messages", conv_history.len());
        let preview_count = conv_history.len().min(3);
        for (idx, (role, content)) in conv_history.iter().rev().take(preview_count).enumerate() {
            let preview = if content.len() > 60 {
                format!("{}...", &content[..60])
            } else {
                content.clone()
            };
            info!("  [{idx}] {role}: {preview}");
        }
        if conv_history.len() > preview_count {
            info!("  ... and {} more", conv_history.len() - preview_count);
        }
        info!("");
    }

    /// Log file breakdown
    fn log_file_breakdown(_context: &Context) {
        // File list is now printed by the context builder
    }

    /// Dump full context to debug.log
    async fn dump_context_to_log(&self, context: &Context, task: &Task) {
        use tracing::info;

        info!("================== CONTEXT DUMP ==================");
        info!("Task: {}", task.description);
        info!("");

        self.log_conversation_history().await;
        Self::log_system_prompt(context);
        Self::log_statistics(context);

        info!("================================================");
    }

    /// Log conversation history section
    async fn log_conversation_history(&self) {
        use tracing::info;

        let conv_history = self.conversation_history.lock().await;
        if !conv_history.is_empty() {
            info!(
                "=== CONVERSATION HISTORY ({} messages) ===",
                conv_history.len()
            );
            for (idx, (role, content)) in conv_history.iter().enumerate() {
                info!("[{idx}] {role}:");
                info!("{content}");
                info!("");
            }
        }
    }

    /// Log system prompt section
    fn log_system_prompt(context: &Context) {
        use tracing::info;

        info!("=== SYSTEM PROMPT ===");
        info!("{}", context.system_prompt);
        info!("");
    }

    /// Log statistics section
    fn log_statistics(context: &Context) {
        use tracing::info;

        info!("=== STATISTICS ===");
        info!("Estimated tokens: {}", context.token_estimate());
        info!("Files: {}", context.files.len());
        info!(
            "System prompt length: {} chars",
            context.system_prompt.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValidationPipeline;
    use merlin_routing::StrategyRouter;
    use merlin_tooling::{BashTool, ToolRegistry};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_agent_executor_creation() {
        // Use local-only config to avoid needing API keys
        let mut config = RoutingConfig::default();
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let router = StrategyRouter::with_default_strategies();
        if router.is_err() {
            // Expected when providers can't be initialized
            return;
        }
        let router = Arc::new(router.unwrap());

        let validator = Arc::new(ValidationPipeline::with_default_stages());
        let tool_registry = Arc::new(ToolRegistry::default());
        let context_fetcher = ContextFetcher::new(PathBuf::from("."));

        let executor =
            AgentExecutor::new(router, validator, tool_registry, context_fetcher, &config);

        // Without API keys, executor creation may fail
        if executor.is_err() {
            return;
        }

        let _executor = executor.unwrap();
        // Executor created successfully
    }

    #[tokio::test]
    async fn test_tool_registry_integration() {
        let tool_registry = Arc::new(ToolRegistry::default().with_tool(Arc::new(BashTool)));

        assert!(tool_registry.get_tool("bash").is_some());
        assert!(tool_registry.get_tool("nonexistent").is_none());
    }

    #[test]
    fn test_extract_typescript_code_single_block() {
        let text = r#"
I'll read the file using TypeScript:

```typescript
const content = await readFile("src/main.rs");
return {done: true, result: content};
```

That should work!
"#;
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_some());
        let code = code.unwrap();
        assert!(code.contains("readFile"));
        assert!(code.contains("done: true"));
    }

    #[test]
    fn test_extract_typescript_code_ts_language() {
        let text = r#"
```ts
const files = await listFiles("src");
return {done: true, result: files.join(", ")};
```
"#;
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_some());
        let code = code.unwrap();
        assert!(code.contains("listFiles"));
    }

    #[test]
    fn test_extract_typescript_code_multiple_blocks() {
        let text = r#"
First block:
```typescript
const x = 1;
```

Second block:
```typescript
const y = 2;
return {done: true, result: "ok"};
```
"#;
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_some());
        let code = code.unwrap();
        assert!(code.contains("const x = 1"));
        assert!(code.contains("const y = 2"));
    }

    #[test]
    fn test_extract_typescript_code_no_blocks() {
        let text = "Just regular text with no code blocks";
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_none());
    }

    // Old JSON-based tests commented out - replaced by TypeScript-based system
    /*
    #[test]
    fn test_extract_multiple_tool_calls_old() {
        let text = r#"
I'll check both files.

```json
{
  "tool": "read_file",
  "params": {
    "file_path": "src/main.rs"
  }
}
```

```json
{
  "tool": "read_file",
  "params": {
    "file_path": "src/lib.rs"
  }
}
```

Done.
"#;
        let calls = AgentExecutor::extract_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "read_file");
        assert_eq!(calls[0].1["file_path"], "src/main.rs");
        assert_eq!(calls[1].0, "read_file");
        assert_eq!(calls[1].1["file_path"], "src/lib.rs");
    }

    #[test]
    fn test_extract_tool_call_raw_json() {
        let text = r#"
Let me execute this command.

{
  "tool": "run_command",
  "params": {
    "command": "cargo build"
  }
}

Running the build.
"#;
        let calls = AgentExecutor::extract_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "run_command");
        assert_eq!(calls[0].1["command"], "cargo build");
    }

    #[test]
    fn test_extract_tool_call_plain_code_block() {
        let text = r#"
Let me write the file.

```
{
  "tool": "write_file",
  "params": {
    "file_path": "test.txt",
    "content": "Hello, world!"
  }
}
```
"#;
        let calls = AgentExecutor::extract_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "write_file");
        assert_eq!(calls[0].1["file_path"], "test.txt");
        assert_eq!(calls[0].1["content"], "Hello, world!");
    }

    #[test]
    fn test_extract_tool_calls_mixed_formats() {
        let text = r#"
First, let me list the files.

```json
{
  "tool": "list_files",
  "params": {
    "directory": "src"
  }
}
```

Now I'll read one.

{
  "tool": "read_file",
  "params": {
    "file_path": "src/main.rs"
  }
}

And run a command.

```
{
  "tool": "run_command",
  "params": {
    "command": "cargo test"
  }
}
```
"#;
        let calls = AgentExecutor::extract_tool_calls(text);
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "list_files");
        assert_eq!(calls[1].0, "read_file");
        assert_eq!(calls[2].0, "run_command");
    }

    #[test]
    fn test_extract_tool_calls_nested_braces() {
        let text = r#"
{
  "tool": "write_file",
  "params": {
    "file_path": "config.json",
    "content": "{\"nested\": {\"value\": 42}}"
  }
}
"#;
        let calls = AgentExecutor::extract_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "write_file");
        assert!(calls[0].1["content"].as_str().unwrap().contains("nested"));
    }

    #[test]
    fn test_extract_tool_calls_no_tools() {
        let text = "This is just a regular response with no tool calls.";
        let calls = AgentExecutor::extract_tool_calls(text);
        assert_eq!(calls.len(), 0);
    }

    #[test]
    fn test_extract_tool_calls_invalid_json() {
        let text = r#"
```json
{
  "tool": "read_file",
  "params": {
    "file_path": "src/main.rs"
  }
  missing closing brace
```
"#;
        let calls = AgentExecutor::extract_tool_calls(text);
        assert_eq!(calls.len(), 0, "Invalid JSON should be skipped");
    }

    #[test]
    fn test_extract_output_section_single() {
        let text = "
Some reasoning text here.

<output>
This is the output for the user.
</output>

More internal notes.
";
        let output = AgentExecutor::extract_output_section(text);
        assert_eq!(output, "This is the output for the user.");
    }

    #[test]
    fn test_extract_output_section_multiple() {
        let text = "
<output>
First output section.
</output>

Some reasoning.

<output>
Second output section.
</output>
";
        let output = AgentExecutor::extract_output_section(text);
        assert_eq!(output, "First output section.\n\nSecond output section.");
    }

    #[test]
    fn test_extract_output_section_no_tags() {
        let text = "This is a response without output tags.";
        let output = AgentExecutor::extract_output_section(text);
        assert_eq!(output, text, "Should return original text when no tags");
    }

    #[test]
    fn test_extract_output_section_empty() {
        let text = "<output>\n</output>";
        let output = AgentExecutor::extract_output_section(text);
        assert_eq!(output, "", "Empty output tags should return empty string");
    }

    #[test]
    fn test_extract_output_section_whitespace() {
        let text = "
<output>
   This has leading and trailing whitespace.
</output>
";
        let output = AgentExecutor::extract_output_section(text);
        assert_eq!(
            output, "This has leading and trailing whitespace.",
            "Should trim whitespace"
        );
    }

    #[test]
    fn test_extract_output_section_malformed() {
        let text = "
<output>
This has an opening tag but no closing tag.
";
        let output = AgentExecutor::extract_output_section(text);
        assert_eq!(output, text, "Malformed tags should return original text");
    }

    #[test]
    fn test_extract_output_section_with_tool_calls() {
        let text = "
Let me check the file.

{
  \"tool\": \"read_file\",
  \"params\": {
    \"file_path\": \"src/main.rs\"
  }
}

<output>
The main.rs file contains the application entry point.
</output>
";
        let output = AgentExecutor::extract_output_section(text);
        assert_eq!(
            output,
            "The main.rs file contains the application entry point."
        );
    }

    #[test]
    fn test_extract_output_multiline_content() {
        let text = "
<output>
This is a multi-line output.

It contains multiple paragraphs.

And some code:
```rust
fn main() {
    println!(\"Hello\");
}
```
</output>
";
        let output = AgentExecutor::extract_output_section(text);
        assert!(output.contains("multi-line output"));
        assert!(output.contains("multiple paragraphs"));
        assert!(output.contains("fn main()"));
    }
    */

    #[test]
    fn test_plain_string_result_handling() {
        // Test that plain string results are treated as "done"
        use serde_json::json;

        let string_value = json!("List of todos:\nTODO: Fix this\nTODO: Test that");

        // Simulate what the executor does
        let execution_result: AgentExecutionResult = if string_value.is_string() {
            let result_str = string_value.as_str().unwrap_or("").to_owned();
            AgentExecutionResult::done(result_str)
        } else {
            panic!("Expected string value");
        };

        assert!(execution_result.is_done());
        assert_eq!(
            execution_result.get_result(),
            Some("List of todos:\nTODO: Fix this\nTODO: Test that")
        );
    }

    #[test]
    fn test_structured_result_handling() {
        // Test that structured results are parsed correctly
        use serde_json::json;

        let structured_value = json!({
            "done": "true",
            "result": "Task completed successfully"
        });

        let execution_result: AgentExecutionResult = from_value(structured_value).unwrap();

        assert!(execution_result.is_done());
        assert_eq!(
            execution_result.get_result(),
            Some("Task completed successfully")
        );
    }

    #[test]
    fn test_continue_result_handling() {
        // Test that continue results are parsed correctly
        use serde_json::json;

        let continue_value = json!({
            "done": "false",
            "continue": "Check the logs for errors"
        });

        let execution_result: AgentExecutionResult = from_value(continue_value).unwrap();

        assert!(!execution_result.is_done());
        assert_eq!(
            execution_result.get_next_task(),
            Some("Check the logs for errors")
        );
    }

    #[test]
    fn test_extract_typescript_code_syntax_error() {
        // Test that TypeScript code with syntax errors can still be extracted
        let text = r"
```typescript
const x = ;  // Syntax error
return {done: true};
```
";
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_some());
        let code = code.unwrap();
        assert!(code.contains("const x ="));
    }

    #[test]
    fn test_extract_typescript_code_no_code_blocks() {
        // Test that text without code blocks returns None
        let text = "This is just plain text without any code blocks.";
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_none());
    }

    #[test]
    fn test_extract_typescript_code_empty_block() {
        // Test that empty code blocks are filtered out and return None
        let text = r"
```typescript
```
";
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_none(), "Empty code blocks should be filtered out");
    }

    #[test]
    fn test_agent_execution_result_error_handling() {
        // Test that error results without "done" or "continue" fields are handled
        use serde_json::{Result as SerdeResult, json};

        let error_value = json!({
            "error": "Something went wrong",
            "message": "Detailed error message"
        });

        // When neither done nor continue is present, it should fail to parse
        let result: SerdeResult<AgentExecutionResult> = from_value(error_value);
        // This should fail to parse since the structure is malformed
        assert!(
            result.is_err(),
            "Malformed execution results should fail to parse"
        );
    }

    #[test]
    fn test_extract_typescript_code_with_indentation() {
        // Test that indented code blocks are preserved
        let text = r#"
Here's the code:

```typescript
function test() {
    if (true) {
        const nested = "value";
        return {done: true, result: nested};
    }
}
```
"#;
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_some());
        let code = code.unwrap();
        assert!(code.contains("    if (true)"));
        assert!(code.contains("        const nested"));
    }

    #[test]
    fn test_extract_typescript_code_mixed_languages() {
        // Test that only TypeScript blocks are extracted, not other languages
        let text = r"
```rust
fn main() {}
```

```typescript
const x = 1;
return {done: true};
```

```python
def test():
    pass
```
";
        let code = AgentExecutor::extract_typescript_code(text);
        assert!(code.is_some());
        let code = code.unwrap();
        assert!(code.contains("const x = 1"));
        assert!(!code.contains("fn main"));
        assert!(!code.contains("def test"));
    }
}
