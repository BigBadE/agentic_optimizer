//! Task count verification logic.

use crate::fixture::UiVerify;
use crate::verification_result::VerificationResult;
use merlin_cli::ui::task_manager::{TaskManager, TaskStatus};

/// Verify task counts (pending, running, completed, failed, displayed)
pub fn verify_task_counts(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    verify_total_task_count(result, task_manager, verify);
    verify_pending_task_count(result, task_manager, verify);
    verify_running_task_count(result, task_manager, verify);
    verify_completed_task_count(result, task_manager, verify);
    verify_failed_task_count(result, task_manager, verify);
}

/// Verify total number of displayed tasks
fn verify_total_task_count(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected_count) = verify.tasks_displayed else {
        return;
    };

    let actual_count = task_manager.task_order().len();
    if actual_count == expected_count {
        result.add_success(format!("Task count matches: {expected_count}"));
    } else {
        result.add_failure(format!(
            "Task count mismatch. Expected: {expected_count}, Actual: {actual_count}"
        ));
    }
}

/// Verify count of pending tasks
fn verify_pending_task_count(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected) = verify.pending_tasks_count else {
        return;
    };

    let actual = task_manager
        .task_order()
        .iter()
        .filter(|task_id| {
            task_manager
                .get_task(**task_id)
                .is_some_and(|task| matches!(task.status, TaskStatus::Pending))
        })
        .count();

    if actual == expected {
        result.add_success(format!("Pending tasks count matches: {expected}"));
    } else {
        result.add_failure(format!(
            "Pending tasks count mismatch. Expected: {expected}, Actual: {actual}"
        ));
    }
}

/// Verify count of running tasks
fn verify_running_task_count(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected) = verify.running_tasks_count else {
        return;
    };

    let actual = task_manager
        .task_order()
        .iter()
        .filter(|task_id| {
            task_manager
                .get_task(**task_id)
                .is_some_and(|task| matches!(task.status, TaskStatus::Running))
        })
        .count();

    if actual == expected {
        result.add_success(format!("Running tasks count matches: {expected}"));
    } else {
        result.add_failure(format!(
            "Running tasks count mismatch. Expected: {expected}, Actual: {actual}"
        ));
    }
}

/// Verify count of completed tasks
fn verify_completed_task_count(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected) = verify.completed_tasks_count else {
        return;
    };

    let actual = task_manager
        .task_order()
        .iter()
        .filter(|task_id| {
            task_manager
                .get_task(**task_id)
                .is_some_and(|task| matches!(task.status, TaskStatus::Completed))
        })
        .count();

    if actual == expected {
        result.add_success(format!("Completed tasks count matches: {expected}"));
    } else {
        result.add_failure(format!(
            "Completed tasks count mismatch. Expected: {expected}, Actual: {actual}"
        ));
    }
}

/// Verify count of failed tasks
fn verify_failed_task_count(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected) = verify.failed_tasks_count else {
        return;
    };

    let actual = task_manager
        .task_order()
        .iter()
        .filter(|task_id| {
            task_manager
                .get_task(**task_id)
                .is_some_and(|task| matches!(task.status, TaskStatus::Failed))
        })
        .count();

    if actual == expected {
        result.add_success(format!("Failed tasks count matches: {expected}"));
    } else {
        result.add_failure(format!(
            "Failed tasks count mismatch. Expected: {expected}, Actual: {actual}"
        ));
    }
}
