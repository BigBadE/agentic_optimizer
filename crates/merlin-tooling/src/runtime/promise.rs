//! Promise extraction and handling utilities.

use merlin_deps::boa_engine::property::Attribute;
use merlin_deps::boa_engine::{Context, JsValue, Source};

use crate::{ToolError, ToolResult};

/// Extract Promise value if the result is a Promise
///
/// # Errors
/// Returns error if Promise extraction fails
pub fn extract_promise_if_needed(result: JsValue, context: &mut Context) -> ToolResult<JsValue> {
    let Some(obj) = result.as_object() else {
        return Ok(result);
    };

    // Check if it's a Promise by looking at its constructor name
    let is_promise = obj
        .get(boa_engine::js_string!("constructor"), context)
        .ok()
        .and_then(|constructor| constructor.as_object())
        .and_then(|constructor_obj| {
            constructor_obj
                .get(boa_engine::js_string!("name"), context)
                .ok()
        })
        .and_then(|name| {
            name.as_string()
                .map(|js_str| js_str.to_std_string_escaped())
        })
        .is_some_and(|name| name == "Promise");

    if !is_promise {
        return Ok(result);
    }

    merlin_deps::tracing::debug!("Result is a Promise, extracting resolved value");

    // Store the promise in a global variable and use JavaScript to extract its value
    context
        .register_global_property(
            boa_engine::js_string!("__promise__"),
            result,
            Attribute::all(),
        )
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to register promise: {err}")))?;

    // Use a JavaScript helper to extract the resolved value
    // Use `var` instead of `let` to avoid duplicate declaration errors in persistent runtime
    let setup_handler = r"
        var __result__;
        var __error__;
        __promise__.then(
            value => { __result__ = value; },
            error => { __error__ = error; }
        );
    ";

    context
        .eval(Source::from_bytes(setup_handler))
        .map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to setup promise handler: {err}"))
        })?;

    // Now run jobs to execute the .then() callback
    let _result = context.run_jobs();

    // Check if there was an error
    let error_check = context
        .eval(Source::from_bytes("__error__"))
        .map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to check promise error: {err}"))
        })?;
    if !error_check.is_undefined() {
        let error_msg = extract_error_message(&error_check, context);
        return Err(ToolError::ExecutionFailed(format!(
            "Promise rejected: {error_msg}"
        )));
    }

    // Get the result
    context
        .eval(Source::from_bytes("__result__"))
        .map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to extract promise value: {err}"))
        })
}

/// Extract error message from a JavaScript error value
pub fn extract_error_message(error_check: &JsValue, context: &mut Context) -> String {
    error_check.as_object().map_or_else(
        || format!("{error_check:?}"),
        |err_obj| {
            let result = (|| {
                let val = err_obj
                    .get(boa_engine::js_string!("message"), context)
                    .ok()?;
                val.as_string().map(|js_str| js_str.to_std_string_escaped())
            })();
            result.unwrap_or_else(|| format!("{error_check:?}"))
        },
    )
}
