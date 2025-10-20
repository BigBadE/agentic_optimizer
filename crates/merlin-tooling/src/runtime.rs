//! TypeScript/JavaScript runtime using Boa engine.
//!
//! Provides a sandboxed environment for executing JavaScript code with tool integration.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::scope;
use std::time::Duration;

use boa_engine::JsNativeError;
use boa_engine::object::JsObject;
use boa_engine::object::builtins::JsArray;
use boa_engine::property::Attribute;
use boa_engine::{
    Context, JsResult, JsString, JsValue, NativeFunction, Source, job::SimpleJobQueue, js_string,
};
use serde_json::{Map, Number, Value};
use tokio::runtime::Builder;
use tokio::task::spawn_blocking;
use tokio::time;

use crate::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Maximum execution time for JavaScript code
const MAX_EXECUTION_TIME: Duration = Duration::from_secs(30);

/// JavaScript runtime for executing tool calls (extracted from TypeScript code blocks)
///
/// Note: Currently executes JavaScript code. TypeScript type annotations should be
/// stripped before passing code to this runtime. Future enhancement will add automatic
/// TypeScript-to-JavaScript transformation.
pub struct TypeScriptRuntime {
    /// Registry of available tools
    tools: HashMap<String, Arc<dyn Tool>>,
    /// Execution timeout
    timeout: Duration,
}

