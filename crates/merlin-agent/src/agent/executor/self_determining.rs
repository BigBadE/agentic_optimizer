//! Self-determining task execution

use crate::SelfAssessor;
use merlin_core::{
    Context, ExecutionContext, ExecutionMode, ModelProvider, Query, Response, Result, RoutingError,
    Subtask, Task, TaskDecision, TaskId, TaskResult, TokenUsage, ValidationResult,
    ui::{TaskProgress, UiChannel, UiEvent},
};
use merlin_routing::UiChannel as RoutingUiChannel;
use merlin_tooling::ToolRegistry;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use super::typescript;

/// Type alias for boxed future returning `TaskResult`
pub type BoxedTaskFuture<'future> =
    Pin<Box<dyn Future<Output = Result<TaskResult>> + Send + 'future>>;

/// Parameters for streaming execution
pub struct StreamingParams<'life> {
    /// Model provider
    pub provider: &'life Arc<dyn ModelProvider>,
    /// Query to execute
    pub query: &'life Query,
    /// Context for execution
    pub context: &'life Context,
    /// Tool registry
    pub tool_registry: &'life Arc<ToolRegistry>,
    /// Task ID
    pub task_id: TaskId,
    /// UI channel
    pub ui_channel: &'life RoutingUiChannel,
}

/// Self-determining execution logic
pub struct SelfDeterminingExecutor;

impl SelfDeterminingExecutor {
    /// Create a task completion result
    pub fn create_completion_result(
        task_id: TaskId,
        result: String,
        model: String,
        duration_ms: u64,
    ) -> TaskResult {
        TaskResult {
            task_id,
            response: Response {
                text: result,
                confidence: 1.0,
                tokens_used: TokenUsage::default(),
                provider: model.clone(),
                latency_ms: duration_ms,
            },
            tier_used: model,
            tokens_used: TokenUsage::default(),
            validation: ValidationResult::default(),
            duration_ms,
            work_unit: None,
        }
    }
    /// Complete analysis step
    pub fn complete_analysis_step(ui_channel: &UiChannel, task_id: TaskId) {
        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "analysis".to_owned(),
        });
    }

    /// Assess a task using the given provider and write assessment output to UI.
    ///
    /// # Errors
    /// Returns an error if the provider generation fails or the assessment cannot be parsed.
    pub async fn assess_task_with_provider(
        provider: &Arc<dyn ModelProvider>,
        task: &Task,
        ui_channel: &RoutingUiChannel,
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

    /// Execute a task with subtasks (unused, kept for future use)
    ///
    /// # Errors
    /// Returns an error if subtask execution fails
    #[allow(dead_code, reason = "Kept for future parallel execution")]
    pub fn execute_with_subtasks<'life, F>(
        task_id: TaskId,
        subtasks: Vec<Subtask>,
        _execution_mode: ExecutionMode,
        ui_channel: RoutingUiChannel,
        execute_fn: F,
    ) -> BoxedTaskFuture<'life>
    where
        F: Fn(Task, RoutingUiChannel) -> BoxedTaskFuture<'life> + Send + 'life,
    {
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

                let result = execute_fn(subtask, ui_channel.clone()).await?;
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
    pub fn gather_context(exec_context: &mut ExecutionContext, needs: &[String]) {
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
}

/// Execute with streaming and TypeScript code execution
///
/// # Errors
/// Returns an error if provider generation fails or code execution fails.
pub async fn execute_with_streaming(params: StreamingParams<'_>) -> Result<Response> {
    // Execute the query directly
    let response = params
        .provider
        .generate(params.query, params.context)
        .await
        .map_err(|err| RoutingError::Other(format!("Provider error: {err}")))?;

    // Log agent response to debug.log
    tracing::debug!("Agent response text: {}", response.text);

    // Extract TypeScript code from response
    let code = typescript::extract_typescript_code(&response.text);

    if let Some(typescript_code) = code {
        tracing::debug!("Found TypeScript code, executing...");

        // Execute TypeScript code and get result
        let execution_result = typescript::execute_typescript_code(
            params.tool_registry,
            params.task_id,
            &typescript_code,
            params.ui_channel,
        )
        .await?;

        // Handle result based on done status
        if let Some(result_text) = execution_result.get_result() {
            // Task is done - send final result to UI
            params.ui_channel.send(UiEvent::TaskOutput {
                task_id: params.task_id,
                output: result_text.to_owned(),
            });
        } else if let Some(next_task_desc) = execution_result.get_next_task() {
            // Task wants to continue - spawn new task
            tracing::info!("Task requested continuation: {}", next_task_desc);
            params.ui_channel.send(UiEvent::TaskOutput {
                task_id: params.task_id,
                output: format!("Continuing with: {next_task_desc}"),
            });
            // Note: Actual task spawning would happen in the orchestrator
            // For now, we just indicate the continuation request
        }
    } else {
        // No TypeScript code found - send response text as-is
        tracing::debug!("No TypeScript code found in response");
        params.ui_channel.send(UiEvent::TaskOutput {
            task_id: params.task_id,
            output: response.text.clone(),
        });
    }

    Ok(response)
}
