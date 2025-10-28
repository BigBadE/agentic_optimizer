//! Thread state verification logic.

use crate::fixture::UiVerify;
use crate::verification_result::VerificationResult;
use merlin_agent::ThreadStore;
use merlin_cli::TuiApp;
use merlin_cli::ui::state::UiState;
use merlin_deps::ratatui::backend::TestBackend;

/// Verify thread state (count, selected thread ID, thread names)
pub fn verify_thread_state(
    result: &mut VerificationResult,
    app: &TuiApp<TestBackend>,
    state: &UiState,
    verify: &UiVerify,
) {
    if let Some(expected) = verify.thread_count {
        let thread_store = app.test_thread_store();
        let actual = thread_store.active_threads().len();
        if actual == expected {
            result.add_success(format!("Thread count matches: {expected}"));
        } else {
            result.add_failure(format!(
                "Thread count mismatch. Expected: {expected}, Actual: {actual}"
            ));
        }
    }

    if let Some(expected_id) = &verify.selected_thread_id {
        if let Some(actual_id) = &state.active_thread_id {
            let actual_id_string = actual_id.to_string();
            if expected_id == "any" || actual_id_string.contains(expected_id) {
                result.add_success(format!("Selected thread ID check passed: '{expected_id}'"));
            } else {
                result.add_failure(format!(
                    "Selected thread ID mismatch. Expected contains: '{expected_id}', Actual: '{actual_id_string}'"
                ));
            }
        } else {
            result.add_failure(format!(
                "Expected thread ID '{expected_id}' but no thread selected"
            ));
        }
    }

    if !verify.thread_names_visible.is_empty() {
        verify_thread_names(result, app.test_thread_store(), verify);
    }
}

/// Verify thread names are visible
fn verify_thread_names(
    result: &mut VerificationResult,
    thread_store: &ThreadStore,
    verify: &UiVerify,
) {
    let threads = thread_store.active_threads();
    for expected_name in &verify.thread_names_visible {
        let found = threads
            .iter()
            .any(|thread| thread.name.contains(expected_name));
        if found {
            result.add_success(format!("Thread name visible: '{expected_name}'"));
        } else {
            result.add_failure(format!("Thread name not found: '{expected_name}'"));
        }
    }
}
