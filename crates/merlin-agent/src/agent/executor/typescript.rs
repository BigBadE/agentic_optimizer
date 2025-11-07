//! TypeScript code extraction and execution

use merlin_core::{
    AgentResponse, JsValueHandle as CoreJsValueHandle, Result, RoutingError, StepType, TaskId,
    TaskList, TaskStep, ui::UiEvent,
};
use merlin_deps::serde_json::to_string;
use merlin_deps::tracing::{Level, span};
use merlin_routing::UiChannel;
use merlin_tooling::bulk_extraction::ExtractedTaskStep;
use merlin_tooling::{PersistentTypeScriptRuntime, ToolingJsValueHandle};
use tracing_futures::Instrument as _;

/// Extract TypeScript code from markdown code blocks
///
/// Looks for ```typescript or ```ts blocks and returns the concatenated code
pub fn extract_typescript_code(text: &str) -> Option<String> {
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

/// Execute TypeScript code using the persistent TypeScript runtime
///
/// Returns either a String (task completed) or a `TaskList` (decompose into subtasks)
///
/// # Errors
/// Returns an error if code execution fails or result parsing fails
pub async fn execute_typescript_code(
    runtime: &mut PersistentTypeScriptRuntime,
    task_id: TaskId,
    code: &str,
    ui_channel: &UiChannel,
) -> Result<AgentResponse> {
    let span = span!(Level::INFO, "execute_typescript_code", task_id = ?task_id);

    async move {
        // Send step started event
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "typescript_execution".to_owned(),
            step_type: "tool_call".to_owned(),
            content: "Executing TypeScript code".to_owned(),
        });

        // Execute code with detailed timing
        merlin_deps::tracing::debug!("Executing TypeScript code:\n{}", code);
        let result_handle = {
            let exec_span = span!(Level::INFO, "typescript_runtime_execute");
            runtime
                .execute(code)
                .instrument(exec_span)
                .await
                .map_err(|err| {
                    merlin_deps::tracing::info!(
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
                })?
        };

        // Parse result as String or TaskList by checking JavaScript object properties
        let agent_response = parse_agent_response_from_handle(runtime, result_handle).await?;

        // Send step completed event
        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "typescript_execution".to_owned(),
        });

        merlin_deps::tracing::debug!("TypeScript execution result: {:?}", agent_response);

        Ok(agent_response)
    }
    .instrument(span)
    .await
}

/// Parse agent result from JavaScript handle as String or `TaskList`
///
/// Uses bulk extraction to minimize overhead - single operation instead of ~50-60 IPC calls
///
/// # Errors
/// Returns error if extraction fails or `TaskList` is malformed
async fn parse_agent_response_from_handle(
    runtime: &mut PersistentTypeScriptRuntime,
    handle: ToolingJsValueHandle,
) -> Result<AgentResponse> {
    let span = span!(Level::INFO, "parse_agent_response_from_handle");

    async move {
        // Try bulk extraction first - single operation
        let extracted = runtime
            .extract_task_list(&handle)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to extract TaskList: {err}")))?;

        if let Some(task_list) = extracted {
            // Convert extracted task list to core types
            let steps = convert_extracted_steps(task_list.steps)?;

            merlin_deps::tracing::debug!(
                "Parsed agent response as TaskList with {} steps",
                steps.len()
            );
            Ok(AgentResponse::TaskList(TaskList {
                title: task_list.title,
                steps,
            }))
        } else {
            // Not a TaskList - convert to JSON string (DirectResult)
            let json_value = runtime
                .to_json(handle)
                .await
                .map_err(|err| RoutingError::Other(format!("Failed to convert to JSON: {err}")))?;
            let json_string = to_string(&json_value)
                .map_err(|err| RoutingError::Other(format!("Failed to serialize JSON: {err}")))?;
            merlin_deps::tracing::debug!("Agent response is DirectResult");
            Ok(AgentResponse::DirectResult(json_string))
        }
    }
    .instrument(span)
    .await
}

/// Convert extracted steps to core `TaskStep` types
///
/// # Errors
/// Returns error if `step_type` is invalid
fn convert_extracted_steps(extracted_steps: Vec<ExtractedTaskStep>) -> Result<Vec<TaskStep>> {
    let mut steps = Vec::with_capacity(extracted_steps.len());

    for extracted in extracted_steps {
        let step_type = parse_step_type(&extracted.step_type)?;

        // Convert exit_requirement handle ID to core handle
        let exit_requirement = extracted.exit_requirement.map(CoreJsValueHandle::new);

        steps.push(TaskStep {
            title: extracted.title,
            description: extracted.description,
            step_type,
            exit_requirement,
            context: None, // Not currently supported
            dependencies: extracted.dependencies,
        });
    }

    Ok(steps)
}

/// Parse step type string
///
/// # Errors
/// Returns error if step type is invalid
fn parse_step_type(step_type: &str) -> Result<StepType> {
    match step_type.to_lowercase().as_str() {
        "research" => Ok(StepType::Research),
        "planning" => Ok(StepType::Planning),
        "implementation" => Ok(StepType::Implementation),
        "validation" => Ok(StepType::Validation),
        "documentation" => Ok(StepType::Documentation),
        _ => Err(RoutingError::Other(format!(
            "Invalid step_type: {step_type}"
        ))),
    }
}
