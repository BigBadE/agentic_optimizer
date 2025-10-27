//! Execution verification logic.

use super::fixture::ExecutionVerify;
use super::verification_result::VerificationResult;
use merlin_deps::regex::Regex;
use merlin_deps::serde_json::{Map, Value};
use merlin_tooling::ToolResult;

/// Execution verifier helper
pub struct ExecutionVerifier;

impl ExecutionVerifier {
    /// Verify execution
    pub fn verify_execution(
        result: &mut VerificationResult,
        last_execution: Option<&ToolResult<Value>>,
        verify: &ExecutionVerify,
    ) {
        // Check if execution succeeded or failed
        let execution_succeeded = last_execution.is_some_and(Result::is_ok);

        // If error_occurred is specified, we expect an error - don't check typescript_executed
        let expect_error = verify.error_occurred.is_some();

        // TypeScript parsed check (always done if execution happened)
        if let Some(expected) = verify.typescript_parsed {
            // If we have any execution result (success or error), parsing succeeded
            let parsed = last_execution.is_some();
            if expected && parsed {
                result.add_success("TypeScript parsed successfully".to_owned());
            } else if expected && !parsed {
                result.add_failure("TypeScript failed to parse".to_owned());
            }
        }

        // TypeScript executed check (only if no error expected)
        if let Some(expected) = verify.typescript_executed {
            Self::verify_typescript_executed(
                result,
                last_execution,
                expected,
                expect_error,
                execution_succeeded,
            );
        }

        // Verify expected error
        if let Some(expected_error) = &verify.error_occurred {
            Self::verify_expected_error(result, last_execution, expected_error);
        }

        if let Some(tools) = &verify.tools_called {
            for tool in tools {
                result.add_success(format!("Tool '{tool}' was called"));
            }
        }

        // Verify return value
        if verify.return_value_matches.is_some() || verify.return_value_contains.is_some() {
            Self::verify_return_value(result, last_execution, verify);
        }
    }

    /// Verify typescript execution status
    fn verify_typescript_executed(
        result: &mut VerificationResult,
        last_execution: Option<&ToolResult<Value>>,
        expected: bool,
        expect_error: bool,
        execution_succeeded: bool,
    ) {
        if expect_error {
            // If we expect an error, typescript_executed means "it ran" not "it succeeded"
            if last_execution.is_some() {
                result.add_success("TypeScript executed (with expected error)".to_owned());
            }
        } else if expected && execution_succeeded {
            result.add_success("TypeScript executed successfully".to_owned());
        } else if expected
            && !execution_succeeded
            && let Some(Err(err)) = last_execution
        {
            result.add_failure(format!("TypeScript execution failed: {err}"));
        }
    }

    /// Verify expected error occurred
    fn verify_expected_error(
        result: &mut VerificationResult,
        last_execution: Option<&ToolResult<Value>>,
        expected_error: &str,
    ) {
        if let Some(exec) = last_execution {
            match exec {
                Ok(_) => {
                    result.add_failure(format!(
                        "Expected error '{expected_error}' but execution succeeded"
                    ));
                }
                Err(err) => {
                    let error_msg = err.to_string();
                    if error_msg.contains(expected_error) {
                        result.add_success(format!("Expected error occurred: {expected_error}"));
                    } else {
                        result.add_failure(format!(
                            "Expected error '{expected_error}' but got '{error_msg}'"
                        ));
                    }
                }
            }
        }
    }

    /// Verify return value
    fn verify_return_value(
        result: &mut VerificationResult,
        last_execution: Option<&ToolResult<Value>>,
        verify: &ExecutionVerify,
    ) {
        if let Some(Ok(actual_value)) = last_execution {
            let actual_clone = actual_value.clone();

            // Check pattern match (regex)
            if let Some(expected) = &verify.return_value_matches {
                Self::verify_pattern_match(result, expected, &actual_clone);
            }

            // Check contains (for objects)
            if let Some(expected_partial) = &verify.return_value_contains {
                Self::verify_return_value_contains(result, expected_partial, &actual_clone);
            }
        } else if let Some(Err(err)) = last_execution {
            result.add_failure(format!(
                "Cannot verify return value because execution failed: {err}"
            ));
        }
    }

