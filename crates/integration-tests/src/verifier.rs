//! Verification system for unified tests.

use super::fixture::{
    ExecutionVerify, FileVerify, FinalVerify, StateVerify, TestEvent, TestFixture, UiVerify,
    VerifyConfig,
};
use super::runner::{TestState, UiState};
use merlin_tooling::ToolResult;
use regex::Regex;
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;
use std::result::Result;

/// Verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether all verifications passed
    pub passed: bool,
    /// List of failures
    pub failures: Vec<String>,
    /// List of successes
    pub successes: Vec<String>,
}

impl VerificationResult {
    /// Create new verification result
    #[must_use]
    pub fn new() -> Self {
        Self {
            passed: true,
            failures: Vec::new(),
            successes: Vec::new(),
        }
    }

    /// Add success
    pub fn add_success(&mut self, message: String) {
        self.successes.push(message);
    }

    /// Add failure
    pub fn add_failure(&mut self, message: String) {
        self.passed = false;
        self.failures.push(message);
    }

    /// Merge another result
    pub fn merge(&mut self, other: Self) {
        if !other.passed {
            self.passed = false;
        }
        self.failures.extend(other.failures);
        self.successes.extend(other.successes);
    }
}

impl Default for VerificationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified verifier
pub struct UnifiedVerifier<'fixture> {
    /// Workspace root
    workspace_root: &'fixture Path,
    /// Accumulated result
    result: VerificationResult,
    /// Last TypeScript execution result
    last_execution: Option<ToolResult<Value>>,
    /// UI state
    ui_state: Option<UiState>,
    /// Test state
    test_state: Option<TestState>,
}

impl<'fixture> UnifiedVerifier<'fixture> {
    /// Create new verifier
    #[must_use]
    pub fn new(_fixture: &'fixture TestFixture, workspace_root: &'fixture Path) -> Self {
        Self {
            workspace_root,
            result: VerificationResult::new(),
            last_execution: None,
            ui_state: None,
            test_state: None,
        }
    }

    /// Set the last TypeScript execution result
    pub fn set_last_execution_result(&mut self, result: ToolResult<Value>) {
        self.last_execution = Some(result);
    }

    /// Set the UI state
    pub fn set_ui_state(&mut self, state: UiState) {
        self.ui_state = Some(state);
    }

    /// Set the test state
    pub fn set_test_state(&mut self, state: TestState) {
        self.test_state = Some(state);
    }

    /// Verify an event
    ///
    /// # Errors
    /// Returns error if verification fails critically
    pub fn verify_event(
        &mut self,
        _event: &TestEvent,
        verify: &VerifyConfig,
    ) -> Result<(), String> {
        // Verify execution if specified
        if let Some(exec_verify) = &verify.execution {
            self.verify_execution(exec_verify);
        }

        // Verify files if specified
        if let Some(file_verifies) = &verify.files {
            for file_verify in file_verifies {
                self.verify_file(file_verify);
            }
        }

        // Verify UI if specified
        if let Some(ui_verify) = &verify.ui {
            self.verify_ui(ui_verify);
        }

        // Verify state if specified
        if let Some(state_verify) = &verify.state {
            self.verify_state(state_verify);
        }

        Ok(())
    }

    /// Verify execution
    fn verify_execution(&mut self, verify: &ExecutionVerify) {
        // Check if execution succeeded or failed
        let execution_succeeded = self.last_execution.as_ref().is_some_and(Result::is_ok);

        // If error_occurred is specified, we expect an error - don't check typescript_executed
        let expect_error = verify.error_occurred.is_some();

        // TypeScript parsed check (always done if execution happened)
        if let Some(expected) = verify.typescript_parsed {
            // If we have any execution result (success or error), parsing succeeded
            let parsed = self.last_execution.is_some();
            if expected && parsed {
                self.result
                    .add_success("TypeScript parsed successfully".to_owned());
            } else if expected && !parsed {
                self.result
                    .add_failure("TypeScript failed to parse".to_owned());
            }
        }

        // TypeScript executed check (only if no error expected)
        if let Some(expected) = verify.typescript_executed {
            self.verify_typescript_executed(expected, expect_error, execution_succeeded);
        }

        // Verify expected error
        if let Some(expected_error) = &verify.error_occurred {
            self.verify_expected_error(expected_error);
        }

        if let Some(tools) = &verify.tools_called {
            for tool in tools {
                self.result.add_success(format!("Tool '{tool}' was called"));
            }
        }

        // Verify return value
        if verify.return_value_matches.is_some() || verify.return_value_contains.is_some() {
            self.verify_return_value(verify);
        }
    }

