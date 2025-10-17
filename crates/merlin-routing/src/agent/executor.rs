use serde_json::Value;
use std::future::Future;
use std::mem::replace;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::{
    ExecutionContext, ExecutionMode, ModelRouter, ModelTier, Result, RoutingConfig, RoutingError,
    SubtaskSpec, Task, TaskAction, TaskDecision, TaskId, TaskResult, TaskState, ToolRegistry,
    UiChannel, UiEvent, ValidationResult, Validator, streaming::StepType,
    user_interface::events::TaskProgress,
};
use merlin_core::{Context, ModelProvider, Query, Response, TokenUsage};
use merlin_local::LocalModelProvider;
use merlin_providers::{AnthropicProvider, GroqProvider, OpenRouterProvider};
use std::env;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{ContextFetcher, SelfAssessor, StepTracker};

/// Type alias for boxed future returning `TaskResult`
type BoxedTaskFuture<'future> = Pin<Box<dyn Future<Output = Result<TaskResult>> + Send + 'future>>;

/// Type alias for conversation history
type ConversationHistory = Vec<(String, String)>;

/// Agent executor that streams task execution with tool calling
#[derive(Clone)]
pub struct AgentExecutor {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    tool_registry: Arc<ToolRegistry>,
    step_tracker: StepTracker,
    context_fetcher: Arc<Mutex<ContextFetcher>>,
    conversation_history: Arc<Mutex<ConversationHistory>>,
    context_dump_enabled: Arc<AtomicBool>,
    config: Arc<RoutingConfig>,
}

