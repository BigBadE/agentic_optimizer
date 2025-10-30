//! Exit requirement validators for task list execution model

use std::{collections::HashMap, fs, path::Path, process::Command, result};

use merlin_core::{ExitRequirement, ValidationErrorType};
use merlin_deps::{
    regex::Regex,
    serde_json::{Value, from_str},
};

type Result<T> = result::Result<T, ValidationErrorType>;

/// Built-in exit requirement validators
pub struct ExitRequirementValidators;

impl ExitRequirementValidators {
    /// Validate an exit requirement against a result
    ///
    /// # Errors
    /// Returns `ValidationErrorType` if validation fails
    pub fn validate(
        requirement: &ExitRequirement,
        result: &str,
        workspace_root: &Path,
    ) -> Result<()> {
        match requirement {
            ExitRequirement::Callback {
                function_name,
                args,
            } => Self::validate_callback(function_name, args, result, workspace_root),

            ExitRequirement::Pattern { pattern } => Self::validate_pattern(pattern, result),

            ExitRequirement::Validation { validator } => {
                Self::validate_with_named_validator(validator, result);
                Ok(())
            }
        }
    }

    /// Validate using callback function
    ///
    /// # Errors
    /// Returns validation error if callback validation fails
    fn validate_callback(
        function_name: &str,
        args: &HashMap<String, Value>,
        _result: &str,
        workspace_root: &Path,
    ) -> Result<()> {
        match function_name {
            "file_exists" => Self::validator_file_exists(args, workspace_root),
            "file_contains" => Self::validator_file_contains(args, workspace_root),
            "command_succeeds" => Self::validator_command_succeeds(args, workspace_root),
            "json_valid" => Self::validator_json_valid(args),
            "no_errors_in" => Self::validator_no_errors_in(args),
            _ => Err(ValidationErrorType::Hard(format!(
                "Unknown validation function: {function_name}"
            ))),
        }
    }

    /// Validate using pattern matching
    ///
    /// # Errors
    /// Returns validation error if pattern doesn't match or regex is invalid
    fn validate_pattern(pattern: &str, result: &str) -> Result<()> {
        let regex = Regex::new(pattern).map_err(|err| {
            ValidationErrorType::Hard(format!("Invalid regex pattern '{pattern}': {err}"))
        })?;

        if regex.is_match(result) {
            Ok(())
        } else {
            Err(ValidationErrorType::Soft(format!(
                "Result does not match pattern '{pattern}'"
            )))
        }
    }

    /// Validate using named validator from pipeline
    fn validate_with_named_validator(_validator: &str, _result: &str) {
        // TODO: Integrate with ValidationPipeline
    }

    // ========================================================================
    // Built-in Callback Validators
    // ========================================================================

    /// Check if a file exists
    ///
    /// # Errors
    /// Returns validation error if path argument missing or file doesn't exist
    fn validator_file_exists(args: &HashMap<String, Value>, workspace_root: &Path) -> Result<()> {
        let path_str = args
            .get("path")
            .and_then(|val| val.as_str())
            .ok_or_else(|| ValidationErrorType::Hard("Missing 'path' argument".to_owned()))?;

        let full_path = workspace_root.join(path_str);

        if full_path.exists() {
            Ok(())
        } else {
            Err(ValidationErrorType::Soft(format!(
                "File does not exist: {}",
                full_path.display()
            )))
        }
    }

    /// Check if a file contains a pattern
    ///
    /// # Errors
    /// Returns validation error if arguments missing, file unreadable, or pattern not found
    fn validator_file_contains(args: &HashMap<String, Value>, workspace_root: &Path) -> Result<()> {
        let path_str = args
            .get("path")
            .and_then(|val| val.as_str())
            .ok_or_else(|| ValidationErrorType::Hard("Missing 'path' argument".to_owned()))?;

        let pattern = args
            .get("pattern")
            .and_then(|val| val.as_str())
            .ok_or_else(|| ValidationErrorType::Hard("Missing 'pattern' argument".to_owned()))?;

        let full_path = workspace_root.join(path_str);

        let content = fs::read_to_string(&full_path).map_err(|err| {
            ValidationErrorType::Soft(format!(
                "Failed to read file {}: {err}",
                full_path.display()
            ))
        })?;

        let regex = Regex::new(pattern).map_err(|err| {
            ValidationErrorType::Hard(format!("Invalid regex pattern '{pattern}': {err}"))
        })?;

        if regex.is_match(&content) {
            Ok(())
        } else {
            Err(ValidationErrorType::Soft(format!(
                "File {} does not contain pattern '{pattern}'",
                full_path.display()
            )))
        }
    }

