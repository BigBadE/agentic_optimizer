//! JavaScript/JSON value conversion utilities.

use boa_engine::object::JsObject;
use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsResult, JsValue};
use serde_json::{Map, Number, Value};

use crate::ToolError;
use crate::ToolResult;

/// Convert JS value to JSON
///
/// # Errors
/// Returns error if conversion fails
pub fn js_value_to_json(value: &JsValue, context: &mut Context) -> ToolResult<Value> {
    use tracing::{Level, span};

    let span = span!(Level::INFO, "js_value_to_json");
    let enter_guard = span.enter();
    let result = js_value_to_json_static(value, context)
        .map_err(|err| ToolError::ExecutionFailed(format!("Failed to convert JS value: {err}")));
    drop(enter_guard);
    result
}

/// Convert JS value to JSON (static version for closures)
///
/// # Errors
/// Returns error if conversion fails
pub fn js_value_to_json_static(value: &JsValue, context: &mut Context) -> JsResult<Value> {
    if value.is_null() || value.is_undefined() {
        Ok(Value::Null)
    } else if let Some(boolean) = value.as_boolean() {
        Ok(Value::Bool(boolean))
    } else if let Some(number) = value.as_number() {
        // Check if it's an integer value (no fractional part)
        if number.fract().abs() < f64::EPSILON && number.is_finite() {
            // It's a whole number, convert to i64 if in range
            let int_value = number.round() as i64;
            Ok(Value::Number(Number::from(int_value)))
        } else {
            Ok(Number::from_f64(number).map_or(Value::Null, Value::Number))
        }
    } else if let Some(string) = value.as_string() {
        Ok(Value::String(string.to_std_string_escaped()))
    } else if let Some(obj) = value.as_object() {
        // Check if it's an array
        if obj.is_array() {
            let length = obj
                .get(boa_engine::js_string!("length"), context)?
                .to_u32(context)
                .unwrap_or(0);
            let mut array = Vec::new();
            for index in 0..length {
                let element = obj.get(index, context)?;
                array.push(js_value_to_json_static(&element, context)?);
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
                    js_value_to_json_static(&prop_value, context)?,
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
pub fn json_to_js_value_static(value: &Value, context: &mut Context) -> JsResult<JsValue> {
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
        Value::String(string) => Ok(JsValue::from(boa_engine::js_string!(string.as_str()))),
        Value::Array(array) => {
            let js_array = JsArray::new(context);
            for (index, val) in array.iter().enumerate() {
                let js_val = json_to_js_value_static(val, context)?;
                js_array.set(index, js_val, true, context)?;
            }
            Ok(js_array.into())
        }
        Value::Object(obj) => {
            let js_obj = JsObject::with_object_proto(context.intrinsics());
            for (key, val) in obj {
                let js_val = json_to_js_value_static(val, context)?;
                js_obj.set(boa_engine::js_string!(key.as_str()), js_val, true, context)?;
            }
            Ok(js_obj.into())
        }
    }
}
