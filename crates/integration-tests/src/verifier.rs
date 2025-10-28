//! Verification system for unified tests.

use super::execution_tracker::{ExecutionRecord, ExecutionResultTracker};
use super::execution_verifier::ExecutionVerifier;
use super::file_verifier::FileVerifier;
use super::fixture::{FinalVerify, TestEvent, VerifyConfig};
use super::ui_verifier::UiVerifier;
use super::verification_result::VerificationResult;
use merlin_cli::TuiApp;
use merlin_cli::ui::task_manager::{TaskManager, TaskStatus};
use merlin_deps::ratatui::backend::TestBackend;
use std::path::Path;
use std::result::Result;

/// Unified verifier
pub struct UnifiedVerifier<'fixture> {
    /// Workspace root
    workspace_root: &'fixture Path,
    /// Accumulated result
    result: VerificationResult,
}

impl<'fixture> UnifiedVerifier<'fixture> {
    /// Create new verifier
    #[must_use]
    pub fn new(workspace_root: &'fixture Path) -> Self {
        Self {
            workspace_root,
            result: VerificationResult::new(),
        }
    }

    /// Verify an event
    ///
    /// # Errors
    /// Returns error if verification fails critically
    pub fn verify_event(
        &mut self,
        _event: &TestEvent,
        verify: &VerifyConfig,
        tui_app: Option<&TuiApp<TestBackend>>,
        execution_tracker: &ExecutionResultTracker,
    ) -> Result<(), String> {
        // Verify execution if specified
        if let Some(exec_verify) = &verify.execution {
            // Get the most recent execution result from tracker
            let last_result = execution_tracker.last_result().map(ExecutionRecord::result);
            ExecutionVerifier::verify_execution(&mut self.result, last_result, exec_verify);
        }

        // Verify files if specified
        if let Some(file_verifies) = &verify.files {
            for file_verify in file_verifies {
                FileVerifier::verify_file(&mut self.result, self.workspace_root, file_verify);
            }
        }

        // Verify UI if specified
        if let Some(ui_verify) = &verify.ui {
            UiVerifier::verify_ui(&mut self.result, tui_app, ui_verify);
        }

        // Verify state if specified
        if let Some(state_verify) = &verify.state {
            UiVerifier::verify_state(&mut self.result, tui_app, state_verify);
        }

        Ok(())
    }

    /// Verify final state
    ///
    /// # Errors
    /// Returns error if verification fails
    pub fn verify_final(
        &mut self,
        verify: &FinalVerify,
        tui_app: Option<&TuiApp<TestBackend>>,
        execution_tracker: &ExecutionResultTracker,
    ) -> Result<(), String> {
        // Verify final execution state
        if let Some(exec_verify) = &verify.execution {
            // Verify all tasks completed if specified
            if let Some(expected) = exec_verify.all_tasks_completed {
                Self::verify_all_tasks_completed(&mut self.result, tui_app, expected);
            }

            // Verify validation passed if specified
            if let Some(expected) = exec_verify.validation_passed {
                Self::verify_validation_passed(&mut self.result, execution_tracker, expected);
            }

            // Verify return value for final execution if specified
            if exec_verify.return_value_matches.is_some()
                || exec_verify.return_value_contains.is_some()
            {
                let last_result = execution_tracker.last_result().map(ExecutionRecord::result);
                ExecutionVerifier::verify_execution(&mut self.result, last_result, exec_verify);
            }
        }

        // Verify final files
        if let Some(file_verifies) = &verify.files {
            for file_verify in file_verifies {
                FileVerifier::verify_file(&mut self.result, self.workspace_root, file_verify);
            }
        }

        // Verify final UI state if specified
        if let Some(ui_verify) = &verify.ui {
            UiVerifier::verify_ui(&mut self.result, tui_app, ui_verify);
        }

        // Verify final state if specified
        if let Some(state_verify) = &verify.state {
            UiVerifier::verify_state(&mut self.result, tui_app, state_verify);
        }

        Ok(())
    }

    /// Get accumulated result
    #[must_use]
    pub fn result(self) -> VerificationResult {
        self.result
    }

    /// Verify all tasks completed
    fn verify_all_tasks_completed(
        result: &mut VerificationResult,
        tui_app: Option<&TuiApp<TestBackend>>,
        expected: bool,
    ) {
        let Some(app) = tui_app else {
            result.add_failure("Cannot verify all_tasks_completed without TUI app".to_owned());
            return;
        };

        let task_manager = app.test_task_manager();
        let total_tasks = task_manager.task_order().len();
        let completed_tasks = Self::count_completed_tasks(task_manager);

        let all_completed = total_tasks > 0 && completed_tasks == total_tasks;
        if all_completed == expected {
            result.add_success(format!(
                "All tasks completed check passed: expected={expected}, actual={all_completed}"
            ));
        } else {
            result.add_failure(format!(
                "All tasks completed mismatch: expected={expected}, actual={all_completed} ({completed_tasks}/{total_tasks})"
            ));
        }
    }

    /// Count completed tasks
    fn count_completed_tasks(task_manager: &TaskManager) -> usize {
        task_manager
            .task_order()
            .iter()
            .filter(|task_id| {
                task_manager
                    .get_task(**task_id)
                    .is_some_and(|task| task.status == TaskStatus::Completed)
            })
            .count()
    }

    /// Verify validation passed
    fn verify_validation_passed(
        result: &mut VerificationResult,
        execution_tracker: &ExecutionResultTracker,
        expected: bool,
    ) {
        let Some(record) = execution_tracker.last_result() else {
            result.add_failure("Cannot verify validation_passed: no execution results".to_owned());
            return;
        };

        let task_result = record.task_result();
        let validation_passed = task_result.validation.passed;
        if validation_passed == expected {
            result.add_success(format!(
                "Validation passed check: expected={expected}, actual={validation_passed}"
            ));
        } else {
            result.add_failure(format!(
                "Validation passed mismatch: expected={expected}, actual={validation_passed}"
            ));
        }
    }
}