    /// Check if a command succeeds (exit code 0)
    ///
    /// # Errors
    /// Returns validation error if cmd argument missing or command fails
    fn validator_command_succeeds(
        args: &HashMap<String, Value>,
        workspace_root: &Path,
    ) -> Result<()> {
        let cmd_str = args
            .get("cmd")
            .and_then(|val| val.as_str())
            .ok_or_else(|| ValidationErrorType::Hard("Missing 'cmd' argument".to_owned()))?;

        // Parse command and args
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ValidationErrorType::Hard("Empty command".to_owned()));
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(workspace_root)
            .output()
            .map_err(|err| {
                ValidationErrorType::Hard(format!("Failed to execute command '{cmd_str}': {err}"))
            })?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ValidationErrorType::Hard(format!(
                "Command '{cmd_str}' failed with exit code {:?}: {stderr}",
                output.status.code()
            )))
        }
    }

    /// Check if content is valid JSON
    ///
    /// # Errors
    /// Returns validation error if content argument missing or JSON is invalid
    fn validator_json_valid(args: &HashMap<String, Value>) -> Result<()> {
        let content = args
            .get("content")
            .and_then(|val| val.as_str())
            .ok_or_else(|| ValidationErrorType::Hard("Missing 'content' argument".to_owned()))?;

        from_str::<Value>(content)
            .map_err(|err| ValidationErrorType::Soft(format!("Invalid JSON: {err}")))?;

        Ok(())
    }

    /// Check for error patterns in output
    ///
    /// # Errors
    /// Returns validation error if output argument missing or error patterns found
    fn validator_no_errors_in(args: &HashMap<String, Value>) -> Result<()> {
        let output = args
            .get("output")
            .and_then(|val| val.as_str())
            .ok_or_else(|| ValidationErrorType::Hard("Missing 'output' argument".to_owned()))?;

        let error_patterns = [
            r"(?i)error:",
            r"(?i)panic:",
            r"(?i)fatal:",
            r"(?i)exception:",
            r"(?i)failed:",
        ];

        for pattern in &error_patterns {
            let regex = Regex::new(pattern)
                .map_err(|err| ValidationErrorType::Hard(format!("Regex error: {err}")))?;

            if regex.is_match(output) {
                return Err(ValidationErrorType::Hard(format!(
                    "Output contains error pattern: {pattern}"
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_pattern_validation_success() {
        let result = ExitRequirementValidators::validate_pattern(r"Success: \d+", "Success: 42");
        assert!(result.is_ok(), "Pattern validation should succeed");
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_pattern_validation_failure() {
        let result = ExitRequirementValidators::validate_pattern(r"Success: \d+", "Failed: 42");
        assert!(result.is_err());
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_json_valid_success() {
        let mut args = HashMap::new();
        args.insert(
            "content".to_owned(),
            Value::String(r#"{"key": "value"}"#.to_owned()),
        );
        let result = ExitRequirementValidators::validator_json_valid(&args);
        assert!(result.is_ok(), "JSON validation should succeed");
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_json_valid_failure() {
        let mut args = HashMap::new();
        args.insert(
            "content".to_owned(),
            Value::String("{invalid json}".to_owned()),
        );
        let result = ExitRequirementValidators::validator_json_valid(&args);
        assert!(result.is_err());
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_no_errors_in_success() {
        let mut args = HashMap::new();
        args.insert(
            "output".to_owned(),
            Value::String("All tests passed successfully".to_owned()),
        );
        let result = ExitRequirementValidators::validator_no_errors_in(&args);
        assert!(result.is_ok(), "No errors validation should succeed");
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_no_errors_in_failure() {
        let mut args = HashMap::new();
        args.insert(
            "output".to_owned(),
            Value::String("Error: Something went wrong".to_owned()),
        );
        let result = ExitRequirementValidators::validator_no_errors_in(&args);
        assert!(result.is_err());
    }
}
