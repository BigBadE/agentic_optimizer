//! Task selection verification logic.

use crate::fixture::UiVerify;
use crate::verification_result::VerificationResult;
use merlin_cli::ui::state::UiState;
use merlin_cli::ui::task_manager::{TaskManager, TaskStatus};

/// Verify selected task (status and description)
pub fn verify_selected_task(
    result: &mut VerificationResult,
    state: &UiState,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    verify_task_status(result, state, task_manager, verify);
    verify_selected_task_description(result, state, task_manager, verify);
}

/// Verify task status of selected task
fn verify_task_status(
    result: &mut VerificationResult,
    state: &UiState,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected_status) = &verify.task_status else {
        return;
    };

    let Some(active_task_id) = state.active_task_id else {
        result.add_failure("Expected task status but no active task selected".to_owned());
        return;
    };

    let Some(task) = task_manager.get_task(active_task_id) else {
        result.add_failure("Active task not found in task manager".to_owned());
        return;
    };

    let actual_status = match task.status {
        TaskStatus::Running => "running",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
    };

    if actual_status == expected_status {
        result.add_success(format!("Task status matches: {expected_status}"));
    } else {
        result.add_failure(format!(
            "Task status mismatch. Expected: {expected_status}, Actual: {actual_status}"
        ));
    }
}

/// Verify selected task description contains text
fn verify_selected_task_description(
    result: &mut VerificationResult,
    state: &UiState,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected_text) = &verify.selected_task_contains else {
        return;
    };

    let Some(active_task_id) = state.active_task_id else {
        result.add_failure("Expected selected task description but no active task".to_owned());
        return;
    };

    let Some(task) = task_manager.get_task(active_task_id) else {
        result.add_failure("Active task not found in task manager".to_owned());
        return;
    };

    if task.description.contains(expected_text) {
        result.add_success(format!(
            "Selected task description contains: '{expected_text}'"
        ));
    } else {
        result.add_failure(format!(
            "Selected task description doesn't contain '{expected_text}'. Actual: '{}'",
            task.description
        ));
    }
}
