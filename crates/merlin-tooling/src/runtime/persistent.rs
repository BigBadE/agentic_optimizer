//! Persistent TypeScript runtime with long-lived Boa context using `LocalSet`

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use boa_engine::{Context, JsValue, JsValue as BoaJsValue, Source};
use serde_json::Value;
use tokio::task::LocalSet;
use uuid::Uuid;

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
    /// Cache for wrapped code (input code -> wrapped JavaScript)
    code_cache: HashMap<String, String>,
    /// Pre-generated UUID pool for handle generation
    uuid_pool: VecDeque<String>,
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

        // Pre-generate UUID pool (batch of 100)
        let uuid_pool: VecDeque<String> = (0..100).map(|_| Uuid::new_v4().to_string()).collect();

        Ok(Self {
            context,
            value_storage: HashMap::new(),
            local_set: LocalSet::new(),
            code_cache: HashMap::new(),
            uuid_pool,
        })
    }

    /// Get a UUID from the pool, refilling if needed
    ///
    /// # Panics
    /// Never panics - pool is refilled immediately before pop if empty
    fn get_uuid(&mut self) -> String {
        if let Some(uuid) = self.uuid_pool.pop_front() {
            uuid
        } else {
            // Refill pool if empty
            self.uuid_pool
                .extend((0..100).map(|_| Uuid::new_v4().to_string()));
            // Pool was just filled with 100 UUIDs, so pop cannot fail
            self.uuid_pool.pop_front().unwrap_or_else(|| {
                // Fallback: generate on demand if pool somehow empty
                Uuid::new_v4().to_string()
            })
        }
    }

    /// Execute JavaScript code and return a handle to the result
    ///
    /// The result is stored in the runtime and can be accessed via the returned handle.
    ///
    /// # Errors
    /// Returns error if code execution fails
    pub async fn execute(&mut self, code: &str) -> ToolResult<JsValueHandle> {
        // Check cache for wrapped code
        let wrapped_code = if let Some(cached) = self.code_cache.get(code) {
            cached.clone()
        } else {
            // Wrap code and cache it
            let wrapped = super::typescript::wrap_code(code);
            self.code_cache.insert(code.to_owned(), wrapped.clone());
            wrapped
        };

        // Get UUID from pool before async block
        let handle_id = self.get_uuid();

        // Run in LocalSet to allow !Send Context
        self.local_set
            .run_until(async {
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
        // Get UUID from pool before async block
        let new_handle_id = self.get_uuid();

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
                self.value_storage
                    .insert(new_handle_id.clone(), final_result);

                Ok(JsValueHandle::new(new_handle_id))
            })
            .await
    }
}
