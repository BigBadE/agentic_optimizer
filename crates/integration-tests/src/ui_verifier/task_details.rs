//! Task details verification logic.

use crate::fixture::UiVerify;
use crate::verification_result::VerificationResult;
use merlin_cli::TuiApp;
use merlin_cli::ui::renderer::FocusedPane;
use merlin_cli::ui::task_manager::{TaskManager, TaskStatus};
use merlin_deps::ratatui::backend::TestBackend;

/// Verify task details (descriptions, all completed, task created, focus changed)
pub fn verify_task_details(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    app: &TuiApp<TestBackend>,
    verify: &UiVerify,
) {
    verify_task_descriptions_visible(result, task_manager, verify);
    verify_all_tasks_completed(result, task_manager, verify);
    verify_task_created(result, task_manager, verify);
    verify_focus_changed(result, app, verify);
}

/// Verify task descriptions are visible
fn verify_task_descriptions_visible(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    if verify.task_descriptions_visible.is_empty() {
        return;
    }

    for expected_desc in &verify.task_descriptions_visible {
        let found = task_manager.task_order().iter().any(|task_id| {
            task_manager
                .get_task(*task_id)
                .is_some_and(|task| task.description.contains(expected_desc))
        });
        if found {
            result.add_success(format!("Task description visible: '{expected_desc}'"));
        } else {
            result.add_failure(format!("Task description not found: '{expected_desc}'"));
        }
    }
}

/// Verify all tasks are completed
fn verify_all_tasks_completed(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected) = verify.all_tasks_completed else {
        return;
    };

    let total = task_manager.task_order().len();
    let completed = task_manager
        .task_order()
        .iter()
        .filter(|task_id| {
            task_manager
                .get_task(**task_id)
                .is_some_and(|task| matches!(task.status, TaskStatus::Completed))
        })
        .count();
    let all_completed = total > 0 && completed == total;

    if all_completed == expected {
        result.add_success(format!("All tasks completed check matches: {expected}"));
    } else {
        result.add_failure(format!(
            "All tasks completed mismatch. Expected: {expected}, Actual: {all_completed} ({completed}/{total})"
        ));
    }
}

/// Verify task was created
fn verify_task_created(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    let Some(expected) = verify.task_created else {
        return;
    };

    let has_tasks = !task_manager.task_order().is_empty();
    if has_tasks == expected {
        result.add_success(format!("Task created check matches: {expected}"));
    } else {
        result.add_failure(format!(
            "Task created mismatch. Expected: {expected}, Actual: {has_tasks}"
        ));
    }
}

/// Verify focus has changed
fn verify_focus_changed(
    result: &mut VerificationResult,
    app: &TuiApp<TestBackend>,
    verify: &UiVerify,
) {
    let Some(expected) = verify.focus_changed else {
        return;
    };

    let focus_changed = !matches!(app.test_focused_pane(), FocusedPane::Input);
    if focus_changed == expected {
        result.add_success(format!("Focus changed check matches: {expected}"));
    } else {
        result.add_failure(format!(
            "Focus changed mismatch. Expected: {expected}, Actual: {focus_changed}"
        ));
    }
}
