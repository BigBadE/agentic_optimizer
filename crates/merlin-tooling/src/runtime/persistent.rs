//! Persistent TypeScript runtime with long-lived Boa context using `LocalSet`

use std::collections::HashMap;
use std::sync::Arc;

use merlin_deps::boa_engine::{Context, JsValue, JsValue as BoaJsValue, Source};
use merlin_deps::serde_json::Value;
use merlin_deps::uuid::Uuid;
use tokio::task::LocalSet;

use super::bulk_extraction::{self, ExtractedTaskList};
use super::conversion::js_value_to_json_static;
use super::handle::JsValueHandle;
use super::tool_registration::register_tool_functions;
use crate::{Tool, ToolError, ToolResult};

/// Persistent TypeScript runtime with long-lived Boa context
///
/// Uses Tokio's `LocalSet` to run `!Send` Boa `Context` in async context,
/// eliminating IPC overhead while maintaining persistent state.
pub struct PersistentTypeScriptRuntime {
    /// The Boa JavaScript context
    context: Context,
    /// Storage for JavaScript values referenced by handles
    value_storage: HashMap<String, JsValue>,
    /// `LocalSet` for running `!Send` futures
    local_set: LocalSet,
}

impl PersistentTypeScriptRuntime {
    /// Create a new persistent runtime
    ///
    /// The context is created immediately and persists for the runtime's lifetime.
    /// Tools are registered and available to all executed code.
    ///
    /// # Errors
    /// Returns error if context creation or tool registration fails
    pub fn new(tools: &HashMap<String, Arc<dyn Tool>>) -> ToolResult<Self> {
        // Create context
        let mut context = Context::default();

        // Register tools
        register_tool_functions(&mut context, tools)?;

        Ok(Self {
            context,
            value_storage: HashMap::new(),
            local_set: LocalSet::new(),
        })
    }

    /// Execute JavaScript code and return a handle to the result
    ///
    /// The result is stored in the runtime and can be accessed via the returned handle.
    ///
    /// # Errors
    /// Returns error if code execution fails
    pub async fn execute(&mut self, code: &str) -> ToolResult<JsValueHandle> {
        let code = code.to_owned();

        // Run in LocalSet to allow !Send Context
        self.local_set
            .run_until(async {
                // Wrap code for execution
                let wrapped_code = super::typescript::wrap_code(&code);

                // Execute code
                let result = self
                    .context
                    .eval(Source::from_bytes(&wrapped_code))
                    .map_err(|err| {
                        ToolError::ExecutionFailed(format!("JavaScript error: {err}"))
                    })?;

                // Run jobs (synchronous - tools block)
                drop(self.context.run_jobs());

                // Extract promise if needed
                let final_result =
                    super::promise::extract_promise_if_needed(result, &mut self.context)?;

                // Store result with handle
                let handle_id = Uuid::new_v4().to_string();
                self.value_storage.insert(handle_id.clone(), final_result);

                Ok(JsValueHandle::new(handle_id))
            })
            .await
    }

    /// Extract complete `TaskList` from a handle in one operation
    ///
    /// This performs bulk extraction to minimize overhead.
    ///
    /// # Errors
    /// Returns error if handle is invalid or extraction fails
    pub async fn extract_task_list(
        &mut self,
        handle: &JsValueHandle,
    ) -> ToolResult<Option<ExtractedTaskList>> {
        let handle = handle.clone();

        self.local_set
            .run_until(async {
                bulk_extraction::extract_task_list(
                    &handle,
                    &mut self.context,
                    &mut self.value_storage,
                )
            })
            .await
    }

    /// Convert handle to JSON (for `DirectResult` case)
    ///
    /// # Errors
    /// Returns error if handle is invalid or conversion fails
    pub async fn to_json(&mut self, handle: JsValueHandle) -> ToolResult<Value> {
        self.local_set
            .run_until(async {
                let value = self.value_storage.get(handle.id()).ok_or_else(|| {
                    ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id()))
                })?;

                js_value_to_json_static(value, &mut self.context).map_err(|err| {
                    ToolError::ExecutionFailed(format!("Failed to convert to JSON: {err}"))
                })
            })
            .await
    }

    /// Call a function stored in a handle (for exit requirements)
    ///
    /// # Errors
    /// Returns error if handle is invalid, value is not callable, or call fails
    pub async fn call_function(&mut self, handle: JsValueHandle) -> ToolResult<JsValueHandle> {
        self.local_set
            .run_until(async {
                let value = self.value_storage.get(handle.id()).ok_or_else(|| {
                    ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id()))
                })?;

                let callable = value.as_callable().ok_or_else(|| {
                    ToolError::ExecutionFailed("Value is not callable".to_owned())
                })?;

                // Call with no arguments, undefined as this
                let result = callable
                    .call(&BoaJsValue::undefined(), &[], &mut self.context)
                    .map_err(|err| {
                        ToolError::ExecutionFailed(format!("Function call failed: {err}"))
                    })?;

                // Run jobs to resolve Promises
                drop(self.context.run_jobs());

                // Extract Promise value if needed
                let final_result =
                    super::promise::extract_promise_if_needed(result, &mut self.context)?;

                // Store result
                let handle_id = Uuid::new_v4().to_string();
                self.value_storage.insert(handle_id.clone(), final_result);

                Ok(JsValueHandle::new(handle_id))
            })
            .await
    }
}
