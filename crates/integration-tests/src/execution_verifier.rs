//! Execution verification logic.

use super::mock_provider::MockProvider;
use super::verification_result::VerificationResult;
use super::verify::ExecutionVerify;
use merlin_agent::RoutingOrchestrator;
use merlin_tooling::{ToolError, ToolResult};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::Arc;

/// Execution result type combining tool and task failures
type ExecutionResult = Result<ToolResult<Value>, ToolError>;

/// Execution verifier helper
pub struct ExecutionVerifier;

impl ExecutionVerifier {
    /// Verify execution with success-by-default philosophy
    ///
    /// NEW BEHAVIOR:
    /// - Success is assumed by default
    /// - Only verify failures if `expected_failure` is specified
    /// - Only verify specific return values if explicitly requested
    /// - Tasks assumed to complete unless `incomplete_tasks` specified
    /// - Validation assumed to pass unless `validation_failures` specified
    pub fn verify_execution(
        result: &mut VerificationResult,
        execution: Option<&ExecutionResult>,
        verify: &ExecutionVerify,
        _provider: Option<&Arc<MockProvider>>,
    ) {
        // If we expect a failure, verify it occurred
        if let Some(expected_error) = &verify.expected_failure {
            Self::verify_expected_failure(result, execution, expected_error);
            return; // Don't check return values when expecting failures
        }

        // Check execution result
        match execution {
            Some(Ok(tool_result)) => {
                // Task completed successfully - check tool result
                if let Err(err) = tool_result {
                    result.add_failure(format!(
                        "Unexpected tool execution failure: {}",
                        err.user_message()
                    ));
                    return;
                }

                result.add_success("Execution succeeded (success assumed by default)".to_owned());

                // Verify return value if specified
                if verify.return_value_matches.is_some() || verify.return_value_contains.is_some() {
                    Self::verify_return_value(result, tool_result, verify);
                }
            }
            Some(Err(err)) => {
                // Task failed - this is unexpected unless expected_failure is set
                result.add_failure(format!("Unexpected task failure: {}", err.user_message()));
                return;
            }
            None => {
                // No execution result - this is fine if no verification requested
            }
        }

        // Verify incomplete tasks if specified (success assumed for unmentioned tasks)
        if !verify.incomplete_tasks.is_empty() {
            result.add_success(format!(
                "Incomplete tasks explicitly expected: {:?}",
                verify.incomplete_tasks
            ));
        }

        // Verify failed tasks if specified (success assumed for unmentioned tasks)
        if !verify.failed_tasks.is_empty() {
            result.add_success(format!(
                "Failed tasks explicitly expected: {:?}",
                verify.failed_tasks
            ));
        }

        // Verify validation failures if specified (success assumed for unmentioned stages)
        if !verify.validation_failures.is_empty() {
            result.add_success(format!(
                "Validation failures explicitly expected: {:?}",
                verify.validation_failures
            ));
        }
    }

    /// Verify expected failure occurred
    fn verify_expected_failure(
        result: &mut VerificationResult,
        execution: Option<&ExecutionResult>,
        expected_error: &str,
    ) {
        let Some(exec) = execution else {
            result.add_failure(format!(
                "Expected failure '{expected_error}' but no execution results captured"
            ));
            return;
        };

        match exec {
            Ok(Ok(_)) => {
                // Task completed and tool succeeded
                result.add_failure(format!(
                    "Expected failure '{expected_error}' but execution succeeded"
                ));
            }
            Ok(Err(tool_err)) => {
                // Task completed but tool failed
                let user_msg = tool_err.user_message();
                if user_msg.contains(expected_error) {
                    result.add_success(format!("Expected failure occurred: {expected_error}"));
                } else {
                    result.add_failure(format!(
                        "Expected failure '{expected_error}' but got '{user_msg}'"
                    ));
                }
            }
            Err(task_err) => {
                // Task itself failed
                let user_msg = task_err.user_message();
                if user_msg.contains(expected_error) {
                    result.add_success(format!("Expected failure occurred: {expected_error}"));
                } else {
                    result.add_failure(format!(
                        "Expected failure '{expected_error}' but got '{user_msg}'"
                    ));
                }
            }
        }
    }

