//! Tests for work units and subtasks.

use super::{SubtaskStatus, WorkStatus, WorkUnit};
use crate::TaskId;

/// Tests work unit creation and basic state.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_work_unit_creation() {
    let work = WorkUnit::new(TaskId::default(), "groq".to_owned());
    assert_eq!(work.tier_used, "groq");
    assert_eq!(work.status, WorkStatus::InProgress);
    assert_eq!(work.retry_count, 0);
    assert!(work.subtasks.is_empty());
    assert!(!work.is_terminal());
}

/// Tests subtask addition and tracking.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_subtask_management() {
    let mut work = WorkUnit::new(TaskId::default(), "local".to_owned());

    let id1 = work.add_subtask("Task 1".to_owned(), 5);
    let id2 = work.add_subtask("Task 2".to_owned(), 7);

    assert_eq!(work.subtasks.len(), 2);
    assert_eq!(work.subtasks[0].description, "Task 1");
    assert_eq!(work.subtasks[0].difficulty, 5);
    assert_eq!(work.subtasks[1].description, "Task 2");

    work.start_subtask(id1);
    assert_eq!(work.subtasks[0].status, SubtaskStatus::InProgress);

    work.complete_subtask(id1, Some("Done".to_owned()));
    assert_eq!(work.subtasks[0].status, SubtaskStatus::Completed);
    assert_eq!(work.subtasks[0].result, Some("Done".to_owned()));
    assert!(work.subtasks[0].error.is_none());

    work.fail_subtask(id2, "Error occurred".to_owned());
    assert_eq!(work.subtasks[1].status, SubtaskStatus::Failed);
    assert_eq!(work.subtasks[1].error, Some("Error occurred".to_owned()));
}

/// Tests next pending subtask retrieval.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_next_pending_subtask() {
    let mut work = WorkUnit::new(TaskId::default(), "local".to_owned());

    let id1 = work.add_subtask("Task 1".to_owned(), 5);
    let id2 = work.add_subtask("Task 2".to_owned(), 5);
    let _id3 = work.add_subtask("Task 3".to_owned(), 5);

    let next1 = work.next_pending_subtask();
    assert!(next1.is_some());
    assert_eq!(
        next1.map_or_else(String::new, |subtask| subtask.description.clone()),
        "Task 1"
    );

    work.complete_subtask(id1, None);
    let next2 = work.next_pending_subtask();
    assert!(next2.is_some());
    assert_eq!(
        next2.map_or_else(String::new, |subtask| subtask.description.clone()),
        "Task 2"
    );

    work.start_subtask(id2);
    let next3 = work.next_pending_subtask();
    assert!(next3.is_some());
    assert_eq!(
        next3.map_or_else(String::new, |subtask| subtask.description.clone()),
        "Task 3"
    );
}

/// Tests progress percentage calculation.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_progress_percentage() {
    let mut work = WorkUnit::new(TaskId::default(), "local".to_owned());

    // No subtasks - based on status
    assert_eq!(work.progress_percentage(), 50); // InProgress = 50%

    let id1 = work.add_subtask("Task 1".to_owned(), 5);
    let id2 = work.add_subtask("Task 2".to_owned(), 5);
    let id3 = work.add_subtask("Task 3".to_owned(), 5);
    let id4 = work.add_subtask("Task 4".to_owned(), 5);

    // 0/4 complete = 0%
    assert_eq!(work.progress_percentage(), 0);

    work.complete_subtask(id1, None);
    work.complete_subtask(id2, None);

    // 2/4 complete = 50%
    assert_eq!(work.progress_percentage(), 50);

    work.complete_subtask(id3, None);
    work.complete_subtask(id4, None);

    // 4/4 complete = 100%
    assert_eq!(work.progress_percentage(), 100);
}

/// Tests work unit state transitions.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_work_state_transitions() {
    let mut work = WorkUnit::new(TaskId::default(), "local".to_owned());

    assert_eq!(work.status, WorkStatus::InProgress);
    assert!(!work.is_terminal());

    work.complete();
    assert_eq!(work.status, WorkStatus::Completed);
    assert!(work.is_terminal());

    let mut work2 = WorkUnit::new(TaskId::default(), "groq".to_owned());
    work2.fail();
    assert_eq!(work2.status, WorkStatus::Failed);
    assert!(work2.is_terminal());

    let mut work3 = WorkUnit::new(TaskId::default(), "premium".to_owned());
    work3.cancel();
    assert_eq!(work3.status, WorkStatus::Cancelled);
    assert!(work3.is_terminal());
}

/// Tests retry logic.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_work_retry() {
    let mut work = WorkUnit::new(TaskId::default(), "local".to_owned());

    assert_eq!(work.retry_count, 0);
    assert_eq!(work.status, WorkStatus::InProgress);

    work.retry();
    assert_eq!(work.retry_count, 1);
    assert_eq!(work.status, WorkStatus::Retrying);
    assert!(!work.is_terminal());

    work.retry();
    assert_eq!(work.retry_count, 2);
    assert_eq!(work.status, WorkStatus::Retrying);
}

/// Tests status emoji representations.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_status_emojis() {
    assert_eq!(WorkStatus::InProgress.emoji(), "⏳");
    assert_eq!(WorkStatus::Completed.emoji(), "✅");
    assert_eq!(WorkStatus::Failed.emoji(), "❌");
    assert_eq!(WorkStatus::Cancelled.emoji(), "⏸️");
    assert_eq!(WorkStatus::Retrying.emoji(), "🔄");

    assert_eq!(SubtaskStatus::Pending.emoji(), "⏳");
    assert_eq!(SubtaskStatus::InProgress.emoji(), "🔄");
    assert_eq!(SubtaskStatus::Completed.emoji(), "✅");
    assert_eq!(SubtaskStatus::Failed.emoji(), "❌");
    assert_eq!(SubtaskStatus::Skipped.emoji(), "⏭️");
}

/// Tests mid-execution progress tracking (33%, 66%, 100%).
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_mid_execution_progress() {
    let mut work = WorkUnit::new(TaskId::default(), "test".to_owned());
    let id1 = work.add_subtask("Step 1".to_owned(), 5);
    let id2 = work.add_subtask("Step 2".to_owned(), 5);
    let id3 = work.add_subtask("Step 3".to_owned(), 5);

    assert_eq!(work.progress_percentage(), 0);
    work.complete_subtask(id1, Some("Done 1".to_owned()));
    assert_eq!(work.progress_percentage(), 33);
    work.complete_subtask(id2, Some("Done 2".to_owned()));
    assert_eq!(work.progress_percentage(), 66);
    work.complete_subtask(id3, Some("Done 3".to_owned()));
    assert_eq!(work.progress_percentage(), 100);
}

/// Tests that `complete()` requires all subtasks to be completed.
///
/// # Panics
/// Panics if assertions fail during test execution.
#[test]
fn test_complete_requires_all_subtasks() {
    let mut work = WorkUnit::new(TaskId::default(), "test".to_owned());
    let id1 = work.add_subtask("Step 1".to_owned(), 5);
    let id2 = work.add_subtask("Step 2".to_owned(), 5);
    let id3 = work.add_subtask("Step 3".to_owned(), 5);

    work.complete();
    assert_eq!(work.status, WorkStatus::InProgress);
    work.complete_subtask(id1, None);
    work.complete_subtask(id2, None);
    work.complete();
    assert_eq!(work.status, WorkStatus::InProgress);
    work.complete_subtask(id3, None);
    work.complete();
    assert_eq!(work.status, WorkStatus::Completed);
}
