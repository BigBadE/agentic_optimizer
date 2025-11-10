//! UI and state verification logic.

use super::verification_result::VerificationResult;
use super::verify::{StateVerify, UiVerify};
use merlin_cli::TuiApp;
use merlin_cli::ui::input::InputManager;
use merlin_cli::ui::renderer::FocusedPane;
use merlin_cli::ui::task_manager::TaskManager;
use ratatui::backend::TestBackend;

mod input;
mod output;
mod rendered_buffer;
mod state;
mod task_counts;
mod task_details;
mod task_selection;
mod threads;
mod work_unit;

use input::verify_input_related_fields;
use output::verify_output_patterns;
use rendered_buffer::verify_rendered_buffer;
use task_counts::verify_task_counts;
use task_details::verify_task_details;
use task_selection::verify_selected_task;
use threads::verify_thread_state;
use work_unit::verify_work_unit;

/// UI verifier helper
pub struct UiVerifier;

impl UiVerifier {
    /// Verify UI
    pub async fn verify_ui(
        result: &mut VerificationResult,
        tui_app: Option<&TuiApp<TestBackend>>,
        verify: &UiVerify,
    ) {
        let Some(app) = tui_app else {
            result.add_failure("TUI app not available for verification".to_owned());
            return;
        };

        let state = &app.ui_components.state;
        let task_manager = &app.ui_components.task_manager;
        let input_manager = &app.ui_components.input_manager;

        verify_input_related_fields(result, input_manager, verify);
        Self::verify_focused_pane(result, app, verify);
        verify_task_counts(result, task_manager, verify);
        verify_task_details(result, task_manager, app, verify);
        verify_selected_task(result, state, task_manager, verify);
        verify_output_patterns(result, task_manager, verify);
        Self::verify_ui_states(result, task_manager, input_manager, verify);
        verify_thread_state(result, app, state, verify);
        verify_rendered_buffer(result, app, verify);

        // Verify WorkUnit if specified
        if let Some(ref work_unit_verify) = verify.work_unit {
            let work_unit_result = verify_work_unit(app, work_unit_verify).await;
            result.merge(work_unit_result);
        }
    }

    /// Verify focused pane
    fn verify_focused_pane(
        result: &mut VerificationResult,
        app: &TuiApp<TestBackend>,
        verify: &UiVerify,
    ) {
        if let Some(expected_focus) = verify.focused_pane.as_deref() {
            let actual_focus = match app.ui_components.focused_pane {
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
    }

    /// Verify UI states (placeholder visible)
    fn verify_ui_states(
        result: &mut VerificationResult,
        task_manager: &TaskManager,
        input_manager: &InputManager,
        verify: &UiVerify,
    ) {
        if let Some(expected) = verify.placeholder_visible {
            let placeholder_visible = task_manager.task_order().is_empty()
                && input_manager.input_area().lines().is_empty();
            if placeholder_visible == expected {
                result.add_success(format!("Placeholder visible check matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Placeholder visible mismatch. Expected: {expected}, Actual: {placeholder_visible}"
                ));
            }
        }
    }

    /// Verify state
    pub async fn verify_state(
        result: &mut VerificationResult,
        tui_app: Option<&TuiApp<TestBackend>>,
        verify: &StateVerify,
    ) {
        state::verify_state(result, tui_app, verify).await;
    }
}
