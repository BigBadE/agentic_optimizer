//! Verification system for unified tests.

use super::execution_verifier::ExecutionVerifier;
use super::file_verifier::FileVerifier;
use super::fixture::{FinalVerify, TestEvent, TestFixture, VerifyConfig};
use super::ui_verifier::UiVerifier;
use super::verification_result::VerificationResult;
use merlin_cli::TuiApp;
use merlin_deps::ratatui::backend::TestBackend;
use merlin_deps::serde_json::Value;
use merlin_tooling::ToolResult;
use std::path::Path;
use std::result::Result;

/// Unified verifier
pub struct UnifiedVerifier<'fixture> {
    /// Workspace root
    workspace_root: &'fixture Path,
    /// Accumulated result
    result: VerificationResult,
    /// Last TypeScript execution result
    last_execution: Option<ToolResult<Value>>,
    /// Reference to TUI app for state verification
    tui_app: Option<&'fixture TuiApp<TestBackend>>,
}

impl<'fixture> UnifiedVerifier<'fixture> {
    /// Create new verifier
    #[must_use]
    pub fn new(_fixture: &'fixture TestFixture, workspace_root: &'fixture Path) -> Self {
        Self {
            workspace_root,
            result: VerificationResult::new(),
            last_execution: None,
            tui_app: None,
        }
    }

    /// Set the last TypeScript execution result
    pub fn set_last_execution_result(&mut self, result: ToolResult<Value>) {
        self.last_execution = Some(result);
    }

    /// Set the TUI app for state verification
    pub fn set_tui_app(&mut self, app: &'fixture TuiApp<TestBackend>) {
        self.tui_app = Some(app);
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
            ExecutionVerifier::verify_execution(
                &mut self.result,
                self.last_execution.as_ref(),
                exec_verify,
            );
        }

        // Verify files if specified
        if let Some(file_verifies) = &verify.files {
            for file_verify in file_verifies {
                FileVerifier::verify_file(&mut self.result, self.workspace_root, file_verify);
            }
        }

        // Verify UI if specified
        if let Some(ui_verify) = &verify.ui {
            UiVerifier::verify_ui(&mut self.result, self.tui_app, ui_verify);
        }

        // Verify state if specified
        if let Some(state_verify) = &verify.state {
            UiVerifier::verify_state(&mut self.result, self.tui_app, state_verify);
        }

        Ok(())
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
                FileVerifier::verify_file(&mut self.result, self.workspace_root, file_verify);
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
