//! Step executor for recursive task decomposition with exit requirements

use std::{result, sync::Arc, time::Instant};

use merlin_core::{
    AgentResponse, Context, ContextSpec, JsValueHandle, ModelProvider, Query, Result, RoutingError,
    TaskId, TaskList, TaskStep, ValidationErrorType, WorkUnit,
};
use merlin_deps::tracing::{Level, span};
use merlin_routing::UiChannel;
use merlin_tooling::{PersistentTypeScriptRuntime, ToolRegistry, ToolingJsValueHandle};
use tokio::sync::Mutex;
use tracing_futures::Instrument as _;

use super::typescript::{execute_typescript_code, extract_typescript_code};

/// Maximum recursion depth for task decomposition
const MAX_RECURSION_DEPTH: usize = 10;

/// Maximum retry attempts per step
const MAX_RETRY_ATTEMPTS: usize = 3;

/// Step execution result - accumulated from recursive executions
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Final result text
    pub text: String,
    /// Total execution time
    pub duration_ms: u64,
    /// Whether this step succeeded
    pub success: bool,
}

/// Parameters for step execution
pub struct StepExecutionParams<'params> {
    /// The step to execute
    pub step: &'params TaskStep,
    /// Base context to build from
    pub base_context: &'params Context,
    /// Step results from previous steps
    pub previous_results: &'params [StepResult],
    /// Provider for execution
    pub provider: &'params Arc<dyn ModelProvider>,
    /// Tool registry
    pub tool_registry: &'params Arc<ToolRegistry>,
    /// Persistent TypeScript runtime
    pub runtime: &'params mut PersistentTypeScriptRuntime,
    /// Task ID for UI events
    pub task_id: TaskId,
    /// UI channel
    pub ui_channel: &'params UiChannel,
    /// Current recursion depth
    pub recursion_depth: usize,
}

/// Parameters for task list execution
pub struct TaskListExecutionParams<'params> {
    /// The task list to execute
    pub task_list: &'params TaskList,
    /// Base context to build from
    pub base_context: &'params Context,
    /// Provider for execution
    pub provider: &'params Arc<dyn ModelProvider>,
    /// Tool registry
    pub tool_registry: &'params Arc<ToolRegistry>,
    /// Persistent TypeScript runtime
    pub runtime: &'params mut PersistentTypeScriptRuntime,
    /// Task ID for UI events
    pub task_id: TaskId,
    /// UI channel
    pub ui_channel: &'params UiChannel,
    /// Current recursion depth
    pub recursion_depth: usize,
    /// Work unit for tracking subtask progress (optional, only for top-level decomposition)
    pub work_unit: Option<&'params Arc<Mutex<WorkUnit>>>,
}

/// Parameters for agent execution
pub struct AgentExecutionParams<'params> {
    /// The step to execute
    pub step: &'params TaskStep,
    /// Context for execution
    pub context: &'params Context,
    /// Provider for execution
    pub provider: &'params Arc<dyn ModelProvider>,
    /// Tool registry
    pub tool_registry: &'params Arc<ToolRegistry>,
    /// Persistent TypeScript runtime
    pub runtime: &'params mut PersistentTypeScriptRuntime,
    /// Task ID for UI events
    pub task_id: TaskId,
    /// UI channel
    pub ui_channel: &'params UiChannel,
}

/// Step executor handles recursive task decomposition and validation
pub struct StepExecutor;

impl StepExecutor {
    /// Handle validation error and determine retry strategy
    ///
    /// # Errors
    /// Returns `Err(())` if validation failed and retry should be attempted
    fn handle_validation_error(
        validation_result: result::Result<(), ValidationErrorType>,
        step_title: &str,
        attempt: &mut usize,
    ) -> result::Result<(), ()> {
        match validation_result {
            Ok(()) => Ok(()),
            Err(ValidationErrorType::Hard(err)) => {
                merlin_deps::tracing::warn!("Hard validation error for step '{step_title}': {err}");
                *attempt += 1;
                // TODO: Implement model tier escalation
                Err(())
            }
            Err(ValidationErrorType::Soft(err)) => {
                merlin_deps::tracing::info!("Soft validation error for step '{step_title}': {err}");
                *attempt += 1;
                // TODO: Add feedback to context for next attempt
                Err(())
            }
        }
    }