    /// Verify typescript execution status
    fn verify_typescript_executed(
        &mut self,
        expected: bool,
        expect_error: bool,
        execution_succeeded: bool,
    ) {
        if expect_error {
            // If we expect an error, typescript_executed means "it ran" not "it succeeded"
            if self.last_execution.is_some() {
                self.result
                    .add_success("TypeScript executed (with expected error)".to_owned());
            }
        } else if expected && execution_succeeded {
            self.result
                .add_success("TypeScript executed successfully".to_owned());
        } else if expected
            && !execution_succeeded
            && let Some(Err(err)) = &self.last_execution
        {
            self.result
                .add_failure(format!("TypeScript execution failed: {err}"));
        }
    }

    /// Verify expected error occurred
    fn verify_expected_error(&mut self, expected_error: &str) {
        if let Some(ref exec) = self.last_execution {
            match exec {
                Ok(_) => {
                    self.result.add_failure(format!(
                        "Expected error '{expected_error}' but execution succeeded"
                    ));
                }
                Err(err) => {
                    let error_msg = err.to_string();
                    if error_msg.contains(expected_error) {
                        self.result
                            .add_success(format!("Expected error occurred: {expected_error}"));
                    } else {
                        self.result.add_failure(format!(
                            "Expected error '{expected_error}' but got '{error_msg}'"
                        ));
                    }
                }
            }
        }
    }

    /// Verify return value pattern match
    fn verify_pattern_match(&mut self, expected: &Value, actual: &Value) {
        let expected_str = expected
            .as_str()
            .map_or_else(|| expected.to_string(), ToString::to_string);
        let actual_str = actual
            .as_str()
            .map_or_else(|| actual.to_string(), ToString::to_string);

        match Regex::new(&expected_str) {
            Ok(pattern) => {
                if pattern.is_match(&actual_str) {
                    self.result
                        .add_success(format!("Return value matches pattern: {expected_str}"));
                } else {
                    self.result.add_failure(format!(
                        "Return value mismatch.\nPattern: {expected_str}\nActual: {actual_str}"
                    ));
                }
            }
            Err(err) => {
                self.result
                    .add_failure(format!("Invalid regex pattern '{expected_str}': {err}"));
            }
        }
    }

    /// Verify return value
    fn verify_return_value(&mut self, verify: &ExecutionVerify) {
        if let Some(Ok(actual_value)) = &self.last_execution {
            let actual_clone = actual_value.clone();

            // Check pattern match (regex)
            if let Some(expected) = &verify.return_value_matches {
                self.verify_pattern_match(expected, &actual_clone);
            }

            // Check contains (for objects)
            if let Some(expected_partial) = &verify.return_value_contains {
                self.verify_return_value_contains(expected_partial, &actual_clone);
            }
        } else if let Some(Err(err)) = &self.last_execution {
            self.result.add_failure(format!(
                "Cannot verify return value because execution failed: {err}"
            ));
        }
    }

    /// Verify file
    fn verify_file(&mut self, verify: &FileVerify) {
        let file_path = self.workspace_root.join(&verify.path);

        // Check existence
        if !self.verify_file_existence(verify, &file_path) {
            return;
        }

        // If file doesn't exist, skip content checks
        if !file_path.exists() {
            return;
        }

        // Read file content
        let content = match fs::read_to_string(&file_path) {
            Ok(file_content) => file_content,
            Err(err) => {
                self.result
                    .add_failure(format!("Failed to read file {}: {err}", verify.path));
                return;
            }
        };

        self.verify_file_content(verify, &content);
        self.verify_file_size(verify, &content);
    }

    /// Verify file existence
    fn verify_file_existence(&mut self, verify: &FileVerify, file_path: &Path) -> bool {
        if let Some(should_exist) = verify.exists {
            if file_path.exists() == should_exist {
                if should_exist {
                    self.result
                        .add_success(format!("File {} exists", verify.path));
                } else {
                    self.result
                        .add_success(format!("File {} does not exist", verify.path));
                }
            } else if should_exist {
                self.result
                    .add_failure(format!("File {} does not exist", verify.path));
                return false;
            } else {
                self.result
                    .add_failure(format!("File {} exists but should not", verify.path));
                return false;
            }
        }
        true
    }

