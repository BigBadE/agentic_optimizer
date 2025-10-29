//! Step executor for recursive task decomposition with exit requirements

use std::{future::Future, path::Path, pin::Pin, result, sync::Arc, time::Instant};

use merlin_core::{
    AgentResponse, Context, ContextSpec, ExitRequirement, ModelProvider, Query, Result,
    RoutingError, TaskId, TaskList, TaskStep, ValidationErrorType,
};
use merlin_deps::serde_json::from_str;
use merlin_routing::UiChannel;
use merlin_tooling::ToolRegistry;

use super::typescript::{execute_typescript_code, extract_typescript_code};
use crate::{AgentExecutionResult, ExitRequirementValidators};

/// Future returned by async step execution
type StepFuture<'fut> = Pin<Box<dyn Future<Output = Result<StepResult>> + Send + 'fut>>;

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
    /// Task ID for UI events
    pub task_id: TaskId,
    /// UI channel
    pub ui_channel: &'params UiChannel,
    /// Current recursion depth
    pub recursion_depth: usize,
}

/// Step executor handles recursive task decomposition and validation
pub struct StepExecutor;

impl StepExecutor {
    /// Execute a single step with retry logic and exit requirement validation
    ///
    /// # Errors
    /// Returns an error if max retries exceeded or critical failure occurs
    pub fn execute_step(params: StepExecutionParams<'_>) -> StepFuture<'_> {
        Box::pin(async move { Self::execute_step_impl(params).await })
    }

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
        params: &StepExecutionParams<'_>,
        context: &Context,
        attempt: &mut usize,
        start: Instant,
    ) -> Result<Option<StepResult>> {
        let response = Self::execute_with_agent(
            &params.step.description,
            context,
            params.provider,
            params.tool_registry,
            params.task_id,
            params.ui_channel,
        )
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
                    &params.step.exit_requirement,
                    &result,
                    params.tool_registry,
                );

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
                let combined_result = Self::execute_task_list_impl(
                    &task_list,
                    context,
                    params.provider,
                    params.tool_registry,
                    params.task_id,
                    params.ui_channel,
                    params.recursion_depth + 1,
                )
                .await?;

                Self::validate_exit_requirement(
                    &params.step.exit_requirement,
                    &combined_result.text,
                    params.tool_registry,
                )
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
    async fn execute_step_impl(params: StepExecutionParams<'_>) -> Result<StepResult> {
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
                Self::process_step_attempt(&params, &context, &mut attempt, start).await?
            {
                return Ok(result);
            }
        }
    }

    /// Execute a task list by running all steps sequentially
    ///
    /// # Errors
    /// Returns an error if any step fails
    #[allow(clippy::too_many_arguments, reason = "Internal helper function")]
    pub fn execute_task_list<'list>(
        task_list: &'list TaskList,
        base_context: &'list Context,
        provider: &'list Arc<dyn ModelProvider>,
        tool_registry: &'list Arc<ToolRegistry>,
        task_id: TaskId,
        ui_channel: &'list UiChannel,
        recursion_depth: usize,
    ) -> StepFuture<'list> {
        Box::pin(async move {
            Self::execute_task_list_impl(
                task_list,
                base_context,
                provider,
                tool_registry,
                task_id,
                ui_channel,
                recursion_depth,
            )
            .await
        })
    }

    /// Implementation of task list execution
    ///
    /// # Errors
    /// Returns an error if any step fails
    #[allow(clippy::too_many_arguments, reason = "Internal helper function")]
    fn execute_task_list_impl<'list>(
        task_list: &'list TaskList,
        base_context: &'list Context,
        provider: &'list Arc<dyn ModelProvider>,
        tool_registry: &'list Arc<ToolRegistry>,
        task_id: TaskId,
        ui_channel: &'list UiChannel,
        recursion_depth: usize,
    ) -> StepFuture<'list> {
        Box::pin(async move {
            let start = Instant::now();
            let mut step_results = Vec::new();
            let mut combined_output = Vec::new();

            merlin_deps::tracing::debug!(
                "Executing task list '{}' with {} steps at depth {}",
                task_list.title,
                task_list.steps.len(),
                recursion_depth
            );

            for (index, step) in task_list.steps.iter().enumerate() {
                merlin_deps::tracing::debug!(
                    "Executing step {}/{}: {}",
                    index + 1,
                    task_list.steps.len(),
                    step.title
                );

                let step_result = Self::execute_step_impl(StepExecutionParams {
                    step,
                    base_context,
                    previous_results: &step_results,
                    provider,
                    tool_registry,
                    task_id,
                    ui_channel,
                    recursion_depth,
                })
                .await?;

                combined_output.push(format!(
                    "## Step {}: {}\n{}",
                    index + 1,
                    step.title,
                    step_result.text
                ));
                step_results.push(step_result);
            }

            Ok(StepResult {
                text: combined_output.join("\n\n"),
                duration_ms: start.elapsed().as_millis() as u64,
                success: true,
            })
        })
    }

    /// Execute task with agent and parse response
    ///
    /// # Errors
    /// Returns an error if execution or parsing fails
    #[allow(
        clippy::too_many_arguments,
        reason = "Helper function needs all parameters"
    )]
    pub async fn execute_with_agent(
        description: &str,
        context: &Context,
        provider: &Arc<dyn ModelProvider>,
        tool_registry: &Arc<ToolRegistry>,
        task_id: TaskId,
        ui_channel: &UiChannel,
    ) -> Result<AgentResponse> {
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

        let execution_result: AgentExecutionResult =
            execute_typescript_code(tool_registry, task_id, &typescript_code, ui_channel).await?;

        let result_str = execution_result
            .get_result()
            .ok_or_else(|| {
                RoutingError::Other("Agent must return a result, not a continuation".to_owned())
            })?
            .to_owned();

        // Try parsing as AgentResponse (String or TaskList)
        Ok(Self::parse_agent_response(&result_str))
    }

    /// Parse agent result as String or `TaskList`
    fn parse_agent_response(result: &str) -> AgentResponse {
        // Try parsing as JSON first (could be TaskList)
        if let Ok(task_list) = from_str::<TaskList>(result) {
            merlin_deps::tracing::debug!("Parsed agent response as TaskList");
            return AgentResponse::TaskList(task_list);
        }

        merlin_deps::tracing::debug!("Treating agent response as DirectResult");
        // Otherwise treat as direct string result
        AgentResponse::DirectResult(result.to_owned())
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
    fn validate_exit_requirement(
        requirement: &ExitRequirement,
        result: &str,
        _tool_registry: &Arc<ToolRegistry>,
    ) -> result::Result<(), ValidationErrorType> {
        // TODO: Pass workspace root properly
        let workspace_root = Path::new(".");
        ExitRequirementValidators::validate(requirement, result, workspace_root)
    }
}