    /// Process a single execution attempt
    ///
    /// # Errors
    /// Returns `Ok(Some(result))` if step completed successfully,
    /// `Ok(None)` if validation failed and should retry,
    /// `Err` if execution failed
    async fn process_step_attempt(
        params: &mut StepExecutionParams<'_>,
        context: &Context,
        attempt: &mut usize,
        start: Instant,
    ) -> Result<Option<StepResult>> {
        let response = Self::execute_with_agent(AgentExecutionParams {
            step: params.step,
            context,
            provider: params.provider,
            tool_registry: params.tool_registry,
            runtime: params.runtime,
            task_id: params.task_id,
            ui_channel: params.ui_channel,
        })
        .await?;

        merlin_deps::tracing::debug!(
            "Step '{}' returned {}",
            params.step.description,
            match &response {
                AgentResponse::DirectResult(_) => "DirectResult",
                AgentResponse::TaskList(_) => "TaskList",
            }
        );

        match response {
            AgentResponse::DirectResult(result) => {
                let validation = Self::validate_exit_requirement(
                    params.step.exit_requirement.as_ref(),
                    params.runtime,
                )
                .await;

                if Self::handle_validation_error(validation, &params.step.title, attempt).is_ok() {
                    Ok(Some(StepResult {
                        text: result,
                        duration_ms: start.elapsed().as_millis() as u64,
                        success: true,
                    }))
                } else {
                    Ok(None)
                }
            }

            AgentResponse::TaskList(task_list) => {
                let combined_result = Box::pin(super::parallel::execute_task_list_parallel(
                    &mut TaskListExecutionParams {
                        task_list: &task_list,
                        base_context: context,
                        provider: params.provider,
                        tool_registry: params.tool_registry,
                        runtime: params.runtime,
                        task_id: params.task_id,
                        ui_channel: params.ui_channel,
                        recursion_depth: params.recursion_depth + 1,
                        work_unit: None, // No tracking for nested decompositions
                    },
                ))
                .await?;

                Self::validate_exit_requirement(
                    params.step.exit_requirement.as_ref(),
                    params.runtime,
                )
                .await
                .map_err(|err| match err {
                    ValidationErrorType::Hard(msg) | ValidationErrorType::Soft(msg) => {
                        RoutingError::Other(format!("Task list result failed validation: {msg}"))
                    }
                })?;

                Ok(Some(combined_result))
            }
        }
    }

    /// Implementation of step execution
    ///
    /// # Errors
    /// Returns an error if max retries exceeded or critical failure occurs
    pub(super) async fn execute_step_impl(
        mut params: StepExecutionParams<'_>,
    ) -> Result<StepResult> {
        if params.recursion_depth >= MAX_RECURSION_DEPTH {
            return Err(RoutingError::Other(format!(
                "Maximum recursion depth ({MAX_RECURSION_DEPTH}) exceeded"
            )));
        }

        let start = Instant::now();
        let mut attempt = 0;

        loop {
            if attempt >= MAX_RETRY_ATTEMPTS {
                return Err(RoutingError::Other(format!(
                    "Step '{}' failed after {MAX_RETRY_ATTEMPTS} attempts",
                    params.step.title
                )));
            }

            let context = Self::build_context_from_spec(
                params.base_context,
                params.step.context.as_ref(),
                params.previous_results,
            );

            merlin_deps::tracing::debug!(
                "Executing step '{}' (attempt {})",
                params.step.description,
                attempt + 1
            );

            if let Some(result) =
                Self::process_step_attempt(&mut params, &context, &mut attempt, start).await?
            {
                return Ok(result);
            }
        }
    }

