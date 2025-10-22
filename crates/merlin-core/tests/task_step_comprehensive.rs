//! Comprehensive tests for `TaskStep` and `TaskList` state management.

#![cfg_attr(
    test,
    allow(
        clippy::tests_outside_test_module,
        clippy::missing_panics_doc,
        reason = "Test file allows"
    )
)]

use merlin_core::task_list::{StepStatus, StepType, TaskList, TaskStep};

#[test]
fn test_task_step_creation_with_defaults() {
    let step = TaskStep::new(
        "step_1".to_owned(),
        StepType::Feature,
        "Implement feature".to_owned(),
        "Feature works".to_owned(),
    );

    assert_eq!(step.id, "step_1");
    assert_eq!(step.step_type, StepType::Feature);
    assert_eq!(step.status, StepStatus::Pending);
    assert!(step.error.is_none());
    assert!(step.result.is_none());
    assert!(step.exit_command.is_none());
}

#[test]
fn test_task_step_with_custom_exit_command() {
    let step = TaskStep::with_exit_command(
        "step_1".to_owned(),
        StepType::Test,
        "Run tests".to_owned(),
        "Tests pass".to_owned(),
        "cargo test --all".to_owned(),
    );

    assert_eq!(step.get_exit_command(), "cargo test --all");
}

#[test]
fn test_task_step_default_exit_commands() {
    let debug_step = TaskStep::new(
        "debug".to_owned(),
        StepType::Debug,
        "Debug issue".to_owned(),
        "Issue found".to_owned(),
    );
    assert_eq!(debug_step.get_exit_command(), "cargo check");

    let feature_step = TaskStep::new(
        "feature".to_owned(),
        StepType::Feature,
        "Add feature".to_owned(),
        "Feature added".to_owned(),
    );
    assert_eq!(feature_step.get_exit_command(), "cargo check");

    let refactor_step = TaskStep::new(
        "refactor".to_owned(),
        StepType::Refactor,
        "Refactor code".to_owned(),
        "Code refactored".to_owned(),
    );
    assert_eq!(
        refactor_step.get_exit_command(),
        "cargo clippy -- -D warnings"
    );

    let test_step = TaskStep::new(
        "test".to_owned(),
        StepType::Test,
        "Test code".to_owned(),
        "Tests pass".to_owned(),
    );
    assert_eq!(test_step.get_exit_command(), "cargo test");

    let verify_step = TaskStep::new(
        "verify".to_owned(),
        StepType::Verify,
        "Verify".to_owned(),
        "Verified".to_owned(),
    );
    assert_eq!(verify_step.get_exit_command(), "cargo check");
}

#[test]
fn test_task_step_state_transitions() {
    let mut step = TaskStep::new(
        "step".to_owned(),
        StepType::Feature,
        "Task".to_owned(),
        "Verified".to_owned(),
    );

    // Pending -> InProgress
    assert_eq!(step.status, StepStatus::Pending);
    step.start();
    assert_eq!(step.status, StepStatus::InProgress);

    // InProgress -> Completed
    step.complete(Some("Success!".to_owned()));
    assert_eq!(step.status, StepStatus::Completed);
    assert_eq!(step.result, Some("Success!".to_owned()));
    assert!(step.error.is_none());
    assert!(step.is_completed());
    assert!(!step.is_failed());
}

#[test]
fn test_task_step_failure() {
    let mut step = TaskStep::new(
        "step".to_owned(),
        StepType::Test,
        "Test".to_owned(),
        "Pass".to_owned(),
    );

    step.start();
    step.fail("Test failed: assertion error".to_owned());

    assert_eq!(step.status, StepStatus::Failed);
    assert_eq!(step.error, Some("Test failed: assertion error".to_owned()));
    assert!(!step.is_completed());
    assert!(step.is_failed());
}

#[test]
fn test_task_step_skip() {
    let mut step = TaskStep::new(
        "optional".to_owned(),
        StepType::Feature,
        "Optional task".to_owned(),
        "Not needed".to_owned(),
    );

    step.skip();
    assert_eq!(step.status, StepStatus::Skipped);
    assert!(!step.is_completed());
    assert!(!step.is_failed());
}

#[test]
fn test_task_step_is_pending_or_in_progress() {
    let mut step = TaskStep::new(
        "step".to_owned(),
        StepType::Feature,
        "Task".to_owned(),
        "Done".to_owned(),
    );

    assert!(step.is_pending_or_in_progress());

    step.start();
    assert!(step.is_pending_or_in_progress());

    step.complete(None);
    assert!(!step.is_pending_or_in_progress());
}

#[test]
fn test_task_list_creation() {
    let steps = vec![
        TaskStep::new(
            "step_1".to_owned(),
            StepType::Feature,
            "First".to_owned(),
            "Done".to_owned(),
        ),
        TaskStep::new(
            "step_2".to_owned(),
            StepType::Test,
            "Second".to_owned(),
            "Pass".to_owned(),
        ),
    ];

    let task_list = TaskList::new("list_1".to_owned(), "My Task List".to_owned(), steps);

    assert_eq!(task_list.id, "list_1");
    assert_eq!(task_list.title, "My Task List");
    assert_eq!(task_list.steps.len(), 2);
}

