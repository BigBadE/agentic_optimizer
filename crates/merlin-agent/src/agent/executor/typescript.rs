//! TypeScript code extraction and execution

use merlin_core::{AgentResponse, Result, RoutingError, TaskId, TaskList, ui::UiEvent};
use merlin_deps::serde_json::{from_str, to_string as json_to_string};
use merlin_routing::UiChannel;
use merlin_tooling::{ToolRegistry, TypeScriptRuntime};
use std::sync::Arc;

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

/// Execute TypeScript code using the TypeScript runtime
///
/// Returns either a String (task completed) or a `TaskList` (decompose into subtasks)
///
/// # Errors
/// Returns an error if code execution fails or result parsing fails
pub async fn execute_typescript_code(
    tool_registry: &Arc<ToolRegistry>,
    task_id: TaskId,
    code: &str,
    ui_channel: &UiChannel,
) -> Result<AgentResponse> {
    // Create TypeScript runtime and register tools
    let mut runtime = TypeScriptRuntime::new();

    // Get all tool names and then get Arc clones
    let tool_names: Vec<String> = tool_registry
        .list_tools()
        .iter()
        .map(|tool| tool.name().to_owned())
        .collect();

    for tool_name in tool_names {
        if let Some(tool) = tool_registry.get_tool(&tool_name) {
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
    merlin_deps::tracing::debug!("Executing TypeScript code:\n{}", code);
    let result_value = runtime.execute(code).await.map_err(|err| {
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

    // Parse result as String or TaskList
    let result_str = if result_value.is_string() {
        // Plain string result
        result_value.as_str().unwrap_or("").to_owned()
    } else {
        // Serialize to JSON for parsing
        json_to_string(&result_value)
            .map_err(|err| RoutingError::Other(format!("Failed to serialize result: {err}")))?
    };

    // Try parsing as TaskList, otherwise treat as String result
    let agent_response = parse_agent_response(&result_str)?;

    // Send step completed event
    ui_channel.send(UiEvent::TaskStepCompleted {
        task_id,
        step_id: "typescript_execution".to_owned(),
    });

    merlin_deps::tracing::debug!("TypeScript execution result: {:?}", agent_response);

    Ok(agent_response)
}

/// Parse agent result as String or `TaskList`
///
/// # Errors
/// Returns error if result looks like a `TaskList` but fails to parse
fn parse_agent_response(result: &str) -> Result<AgentResponse> {
    let trimmed = result.trim();

    // Check if result looks like a TaskList (has both "title" and "steps" fields)
    let looks_like_tasklist = trimmed.contains("\"title\"") && trimmed.contains("\"steps\"");

    // Try parsing as TaskList if it looks like one
    match from_str::<TaskList>(trimmed) {
        Ok(task_list) => {
            merlin_deps::tracing::debug!("Parsed agent response as TaskList");
            Ok(AgentResponse::TaskList(task_list))
        }
        Err(err) if looks_like_tasklist => {
            // If it looks like a TaskList but failed to parse, that's an error
            merlin_deps::tracing::error!(
                "Failed to parse TaskList: {}\nJSON was:\n{}",
                err,
                result
            );
            Err(RoutingError::Other(format!(
                "Result looks like TaskList but failed to parse: {err}"
            )))
        }
        Err(_) => {
            // Doesn't look like a TaskList, treat as direct string result
            merlin_deps::tracing::debug!("Treating agent response as DirectResult");
            Ok(AgentResponse::DirectResult(result.to_owned()))
        }
    }
}