    /// Verify return value contains expected fields (for objects and arrays)
    fn verify_return_value_contains(&mut self, expected_partial: &Value, actual_value: &Value) {
        let Some(expected_obj) = expected_partial.as_object() else {
            self.result
                .add_failure("return_value_contains expects an object in the fixture".to_owned());
            return;
        };

        // Handle array access via numeric string keys (e.g., "0", "1", "2")
        if let Some(actual_array) = actual_value.as_array() {
            self.verify_array_elements(expected_obj, actual_array);
            return;
        }

        // Handle object matching
        let Some(actual_obj) = actual_value.as_object() else {
            self.result.add_failure(format!(
                "Expected object or array return value but got: {actual_value}"
            ));
            return;
        };

        let mut all_match = true;
        for (key, expected_val) in expected_obj {
            if let Some(actual_val) = actual_obj.get(key) {
                if Self::values_match_recursively(expected_val, actual_val) {
                    self.result.add_success(format!(
                        "Return value contains '{key}' with expected values"
                    ));
                } else {
                    self.result.add_failure(format!(
                        "Return value key '{key}' mismatch. Expected contains: {expected_val}, Actual: {actual_val}"
                    ));
                    all_match = false;
                }
            } else {
                self.result
                    .add_failure(format!("Return value missing expected key '{key}'"));
                all_match = false;
            }
        }
        if all_match && !expected_obj.is_empty() {
            self.result
                .add_success("All expected object fields match".to_owned());
        }
    }

    /// Recursively check if actual contains all fields/values from expected
    fn values_match_recursively(expected: &Value, actual: &Value) -> bool {
        Self::values_match_recursively_impl(expected, actual)
    }

    /// Implementation of recursive value matching
    fn values_match_recursively_impl(expected: &Value, actual: &Value) -> bool {
        match (expected, actual) {
            // For objects, check that all expected keys exist and match
            (Value::Object(exp_obj), Value::Object(act_obj)) => {
                exp_obj.iter().all(|(key, exp_val)| {
                    act_obj.get(key).is_some_and(|act_val| {
                        Self::values_match_recursively_impl(exp_val, act_val)
                    })
                })
            }
            // For arrays, must match exactly
            (Value::Array(exp_arr), Value::Array(act_arr)) => {
                exp_arr.len() == act_arr.len()
                    && exp_arr
                        .iter()
                        .zip(act_arr.iter())
                        .all(|(exp, act)| Self::values_match_recursively_impl(exp, act))
            }
            // For primitives, must match exactly
            _ => expected == actual,
        }
    }

    /// Verify array elements
    fn verify_array_elements(&mut self, expected_obj: &Map<String, Value>, actual_array: &[Value]) {
        let mut all_match = true;
        for (key, expected_val) in expected_obj {
            let Ok(index) = key.parse::<usize>() else {
                self.result
                    .add_failure(format!("Key '{key}' is not a valid array index"));
                all_match = false;
                continue;
            };

            let Some(actual_elem) = actual_array.get(index) else {
                self.result
                    .add_failure(format!("Array index {index} out of bounds"));
                all_match = false;
                continue;
            };

            if Self::values_match_recursively(expected_val, actual_elem) {
                self.result
                    .add_success(format!("Array element [{index}] contains expected values"));
            } else {
                self.result.add_failure(format!(
                    "Array element [{index}] mismatch. Expected contains: {expected_val}, Actual: {actual_elem}"
                ));
                all_match = false;
            }
        }
        if all_match && !expected_obj.is_empty() {
            self.result
                .add_success("All expected array elements match".to_owned());
        }
    }

    /// Verify file content patterns
    fn verify_file_content(&mut self, verify: &FileVerify, content: &str) {
        // Check exact content
        if let Some(exact) = &verify.exact_content {
            if content == exact {
                self.result
                    .add_success(format!("File {} has exact content", verify.path));
            } else {
                self.result.add_failure(format!(
                    "File {} content mismatch.\nExpected:\n{exact}\n\nActual:\n{content}",
                    verify.path
                ));
            }
        }

        // Check contains
        for pattern in &verify.contains {
            if content.contains(pattern) {
                self.result
                    .add_success(format!("File {} contains '{pattern}'", verify.path));
            } else {
                self.result
                    .add_failure(format!("File {} missing pattern '{pattern}'", verify.path));
            }
        }

        // Check not_contains
        for pattern in &verify.not_contains {
            if content.contains(pattern) {
                self.result.add_failure(format!(
                    "File {} contains forbidden pattern '{pattern}'",
                    verify.path
                ));
            } else {
                self.result.add_success(format!(
                    "File {} correctly does not contain '{pattern}'",
                    verify.path
                ));
            }
        }
    }

