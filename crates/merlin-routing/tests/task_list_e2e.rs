//! End-to-end tests for `TaskList` workflow
//!
//! These tests verify that agents can return `TaskList` objects for multi-step workflows
//! and that the structure is correctly parsed and validated.

#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        clippy::min_ident_chars,
        clippy::shadow_unrelated,
        reason = "Test allows"
    )
)]

use merlin_core::{StepStatus, TaskList, TaskListStatus, TaskStepType};
use merlin_routing::TypeScriptRuntime;
use serde_json::from_value;

/// Helper to create a TypeScript runtime for testing
fn create_typescript_runtime() -> TypeScriptRuntime {
    TypeScriptRuntime::new()
}

/// Mock agent response that returns a simple `TaskList`
const MOCK_SIMPLE_TASK_LIST: &str = r#"
async function agent_code() {
    return {
        id: "task_1",
        title: "Read configuration file",
        steps: [
            {
                id: "step_1",
                step_type: "Debug",
                description: "Locate config.toml file",
                verification: "File path is found",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            },
            {
                id: "step_2",
                step_type: "Feature",
                description: "Read config.toml contents",
                verification: "File contents are loaded",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            }
        ],
        status: "NotStarted"
    };
}
"#;

/// Mock agent response that returns a complex bug fix `TaskList`
const MOCK_BUG_FIX_TASK_LIST: &str = r#"
async function agent_code() {
    return {
        id: "fix_auth_bug",
        title: "Fix authentication timeout issue",
        steps: [
            {
                id: "step_1",
                step_type: "Debug",
                description: "Read auth.rs to understand current implementation",
                verification: "File loads and code structure is clear",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            },
            {
                id: "step_2",
                step_type: "Feature",
                description: "Add timeout configuration to AuthConfig struct",
                verification: "Code compiles without errors",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            },
            {
                id: "step_3",
                step_type: "Verify",
                description: "Run cargo check on auth module",
                verification: "cargo check passes",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            },
            {
                id: "step_4",
                step_type: "Test",
                description: "Run authentication tests",
                verification: "All tests pass",
                status: "Pending",
                error: null,
                result: null,
                exit_command: "cargo test --lib auth"
            }
        ],
        status: "NotStarted"
    };
}
"#;

/// Mock agent response for refactoring workflow with `TaskList`
const MOCK_REFACTOR_TASK_LIST: &str = r#"
async function agent_code() {
    return {
        id: "refactor_router",
        title: "Refactor routing module for better maintainability",
        steps: [
            {
                id: "step_1",
                step_type: "Debug",
                description: "Analyze current router.rs structure",
                verification: "Identify code smells and improvement areas",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            },
            {
                id: "step_2",
                step_type: "Refactor",
                description: "Extract strategy selection into separate module",
                verification: "Code is cleaner and more modular",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            },
            {
                id: "step_3",
                step_type: "Refactor",
                description: "Simplify tier selection logic",
                verification: "Logic is easier to understand",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            },
            {
                id: "step_4",
                step_type: "Verify",
                description: "Run cargo clippy to check code quality",
                verification: "No clippy warnings",
                status: "Pending",
                error: null,
                result: null,
                exit_command: "cargo clippy --all-targets -- -D warnings"
            },
            {
                id: "step_5",
                step_type: "Test",
                description: "Run all routing tests",
                verification: "All tests pass",
                status: "Pending",
                error: null,
                result: null,
                exit_command: null
            }
        ],
        status: "NotStarted"
    };
}
"#;

#[tokio::test]
async fn test_simple_task_list_structure() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_SIMPLE_TASK_LIST).await.unwrap();

    // Parse as TaskList
    let task_list: TaskList = from_value(result).unwrap();

    // Verify basic structure
    assert_eq!(task_list.id, "task_1");
    assert_eq!(task_list.title, "Read configuration file");
    assert_eq!(task_list.steps.len(), 2);
    assert!(matches!(task_list.status, TaskListStatus::NotStarted));

    // Verify first step
    let step1 = &task_list.steps[0];
    assert_eq!(step1.id, "step_1");
    assert!(matches!(step1.step_type, TaskStepType::Debug));
    assert_eq!(step1.description, "Locate config.toml file");
    assert_eq!(step1.verification, "File path is found");
    assert!(matches!(step1.status, StepStatus::Pending));
    assert!(step1.error.is_none());
    assert!(step1.result.is_none());

    // Verify second step
    let step2 = &task_list.steps[1];
    assert_eq!(step2.id, "step_2");
    assert!(matches!(step2.step_type, TaskStepType::Feature));
    assert_eq!(step2.description, "Read config.toml contents");
    assert_eq!(step2.verification, "File contents are loaded");
    assert!(matches!(step2.status, StepStatus::Pending));
}

