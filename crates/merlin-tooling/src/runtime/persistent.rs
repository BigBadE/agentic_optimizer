//! Persistent TypeScript runtime with long-lived Boa context

use std::collections::HashMap;
use std::sync::Arc;

use merlin_deps::boa_engine::{Context, JsValue, Source};
use merlin_deps::serde_json::Value;
use merlin_deps::uuid::Uuid;
use tokio::sync::{mpsc, oneshot};
use tokio::task::spawn_blocking;

use super::handle::JsValueHandle;
use super::persistent_helpers;
use super::promise::extract_promise_if_needed;
use super::tool_registration::register_tool_functions;
use super::typescript::wrap_code;
use crate::{Tool, ToolError, ToolResult};

/// Commands sent to the runtime task
enum RuntimeCommand {
    /// Execute code and store the result
    Execute {
        code: String,
        response: oneshot::Sender<ToolResult<JsValueHandle>>,
    },
    /// Get a property from a stored value
    GetProperty {
        handle: JsValueHandle,
        property: String,
        response: oneshot::Sender<ToolResult<JsValueHandle>>,
    },
    /// Get a string value
    GetString {
        handle: JsValueHandle,
        response: oneshot::Sender<ToolResult<String>>,
    },
    /// Get a boolean value
    GetBool {
        handle: JsValueHandle,
        response: oneshot::Sender<ToolResult<bool>>,
    },
    /// Get array length
    GetArrayLength {
        handle: JsValueHandle,
        response: oneshot::Sender<ToolResult<usize>>,
    },
    /// Get array element
    GetArrayElement {
        handle: JsValueHandle,
        index: usize,
        response: oneshot::Sender<ToolResult<JsValueHandle>>,
    },
    /// Call a function
    CallFunction {
        handle: JsValueHandle,
        response: oneshot::Sender<ToolResult<JsValueHandle>>,
    },
    /// Check if value is null or undefined
    IsNullish {
        handle: JsValueHandle,
        response: oneshot::Sender<ToolResult<bool>>,
    },
    /// Convert handle to JSON
    ToJson {
        handle: JsValueHandle,
        response: oneshot::Sender<ToolResult<Value>>,
    },
}

/// Persistent TypeScript runtime with long-lived Boa context
///
/// This runtime maintains a JavaScript context across multiple executions,
/// allowing JavaScript values to be stored and referenced later via handles.
pub struct PersistentTypeScriptRuntime {
    /// Channel for sending commands to the runtime task
    command_tx: mpsc::UnboundedSender<RuntimeCommand>,
}

impl PersistentTypeScriptRuntime {
    /// Create a new persistent runtime
    ///
    /// Spawns a blocking task with a Boa context that lives for the entire
    /// runtime lifetime. Tools are registered in the context and available
    /// to all executed code.
    #[must_use]
    pub fn new(tools: HashMap<String, Arc<dyn Tool>>) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        // Spawn blocking task with Boa context
        spawn_blocking(move || {
            Self::runtime_task(command_rx, &tools);
        });