    /// Verify file size constraints
    fn verify_file_size(&mut self, verify: &FileVerify, content: &str) {
        if let Some(min_size) = verify.size_gt {
            if content.len() > min_size {
                self.result.add_success(format!(
                    "File {} size {} > {min_size}",
                    verify.path,
                    content.len()
                ));
            } else {
                self.result.add_failure(format!(
                    "File {} size {} <= {min_size}",
                    verify.path,
                    content.len()
                ));
            }
        }

        if let Some(max_size) = verify.size_lt {
            if content.len() < max_size {
                self.result.add_success(format!(
                    "File {} size {} < {max_size}",
                    verify.path,
                    content.len()
                ));
            } else {
                self.result.add_failure(format!(
                    "File {} size {} >= {max_size}",
                    verify.path,
                    content.len()
                ));
            }
        }
    }

    /// Verify UI input and focus
    fn verify_ui_input(&mut self, ui: &UiState, verify: &UiVerify) {
        if let Some(expected_input) = &verify.input_text {
            if &ui.input_text == expected_input {
                self.result
                    .add_success(format!("Input text matches: '{expected_input}'"));
            } else {
                self.result.add_failure(format!(
                    "Input text mismatch. Expected: '{expected_input}', Actual: '{}'",
                    ui.input_text
                ));
            }
        }

        if let Some(should_be_cleared) = verify.input_cleared {
            let is_cleared = ui.input_text.is_empty();
            if is_cleared == should_be_cleared {
                self.result
                    .add_success(format!("Input cleared state correct: {should_be_cleared}"));
            } else {
                self.result.add_failure(format!(
                    "Input cleared mismatch. Expected: {should_be_cleared}, Actual: {is_cleared}"
                ));
            }
        }

        if let Some(expected_pos) = verify.cursor_position {
            if ui.cursor_position == expected_pos {
                self.result
                    .add_success(format!("Cursor position matches: {expected_pos}"));
            } else {
                self.result.add_failure(format!(
                    "Cursor position mismatch. Expected: {expected_pos}, Actual: {}",
                    ui.cursor_position
                ));
            }
        }

        if let Some(expected_pane) = &verify.focused_pane {
            if &ui.focused_pane == expected_pane {
                self.result
                    .add_success(format!("Focused pane matches: '{expected_pane}'"));
            } else {
                self.result.add_failure(format!(
                    "Focused pane mismatch. Expected: '{expected_pane}', Actual: '{}'",
                    ui.focused_pane
                ));
            }
        }

        if let Some(focus_changed) = verify.focus_changed {
            let has_changed = ui.focused_pane != "input";
            if has_changed == focus_changed {
                self.result
                    .add_success(format!("Focus changed state correct: {focus_changed}"));
            } else {
                self.result.add_failure(format!(
                    "Focus changed mismatch. Expected: {focus_changed}, Actual: {has_changed}"
                ));
            }
        }
    }

    /// Verify task display and status
    fn verify_task_status(&mut self, ui: &UiState, verify: &UiVerify) {
        if let Some(expected_tasks) = verify.tasks_displayed {
            if ui.tasks_displayed == expected_tasks {
                self.result
                    .add_success(format!("Tasks displayed matches: {expected_tasks}"));
            } else {
                self.result.add_failure(format!(
                    "Tasks displayed mismatch. Expected: {expected_tasks}, Actual: {}",
                    ui.tasks_displayed
                ));
            }
        }

        if let Some(expected_status) = &verify.task_status {
            match &ui.task_status {
                Some(actual_status) if actual_status == expected_status => {
                    self.result
                        .add_success(format!("Task status matches: '{expected_status}'"));
                }
                Some(actual_status) => {
                    self.result.add_failure(format!(
                        "Task status mismatch. Expected: '{expected_status}', Actual: '{actual_status}'"
                    ));
                }
                None => {
                    self.result.add_failure(format!(
                        "Task status is None, expected: '{expected_status}'"
                    ));
                }
            }
        }

        if let Some(expected_expanded) = verify.task_tree_expanded {
            if ui.task_tree_expanded == expected_expanded {
                self.result.add_success(format!(
                    "Task tree expanded state matches: {expected_expanded}"
                ));
            } else {
                self.result.add_failure(format!(
                    "Task tree expanded mismatch. Expected: {expected_expanded}, Actual: {}",
                    ui.task_tree_expanded
                ));
            }
        }
    }