#[tokio::test]
async fn test_bug_fix_task_list_workflow() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_BUG_FIX_TASK_LIST).await.unwrap();

    let task_list: TaskList = from_value(result).unwrap();

    // Verify structure
    assert_eq!(task_list.id, "fix_auth_bug");
    assert_eq!(task_list.title, "Fix authentication timeout issue");
    assert_eq!(task_list.steps.len(), 4);

    // Verify step types follow Debug → Feature → Verify → Test pattern
    assert!(matches!(task_list.steps[0].step_type, TaskStepType::Debug));
    assert!(matches!(
        task_list.steps[1].step_type,
        TaskStepType::Feature
    ));
    assert!(matches!(task_list.steps[2].step_type, TaskStepType::Verify));
    assert!(matches!(task_list.steps[3].step_type, TaskStepType::Test));

    // Verify all steps start as Pending
    for step in &task_list.steps {
        assert!(matches!(step.status, StepStatus::Pending));
        assert!(step.error.is_none());
        assert!(step.result.is_none());
    }

    // Verify each step has verification requirements
    for step in &task_list.steps {
        assert!(!step.verification.is_empty());
    }
}

#[tokio::test]
async fn test_refactor_task_list_workflow() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_REFACTOR_TASK_LIST).await.unwrap();

    let task_list: TaskList = from_value(result).unwrap();

    // Verify structure
    assert_eq!(task_list.id, "refactor_router");
    assert_eq!(
        task_list.title,
        "Refactor routing module for better maintainability"
    );
    assert_eq!(task_list.steps.len(), 5);

    // Verify step types include multiple refactor steps
    assert!(matches!(task_list.steps[0].step_type, TaskStepType::Debug));
    assert!(matches!(
        task_list.steps[1].step_type,
        TaskStepType::Refactor
    ));
    assert!(matches!(
        task_list.steps[2].step_type,
        TaskStepType::Refactor
    ));
    assert!(matches!(task_list.steps[3].step_type, TaskStepType::Verify));
    assert!(matches!(task_list.steps[4].step_type, TaskStepType::Test));

    // Count step types
    let debug_count = task_list
        .steps
        .iter()
        .filter(|s| matches!(s.step_type, TaskStepType::Debug))
        .count();
    let refactor_count = task_list
        .steps
        .iter()
        .filter(|s| matches!(s.step_type, TaskStepType::Refactor))
        .count();
    let verify_count = task_list
        .steps
        .iter()
        .filter(|s| matches!(s.step_type, TaskStepType::Verify))
        .count();
    let test_count = task_list
        .steps
        .iter()
        .filter(|s| matches!(s.step_type, TaskStepType::Test))
        .count();

    assert_eq!(debug_count, 1);
    assert_eq!(refactor_count, 2);
    assert_eq!(verify_count, 1);
    assert_eq!(test_count, 1);
}

#[tokio::test]
async fn test_task_list_step_ids_are_unique() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_BUG_FIX_TASK_LIST).await.unwrap();

    let task_list: TaskList = from_value(result).unwrap();

    // Collect all step IDs
    let step_ids: Vec<&str> = task_list.steps.iter().map(|s| s.id.as_str()).collect();

    // Verify uniqueness
    let mut unique_ids = step_ids.clone();
    unique_ids.sort_unstable();
    unique_ids.dedup();

    assert_eq!(
        step_ids.len(),
        unique_ids.len(),
        "Step IDs should be unique"
    );
}

#[tokio::test]
async fn test_task_list_all_steps_have_descriptions() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_REFACTOR_TASK_LIST).await.unwrap();

    let task_list: TaskList = from_value(result).unwrap();

    // Verify all steps have non-empty descriptions
    for step in &task_list.steps {
        assert!(
            !step.description.is_empty(),
            "Step {} should have a description",
            step.id
        );
        assert!(
            !step.verification.is_empty(),
            "Step {} should have verification requirements",
            step.id
        );
    }
}

#[tokio::test]
async fn test_task_list_status_starts_not_started() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_SIMPLE_TASK_LIST).await.unwrap();

    let task_list: TaskList = from_value(result).unwrap();

    // Verify initial status
    assert!(matches!(task_list.status, TaskListStatus::NotStarted));

    // Verify all steps are pending
    for step in &task_list.steps {
        assert!(matches!(step.status, StepStatus::Pending));
    }
}

