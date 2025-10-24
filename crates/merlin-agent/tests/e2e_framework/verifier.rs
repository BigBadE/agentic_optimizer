//! Comprehensive verification system for E2E test results.

use super::fixture::{E2EFixture, FileVerification, ResponseVerification};
use super::mock_provider::{CallRecord, StatefulMockProvider};
use merlin_core::TaskResult;
use std::fs;
use std::path::Path;

/// Verification result with detailed error messages
#[derive(Debug)]
pub struct VerificationResult {
    /// Whether verification passed
    pub passed: bool,
    /// List of verification failures
    pub failures: Vec<String>,
    /// List of verification successes (for detailed reporting)
    pub successes: Vec<String>,
}

impl VerificationResult {
    /// Create a new verification result
    fn new() -> Self {
        Self {
            passed: true,
            failures: Vec::new(),
            successes: Vec::new(),
        }
    }

    /// Add a failure
    fn add_failure(&mut self, message: String) {
        self.passed = false;
        self.failures.push(message);
    }

    /// Add a success
    fn add_success(&mut self, message: String) {
        self.successes.push(message);
    }

    /// Merge another verification result into this one
    fn merge(&mut self, other: VerificationResult) {
        if !other.passed {
            self.passed = false;
        }
        self.failures.extend(other.failures);
        self.successes.extend(other.successes);
    }
}

/// Comprehensive E2E verifier
pub struct E2EVerifier<'a> {
    fixture: &'a E2EFixture,
    workspace_root: &'a Path,
}

impl<'a> E2EVerifier<'a> {
    /// Create a new verifier
    #[must_use]
    pub fn new(fixture: &'a E2EFixture, workspace_root: &'a Path) -> Self {
        Self {
            fixture,
            workspace_root,
        }
    }

    /// Verify all aspects of the test execution
    pub fn verify_all(
        &self,
        task_result: &TaskResult,
        mock_provider: &StatefulMockProvider,
    ) -> VerificationResult {
        let mut result = VerificationResult::new();

        // 1. Verify task completion
        result.merge(self.verify_task_completion(task_result));

        // 2. Verify file operations
        result.merge(self.verify_files());

        // 3. Verify response content
        result.merge(self.verify_response(task_result));

        // 4. Verify provider calls
        result.merge(self.verify_provider_calls(mock_provider));

        // 5. Verify validation result
        result.merge(self.verify_validation(task_result));

        result
    }

    /// Verify task completion status
    fn verify_task_completion(&self, task_result: &TaskResult) -> VerificationResult {
        let mut result = VerificationResult::new();

        let expected = self.fixture.expected_outcomes.all_tasks_completed;
        let actual = task_result.validation.passed;

        if expected != actual {
            result.add_failure(format!(
                "Task completion mismatch: expected {expected}, got {actual}"
            ));
        } else {
            result.add_success(format!("Task completion matches: {actual}"));
        }

        result
    }

    /// Verify file operations
    fn verify_files(&self) -> VerificationResult {
        let mut result = VerificationResult::new();

        for file_verify in &self.fixture.expected_outcomes.files {
            let file_result = self.verify_file(file_verify);
            result.merge(file_result);
        }

        result
    }

    /// Verify a single file
    fn verify_file(&self, file_verify: &FileVerification) -> VerificationResult {
        let mut result = VerificationResult::new();
        let file_path = self.workspace_root.join(&file_verify.path);

        // Check existence
        let exists = file_path.exists();

        if file_verify.must_exist && !exists {
            result.add_failure(format!("File {} does not exist", file_verify.path));
            return result;
        }

        if file_verify.must_not_exist && exists {
            result.add_failure(format!("File {} exists but should not", file_verify.path));
            return result;
        }

        if !exists {
            if !file_verify.must_not_exist {
                result.add_success(format!(
                    "File {} correctly does not exist",
                    file_verify.path
                ));
            }
            return result;
        }

        result.add_success(format!("File {} exists as expected", file_verify.path));

        // Read content if needed
        if file_verify.exact_content.is_some()
            || !file_verify.contains.is_empty()
            || !file_verify.not_contains.is_empty()
        {
            let content = match fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(e) => {
                    result.add_failure(format!("Failed to read file {}: {e}", file_verify.path));
                    return result;
                }
            };

            // Check exact content
            if let Some(ref exact) = file_verify.exact_content {
                if content == *exact {
                    result.add_success(format!(
                        "File {} has exact expected content",
                        file_verify.path
                    ));
                } else {
                    result.add_failure(format!(
                        "File {} content mismatch.\nExpected:\n{exact}\n\nActual:\n{content}",
                        file_verify.path
                    ));
                }
            } else {
                // Check contains patterns
                for pattern in &file_verify.contains {
                    if content.contains(pattern) {
                        result.add_success(format!(
                            "File {} contains expected pattern: {pattern}",
                            file_verify.path
                        ));
                    } else {
                        result.add_failure(format!(
                            "File {} missing expected pattern: {pattern}",
                            file_verify.path
                        ));
                    }
                }

                // Check not_contains patterns
                for pattern in &file_verify.not_contains {
                    if content.contains(pattern) {
                        result.add_failure(format!(
                            "File {} contains forbidden pattern: {pattern}",
                            file_verify.path
                        ));
                    } else {
                        result.add_success(format!(
                            "File {} correctly does not contain: {pattern}",
                            file_verify.path
                        ));
                    }
                }
            }
        }