#[test]
fn test_task_list_completion_tracking() {
    let mut task_list = TaskList::new(
        "list".to_owned(),
        "List".to_owned(),
        vec![
            TaskStep::new(
                "step_1".to_owned(),
                StepType::Feature,
                "First".to_owned(),
                "Done".to_owned(),
            ),
            TaskStep::new(
                "step_2".to_owned(),
                StepType::Test,
                "Second".to_owned(),
                "Pass".to_owned(),
            ),
        ],
    );

    assert!(!task_list.is_complete());

    // Complete first step
    task_list.steps[0].complete(None);
    assert!(!task_list.is_complete());

    // Complete second step
    task_list.steps[1].complete(None);
    task_list.update_status();
    assert!(task_list.is_complete());
}

#[test]
fn test_task_list_failure_detection() {
    let mut task_list = TaskList::new(
        "list".to_owned(),
        "List".to_owned(),
        vec![
            TaskStep::new(
                "step_1".to_owned(),
                StepType::Feature,
                "First".to_owned(),
                "Done".to_owned(),
            ),
            TaskStep::new(
                "step_2".to_owned(),
                StepType::Test,
                "Second".to_owned(),
                "Pass".to_owned(),
            ),
        ],
    );

    task_list.steps[0].complete(None);
    task_list.steps[1].fail("Test failed".to_owned());
    task_list.update_status();

    assert!(!task_list.is_complete());
    assert!(task_list.has_failures());
}

#[test]
fn test_empty_task_list() {
    let task_list = TaskList::new("empty".to_owned(), "Empty List".to_owned(), vec![]);

    assert_eq!(task_list.steps.len(), 0);
    assert!(task_list.is_complete()); // Empty list is complete by default
}

#[test]
fn test_task_list_with_skipped_steps() {
    let mut task_list = TaskList::new(
        "list".to_owned(),
        "List".to_owned(),
        vec![
            TaskStep::new(
                "step_1".to_owned(),
                StepType::Feature,
                "Required".to_owned(),
                "Done".to_owned(),
            ),
            TaskStep::new(
                "step_2".to_owned(),
                StepType::Feature,
                "Optional".to_owned(),
                "Skip".to_owned(),
            ),
            TaskStep::new(
                "step_3".to_owned(),
                StepType::Verify,
                "Verify".to_owned(),
                "Check".to_owned(),
            ),
        ],
    );

    task_list.steps[0].complete(None);
    task_list.steps[1].skip();
    task_list.steps[2].complete(None);
    task_list.update_status();

    assert!(task_list.is_complete());
}

#[test]
fn test_task_list_partial_completion() {
    let mut task_list = TaskList::new(
        "list".to_owned(),
        "Partial".to_owned(),
        vec![
            TaskStep::new(
                "step_1".to_owned(),
                StepType::Feature,
                "First".to_owned(),
                "Done".to_owned(),
            ),
            TaskStep::new(
                "step_2".to_owned(),
                StepType::Feature,
                "Second".to_owned(),
                "Done".to_owned(),
            ),
            TaskStep::new(
                "step_3".to_owned(),
                StepType::Test,
                "Third".to_owned(),
                "Pass".to_owned(),
            ),
        ],
    );

    task_list.steps[0].complete(None);
    task_list.steps[1].start(); // In progress
    // step 3 still pending

    assert!(!task_list.is_complete());
    assert!(!task_list.has_failures());
}

#[test]
fn test_step_type_display() {
    assert_eq!(format!("{}", StepType::Debug), "Debug");
    assert_eq!(format!("{}", StepType::Feature), "Feature");
    assert_eq!(format!("{}", StepType::Refactor), "Refactor");
    assert_eq!(format!("{}", StepType::Verify), "Verify");
    assert_eq!(format!("{}", StepType::Test), "Test");
}

#[test]
fn test_step_status_display() {
    assert_eq!(format!("{}", StepStatus::Pending), "⏳ Pending");
    assert_eq!(format!("{}", StepStatus::InProgress), "🔄 In Progress");
    assert_eq!(format!("{}", StepStatus::Completed), "✅ Completed");
    assert_eq!(format!("{}", StepStatus::Failed), "❌ Failed");
    assert_eq!(format!("{}", StepStatus::Skipped), "⏭️ Skipped");
}

#[test]
fn test_task_step_complete_clears_error() {
    let mut step = TaskStep::new(
        "step".to_owned(),
        StepType::Feature,
        "Task".to_owned(),
        "Done".to_owned(),
    );

    step.fail("Initial failure".to_owned());
    assert!(step.error.is_some());

    step.complete(Some("Fixed!".to_owned()));
    assert!(step.error.is_none());
    assert_eq!(step.result, Some("Fixed!".to_owned()));
}

#[test]
fn test_task_list_all_steps_completed() {
    let mut task_list = TaskList::new(
        "list".to_owned(),
        "Complete List".to_owned(),
        vec![
            TaskStep::new(
                "step_1".to_owned(),
                StepType::Feature,
                "Step 1".to_owned(),
                "Done".to_owned(),
            ),
            TaskStep::new(
                "step_2".to_owned(),
                StepType::Test,
                "Step 2".to_owned(),
                "Pass".to_owned(),
            ),
            TaskStep::new(
                "step_3".to_owned(),
                StepType::Verify,
                "Step 3".to_owned(),
                "Check".to_owned(),
            ),
        ],
    );

    for step in &mut task_list.steps {
        step.start();
        step.complete(Some("Success".to_owned()));
    }

    task_list.update_status();
    assert!(task_list.is_complete());
}