#[tokio::test]
async fn test_task_list_step_lifecycle_methods() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_SIMPLE_TASK_LIST).await.unwrap();

    let mut task_list: TaskList = from_value(result).unwrap();

    // Test getting next pending step
    let next_step = task_list.next_pending_step();
    assert!(next_step.is_some());
    assert_eq!(next_step.unwrap().id, "step_1");

    // Test completing a step
    if let Some(step) = task_list.get_step_mut("step_1") {
        step.start();
        assert!(matches!(step.status, StepStatus::InProgress));

        step.complete(Some("File located at config.toml".to_owned()));
        assert!(step.is_completed());
        assert_eq!(step.result, Some("File located at config.toml".to_owned()));
    }

    // Test next pending step after first complete
    let next_step = task_list.next_pending_step();
    assert!(next_step.is_some());
    assert_eq!(next_step.unwrap().id, "step_2");

    // Test failing a step
    if let Some(step) = task_list.get_step_mut("step_2") {
        step.start();
        step.fail("File not found".to_owned());
        assert!(step.is_failed());
        assert_eq!(step.error, Some("File not found".to_owned()));
    }

    // Update overall status
    task_list.update_status();
    assert!(matches!(task_list.status, TaskListStatus::Failed));
    assert_eq!(task_list.completed_count(), 1);
    assert_eq!(task_list.failed_count(), 1);
}

#[tokio::test]
async fn test_task_list_progress_tracking() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_BUG_FIX_TASK_LIST).await.unwrap();

    let mut task_list: TaskList = from_value(result).unwrap();

    // Initial progress
    assert_eq!(task_list.progress_percentage(), 0);
    assert_eq!(task_list.completed_count(), 0);
    assert_eq!(task_list.total_count(), 4);

    // Complete first step
    if let Some(step) = task_list.get_step_mut("step_1") {
        step.complete(None);
    }
    assert_eq!(task_list.completed_count(), 1);
    assert_eq!(task_list.progress_percentage(), 25);

    // Complete second step
    if let Some(step) = task_list.get_step_mut("step_2") {
        step.complete(None);
    }
    assert_eq!(task_list.completed_count(), 2);
    assert_eq!(task_list.progress_percentage(), 50);

    // Complete all steps
    if let Some(step) = task_list.get_step_mut("step_3") {
        step.complete(None);
    }
    if let Some(step) = task_list.get_step_mut("step_4") {
        step.complete(None);
    }
    assert_eq!(task_list.completed_count(), 4);
    assert_eq!(task_list.progress_percentage(), 100);

    task_list.update_status();
    assert!(task_list.is_complete());
}

#[tokio::test]
async fn test_exit_commands_default_and_custom() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_BUG_FIX_TASK_LIST).await.unwrap();

    let task_list: TaskList = from_value(result).unwrap();

    // Verify step 1-3 use default commands (exit_command is null)
    assert_eq!(task_list.steps[0].exit_command, None);
    assert_eq!(task_list.steps[1].exit_command, None);
    assert_eq!(task_list.steps[2].exit_command, None);

    // Verify step 4 has custom command
    assert_eq!(
        task_list.steps[3].exit_command,
        Some("cargo test --lib auth".to_owned())
    );

    // Verify get_exit_command returns correct values
    assert_eq!(task_list.steps[0].get_exit_command(), "cargo check");
    assert_eq!(task_list.steps[1].get_exit_command(), "cargo check");
    assert_eq!(task_list.steps[2].get_exit_command(), "cargo check");
    assert_eq!(
        task_list.steps[3].get_exit_command(),
        "cargo test --lib auth"
    );
}

#[tokio::test]
async fn test_default_exit_commands_by_type() {
    use merlin_core::TaskStepType;

    // Verify each step type has appropriate default command
    assert_eq!(TaskStepType::Debug.default_exit_command(), "cargo check");
    assert_eq!(TaskStepType::Feature.default_exit_command(), "cargo check");
    assert_eq!(
        TaskStepType::Refactor.default_exit_command(),
        "cargo clippy -- -D warnings"
    );
    assert_eq!(TaskStepType::Verify.default_exit_command(), "cargo check");
    assert_eq!(TaskStepType::Test.default_exit_command(), "cargo test");
}

#[tokio::test]
async fn test_custom_exit_command_in_refactor_workflow() {
    let ts_runtime = create_typescript_runtime();

    let result = ts_runtime.execute(MOCK_REFACTOR_TASK_LIST).await.unwrap();

    let task_list: TaskList = from_value(result).unwrap();

    // Check step 4 (Verify step) has custom clippy command
    assert_eq!(
        task_list.steps[3].exit_command,
        Some("cargo clippy --all-targets -- -D warnings".to_owned())
    );
    assert_eq!(
        task_list.steps[3].get_exit_command(),
        "cargo clippy --all-targets -- -D warnings"
    );

    // Other steps should use defaults
    assert_eq!(task_list.steps[0].get_exit_command(), "cargo check");
    assert_eq!(
        task_list.steps[1].get_exit_command(),
        "cargo clippy -- -D warnings"
    ); // Refactor type default
    assert_eq!(task_list.steps[4].get_exit_command(), "cargo test"); // Test type default
}