struct ExecInputs<'life> {
    provider: &'life Arc<dyn ModelProvider>,
    query: &'life Query,
    context: &'life Context,
    ui_channel: &'life UiChannel,
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
    pub fn new(
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        tool_registry: Arc<ToolRegistry>,
        context_fetcher: ContextFetcher,
        config: RoutingConfig,
    ) -> Self {
        Self {
            router,
            validator,
            tool_registry,
            step_tracker: StepTracker::default(),
            context_fetcher: Arc::new(Mutex::new(context_fetcher)),
            conversation_history: Arc::new(Mutex::new(Vec::new())),
            context_dump_enabled: Arc::new(AtomicBool::new(false)),
            config: Arc::new(config),
        }
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

        // Step 1: Route the task
        let decision = self.router.route(&task).await?;

        // Step 2: Create provider
        let provider = self.create_provider(&decision.tier)?;

        // Step 3: Build context
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "context_analysis".to_owned(),
            step_type: "thinking".to_owned(),
            content: "Analyzing query intent".to_owned(),
        });

        let context = self.build_context(&task, &ui_channel).await?;

        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "context_analysis".to_owned(),
        });

        // Log context breakdown to debug.log
        self.log_context_breakdown(&context).await;

        // Dump full context if enabled
        if self.context_dump_enabled.load(Ordering::Relaxed) {
            self.dump_context_to_log(&context, &task).await;
        }

        // Step 4: Create query with tool descriptions
        let query = self.create_query_with_tools(&task)?;

        // Step 5: Execute with streaming
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "model_execution".to_owned(),
            step_type: "tool_call".to_owned(),
            content: format!("Executing with {}", decision.tier),
        });

        let response = self
            .execute_with_streaming(
                task_id,
                ExecInputs {
                    provider: &provider,
                    query: &query,
                    context: &context,
                    ui_channel: &ui_channel,
                },
            )
            .await?;

        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "model_execution".to_owned(),
        });

        // Step 6: Validate
        let validation = self.validator.validate(&response, &task).await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(TaskResult {
            task_id,
            response,
            tier_used: decision.tier.to_string(),
            tokens_used: TokenUsage::default(),
            validation,
            duration_ms,
        })
    }

    /// Execute with streaming and tool calling support
    ///
    /// # Errors
    /// Returns an error if provider generation fails or tool execution fails.
    async fn execute_with_streaming(
        &mut self,
        task_id: TaskId,
        inputs: ExecInputs<'_>,
    ) -> Result<Response> {
        let ExecInputs {
            provider,
            query,
            context,
            ui_channel,
        } = inputs;
        // Execute the query directly without extra steps
        let response = provider
            .generate(query, context)
            .await
            .map_err(|err| RoutingError::Other(format!("Provider error: {err}")))?;

        // Log agent response to debug.log BEFORE processing
        tracing::debug!("Agent response text: {}", response.text);

        // Parse tool calls from response
        let tool_calls = Self::extract_tool_calls(&response.text);

        if tool_calls.is_empty() {
            tracing::debug!("No tool calls found in response");
        } else {
            tracing::debug!("Found {} tool call(s)", tool_calls.len());
        }

        if !tool_calls.is_empty() {
            self.execute_tool_calls(task_id, tool_calls, ui_channel)
                .await?;
        }

        // Send output directly as text (no wrapper step)
        ui_channel.send(UiEvent::TaskOutput {
            task_id,
            output: response.text.clone(),
        });

        Ok(response)
    }

    /// Execute all tool calls from the LLM response
    ///
    /// # Errors
    /// Returns an error if any tool execution fails
    async fn execute_tool_calls(
        &mut self,
        task_id: TaskId,
        tool_calls: Vec<(String, Value)>,
        ui_channel: &UiChannel,
    ) -> Result<()> {
        for (tool_name, args) in tool_calls {
            // Send tool call started event
            ui_channel.send(UiEvent::ToolCallStarted {
                task_id,
                tool: tool_name.clone(),
                args: args.clone(),
            });

            let tool_step = self.step_tracker.create_step(
                task_id,
                StepType::ToolCall {
                    tool: tool_name.clone(),
                    args: args.clone(),
                },
                format!("Calling tool: {tool_name}"),
            );

            ui_channel.send(UiEvent::TaskStepStarted {
                task_id,
                step_id: format!("{:?}", tool_step.id),
                step_type: "tool_call".to_owned(),
                content: tool_step.content.clone(),
            });

            // Execute the tool
            tracing::debug!("Executing tool: {} with args: {:?}", tool_name, args);
            let result = match self.execute_tool(&tool_name, args.clone()).await {
                Ok(val) => {
                    tracing::debug!("Tool {} succeeded with result: {:?}", tool_name, val);
                    val
                }
                Err(err) => {
                    tracing::error!("Tool {} failed with error: {}", tool_name, err);
                    tracing::error!("Tool args were: {:?}", args);
                    return Err(err);
                }
            };

            // Send tool call completed event
            ui_channel.send(UiEvent::ToolCallCompleted {
                task_id,
                tool: tool_name.clone(),
                result: result.clone(),
            });

            let result_step = self.step_tracker.create_step(
                task_id,
                StepType::ToolResult {
                    tool: tool_name.clone(),
                    result: result.clone(),
                },
                format!("Tool result: {result}"),
            );

            ui_channel.send(UiEvent::TaskStepCompleted {
                task_id,
                step_id: format!("{:?}", result_step.id),
            });

            // Tool results are shown in the output tree via ToolCallCompleted events,
            // no need to append them to response.text (which would create trailing newlines)
            // In a real implementation, would re-query LLM with tool results to get final response
        }
        Ok(())
    }

    /// Execute a tool by name
    ///
    /// # Errors
    /// Returns an error if the tool cannot be found or execution fails.
    async fn execute_tool(&self, tool_name: &str, args: Value) -> Result<Value> {
        let tool = self
            .tool_registry
            .get_tool(tool_name)
            .ok_or_else(|| RoutingError::Other(format!("Tool not found: {tool_name}")))?;

        tool.execute(args).await
    }

    /// Extract tool calls from LLM response
    ///
    /// Looks for JSON objects in the format: `{"tool": "tool_name", "params": {...}}`
    fn extract_tool_calls(text: &str) -> Vec<(String, Value)> {
        use serde_json::from_str;

        // Try to find JSON in markdown code blocks first
        let json_str = if let Some(start) = text.find("```json") {
            let Some(after_start) = text.get(start + 7..) else {
                return vec![];
            };
            let Some(end) = after_start.find("```") else {
                return vec![];
            };
            let Some(content) = after_start.get(..end) else {
                return vec![];
            };
            content.trim()
        } else if let Some(start) = text.find('{') {
            // Try to find raw JSON
            let Some(end) = text.rfind('}') else {
                return vec![];
            };
            let Some(content) = text.get(start..=end) else {
                return vec![];
            };
            content
        } else {
            return vec![];
        };

        // Parse the JSON
        let Ok(value) = from_str::<Value>(json_str) else {
            tracing::warn!("Failed to parse tool call JSON");
            return vec![];
        };

        // Extract tool name and params
        let Some(tool_name) = value.get("tool").and_then(|val| val.as_str()) else {
            tracing::warn!("Tool call JSON missing 'tool' field");
            return vec![];
        };

        let Some(params) = value.get("params") else {
            tracing::warn!("Tool call JSON missing 'params' field");
            return vec![];
        };

        vec![(tool_name.to_owned(), params.clone())]
    }

    /// Create query with tool descriptions
    ///
    /// # Errors
    /// Returns an error if formatting the prompt fails.
    fn create_query_with_tools(&self, task: &Task) -> Result<Query> {
        // Start with clear user request label
        let mut prompt = format!("# User Request\n\n{}", task.description);

        // Add tool descriptions to the prompt
        let tools = self.tool_registry.list_tools();
        if !tools.is_empty() {
            prompt.push_str("\n\n# Available Tools\n");
            for tool in tools {
                writeln!(prompt, "- {}: {}", tool.name(), tool.description())?;
            }
        }

        Ok(Query::new(prompt))
    }
    /// Create provider based on tier
    ///
    /// # Errors
    /// Returns an error if required API keys are missing or provider initialization fails.
    fn create_provider(&self, tier: &ModelTier) -> Result<Arc<dyn ModelProvider>> {
        match tier {
            ModelTier::Local { model_name } => {
                Ok(Arc::new(LocalModelProvider::new(model_name.clone())))
            }
            ModelTier::Groq { model_name } => {
                // Get Groq API key from config first, then environment
                let api_key_from_config = self.config.get_api_key("groq");

                tracing::debug!(
                    "Groq API key from config: {}",
                    if api_key_from_config.is_some() {
                        "present"
                    } else {
                        "missing"
                    }
                );

                let api_key = api_key_from_config
                    .or_else(|| env::var("GROQ_API_KEY").ok())
                    .ok_or_else(|| {
                        RoutingError::Other(
                            "GROQ_API_KEY not found in config or environment".to_owned(),
                        )
                    })?;

                let provider = GroqProvider::with_api_key_direct(api_key)
                    .map_err(|error| RoutingError::Other(error.to_string()))?
                    .with_model(model_name.clone());
                Ok(Arc::new(provider))
            }
            ModelTier::Premium {
                provider: provider_name,
                model_name,
            } => match provider_name.as_str() {
                "openrouter" => {
                    let api_key = self
                        .config
                        .get_api_key("openrouter")
                        .or_else(|| env::var("OPENROUTER_API_KEY").ok())
                        .ok_or_else(|| {
                            RoutingError::Other(
                                "OPENROUTER_API_KEY not found in config or environment".to_owned(),
                            )
                        })?;
                    let provider = OpenRouterProvider::new(api_key)?.with_model(model_name.clone());
                    Ok(Arc::new(provider))
                }
                "anthropic" => {
                    let api_key = self
                        .config
                        .get_api_key("anthropic")
                        .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
                        .ok_or_else(|| {
                            RoutingError::Other(
                                "ANTHROPIC_API_KEY not found in config or environment".to_owned(),
                            )
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

    /// Build context for conversational queries (no files)
    async fn build_conversational_context(
        conversation_history: &Arc<Mutex<ConversationHistory>>,
    ) -> Context {
        use merlin_core::prompts::load_prompt;

        // Always load the proper coding assistant prompt
        let base_prompt = load_prompt("coding_assistant")
            .unwrap_or_else(|_| {
                "You are a helpful AI coding assistant. Answer the user's question directly and conversationally.".to_owned()
            });

        let conv_history = conversation_history.lock().await;
        let system_prompt = if conv_history.is_empty() {
            base_prompt
        } else {
            // Append conversation history to the coding assistant prompt
            let mut prompt = base_prompt;
            prompt.push_str("\n\n## Conversation History\n\n");
            for (role, content) in conv_history.iter() {
                use std::fmt::Write as _;
                #[allow(clippy::expect_used, reason = "Writing to String never fails")]
                writeln!(prompt, "{role}: {content}").expect("Writing to String never fails");
            }
            prompt.push_str("\nAnswer the user's question based on this conversation and your role as a coding assistant.");
            prompt
        };
        drop(conv_history);

        Context::new(system_prompt)
    }

    /// Build context for a task
    ///
    /// # Errors
    /// Returns an error if context building fails
    async fn build_context(&self, task: &Task, ui_channel: &UiChannel) -> Result<Context> {
        let intent = Self::classify_query_intent(&task.description);
        let query = Query::new(task.description.clone());
        let task_id = task.id;

        // For conversational queries, skip file fetching and use minimal context
        if intent == QueryIntent::Conversational {
            return Ok(Self::build_conversational_context(&self.conversation_history).await);
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

                // Route and create provider
                let decision_result = self.router.route(&task).await?;
                let provider = self.create_provider(&decision_result.tier)?;

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
                                provider: decision_result.tier.to_string(),
                                latency_ms: duration_ms,
                            },
                            tier_used: decision_result.tier.to_string(),
                            tokens_used: TokenUsage::default(),
                            validation: ValidationResult::default(),
                            duration_ms,
                        });
                    }

                    TaskAction::Decompose {
                        subtasks,
                        execution_mode,
                    } => {
                        // Task needs to be decomposed into subtasks
                        task.state = TaskState::AwaitingSubtasks;

                        return self
                            .execute_with_subtasks(task, subtasks, execution_mode, ui_channel)
                            .await;
                    }

                    TaskAction::GatherContext { needs } => {
                        // Task needs more context before proceeding
                        ui_channel.send(UiEvent::TaskProgress {
                            task_id,
                            progress: super::super::user_interface::events::TaskProgress {
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
    #[allow(
        clippy::needless_pass_by_value,
        reason = "Task is consumed by async block"
    )]
    fn execute_with_subtasks(
        &mut self,
        task: Task,
        subtasks: Vec<SubtaskSpec>,
        _execution_mode: ExecutionMode,
        ui_channel: UiChannel,
    ) -> BoxedTaskFuture<'_> {
        Box::pin(async move {
            let task_id = task.id;
            let start = Instant::now();

            ui_channel.send(UiEvent::TaskProgress {
                task_id,
                progress: super::super::user_interface::events::TaskProgress {
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
            for (index, spec) in subtasks.into_iter().enumerate() {
                let subtask = Task::new(spec.description).with_complexity(spec.complexity);

                // Update progress
                ui_channel.send(UiEvent::TaskProgress {
                    task_id,
                    progress: super::super::user_interface::events::TaskProgress {
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
    fn log_file_breakdown(context: &Context) {
        use tracing::info;

        if context.files.is_empty() {
            return;
        }

        info!("📁 File breakdown: {} files included", context.files.len());
        for (index, file) in context.files.iter().enumerate() {
            let tokens = file.content.len() / 4;
            info!(
                "  {}. {} - {} tokens",
                index + 1,
                file.path.display(),
                tokens
            );
        }
    }

    /// Dump full context to debug.log
    async fn dump_context_to_log(&self, context: &Context, task: &Task) {
        use tracing::info;

        info!("================== CONTEXT DUMP ==================");
        info!("Task: {}", task.description);
        info!("");

        // Conversation history
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
        drop(conv_history);

        // System prompt
        info!("=== SYSTEM PROMPT ===");
        info!("{}", context.system_prompt);
        info!("");

        // Files
        if !context.files.is_empty() {
            info!("=== FILES ({}) ===", context.files.len());
            for (index, file) in context.files.iter().enumerate() {
                info!(
                    "  {}. {} ({} bytes, ~{} tokens)",
                    index + 1,
                    file.path.display(),
                    file.content.len(),
                    file.content.len() / 4
                );
            }
        }

        // Token estimate
        info!("=== STATISTICS ===");
        info!("Estimated tokens: {}", context.token_estimate());
        info!("Files: {}", context.files.len());
        info!(
            "System prompt length: {} chars",
            context.system_prompt.len()
        );
        info!("================================================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ListFilesTool, ReadFileTool, RunCommandTool, StrategyRouter, ToolRegistry,
        ValidationPipeline, WriteFileTool,
    };
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_agent_executor_creation() {
        let router = Arc::new(StrategyRouter::with_default_strategies());
        let validator = Arc::new(ValidationPipeline::with_default_stages());
        let tool_registry = Arc::new(ToolRegistry::default());
        let context_fetcher = ContextFetcher::new(PathBuf::from("."));
        let config = RoutingConfig::default();

        let executor =
            AgentExecutor::new(router, validator, tool_registry, context_fetcher, config);

        // Just verify it was created successfully
        assert!(
            executor
                .step_tracker
                .get_steps(&TaskId::default())
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_tool_registry_integration() {
        let workspace = PathBuf::from(".");
        let tool_registry = Arc::new(
            ToolRegistry::default()
                .with_tool(Arc::new(ReadFileTool::new(workspace.clone())))
                .with_tool(Arc::new(WriteFileTool::new(workspace.clone())))
                .with_tool(Arc::new(ListFilesTool::new(workspace.clone())))
                .with_tool(Arc::new(RunCommandTool::new(workspace))),
        );

        assert!(tool_registry.get_tool("read_file").is_some());
        assert!(tool_registry.get_tool("write_file").is_some());
        assert!(tool_registry.get_tool("list_files").is_some());
        assert!(tool_registry.get_tool("run_command").is_some());
        assert!(tool_registry.get_tool("nonexistent").is_none());
    }
}
