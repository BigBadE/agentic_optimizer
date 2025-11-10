//! `WorkUnit` verification for fixture tests

use crate::ui_verifier::VerificationResult;
use crate::verify::WorkUnitVerify;
use merlin_cli::TuiApp;
use merlin_core::{SubtaskStatus, WorkStatus, WorkUnit};
use ratatui::backend::TestBackend;

/// Get `WorkUnit` from `TaskDisplay` (live during execution) or last message (after completion)
async fn get_work_unit(app: &TuiApp<TestBackend>) -> Option<WorkUnit> {
    // First check if there's a live WorkUnit in the active task
    if let Some(active_task_id) = app.ui_components.state.active_task_id
        && let Some(task_display) = app.ui_components.task_manager.get_task(active_task_id)
        && let Some(ref work_unit_arc) = task_display.work_unit
    {
        // Clone the WorkUnit from Arc<Mutex<>> - wait for lock
        let work_unit_guard = work_unit_arc.lock().await;
        let work_unit = work_unit_guard.clone();
        return Some(work_unit);
    }

    // Fall back to checking the message's WorkUnit (after completion)
    let orch = app.runtime_state.orchestrator.as_ref()?;
    let store_arc = orch.thread_store()?;
    let store = store_arc.lock().ok()?;
    let threads = store.active_threads();
    let thread = threads.last()?;
    let msg = thread.messages.last()?;
    msg.work.clone()
}

/// Verify `WorkUnit` status field
fn verify_status(work: &WorkUnit, expected: &str, result: &mut VerificationResult) {
    let actual = match work.status {
        WorkStatus::InProgress => "in_progress",
        WorkStatus::Completed => "completed",
        WorkStatus::Failed => "failed",
        WorkStatus::Cancelled => "cancelled",
        WorkStatus::Retrying => "retrying",
    };

    if actual == expected {
        result.add_success(format!("WorkUnit status is '{expected}'"));
    } else {
        result.add_failure(format!(
            "WorkUnit status mismatch: expected '{expected}', got '{actual}'"
        ));
    }
}

/// Verify basic `WorkUnit` fields (subtask count, progress, retries, duration, tier, terminal state)
fn verify_basic_fields(work: &WorkUnit, verify: &WorkUnitVerify, result: &mut VerificationResult) {
    if let Some(expected) = verify.subtask_count {
        let actual = work.subtasks.len();
        if actual == expected {
            result.add_success(format!("WorkUnit has {expected} subtasks"));
        } else {
            result.add_failure(format!(
                "Subtask count mismatch: expected {expected}, got {actual}"
            ));
        }
    }

    if let Some(expected) = verify.progress_percentage {
        let actual = work.progress_percentage();
        if actual == expected {
            result.add_success(format!("Progress is {expected}%"));
        } else {
            result.add_failure(format!(
                "Progress mismatch: expected {expected}%, got {actual}%"
            ));
        }
    }

    if let Some(expected) = verify.retry_count {
        if work.retry_count == expected {
            result.add_success(format!("Retry count is {expected}"));
        } else {
            result.add_failure(format!(
                "Retry count mismatch: expected {expected}, got {}",
                work.retry_count
            ));
        }
    }

    if let Some(expected) = verify.duration_ms {
        if work.duration_ms == expected {
            result.add_success(format!("Duration is {expected}ms"));
        } else {
            result.add_failure(format!(
                "Duration mismatch: expected {expected}ms, got {}ms",
                work.duration_ms
            ));
        }
    }

    if let Some(ref expected) = verify.tier_used {
        if work.tier_used == *expected {
            result.add_success(format!("Tier used is '{expected}'"));
        } else {
            result.add_failure(format!(
                "Tier mismatch: expected '{expected}', got '{}'",
                work.tier_used
            ));
        }
    }

    if let Some(expected) = verify.is_terminal {
        let actual = work.is_terminal();
        if actual == expected {
            result.add_success(format!("WorkUnit terminal state is {expected}"));
        } else {
            result.add_failure(format!(
                "Terminal state mismatch: expected {expected}, got {actual}"
            ));
        }
    }
}

