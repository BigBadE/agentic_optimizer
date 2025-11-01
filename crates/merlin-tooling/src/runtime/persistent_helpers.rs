//! Helper functions for persistent runtime value extraction

use std::collections::HashMap;

use merlin_deps::boa_engine::{Context, JsValue};
use merlin_deps::uuid::Uuid;

use super::handle::JsValueHandle;
use crate::{ToolError, ToolResult};

/// Get array element from stored value
///
/// # Errors
/// Returns error if handle is invalid, value is not an array, or index is out of bounds
pub(super) fn get_array_element(
    handle: &JsValueHandle,
    index: usize,
    context: &mut Context,
    value_storage: &mut HashMap<String, JsValue>,
) -> ToolResult<JsValueHandle> {
    let value = value_storage
        .get(handle.id())
        .ok_or_else(|| ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id())))?;

    let obj = value
        .as_object()
        .ok_or_else(|| ToolError::ExecutionFailed("Value is not an object".to_owned()))?;

    let element = obj
        .get(index, context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Array access failed: {err}")))?;

    // Store element
    let handle_id = Uuid::new_v4().to_string();
    value_storage.insert(handle_id.clone(), element);

    Ok(JsValueHandle::new(handle_id))
}

/// Call function stored in handle
///
/// # Errors
/// Returns error if handle is invalid, value is not callable, or function call fails
pub(super) fn call_function(
    handle: &JsValueHandle,
    context: &mut Context,
    value_storage: &mut HashMap<String, JsValue>,
) -> ToolResult<JsValueHandle> {
    let value = value_storage
        .get(handle.id())
        .ok_or_else(|| ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id())))?;

    let callable = value
        .as_callable()
        .ok_or_else(|| ToolError::ExecutionFailed("Value is not callable".to_owned()))?;

    // Call with no arguments, undefined as this
    let result = callable
        .call(&JsValue::undefined(), &[], context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Function call failed: {err}")))?;

    // Run jobs to resolve Promises
    let _job_result = context.run_jobs();

    // Extract Promise value if needed
    let final_result = super::promise::extract_promise_if_needed(result, context)?;

    // Store result
    let handle_id = Uuid::new_v4().to_string();
    value_storage.insert(handle_id.clone(), final_result);

    Ok(JsValueHandle::new(handle_id))
}

/// Check if value is null or undefined
///
/// # Errors
/// Returns error if handle is invalid
pub(super) fn is_nullish(
    handle: &JsValueHandle,
    value_storage: &HashMap<String, JsValue>,
) -> ToolResult<bool> {
    let value = value_storage
        .get(handle.id())
        .ok_or_else(|| ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id())))?;

    Ok(value.is_null() || value.is_undefined())
}