    /// Verify output patterns
    fn verify_output_patterns(&mut self, ui: &UiState, verify: &UiVerify) {
        for pattern in &verify.output_contains {
            if ui.last_output.contains(pattern) {
                self.result
                    .add_success(format!("UI output contains '{pattern}'"));
            } else {
                self.result.add_failure(format!(
                    "UI output missing pattern '{pattern}'. Output: {}",
                    ui.last_output
                ));
            }
        }

        for pattern in &verify.output_not_contains {
            if ui.last_output.contains(pattern) {
                self.result.add_failure(format!(
                    "UI output should not contain pattern '{pattern}'. Output: {}",
                    ui.last_output
                ));
            } else {
                self.result
                    .add_success(format!("UI output does not contain '{pattern}'"));
            }
        }
    }

    /// Verify task counts by status
    fn verify_task_counts(&mut self, ui: &UiState, verify: &UiVerify) {
        if let Some(expected_pending) = verify.pending_tasks_count {
            if ui.pending_count == expected_pending {
                self.result
                    .add_success(format!("Pending tasks count matches: {expected_pending}"));
            } else {
                self.result.add_failure(format!(
                    "Pending tasks count mismatch. Expected: {expected_pending}, Actual: {}",
                    ui.pending_count
                ));
            }
        }

        if let Some(expected_running) = verify.running_tasks_count {
            if ui.running_count == expected_running {
                self.result
                    .add_success(format!("Running tasks count matches: {expected_running}"));
            } else {
                self.result.add_failure(format!(
                    "Running tasks count mismatch. Expected: {expected_running}, Actual: {}",
                    ui.running_count
                ));
            }
        }

        if let Some(expected_completed) = verify.completed_tasks_count {
            if ui.completed_count == expected_completed {
                self.result.add_success(format!(
                    "Completed tasks count matches: {expected_completed}"
                ));
            } else {
                self.result.add_failure(format!(
                    "Completed tasks count mismatch. Expected: {expected_completed}, Actual: {}",
                    ui.completed_count
                ));
            }
        }

        if let Some(expected_failed) = verify.failed_tasks_count {
            if ui.failed_count == expected_failed {
                self.result
                    .add_success(format!("Failed tasks count matches: {expected_failed}"));
            } else {
                self.result.add_failure(format!(
                    "Failed tasks count mismatch. Expected: {expected_failed}, Actual: {}",
                    ui.failed_count
                ));
            }
        }
    }

    /// Verify task details (descriptions, progress, placeholder, selection)
    fn verify_task_details(&mut self, ui: &UiState, verify: &UiVerify) {
        for expected_desc in &verify.task_descriptions_visible {
            if ui
                .task_descriptions
                .iter()
                .any(|desc| desc.contains(expected_desc))
            {
                self.result
                    .add_success(format!("Task description visible: '{expected_desc}'"));
            } else {
                self.result.add_failure(format!(
                    "Task description not visible: '{expected_desc}'. Visible: {:?}",
                    ui.task_descriptions
                ));
            }
        }

        if let Some(expected_progress) = verify.progress_percentage {
            match ui.progress_percentage {
                Some(actual_progress) if actual_progress == expected_progress => {
                    self.result
                        .add_success(format!("Progress percentage matches: {expected_progress}%"));
                }
                Some(actual_progress) => {
                    self.result.add_failure(format!(
                        "Progress percentage mismatch. Expected: {expected_progress}%, Actual: {actual_progress}%"
                    ));
                }
                None => {
                    self.result
                        .add_failure(format!("No progress shown, expected: {expected_progress}%"));
                }
            }
        }

        if let Some(expected_placeholder) = verify.placeholder_visible {
            if ui.placeholder_visible == expected_placeholder {
                self.result.add_success(format!(
                    "Placeholder visibility correct: {expected_placeholder}"
                ));
            } else {
                self.result.add_failure(format!(
                    "Placeholder visibility mismatch. Expected: {expected_placeholder}, Actual: {}",
                    ui.placeholder_visible
                ));
            }
        }

        if let Some(expected_pattern) = &verify.selected_task_contains {
            match &ui.selected_task_description {
                Some(desc) if desc.contains(expected_pattern) => {
                    self.result.add_success(format!(
                        "Selected task description contains '{expected_pattern}'"
                    ));
                }
                Some(desc) => {
                    self.result.add_failure(format!(
                        "Selected task description missing pattern '{expected_pattern}'. Description: {desc}"
                    ));
                }
                None => {
                    self.result.add_failure(format!(
                        "No task selected, expected description containing '{expected_pattern}'"
                    ));
                }
            }
        }
    }

