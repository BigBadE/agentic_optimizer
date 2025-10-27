//! TypeScript code extraction and execution

use crate::agent::AgentExecutionResult;
use merlin_core::{Result, RoutingError, TaskId, ui::UiEvent};
use merlin_routing::UiChannel;
use merlin_tooling::{ToolRegistry, TypeScriptRuntime};
use serde_json::from_value;
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
/// # Errors
/// Returns an error if code execution fails or result parsing fails
pub async fn execute_typescript_code(
    tool_registry: &Arc<ToolRegistry>,
    task_id: TaskId,
    code: &str,
    ui_channel: &UiChannel,
) -> Result<AgentExecutionResult> {
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
