//! Bulk extraction operations for efficient IPC
//!
//! These operations perform complete extraction in the blocking task,
//! minimizing async channel round-trips from ~50-60 to 1-2 per `TaskList`.

use std::collections::HashMap;
use std::hash::BuildHasher;

use merlin_deps::boa_engine::{Context, JsValue};
use merlin_deps::uuid::Uuid;

use super::handle::JsValueHandle;
use crate::{ToolError, ToolResult};

/// Extracted task list data
#[derive(Debug, Clone)]
pub struct ExtractedTaskList {
    /// Task list title
    pub title: String,
    /// Task steps
    pub steps: Vec<ExtractedTaskStep>,
}

/// Extracted task step data
#[derive(Debug, Clone)]
pub struct ExtractedTaskStep {
    /// Step title
    pub title: String,
    /// Step description
    pub description: String,
    /// Step type string
    pub step_type: String,
    /// Exit requirement function handle (if present)
    pub exit_requirement: Option<String>,
    /// Step dependencies
    pub dependencies: Vec<String>,
}

/// Extract complete `TaskList` from JavaScript object in one operation
///
/// This replaces ~50-60 IPC round-trips with a single blocking operation.
///
/// # Errors
/// Returns error if handle is invalid or extraction fails
pub fn extract_task_list<S: BuildHasher>(
    handle: &JsValueHandle,
    context: &mut Context,
    value_storage: &mut HashMap<String, JsValue, S>,
) -> ToolResult<Option<ExtractedTaskList>> {
    let value = value_storage
        .get(handle.id())
        .ok_or_else(|| ToolError::ExecutionFailed(format!("Handle not found: {}", handle.id())))?;

    // Try to extract as object
    let Some(obj) = value.as_object() else {
        return Ok(None); // Not an object, not a TaskList
    };

    // Try to get title and steps properties
    let title_value = obj
        .get(merlin_deps::boa_engine::js_string!("title"), context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to get title: {err}")))?;

    let steps_value = obj
        .get(merlin_deps::boa_engine::js_string!("steps"), context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to get steps: {err}")))?;

    // Check if title/steps are nullish
    if title_value.is_null() || title_value.is_undefined() {
        return Ok(None);
    }
    if steps_value.is_null() || steps_value.is_undefined() {
        return Ok(None);
    }

    // Extract title string
    let title = title_value
        .to_string(context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to convert title: {err}")))?
        .to_std_string_escaped();

    // Extract steps array
    let steps_obj = steps_value
        .as_object()
        .ok_or_else(|| ToolError::ExecutionFailed("Steps is not an array".to_owned()))?;

    let steps_len = steps_obj
        .get(merlin_deps::boa_engine::js_string!("length"), context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to get length: {err}")))?
        .to_u32(context)
        .unwrap_or(0) as usize;

    let mut steps = Vec::with_capacity(steps_len);

    // Extract each step
    for idx in 0..steps_len {
        let element = steps_obj.get(idx, context).map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to get step {idx}: {err}"))
        })?;

        let step = extract_single_step(&element, context, value_storage)?;
        steps.push(step);
    }

    Ok(Some(ExtractedTaskList { title, steps }))
}

/// Extract a single `TaskStep` from JavaScript object
///
/// # Errors
/// Returns error if required properties are missing or malformed
fn extract_single_step<S: BuildHasher>(
    value: &JsValue,
    context: &mut Context,
    value_storage: &mut HashMap<String, JsValue, S>,
) -> ToolResult<ExtractedTaskStep> {
    let obj = value
        .as_object()
        .ok_or_else(|| ToolError::ExecutionFailed("Step is not an object".to_owned()))?;

    // Extract title
    let title_value = obj
        .get(merlin_deps::boa_engine::js_string!("title"), context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Step missing title: {err}")))?;
    let title = title_value
        .to_string(context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to convert title: {err}")))?
        .to_std_string_escaped();

    // Extract description
    let desc_value = obj
        .get(merlin_deps::boa_engine::js_string!("description"), context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Step missing description: {err}")))?;
    let description = desc_value
        .to_string(context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to convert description: {err}")))?
        .to_std_string_escaped();

    // Extract step_type
    let type_value = obj
        .get(merlin_deps::boa_engine::js_string!("step_type"), context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Step missing step_type: {err}")))?;
    let step_type = type_value
        .to_string(context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to convert step_type: {err}")))?
        .to_std_string_escaped();

    // Extract optional exit_requirement (store function handle)
    let exit_requirement = match obj.get(
        merlin_deps::boa_engine::js_string!("exit_requirement"),
        context,
    ) {
        Ok(req_value) if !req_value.is_null() && !req_value.is_undefined() => {
            // Store the function in value_storage and return its handle ID
            let handle_id = Uuid::new_v4().to_string();
            value_storage.insert(handle_id.clone(), req_value);
            Some(handle_id)
        }
        _ => None,
    };

    // Extract optional dependencies array
    let dependencies = match obj.get(merlin_deps::boa_engine::js_string!("dependencies"), context) {
        Ok(deps_value) if !deps_value.is_null() && !deps_value.is_undefined() => {
            extract_string_array(&deps_value, context)?
        }
        _ => Vec::new(),
    };

    Ok(ExtractedTaskStep {
        title,
        description,
        step_type,
        exit_requirement,
        dependencies,
    })
}

/// Extract array of strings from JavaScript array
///
/// # Errors
/// Returns error if value is not an array or contains non-strings
fn extract_string_array(value: &JsValue, context: &mut Context) -> ToolResult<Vec<String>> {
    let obj = value
        .as_object()
        .ok_or_else(|| ToolError::ExecutionFailed("Value is not an array".to_owned()))?;

    let len = obj
        .get(merlin_deps::boa_engine::js_string!("length"), context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to get array length: {err}")))?
        .to_u32(context)
        .unwrap_or(0) as usize;

    let mut result = Vec::with_capacity(len);

    for idx in 0..len {
        let element = obj.get(idx, context).map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to get element {idx}: {err}"))
        })?;

        let string = element
            .to_string(context)
            .map_err(|err| {
                ToolError::ExecutionFailed(format!("Failed to convert element {idx}: {err}"))
            })?
            .to_std_string_escaped();

        result.push(string);
    }

    Ok(result)
}