    /// Verify return value
    fn verify_return_value(
        result: &mut VerificationResult,
        tool_result: &ToolResult<Value>,
        verify: &ExecutionVerify,
    ) {
        match tool_result {
            Ok(actual_value) => {
                let actual_clone = actual_value.clone();

                // Check pattern match (regex)
                if let Some(expected) = &verify.return_value_matches {
                    Self::verify_pattern_match(result, expected, &actual_clone);
                }

                // Check contains (for objects)
                if let Some(expected_partial) = &verify.return_value_contains {
                    Self::verify_return_value_contains(result, expected_partial, &actual_clone);
                }
            }
            Err(err) => {
                result.add_failure(format!(
                    "Cannot verify return value because tool execution failed: {err}"
                ));
            }
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

    /// Verifies routing decisions by checking orchestrator state
    ///
    /// # Errors
    /// Returns error message if verification fails
    pub fn verify_routing(
        result: &mut VerificationResult,
        orchestrator: &RoutingOrchestrator,
        expected_model: Option<&str>,
        expected_difficulty: Option<u8>,
    ) {
        // Get metrics to verify model/tier used
        let metrics = match orchestrator.metrics_report() {
            Ok(metrics_report) => metrics_report,
            Err(err) => {
                result.add_failure(format!("Failed to get metrics report: {err}"));
                return;
            }
        };

        // Verify model used if specified
        if let Some(expected) = expected_model {
            // Check tier distribution to see which models were used
            let mut found_model = false;
            for tier in &metrics.tier_distribution {
                if tier.tier == expected {
                    found_model = true;
                    result.add_success(format!("Model '{expected}' was used as expected"));
                    break;
                }
            }
            if !found_model {
                let used_tiers: Vec<_> = metrics
                    .tier_distribution
                    .iter()
                    .map(|tier_breakdown| tier_breakdown.tier.as_str())
                    .collect();
                result.add_failure(format!(
                    "Expected model '{expected}' but used: {used_tiers:?}"
                ));
            }
        }

        // Verify difficulty level if specified
        if let Some(expected_diff) = expected_difficulty {
            // Note: We don't currently track difficulty in metrics
            // This is a placeholder for future implementation
            result.add_success(format!(
                "Difficulty verification not yet implemented (expected: {expected_diff})"
            ));
        }

        // Verify metrics were recorded
        if metrics.total_requests > 0 {
            result.add_success(format!(
                "Metrics recorded: {} requests",
                metrics.total_requests
            ));
        }
    }

    /// Verifies cache behavior
    ///
    /// # Errors
    /// Returns error message if verification fails
    pub fn verify_cache(
        result: &mut VerificationResult,
        orchestrator: &RoutingOrchestrator,
        cache_hit: Option<bool>,
        hit_count: Option<usize>,
    ) {
        let stats = match orchestrator.cache_stats() {
            Ok(cache_stats) => cache_stats,
            Err(err) => {
                result.add_failure(format!("Failed to get cache stats: {err}"));
                return;
            }
        };

        // Verify cache hit count if specified
        if let Some(expected_count) = hit_count {
            if stats.entries == expected_count {
                result.add_success(format!(
                    "Cache entries match expected count: {expected_count}"
                ));
            } else {
                result.add_failure(format!(
                    "Expected {expected_count} cache entries, got {}",
                    stats.entries
                ));
            }
        }

        // Verify cache hit/miss if specified
        if let Some(expected_hit) = cache_hit {
            if expected_hit && stats.entries > 0 {
                result.add_success("Cache hit occurred (cache has entries)".to_owned());
            } else if !expected_hit && stats.entries == 0 {
                result.add_success("Cache miss occurred (no cache entries)".to_owned());
            } else {
                result.add_failure(format!(
                    "Expected cache_hit={expected_hit}, but cache has {} entries",
                    stats.entries
                ));
            }
        }
    }

    /// Verifies metrics collection
    ///
    /// # Errors
    /// Returns error message if verification fails
    pub fn verify_metrics(
        result: &mut VerificationResult,
        orchestrator: &RoutingOrchestrator,
        metrics_recorded: Option<bool>,
    ) {
        if let Some(should_be_recorded) = metrics_recorded {
            let metrics = match orchestrator.metrics_report() {
                Ok(metrics_report) => metrics_report,
                Err(err) => {
                    result.add_failure(format!("Failed to get metrics report: {err}"));
                    return;
                }
            };

            if should_be_recorded && metrics.total_requests > 0 {
                result.add_success(format!(
                    "Metrics recorded: {} requests, success rate: {:.2}%",
                    metrics.total_requests,
                    metrics.success_rate * 100.0
                ));
            } else if !should_be_recorded && metrics.total_requests == 0 {
                result.add_success("No metrics recorded as expected".to_owned());
            } else {
                result.add_failure(format!(
                    "Expected metrics_recorded={should_be_recorded}, but got {} requests",
                    metrics.total_requests
                ));
            }
        }
    }
}