        result
    }

    /// Verify response content
    fn verify_response(&self, task_result: &TaskResult) -> VerificationResult {
        let mut result = VerificationResult::new();

        if let Some(ref response_verify) = self.fixture.expected_outcomes.response {
            let response_text = &task_result.response.text;

            result.merge(self.verify_response_patterns(response_text, response_verify));
        }

        result
    }

    /// Verify response patterns
    fn verify_response_patterns(
        &self,
        response_text: &str,
        response_verify: &ResponseVerification,
    ) -> VerificationResult {
        let mut result = VerificationResult::new();

        // Check min length
        if let Some(min_len) = response_verify.min_length {
            if response_text.len() >= min_len {
                result.add_success(format!(
                    "Response length {} >= minimum {min_len}",
                    response_text.len()
                ));
            } else {
                result.add_failure(format!(
                    "Response too short: {} < {min_len}",
                    response_text.len()
                ));
            }
        }

        // Check contains patterns
        for pattern in &response_verify.contains {
            if response_text.contains(pattern) {
                result.add_success(format!("Response contains: {pattern}"));
            } else {
                result.add_failure(format!("Response missing: {pattern}"));
            }
        }

        // Check not_contains patterns
        for pattern in &response_verify.not_contains {
            if response_text.contains(pattern) {
                result.add_failure(format!("Response contains forbidden: {pattern}"));
            } else {
                result.add_success(format!("Response correctly does not contain: {pattern}"));
            }
        }

        result
    }

    /// Verify provider calls
    fn verify_provider_calls(&self, mock_provider: &StatefulMockProvider) -> VerificationResult {
        let mut result = VerificationResult::new();
        let call_count = mock_provider.call_count();

        // Check min calls
        if let Some(min_calls) = self.fixture.expected_outcomes.min_provider_calls {
            if call_count >= min_calls {
                result.add_success(format!(
                    "Provider calls {call_count} >= minimum {min_calls}"
                ));
            } else {
                result.add_failure(format!(
                    "Too few provider calls: {call_count} < {min_calls}"
                ));
            }
        }

        // Check max calls
        if let Some(max_calls) = self.fixture.expected_outcomes.max_provider_calls {
            if call_count <= max_calls {
                result.add_success(format!(
                    "Provider calls {call_count} <= maximum {max_calls}"
                ));
            } else {
                result.add_failure(format!(
                    "Too many provider calls: {call_count} > {max_calls}"
                ));
            }
        }

        // Verify call patterns
        result.merge(self.verify_call_patterns(mock_provider.get_call_history()));

        result
    }

    /// Verify call patterns match expected
    fn verify_call_patterns(&self, calls: Vec<CallRecord>) -> VerificationResult {
        let mut result = VerificationResult::new();

        // Check that all expected patterns were called
        for mock_response in &self.fixture.mock_responses {
            let pattern_calls: Vec<_> = calls
                .iter()
                .filter(|c| c.matched_pattern == mock_response.pattern)
                .collect();

            if mock_response.use_once {
                if pattern_calls.len() == 1 {
                    result.add_success(format!(
                        "Pattern '{}' called exactly once as expected",
                        mock_response.pattern
                    ));
                } else {
                    result.add_failure(format!(
                        "Pattern '{}' should be called once, but was called {} times",
                        mock_response.pattern,
                        pattern_calls.len()
                    ));
                }
            }

            // Check for error calls
            for call in &pattern_calls {
                if call.was_error && !mock_response.should_fail {
                    result.add_failure(format!(
                        "Pattern '{}' resulted in unexpected error: {}",
                        mock_response.pattern, call.response
                    ));
                }
            }
        }

        result
    }

    /// Verify validation result
    fn verify_validation(&self, task_result: &TaskResult) -> VerificationResult {
        let mut result = VerificationResult::new();

        let expected = self.fixture.expected_outcomes.validation_passed;
        let actual = task_result.validation.passed;

        if expected != actual {
            result.add_failure(format!(
                "Validation result mismatch: expected {expected}, got {actual}"
            ));
        } else {
            result.add_success(format!("Validation result matches: {actual}"));
        }

        result
    }
}

/// Pretty print verification result
pub fn print_verification_result(fixture_name: &str, result: &VerificationResult) {
    println!("\n========================================");
    println!("Verification Results for: {fixture_name}");
    println!("========================================");

    if result.passed {
        println!("✅ ALL VERIFICATIONS PASSED");
    } else {
        println!("❌ VERIFICATION FAILED");
    }

    if !result.successes.is_empty() {
        println!("\n✅ Successes ({}):", result.successes.len());
        for success in &result.successes {
            println!("  ✓ {success}");
        }
    }

    if !result.failures.is_empty() {
        println!("\n❌ Failures ({}):", result.failures.len());
        for failure in &result.failures {
            println!("  ✗ {failure}");
        }
    }

    println!("========================================\n");
}