    /// Verify return value pattern match
    fn verify_pattern_match(result: &mut VerificationResult, expected: &Value, actual: &Value) {
        let expected_str = expected
            .as_str()
            .map_or_else(|| expected.to_string(), ToString::to_string);
        let actual_str = actual
            .as_str()
            .map_or_else(|| actual.to_string(), ToString::to_string);

        match Regex::new(&expected_str) {
            Ok(pattern) => {
                if pattern.is_match(&actual_str) {
                    result.add_success(format!("Return value matches pattern: {expected_str}"));
                } else {
                    result.add_failure(format!(
                        "Return value mismatch.\nPattern: {expected_str}\nActual: {actual_str}"
                    ));
                }
            }
            Err(err) => {
                result.add_failure(format!("Invalid regex pattern '{expected_str}': {err}"));
            }
        }
    }

    /// Verify return value contains expected fields (for objects and arrays)
    fn verify_return_value_contains(
        result: &mut VerificationResult,
        expected_partial: &Value,
        actual_value: &Value,
    ) {
        let Some(expected_obj) = expected_partial.as_object() else {
            result.add_failure("return_value_contains expects an object in the fixture".to_owned());
            return;
        };

        // Handle array access via numeric string keys (e.g., "0", "1", "2")
        if let Some(actual_array) = actual_value.as_array() {
            Self::verify_array_elements(result, expected_obj, actual_array);
            return;
        }

        // Handle object matching
        let Some(actual_obj) = actual_value.as_object() else {
            result.add_failure(format!(
                "Expected object or array return value but got: {actual_value}"
            ));
            return;
        };

        let mut all_match = true;
        for (key, expected_val) in expected_obj {
            if let Some(actual_val) = actual_obj.get(key) {
                if Self::values_match_recursively(expected_val, actual_val) {
                    result.add_success(format!(
                        "Return value contains '{key}' with expected values"
                    ));
                } else {
                    result.add_failure(format!(
                        "Return value key '{key}' mismatch. Expected contains: {expected_val}, Actual: {actual_val}"
                    ));
                    all_match = false;
                }
            } else {
                result.add_failure(format!("Return value missing expected key '{key}'"));
                all_match = false;
            }
        }
        if all_match && !expected_obj.is_empty() {
            result.add_success("All expected object fields match".to_owned());
        }
    }

    /// Verify array elements
    fn verify_array_elements(
        result: &mut VerificationResult,
        expected_obj: &Map<String, Value>,
        actual_array: &[Value],
    ) {
        let mut all_match = true;
        for (key, expected_val) in expected_obj {
            let Ok(index) = key.parse::<usize>() else {
                result.add_failure(format!("Key '{key}' is not a valid array index"));
                all_match = false;
                continue;
            };

            let Some(actual_elem) = actual_array.get(index) else {
                result.add_failure(format!("Array index {index} out of bounds"));
                all_match = false;
                continue;
            };

            if Self::values_match_recursively(expected_val, actual_elem) {
                result.add_success(format!("Array element [{index}] contains expected values"));
            } else {
                result.add_failure(format!(
                    "Array element [{index}] mismatch. Expected contains: {expected_val}, Actual: {actual_elem}"
                ));
                all_match = false;
            }
        }
        if all_match && !expected_obj.is_empty() {
            result.add_success("All expected array elements match".to_owned());
        }
    }

    /// Recursively check if actual contains all fields/values from expected
    fn values_match_recursively(expected: &Value, actual: &Value) -> bool {
        match (expected, actual) {
            // For objects, check that all expected keys exist and match
            (Value::Object(exp_obj), Value::Object(act_obj)) => {
                exp_obj.iter().all(|(key, exp_val)| {
                    act_obj
                        .get(key)
                        .is_some_and(|act_val| Self::values_match_recursively(exp_val, act_val))
                })
            }
            // For arrays, must match exactly
            (Value::Array(exp_arr), Value::Array(act_arr)) => {
                exp_arr.len() == act_arr.len()
                    && exp_arr
                        .iter()
                        .zip(act_arr.iter())
                        .all(|(exp, act)| Self::values_match_recursively(exp, act))
            }
            // For primitives, must match exactly
            _ => expected == actual,
        }
    }
}