/// Verify subtask titles exist
fn verify_subtask_titles(work: &WorkUnit, titles: &[String], result: &mut VerificationResult) {
    let actual: Vec<&str> = work
        .subtasks
        .iter()
        .map(|sub| sub.description.as_str())
        .collect();
    for expected in titles {
        if actual.contains(&expected.as_str()) {
            result.add_success(format!("Subtask '{expected}' exists"));
        } else {
            result.add_failure(format!(
                "Expected subtask '{expected}' not found. Available: {}",
                actual.join(", ")
            ));
        }
    }
}

/// Verify subtask counts by status
fn verify_subtask_counts(
    work: &WorkUnit,
    verify: &WorkUnitVerify,
    result: &mut VerificationResult,
) {
    let completed = work
        .subtasks
        .iter()
        .filter(|sub| matches!(sub.status, SubtaskStatus::Completed))
        .count();
    let pending = work
        .subtasks
        .iter()
        .filter(|sub| matches!(sub.status, SubtaskStatus::Pending))
        .count();
    let in_progress = work
        .subtasks
        .iter()
        .filter(|sub| matches!(sub.status, SubtaskStatus::InProgress))
        .count();
    let failed = work
        .subtasks
        .iter()
        .filter(|sub| matches!(sub.status, SubtaskStatus::Failed))
        .count();

    if let Some(expected) = verify.completed_subtasks {
        if completed == expected {
            result.add_success(format!("{expected} subtasks completed"));
        } else {
            result.add_failure(format!(
                "Completed subtasks mismatch: expected {expected}, got {completed}"
            ));
        }
    }

    if let Some(expected) = verify.pending_subtasks {
        if pending == expected {
            result.add_success(format!("{expected} subtasks pending"));
        } else {
            result.add_failure(format!(
                "Pending subtasks mismatch: expected {expected}, got {pending}"
            ));
        }
    }

    if let Some(expected) = verify.in_progress_subtasks {
        if in_progress == expected {
            result.add_success(format!("{expected} subtasks in progress"));
        } else {
            result.add_failure(format!(
                "In-progress subtasks mismatch: expected {expected}, got {in_progress}"
            ));
        }
    }

    if let Some(expected) = verify.failed_subtasks {
        if failed == expected {
            result.add_success(format!("{expected} subtasks failed"));
        } else {
            result.add_failure(format!(
                "Failed subtasks mismatch: expected {expected}, got {failed}"
            ));
        }
    }
}

/// Verify `WorkUnit` state
pub(super) async fn verify_work_unit(
    app: &TuiApp<TestBackend>,
    verify: &WorkUnitVerify,
) -> VerificationResult {
    let mut result = VerificationResult::new();

    // Get the last message's WorkUnit from the current thread
    let work_unit = get_work_unit(app).await;

    // Check existence
    if let Some(should_exist) = verify.exists {
        if should_exist && work_unit.is_some() {
            result.add_success("WorkUnit exists on message".to_owned());
        } else if should_exist && work_unit.is_none() {
            result.add_failure("Expected WorkUnit to exist, but none found".to_owned());
            return result; // Early return if doesn't exist
        } else if !should_exist && work_unit.is_none() {
            result.add_success("WorkUnit does not exist as expected".to_owned());
            return result; // No WorkUnit, nothing else to verify
        } else {
            result.add_failure("Expected no WorkUnit, but one exists".to_owned());
            return result;
        }
    }

    // If we're here and no WorkUnit, but other checks are specified, that's a failure
    let Some(work) = work_unit else {
        if verify.status.is_some()
            || verify.subtask_count.is_some()
            || verify.progress_percentage.is_some()
        {
            result.add_failure(
                "Cannot verify WorkUnit properties - no WorkUnit found on message".to_owned(),
            );
        }
        return result;
    };

    // Verify all fields
    if let Some(ref expected_status) = verify.status {
        verify_status(&work, expected_status, &mut result);
    }
    verify_basic_fields(&work, verify, &mut result);
    if !verify.subtask_titles.is_empty() {
        verify_subtask_titles(&work, &verify.subtask_titles, &mut result);
    }
    verify_subtask_counts(&work, verify, &mut result);

    result
}