        Self { command_tx }
    }

    /// Runtime task - runs in blocking context with Boa
    ///
    /// This function runs in a `spawn_blocking` task and owns the Boa context.
    /// It processes commands via channels and maintains value storage.
    fn runtime_task(
        mut command_rx: mpsc::UnboundedReceiver<RuntimeCommand>,
        tools: &HashMap<String, Arc<dyn Tool>>,
    ) {
        merlin_deps::tracing::debug!("Starting persistent runtime task");

        // Create Boa context
        let mut context = Context::default();

        // Register tools
        if let Err(err) = register_tool_functions(&mut context, tools) {
            merlin_deps::tracing::error!("Failed to register tools: {err}");
            return;
        }

        // Value storage - keeps JavaScript values alive
        let mut value_storage: HashMap<String, JsValue> = HashMap::new();

        // Process commands
        while let Some(command) = command_rx.blocking_recv() {
            match command {
                RuntimeCommand::Execute { code, response } => {
                    let result = Self::exec_and_store(&code, &mut context, &mut value_storage);
                    drop(response.send(result));
                }
                RuntimeCommand::GetProperty {
                    handle,
                    property,
                    response,
                } => {
                    let result =
                        Self::prop_get(&handle, &property, &mut context, &mut value_storage);
                    drop(response.send(result));
                }
                RuntimeCommand::GetString { handle, response } => {
                    let result = Self::string_get(&handle, &mut context, &value_storage);
                    drop(response.send(result));
                }
                RuntimeCommand::GetBool { handle, response } => {
                    let result = Self::bool_get(&handle, &value_storage);
                    drop(response.send(result));
                }
                RuntimeCommand::GetArrayLength { handle, response } => {
                    let result = Self::array_len(&handle, &mut context, &value_storage);
                    drop(response.send(result));
                }
                RuntimeCommand::GetArrayElement {
                    handle,
                    index,
                    response,
                } => {
                    let result = persistent_helpers::get_array_element(
                        &handle,
                        index,
                        &mut context,
                        &mut value_storage,
                    );
                    drop(response.send(result));
                }
                RuntimeCommand::CallFunction { handle, response } => {
                    let result = persistent_helpers::call_function(
                        &handle,
                        &mut context,
                        &mut value_storage,
                    );
                    drop(response.send(result));
                }
                RuntimeCommand::IsNullish { handle, response } => {
                    let result = persistent_helpers::is_nullish(&handle, &value_storage);
                    drop(response.send(result));
                }
                RuntimeCommand::ToJson { handle, response } => {
                    let result = Self::handle_to_json(&handle, &mut context, &value_storage);
                    drop(response.send(result));
                }
            }
        }

        merlin_deps::tracing::debug!("Runtime task shutting down");
    }

    /// Execute code and store result
    ///
    /// # Errors
    /// Returns error if code execution or Promise resolution fails
    fn exec_and_store(
        code: &str,
        context: &mut Context,
        value_storage: &mut HashMap<String, JsValue>,
    ) -> ToolResult<JsValueHandle> {
        let wrapped_code = wrap_code(code);

        // Execute the code
        let result = context
            .eval(Source::from_bytes(&wrapped_code))
            .map_err(|err| ToolError::ExecutionFailed(format!("JavaScript error: {err}")))?;

        // Run jobs to resolve Promises
        let _job_result = context.run_jobs();

        // Extract Promise value if needed
        let final_result = extract_promise_if_needed(result, context)?;

        // Store the value
        let handle_id = Uuid::new_v4().to_string();
        value_storage.insert(handle_id.clone(), final_result);

        Ok(JsValueHandle::new(handle_id))
    }

    /// Get property from stored value
    ///
    /// # Errors
    /// Returns error if handle is invalid, value is not an object, or property access fails
    fn prop_get(
        handle: &JsValueHandle,
        property: &str,
        context: &mut Context,
        value_storage: &mut HashMap<String, JsValue>,
    ) -> ToolResult<JsValueHandle> {
        let value = value_storage.get(handle.id()).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id()))
        })?;

        let prop_value = value
            .as_object()
            .ok_or_else(|| ToolError::ExecutionFailed("Value is not an object".to_owned()))?
            .get(merlin_deps::boa_engine::js_string!(property), context)
            .map_err(|err| ToolError::ExecutionFailed(format!("Property access failed: {err}")))?;

        // Store property value
        let handle_id = Uuid::new_v4().to_string();
        value_storage.insert(handle_id.clone(), prop_value);

        Ok(JsValueHandle::new(handle_id))
    }

    /// Get string from stored value
    ///
    /// # Errors
    /// Returns error if handle is invalid or string conversion fails
    fn string_get(
        handle: &JsValueHandle,
        context: &mut Context,
        value_storage: &HashMap<String, JsValue>,
    ) -> ToolResult<String> {
        let value = value_storage.get(handle.id()).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id()))
        })?;

        value
            .to_string(context)
            .map(|js_str| js_str.to_std_string_escaped())
            .map_err(|err| ToolError::ExecutionFailed(format!("String conversion failed: {err}")))
    }

    /// Get boolean from stored value
    ///
    /// # Errors
    /// Returns error if handle is invalid or value is not a boolean
    fn bool_get(
        handle: &JsValueHandle,
        value_storage: &HashMap<String, JsValue>,
    ) -> ToolResult<bool> {
        let value = value_storage.get(handle.id()).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id()))
        })?;

        value
            .as_boolean()
            .ok_or_else(|| ToolError::ExecutionFailed("Value is not a boolean".to_owned()))
    }

    /// Get array length from stored value
    ///
    /// # Errors
    /// Returns error if handle is invalid, value is not an array, or length is invalid
    fn array_len(
        handle: &JsValueHandle,
        context: &mut Context,
        value_storage: &HashMap<String, JsValue>,
    ) -> ToolResult<usize> {
        let value = value_storage.get(handle.id()).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id()))
        })?;

        let obj = value
            .as_object()
            .ok_or_else(|| ToolError::ExecutionFailed("Value is not an object".to_owned()))?;

        let length = obj
            .get(merlin_deps::boa_engine::js_string!("length"), context)
            .map_err(|err| ToolError::ExecutionFailed(format!("Length access failed: {err}")))?;

        length
            .as_number()
            .and_then(|num| (num >= 0.0 && num.fract() == 0.0).then_some(num as usize))
            .ok_or_else(|| ToolError::ExecutionFailed("Length is not a valid number".to_owned()))
    }

    /// Convert handle to JSON
    ///
    /// # Errors
    /// Returns error if handle is invalid or JSON conversion fails
    fn handle_to_json(
        handle: &JsValueHandle,
        context: &mut Context,
        value_storage: &HashMap<String, JsValue>,
    ) -> ToolResult<Value> {
        let value = value_storage.get(handle.id()).ok_or_else(|| {
            ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id()))
        })?;

        super::conversion::js_value_to_json(value, context)
    }

    /// Execute JavaScript code and return a handle to the result
    ///
    /// # Errors
    /// Returns error if code execution fails
    pub async fn execute(&self, code: &str) -> ToolResult<JsValueHandle> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::Execute {
                code: code.to_owned(),
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }

    /// Get a property from a stored JavaScript object
    ///
    /// # Errors
    /// Returns error if handle is invalid or property access fails
    pub async fn get_property(
        &self,
        handle: JsValueHandle,
        property: String,
    ) -> ToolResult<JsValueHandle> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::GetProperty {
                handle,
                property,
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }

    /// Get a string value from a handle
    ///
    /// # Errors
    /// Returns error if handle is invalid or value is not a string
    pub async fn get_string(&self, handle: JsValueHandle) -> ToolResult<String> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::GetString {
                handle,
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }

    /// Get a boolean value from a handle
    ///
    /// # Errors
    /// Returns error if handle is invalid or value is not a boolean
    pub async fn get_bool(&self, handle: JsValueHandle) -> ToolResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::GetBool {
                handle,
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }

    /// Get the length of an array
    ///
    /// # Errors
    /// Returns error if handle is invalid or value is not an array
    pub async fn get_array_length(&self, handle: JsValueHandle) -> ToolResult<usize> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::GetArrayLength {
                handle,
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }

    /// Get an element from an array
    ///
    /// # Errors
    /// Returns error if handle is invalid or index is out of bounds
    pub async fn get_array_element(
        &self,
        handle: JsValueHandle,
        index: usize,
    ) -> ToolResult<JsValueHandle> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::GetArrayElement {
                handle,
                index,
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }

    /// Call a stored JavaScript function
    ///
    /// # Errors
    /// Returns error if handle is invalid or function call fails
    pub async fn call_function(&self, handle: JsValueHandle) -> ToolResult<JsValueHandle> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::CallFunction {
                handle,
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }

    /// Check if a value is null or undefined
    ///
    /// # Errors
    /// Returns error if handle is invalid
    pub async fn is_nullish(&self, handle: JsValueHandle) -> ToolResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::IsNullish {
                handle,
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }

    /// Convert a handle to JSON
    ///
    /// # Errors
    /// Returns error if handle is invalid or JSON conversion fails
    pub async fn to_json(&self, handle: JsValueHandle) -> ToolResult<Value> {
        let (response_tx, response_rx) = oneshot::channel();

        self.command_tx
            .send(RuntimeCommand::ToJson {
                handle,
                response: response_tx,
            })
            .map_err(|_| ToolError::ExecutionFailed("Runtime task has shut down".to_owned()))?;

        response_rx
            .await
            .map_err(|_| ToolError::ExecutionFailed("Response channel closed".to_owned()))?
    }
}