    /// Execute a task list with dependency-aware sequential execution
    ///
    /// # Errors
    /// Returns an error if any step fails
    pub async fn execute_task_list(mut params: TaskListExecutionParams<'_>) -> Result<StepResult> {
        super::parallel::execute_task_list_parallel(&mut params).await
    }

    /// Execute task with agent and parse response
    ///
    /// # Errors
    /// Returns an error if execution or parsing fails
    pub(crate) async fn execute_with_agent(
        params: AgentExecutionParams<'_>,
    ) -> Result<AgentResponse> {
        let span = span!(Level::INFO, "execute_with_agent", task_id = ?params.task_id);

        async move {
            let description = params.step.description.as_str();
            let context = params.context;
            let provider = params.provider;
            let runtime = params.runtime;
            let task_id = params.task_id;
            let ui_channel = params.ui_channel;

            let query = Query::new(description.to_owned());

            // Execute query with provider
            let response = provider
                .generate(&query, context)
                .await
                .map_err(|err| RoutingError::Other(format!("Provider error: {err}")))?;

            merlin_deps::tracing::debug!("Agent response: {}", response.text);

            // Extract and execute TypeScript code
            let typescript_code = extract_typescript_code(&response.text).ok_or_else(|| {
                RoutingError::Other(format!(
                    "No TypeScript code found in response: {}",
                    response.text
                ))
            })?;

            // Execute TypeScript code - returns AgentResponse (String | TaskList) directly
            let result =
                execute_typescript_code(runtime, task_id, &typescript_code, ui_channel).await?;

            Ok(result)
        }
        .instrument(span)
        .await
    }

    /// Add previous step results to context
    fn add_previous_step_results(
        mut context: Context,
        step_indices: &[usize],
        previous_results: &[StepResult],
    ) -> Context {
        for &index in step_indices {
            if let Some(result) = previous_results.get(index) {
                let prev_step_result =
                    format!("Result from previous step {index}: {}", result.text);
                context = context.with_additional_content(&prev_step_result);
            }
        }
        context
    }

    /// Build context from specification
    fn build_context_from_spec(
        base_context: &Context,
        spec: Option<&ContextSpec>,
        previous_results: &[StepResult],
    ) -> Context {
        let mut context = base_context.clone();

        if let Some(spec) = spec {
            // Add file patterns to context
            if let Some(ref files) = spec.files {
                for file_pattern in files {
                    // TODO: Implement file pattern matching and addition
                    merlin_deps::tracing::debug!("Adding files matching: {}", file_pattern.pattern);
                }
            }

            // Add previous step results
            if let Some(ref step_indices) = spec.previous_steps {
                context = Self::add_previous_step_results(context, step_indices, previous_results);
            }

            // Add explicit content
            if let Some(ref content) = spec.explicit_content {
                context = context.with_additional_content(content);
            }
        }

        context
    }

    /// Validate exit requirement for a step result
    ///
    /// # Errors
    /// Returns `ValidationErrorType` if validation fails
    async fn validate_exit_requirement(
        requirement: Option<&JsValueHandle>,
        runtime: &mut PersistentTypeScriptRuntime,
    ) -> result::Result<(), ValidationErrorType> {
        // If no exit requirement specified, validation passes
        let Some(func_handle) = requirement else {
            return Ok(());
        };

        merlin_deps::tracing::debug!(
            "Validating exit requirement with function handle: {}",
            func_handle.id()
        );

        // Convert core handle to tooling handle
        let tooling_handle = ToolingJsValueHandle::new(func_handle.id().to_owned());

        // Call the stored JavaScript function directly
        match runtime.call_function(tooling_handle).await {
            Ok(_result_handle) => {
                merlin_deps::tracing::debug!("Exit requirement validation passed");
                Ok(())
            }
            Err(err) => {
                merlin_deps::tracing::debug!("Exit requirement validation failed: {err}");
                Err(ValidationErrorType::Soft(format!(
                    "Exit requirement validation failed: {err}"
                )))
            }
        }
    }
}
