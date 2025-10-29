//! Tool registration and execution within JavaScript context.
//!
//! Tools are registered as native JavaScript functions that return Promises.
//! Failed tool executions (success=false) return resolved Promises with their
//! data object, NOT rejected Promises. This allows TypeScript code to inspect
//! exit codes, error messages, and other failure details without try/catch.

use std::collections::HashMap;
use std::sync::Arc;
use std::thread::scope;

use merlin_deps::boa_engine::JsNativeError;
use merlin_deps::boa_engine::{Context, JsResult, JsValue, NativeFunction};
use merlin_deps::serde_json::Value;
use tokio::runtime::Builder;

use super::conversion::{js_value_to_json_static, json_to_js_value_static};
use crate::{Tool, ToolInput, ToolOutput, ToolResult};

/// Register tool functions in the JavaScript context
///
/// # Errors
/// Returns error if registration fails
#[allow(
    clippy::too_many_lines,
    reason = "Tool registration requires handling each tool type"
)]
pub fn register_tool_functions(
    context: &mut Context,
    tools: &HashMap<String, Arc<dyn Tool>>,
) -> ToolResult<()> {
    for (name, tool) in tools {
        let tool_clone = Arc::clone(tool);

        #[allow(
            unsafe_code,
            reason = "Arc<dyn Tool> is not Trace, but safe to use as documented above"
        )]
        let func =
            // Create tool function
            // SAFETY: Arc<dyn Tool> is not Trace, but it's safe to use here because:
            // 1. The tool registry is owned by TypeScriptRuntime which outlives the Context
            // 2. Tools are immutable and thread-safe (Arc)
            // 3. The closure only captures Arc which is safe to share
            unsafe {
            NativeFunction::from_closure(move |_this, args, ctx| {
                merlin_deps::tracing::debug!("Tool '{}' called from JavaScript", tool_clone.name());

                // Get parameters - handle both object and positional argument patterns
                let params = if args.is_empty() {
                    merlin_deps::serde_json::json!({})
                } else if args.len() == 1 {
                    // Single argument - could be object or simple value
                    js_value_to_json_static(&args[0], ctx)?
                } else {
                    // Multiple arguments - convert to named params based on tool
                    convert_positional_args(&tool_clone, args, ctx)?
                };

                // Create tool input
                let input = ToolInput { params };

                // Execute tool synchronously using thread::scope
                let tool_clone_inner = Arc::clone(&tool_clone);
                let result = scope(|scope_ctx| {
                    scope_ctx
                        .spawn(move || -> Result<ToolOutput, String> {
                            // Create a new Tokio runtime for this tool execution
                            let runtime = Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .map_err(|err| format!("Failed to create runtime: {err}"))?;

                            runtime.block_on(async move {
                                tool_clone_inner
                                    .execute(input)
                                    .await
                                    .map_err(|err| format!("Tool execution failed: {err}"))
                            })
                        })
                        .join()
                        .map_err(|_| "Tool execution panicked".to_owned())?
                })
                .map_err(|err: String| JsNativeError::error().with_message(err))?;

                // Convert result to JS value
                // Always return the data, never reject the Promise
                // Tools that fail (success=false) still return their data object
                // so TypeScript can inspect exit codes, error messages, etc.
                let data = result.data.unwrap_or(Value::String(result.message));
                json_to_js_value_static(&data, ctx)
            })
        };

        context
            .register_global_callable(boa_engine::js_string!(name.as_str()), 0, func)
            .map_err(|err| {
                use crate::ToolError;
                ToolError::ExecutionFailed(format!("Failed to register tool '{name}': {err}"))
            })?;
    }

    Ok(())
}

/// Convert positional arguments to named parameters based on tool type
///
/// # Errors
/// Returns error if conversion fails
fn convert_positional_args(
    tool: &Arc<dyn Tool>,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<Value> {
    match tool.name() {
        "requestContext" => {
            // requestContext(pattern, reason, max_files?)
            let pattern = js_value_to_json_static(&args[0], ctx)?;
            let reason = if args.len() > 1 {
                js_value_to_json_static(&args[1], ctx)?
            } else {
                merlin_deps::serde_json::json!("")
            };
            let max_files = if args.len() > 2 {
                js_value_to_json_static(&args[2], ctx)?
            } else {
                merlin_deps::serde_json::json!(5) // Default max_files
            };

            Ok(merlin_deps::serde_json::json!({
                "pattern": pattern,
                "reason": reason,
                "max_files": max_files
            }))
        }
        "writeFile" => {
            // writeFile(path, content)
            if args.len() < 2 {
                return Err(JsNativeError::error()
                    .with_message("writeFile requires 2 arguments: path and content")
                    .into());
            }
            let path = js_value_to_json_static(&args[0], ctx)?;
            let file_content = js_value_to_json_static(&args[1], ctx)?;

            Ok(merlin_deps::serde_json::json!({
                "path": path,
                "content": file_content
            }))
        }
        "editFile" => {
            // editFile(path, old_string, new_string, options?)
            if args.len() < 3 {
                return Err(JsNativeError::error()
                    .with_message(
                        "editFile requires at least 3 arguments: path, old_string, new_string",
                    )
                    .into());
            }
            let path = js_value_to_json_static(&args[0], ctx)?;
            let old_string = js_value_to_json_static(&args[1], ctx)?;
            let new_string = js_value_to_json_static(&args[2], ctx)?;

            let replace_all = if args.len() > 3 {
                // Check if 4th arg is object with replace_all property
                let opts = js_value_to_json_static(&args[3], ctx)?;
                opts.get("replace_all")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            } else {
                false
            };

            Ok(merlin_deps::serde_json::json!({
                "path": path,
                "old_string": old_string,
                "new_string": new_string,
                "replace_all": replace_all
            }))
        }
        _ => {
            // For other tools, take first argument as params
            js_value_to_json_static(&args[0], ctx)
        }
    }
}
