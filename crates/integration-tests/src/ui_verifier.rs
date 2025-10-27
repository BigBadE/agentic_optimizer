//! UI and state verification logic.

use super::fixture::{StateVerify, UiVerify};
use super::verification_result::VerificationResult;
use merlin_cli::TuiApp;
use merlin_cli::ui::renderer::FocusedPane;
use ratatui::backend::TestBackend;

/// UI verifier helper
pub struct UiVerifier;

impl UiVerifier {
    /// Verify UI
    pub fn verify_ui(
        result: &mut VerificationResult,
        tui_app: Option<&TuiApp<TestBackend>>,
        verify: &UiVerify,
    ) {
        let Some(app) = tui_app else {
            result.add_failure("TUI app not available for verification".to_owned());
            return;
        };

        let _state = app.test_state();
        let task_manager = app.test_task_manager();
        let input_manager = app.test_input_manager();

        // Verify input text
        if let Some(expected_input) = &verify.input_text {
            let actual_input = input_manager.input_area().lines().join("\n");
            if actual_input == *expected_input {
                result.add_success(format!("Input text matches: '{expected_input}'"));
            } else {
                result.add_failure(format!(
                    "Input text mismatch. Expected: '{expected_input}', Actual: '{actual_input}'"
                ));
            }
        }

        // Verify focused pane
        if let Some(expected_focus) = verify.focused_pane.as_deref() {
            let actual_focus = match app.test_focused_pane() {
                FocusedPane::Input => "input",
                FocusedPane::Tasks => "tasks",
                FocusedPane::Output => "output",
                FocusedPane::Threads => "threads",
            };
            if actual_focus == expected_focus {
                result.add_success(format!("Focused pane matches: '{expected_focus}'"));
            } else {
                result.add_failure(format!(
                    "Focused pane mismatch. Expected: '{expected_focus}', Actual: '{actual_focus}'"
                ));
            }
        }

        // Verify task count
        if let Some(expected_count) = verify.tasks_displayed {
            let actual_count = task_manager.task_order().len();
            if actual_count == expected_count {
                result.add_success(format!("Task count matches: {expected_count}"));
            } else {
                result.add_failure(format!(
                    "Task count mismatch. Expected: {expected_count}, Actual: {actual_count}"
                ));
            }
        }

        // Basic UI verification complete - detailed checks can be added as needed
    }

    /// Verify state
    pub fn verify_state(
        result: &mut VerificationResult,
        tui_app: Option<&TuiApp<TestBackend>>,
        verify: &StateVerify,
    ) {
        let Some(app) = tui_app else {
            return;
        };

        let state = app.test_state();

        // Verify conversation count
        if let Some(expected_count) = verify.conversation_count {
            let actual_count = state.conversation_history.len();
            if actual_count == expected_count {
                result.add_success(format!("Conversation count matches: {expected_count}"));
            } else {
                result.add_failure(format!(
                    "Conversation count mismatch. Expected: {expected_count}, Actual: {actual_count}"
                ));
            }
        }

        // Verify active thread
        if let Some(expected_thread) = &verify.selected_task {
            let has_active_thread = state.active_thread_id.is_some();
            if expected_thread == "any" && has_active_thread {
                result.add_success("Has active thread".to_owned());
            } else if !has_active_thread {
                result.add_failure(format!(
                    "Expected thread '{expected_thread}' but none active"
                ));
            }
        }
    }
}