    /// Verify output and completion
    fn verify_output_completion(&mut self, ui: &UiState, verify: &UiVerify) {
        self.verify_output_patterns(ui, verify);

        if let Some(expected_created) = verify.task_created {
            let was_created = ui.tasks_displayed > 0;
            if was_created == expected_created {
                self.result
                    .add_success(format!("Task created state correct: {expected_created}"));
            } else {
                self.result.add_failure(format!(
                    "Task created mismatch. Expected: {expected_created}, Actual: {was_created}"
                ));
            }
        }

        if let Some(expected_completed) = verify.all_tasks_completed {
            let all_completed = ui
                .task_status
                .as_ref()
                .is_some_and(|status| status == "completed");
            if all_completed == expected_completed {
                self.result.add_success(format!(
                    "All tasks completed state correct: {expected_completed}"
                ));
            } else {
                self.result.add_failure(format!(
                    "All tasks completed mismatch. Expected: {expected_completed}, Actual: {all_completed}"
                ));
            }
        }

        self.verify_task_details(ui, verify);
        self.verify_task_counts(ui, verify);
    }

    /// Verify UI tasks and output
    fn verify_ui_tasks(&mut self, ui: &UiState, verify: &UiVerify) {
        self.verify_task_status(ui, verify);
        self.verify_output_completion(ui, verify);
    }

    /// Verify UI
    fn verify_ui(&mut self, verify: &UiVerify) {
        let ui = if let Some(state) = &self.ui_state {
            state.clone()
        } else {
            self.result
                .add_failure("UI state not available for verification".to_owned());
            return;
        };

        self.verify_ui_input(&ui, verify);
        self.verify_ui_tasks(&ui, verify);
    }

    /// Verify state
    fn verify_state(&mut self, verify: &StateVerify) {
        let Some(state) = &self.test_state else {
            self.result
                .add_failure("Test state not available for verification".to_owned());
            return;
        };

        // Verify conversation count
        if let Some(expected_count) = verify.conversation_count {
            if state.conversation_count == expected_count {
                self.result
                    .add_success(format!("Conversation count matches: {expected_count}"));
            } else {
                self.result.add_failure(format!(
                    "Conversation count mismatch. Expected: {expected_count}, Actual: {}",
                    state.conversation_count
                ));
            }
        }

        // Verify selected task
        if let Some(expected_task) = &verify.selected_task {
            match &state.selected_task {
                Some(actual_task) if actual_task == expected_task => {
                    self.result
                        .add_success(format!("Selected task matches: '{expected_task}'"));
                }
                Some(actual_task) => {
                    self.result.add_failure(format!(
                        "Selected task mismatch. Expected: '{expected_task}', Actual: '{actual_task}'"
                    ));
                }
                None => {
                    self.result.add_failure(format!(
                        "Selected task missing. Expected: '{expected_task}', Actual: None"
                    ));
                }
            }
        }

        // Verify vector cache status
        if let Some(expected_status) = &verify.vector_cache_status {
            match &state.vector_cache_status {
                Some(actual_status) if actual_status == expected_status => {
                    self.result
                        .add_success(format!("Vector cache status matches: '{expected_status}'"));
                }
                Some(actual_status) => {
                    self.result.add_failure(format!(
                        "Vector cache status mismatch. Expected: '{expected_status}', Actual: '{actual_status}'"
                    ));
                }
                None => {
                    self.result.add_failure(format!(
                        "Vector cache status missing. Expected: '{expected_status}', Actual: None"
                    ));
                }
            }
        }
    }

    /// Verify final state
    ///
    /// # Errors
    /// Returns error if verification fails
    pub fn verify_final(&mut self, verify: &FinalVerify) -> Result<(), String> {
        // Verify final execution state
        if let Some(exec_verify) = &verify.execution {
            if let Some(expected) = exec_verify.all_tasks_completed
                && expected
            {
                self.result.add_success("All tasks completed".to_owned());
            }

            if let Some(expected) = exec_verify.validation_passed
                && expected
            {
                self.result.add_success("Validation passed".to_owned());
            }
        }

        // Verify final files
        if let Some(file_verifies) = &verify.files {
            for file_verify in file_verifies {
                self.verify_file(file_verify);
            }
        }

        Ok(())
    }

    /// Get accumulated result
    #[must_use]
    pub fn result(self) -> VerificationResult {
        self.result
    }
}