impl TypeScriptRuntime {
    /// Create a new TypeScript runtime with default limits
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            timeout: MAX_EXECUTION_TIME,
        }
    }

    /// Register a tool that can be called from JavaScript
    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_owned(), tool);
    }

    /// Set the execution timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the memory limit (no-op for Boa, kept for API compatibility)
    #[must_use]
    pub fn with_memory_limit(self, _limit: usize) -> Self {
        self
    }

    /// Execute JavaScript/TypeScript code that calls tools
    ///
    /// # Errors
    /// Returns an error if:
    /// - The code cannot be parsed or executed
    /// - Tool calls fail to execute
    /// - Execution times out
    pub async fn execute(&self, code: &str) -> ToolResult<Value> {
        tracing::debug!("TypeScriptRuntime::execute called");

        let wrapped_code = Self::wrap_code(code);
        let tools_clone = self.tools.clone();
        let timeout = self.timeout;

        // Execute with timeout
        time::timeout(timeout, async move {
            // Run in spawn_blocking since Boa context is !Send
            spawn_blocking(move || Self::execute_sync(&wrapped_code, &tools_clone))
                .await
                .map_err(|err| ToolError::ExecutionFailed(format!("Task join failed: {err}")))?
        })
        .await
        .map_err(|_| {
            ToolError::ExecutionFailed(format!(
                "Execution timed out after {} seconds",
                timeout.as_secs()
            ))
        })?
    }

    /// Execute JavaScript synchronously in Boa
    ///
    /// # Errors
    /// Returns error if execution fails
    #[allow(
        clippy::too_many_lines,
        reason = "Complex Promise handling and tool registration logic"
    )]
    fn execute_sync(code: &str, tools: &HashMap<String, Arc<dyn Tool>>) -> ToolResult<Value> {
        tracing::debug!("Creating Boa context with job queue");

        // Create context with a job queue for Promise support
        let job_queue = Rc::new(SimpleJobQueue::new());
        let mut context = Context::builder()
            .job_queue(job_queue)
            .build()
            .map_err(|err| {
                ToolError::ExecutionFailed(format!("Failed to create context: {err}"))
            })?;

        // Register tools as global functions
        Self::register_tool_functions(&mut context, tools)?;

        tracing::debug!("Executing JavaScript code");

        // Execute the code
        let result = context
            .eval(Source::from_bytes(&code))
            .map_err(|err| ToolError::ExecutionFailed(format!("JavaScript error: {err}")))?;

        // Run all pending jobs (resolve Promises)
        tracing::debug!("Running job queue to resolve Promises");
        context.run_jobs();

        // Check if result is a Promise and extract its value by storing it and using a helper
        let final_result = if let Some(obj) = result.as_object() {
            // Check if it's a Promise by looking at its constructor name
            let is_promise = obj
                .get(js_string!("constructor"), &mut context)
                .ok()
                .and_then(|constructor| constructor.as_object().cloned())
                .and_then(|constructor_obj| {
                    constructor_obj.get(js_string!("name"), &mut context).ok()
                })
                .and_then(|name| name.as_string().map(JsString::to_std_string_escaped))
                .is_some_and(|name| name == "Promise");

            if is_promise {
                tracing::debug!("Result is a Promise, extracting resolved value");

                // Store the promise in a global variable and use JavaScript to extract its value
                context
                    .register_global_property(
                        js_string!("__promise__"),
                        result.clone(),
                        Attribute::all(),
                    )
                    .map_err(|err| {
                        ToolError::ExecutionFailed(format!("Failed to register promise: {err}"))
                    })?;

                // Use a JavaScript helper to extract the resolved value
                // Set up the .then() handler first
                let setup_handler = r"
                    let __result__;
                    let __error__;
                    __promise__.then(
                        value => { __result__ = value; },
                        error => { __error__ = error; }
                    );
                ";

                context
                    .eval(Source::from_bytes(setup_handler))
                    .map_err(|err| {
                        ToolError::ExecutionFailed(format!(
                            "Failed to setup promise handler: {err}"
                        ))
                    })?;

                // Now run jobs to execute the .then() callback
                context.run_jobs();

                // Check if there was an error
                let error_check = context
                    .eval(Source::from_bytes("__error__"))
                    .map_err(|err| {
                        ToolError::ExecutionFailed(format!("Failed to check promise error: {err}"))
                    })?;
                if !error_check.is_undefined() {
                    // Try to get the error message
                    let error_msg = Self::extract_error_message(&error_check, &mut context);
                    return Err(ToolError::ExecutionFailed(format!(
                        "Promise rejected: {error_msg}"
                    )));
                }

                // Get the result
                context
                    .eval(Source::from_bytes("__result__"))
                    .map_err(|err| {
                        ToolError::ExecutionFailed(format!(
                            "Failed to extract promise value: {err}"
                        ))
                    })?
            } else {
                result
            }
        } else {
            result
        };

        // Convert result to JSON
        Self::js_value_to_json(&final_result, &mut context)
    }

    /// Extract error message from a JavaScript error value
    fn extract_error_message(error_check: &JsValue, context: &mut Context) -> String {
        error_check.as_object().map_or_else(
            || format!("{error_check:?}"),
            |err_obj| {
                err_obj
                    .get(js_string!("message"), context)
                    .ok()
                    .and_then(|val| val.as_string().map(JsString::to_std_string_escaped))
                    .unwrap_or_else(|| format!("{error_check:?}"))
            },
        )
    }

    /// Register tool functions in the JavaScript context
    ///
    /// # Errors
    /// Returns error if registration fails
    fn register_tool_functions(
        context: &mut Context,
        tools: &HashMap<String, Arc<dyn Tool>>,
    ) -> ToolResult<()> {
        for (name, tool) in tools {
            let tool_clone = Arc::clone(tool);

            // Create tool function
            // SAFETY: Arc<dyn Tool> is not Trace, but it's safe to use here because:
            // 1. The tool registry is owned by TypeScriptRuntime which outlives the Context
            // 2. Tools are immutable and thread-safe (Arc)
            // 3. The closure only captures Arc which is safe to share
            #[allow(
                unsafe_code,
                clippy::undocumented_unsafe_blocks,
                reason = "Arc<dyn Tool> is not Trace, but safe to use as documented above"
            )]
            let func = unsafe {
                NativeFunction::from_closure(move |_this, args, ctx| {
                    tracing::debug!("Tool '{}' called from JavaScript", tool_clone.name());

                    // Get parameters (first argument)
                    let params = if args.is_empty() {
                        serde_json::json!({})
                    } else {
                        Self::js_value_to_json_static(&args[0], ctx)?
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
                    if result.success {
                        let data = result.data.unwrap_or(Value::String(result.message));
                        Self::json_to_js_value_static(&data, ctx)
                    } else {
                        Err(JsNativeError::error().with_message(result.message).into())
                    }
                })
            };

            context
                .register_global_callable(js_string!(name.as_str()), 0, func)
                .map_err(|err| {
                    ToolError::ExecutionFailed(format!("Failed to register tool '{name}': {err}"))
                })?;
        }

        Ok(())
    }

    /// Strip TypeScript type annotations to convert to valid JavaScript using SWC
    fn strip_typescript_types(code: &str) -> String {
        use swc_common::{FileName, GLOBALS, Globals, Mark, SourceMap, sync::Lrc};
        use swc_ecma_ast::EsVersion;
        use swc_ecma_codegen::{Config as CodegenConfig, Emitter, text_writer::JsWriter};
        use swc_ecma_parser::{Syntax, TsSyntax, parse_file_as_program};
        use swc_ecma_transforms_typescript::strip;

        // Create a source map
        let source_map = Lrc::new(SourceMap::default());
        let source_file = source_map.new_source_file(Lrc::new(FileName::Anon), code.to_owned());

        // Configure TypeScript parser
        let syntax = Syntax::Typescript(TsSyntax {
            tsx: false,
            decorators: false,
            dts: false,
            no_early_errors: true,
            disallow_ambiguous_jsx_like: false,
        });

        // Parse the TypeScript code
        let Ok(program) =
            parse_file_as_program(&source_file, syntax, EsVersion::Es2022, None, &mut vec![])
        else {
            // If parsing fails, return original code
            tracing::warn!("Failed to parse TypeScript code, returning original");
            return code.to_owned();
        };

        // Apply TypeScript stripping transform
        let program = GLOBALS.set(&Globals::default(), || {
            let unresolved_mark = Mark::new();
            let top_level_mark = Mark::new();

            // Apply the strip transform
            let pass = strip(unresolved_mark, top_level_mark);
            program.apply(pass)
        });

        // Generate JavaScript code
        let mut buf = vec![];
        {
            let writer = JsWriter::new(Lrc::clone(&source_map), "\n", &mut buf, None);
            let mut emitter = Emitter {
                cfg: CodegenConfig::default(),
                cm: Lrc::clone(&source_map),
                comments: None,
                wr: writer,
            };

            if emitter.emit_program(&program).is_err() {
                tracing::warn!("Failed to emit JavaScript code, returning original");
                return code.to_owned();
            }
        }

        String::from_utf8(buf).unwrap_or_else(|_| {
            tracing::warn!("Failed to convert generated code to UTF-8, returning original");
            code.to_owned()
        })
    }

    /// Wrap code in `agent_code` function if needed
    fn wrap_code(code: &str) -> String {
        // First strip TypeScript type annotations
        let code_without_types = Self::strip_typescript_types(code);
        let trimmed = code_without_types.trim();

        // Check if code already defines agent_code function (async or sync)
        if trimmed.contains("async function agent_code") {
            // Wrap async function call in async IIFE for compatibility
            format!("{trimmed}\n(async () => await agent_code())()")
        } else if trimmed.contains("function agent_code") {
            // Just call sync function
            format!("{trimmed}\nagent_code();")
        } else {
            // Check if code contains a top-level return statement
            let has_return = trimmed
                .lines()
                .any(|line| line.trim_start().starts_with("return "));

            if has_return {
                // Wrap in IIFE since it has explicit return
                // This handles cases like: function foo() { ... } return foo()
                format!("(function() {{ {trimmed} }})()")
            } else {
                // Evaluate directly for simple expressions
                // This allows statements like "const x = 42; x * 2" to work
                trimmed.to_owned()
            }
        }
    }

    /// Convert JS value to JSON
    ///
    /// # Errors
    /// Returns error if conversion fails
    fn js_value_to_json(value: &JsValue, context: &mut Context) -> ToolResult<Value> {
        Self::js_value_to_json_static(value, context)
            .map_err(|err| ToolError::ExecutionFailed(format!("Failed to convert JS value: {err}")))
    }

    /// Convert JS value to JSON (static version for closures)
    ///
    /// # Errors
    /// Returns error if conversion fails
    fn js_value_to_json_static(value: &JsValue, context: &mut Context) -> JsResult<Value> {
        if value.is_null() || value.is_undefined() {
            Ok(Value::Null)
        } else if let Some(boolean) = value.as_boolean() {
            Ok(Value::Bool(boolean))
        } else if let Some(number) = value.as_number() {
            // Check if it's an integer value (no fractional part)
            #[allow(
                clippy::float_cmp,
                reason = "Intentionally comparing to check for exact integer value"
            )]
            if number.fract() == 0.0 && number.is_finite() {
                // It's a whole number, convert to i64 if in range
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Checked that number has no fractional part"
                )]
                Ok(Value::Number(Number::from(number as i64)))
            } else {
                Ok(Number::from_f64(number).map_or(Value::Null, Value::Number))
            }
        } else if let Some(string) = value.as_string() {
            Ok(Value::String(string.to_std_string_escaped()))
        } else if let Some(obj) = value.as_object() {
            // Check if it's an array
            if obj.is_array() {
                let length = obj
                    .get(js_string!("length"), context)?
                    .to_u32(context)
                    .unwrap_or(0);
                let mut array = Vec::new();
                for index in 0..length {
                    let element = obj.get(index, context)?;
                    array.push(Self::js_value_to_json_static(&element, context)?);
                }
                Ok(Value::Array(array))
            } else {
                // Regular object
                let mut map = Map::new();
                for key in obj.own_property_keys(context)? {
                    let key_value = JsValue::from(key.clone());
                    let key_string = key_value.to_string(context)?;
                    let prop_value = obj.get(key.clone(), context)?;
                    map.insert(
                        key_string.to_std_string_escaped(),
                        Self::js_value_to_json_static(&prop_value, context)?,
                    );
                }
                Ok(Value::Object(map))
            }
        } else {
            Ok(Value::String(value.display().to_string()))
        }
    }

    /// Convert JSON to JS value (static version for closures)
    ///
    /// # Errors
    /// Returns error if conversion fails
    fn json_to_js_value_static(value: &Value, context: &mut Context) -> JsResult<JsValue> {
        match value {
            Value::Null => Ok(JsValue::null()),
            Value::Bool(boolean) => Ok(JsValue::from(*boolean)),
            Value::Number(number) => number.as_i64().map_or_else(
                || {
                    number
                        .as_f64()
                        .map_or_else(|| Ok(JsValue::from(0)), |float| Ok(JsValue::from(float)))
                },
                |int| Ok(JsValue::from(int)),
            ),
            Value::String(string) => Ok(JsValue::from(js_string!(string.as_str()))),
            Value::Array(array) => {
                let js_array = JsArray::new(context);
                for (index, val) in array.iter().enumerate() {
                    let js_val = Self::json_to_js_value_static(val, context)?;
                    js_array.set(index, js_val, true, context)?;
                }
                Ok(js_array.into())
            }
            Value::Object(obj) => {
                let js_obj = JsObject::default();
                for (key, val) in obj {
                    let js_val = Self::json_to_js_value_static(val, context)?;
                    js_obj.set(js_string!(key.as_str()), js_val, true, context)?;
                }
                Ok(js_obj.into())
            }
        }
    }

    /// Generate TypeScript type definitions for registered tools
    ///
    /// # Errors
    /// Returns an error if formatting fails (should never happen in practice)
    pub fn generate_type_definitions(&self) -> Result<String, ToolError> {
        let mut defs = String::from("// Available tool functions\n\n");

        for tool in self.tools.values() {
            writeln!(defs, "/**\n * {}\n */", tool.description()).map_err(|err| {
                ToolError::ExecutionFailed(format!("Failed to write type definitions: {err}"))
            })?;
            writeln!(
                defs,
                "declare function {}(params: any): any;\n",
                tool.name()
            )
            .map_err(|err| {
                ToolError::ExecutionFailed(format!("Failed to write type definitions: {err}"))
            })?;
        }

        Ok(defs)
    }
}

impl Default for TypeScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg_attr(
    test,
    allow(
        clippy::absolute_paths,
        reason = "Test module allows absolute paths for clarity"
    )
)]
mod tests {
    use super::*;
    use crate::{ToolInput, ToolOutput};
    use async_trait::async_trait;

    // Mock tool for testing
    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> &'static str {
            "Echoes back the input"
        }

        fn typescript_signature(&self) -> &'static str {
            "/**\n * Echoes back the input\n */\ndeclare function echo(params: any): Promise<any>;"
        }

        async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
            Ok(ToolOutput::success_with_data("echoed", input.params))
        }
    }

    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = TypeScriptRuntime::new();
        assert_eq!(runtime.timeout, MAX_EXECUTION_TIME);
    }

    #[tokio::test]
    async fn test_simple_javascript_execution() {
        let runtime = TypeScriptRuntime::new();
        let code = "1 + 1";
        let result = runtime.execute(code).await;
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_variable_declaration() {
        let runtime = TypeScriptRuntime::new();
        let code = "const x = 42; x * 2";
        let result = runtime.execute(code).await;
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        assert_eq!(result.unwrap(), serde_json::json!(84));
    }
}
