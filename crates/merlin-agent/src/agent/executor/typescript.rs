//! TypeScript code extraction and execution

use merlin_core::{
    AgentResponse, JsValueHandle as CoreJsValueHandle, Result, RoutingError, StepType, TaskId,
    TaskList, TaskStep, ui::UiEvent,
};
use merlin_deps::serde_json::to_string;
use merlin_routing::UiChannel;
use merlin_tooling::{PersistentTypeScriptRuntime, ToolingJsValueHandle};

/// Convert from tooling `JsValueHandle` to core `JsValueHandle`
fn to_core_handle(handle: &ToolingJsValueHandle) -> CoreJsValueHandle {
    CoreJsValueHandle::new(handle.id().to_owned())
}

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
    runtime: &PersistentTypeScriptRuntime,
    task_id: TaskId,
    code: &str,
    ui_channel: &UiChannel,
) -> Result<AgentResponse> {
    // Send step started event
    ui_channel.send(UiEvent::TaskStepStarted {
        task_id,
        step_id: "typescript_execution".to_owned(),
        step_type: "tool_call".to_owned(),
        content: "Executing TypeScript code".to_owned(),
    });

    // Execute code
    merlin_deps::tracing::debug!("Executing TypeScript code:\n{}", code);
    let result_handle = runtime.execute(code).await.map_err(|err| {
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
    })?;

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

/// Parse agent result from JavaScript handle as String or `TaskList`
///
/// # Errors
/// Returns error if property extraction fails or `TaskList` is malformed
async fn parse_agent_response_from_handle(
    runtime: &PersistentTypeScriptRuntime,
    handle: ToolingJsValueHandle,
) -> Result<AgentResponse> {
    // Try to extract TaskList properties
    // If value is not an object, get_property will fail - in that case, it's a DirectResult
    let title_result = runtime
        .get_property(handle.clone(), "title".to_owned())
        .await;

    let steps_result = runtime
        .get_property(handle.clone(), "steps".to_owned())
        .await;

    // If we can't get properties (not an object), it's definitely a DirectResult
    let (Ok(title_handle), Ok(steps_array_handle)) = (title_result, steps_result) else {
        // Not an object, convert to JSON string for DirectResult
        let json_value = runtime
            .to_json(handle)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to convert to JSON: {err}")))?;
        let json_string = to_string(&json_value)
            .map_err(|err| RoutingError::Other(format!("Failed to serialize JSON: {err}")))?;
        merlin_deps::tracing::debug!("Agent response is DirectResult (not an object)");
        return Ok(AgentResponse::DirectResult(json_string));
    };

    // Check if title/steps are nullish (undefined/null)
    let title_is_nullish = runtime
        .is_nullish(title_handle.clone())
        .await
        .unwrap_or(true);
    let steps_is_nullish = runtime
        .is_nullish(steps_array_handle.clone())
        .await
        .unwrap_or(true);

    if title_is_nullish || steps_is_nullish {
        // Not a TaskList, convert to JSON string for DirectResult
        let json_value = runtime
            .to_json(handle)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to convert to JSON: {err}")))?;
        let json_string = to_string(&json_value)
            .map_err(|err| RoutingError::Other(format!("Failed to serialize JSON: {err}")))?;
        merlin_deps::tracing::debug!("Agent response is DirectResult (no title/steps)");
        return Ok(AgentResponse::DirectResult(json_string));
    }

    // Extract title
    let title = runtime
        .get_string(title_handle)
        .await
        .map_err(|err| RoutingError::Other(format!("Failed to get title string: {err}")))?;

    // Extract steps array
    let steps_len = runtime
        .get_array_length(steps_array_handle.clone())
        .await
        .map_err(|err| RoutingError::Other(format!("Failed to get steps array length: {err}")))?;

    let mut steps = Vec::with_capacity(steps_len);
    for idx in 0..steps_len {
        let element_handle = runtime
            .get_array_element(steps_array_handle.clone(), idx)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to get step {idx}: {err}")))?;

        let step = extract_task_step(runtime, element_handle).await?;
        steps.push(step);
    }

    merlin_deps::tracing::debug!(
        "Parsed agent response as TaskList with {} steps",
        steps.len()
    );

    Ok(AgentResponse::TaskList(TaskList { title, steps }))
}

/// Extract a `TaskStep` from a JavaScript object handle
///
/// # Errors
/// Returns error if required properties are missing or malformed
async fn extract_task_step(
    runtime: &PersistentTypeScriptRuntime,
    handle: ToolingJsValueHandle,
) -> Result<TaskStep> {
    // Extract required fields
    let title_handle = runtime
        .get_property(handle.clone(), "title".to_owned())
        .await
        .map_err(|err| RoutingError::Other(format!("Step missing title: {err}")))?;

    let title = runtime
        .get_string(title_handle)
        .await
        .map_err(|err| RoutingError::Other(format!("Failed to get step title: {err}")))?;

    let description_handle = runtime
        .get_property(handle.clone(), "description".to_owned())
        .await
        .map_err(|err| RoutingError::Other(format!("Step missing description: {err}")))?;

    let description = runtime
        .get_string(description_handle)
        .await
        .map_err(|err| RoutingError::Other(format!("Failed to get step description: {err}")))?;

    let step_type_handle = runtime
        .get_property(handle.clone(), "step_type".to_owned())
        .await
        .map_err(|err| RoutingError::Other(format!("Step missing step_type: {err}")))?;

    let step_type_str = runtime
        .get_string(step_type_handle)
        .await
        .map_err(|err| RoutingError::Other(format!("Failed to get step_type: {err}")))?;

    let step_type = parse_step_type(&step_type_str)?;

    // Extract optional exit_requirement (function handle)
    let exit_requirement = if let Ok(req_handle) = runtime
        .get_property(handle.clone(), "exit_requirement".to_owned())
        .await
    {
        // Check if nullish
        if runtime.is_nullish(req_handle.clone()).await.unwrap_or(true) {
            None
        } else {
            // Convert tooling handle to core handle and keep the function reference alive
            Some(to_core_handle(&req_handle))
        }
    } else {
        None
    };

    // Extract optional dependencies
    let dependencies = extract_dependencies(runtime, handle.clone())
        .await
        .unwrap_or_default();

    Ok(TaskStep {
        title,
        description,
        step_type,
        exit_requirement,
        context: None, // Not currently supported in extraction
        dependencies,
    })
}

/// Extract dependencies array from step
///
/// # Errors
/// Returns error if dependencies array is malformed
async fn extract_dependencies(
    runtime: &PersistentTypeScriptRuntime,
    handle: ToolingJsValueHandle,
) -> Result<Vec<String>> {
    let deps_array_handle = runtime
        .get_property(handle, "dependencies".to_owned())
        .await
        .map_err(|err| RoutingError::Other(format!("Failed to get dependencies: {err}")))?;

    if runtime
        .is_nullish(deps_array_handle.clone())
        .await
        .unwrap_or(true)
    {
        return Ok(Vec::new());
    }

    let deps_len = runtime
        .get_array_length(deps_array_handle.clone())
        .await
        .map_err(|err| RoutingError::Other(format!("Failed to get dependencies length: {err}")))?;
    let mut dependencies = Vec::with_capacity(deps_len);

    for idx in 0..deps_len {
        let element_handle = runtime
            .get_array_element(deps_array_handle.clone(), idx)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to get dependency {idx}: {err}")))?;
        let dep_string = runtime.get_string(element_handle).await.map_err(|err| {
            RoutingError::Other(format!("Failed to get dependency {idx} string: {err}"))
        })?;
        dependencies.push(dep_string);
    }

    Ok(dependencies)
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
