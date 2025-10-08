//! Tests for UI event processing and state transitions
#![cfg(test)]

mod common;

use common::*;
use merlin_routing::TaskId;
use merlin_routing::user_interface::events::{MessageLevel, TaskProgress, UiEvent};
use merlin_routing::user_interface::output_tree::StepType;
use merlin_routing::user_interface::task_manager::{TaskManager, TaskStatus};
use std::time::Instant;

/// Handle task lifecycle events
fn handle_task_lifecycle(manager: &mut TaskManager, event: UiEvent) {
    match event {
        UiEvent::TaskStarted {
            task_id,
            description,
            parent_id,
        } => {
            let mut task = create_test_task(&description);
            task.parent_id = parent_id;
            manager.add_task(task_id, task);
        }
        UiEvent::TaskCompleted { task_id, .. } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.status = TaskStatus::Completed;
                task.end_time = Some(Instant::now());
            }
        }
        UiEvent::TaskFailed { task_id, .. } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.status = TaskStatus::Failed;
                task.end_time = Some(Instant::now());
            }
        }
        _ => {}
    }
}

/// Handle task progress and output events
fn handle_task_updates(manager: &mut TaskManager, event: UiEvent) {
    match event {
        UiEvent::TaskProgress {
            task_id, progress, ..
        } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.progress = Some(progress);
            }
        }
        UiEvent::TaskOutput { task_id, output } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.output_lines.push(output);
            }
        }
        UiEvent::ThinkingUpdate { task_id, content } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.output_lines.push(format!("Thinking: {content}"));
            }
        }
        _ => {}
    }
}

/// Handle task step events
fn handle_task_steps(manager: &mut TaskManager, event: UiEvent) {
    match event {
        UiEvent::TaskStepStarted {
            task_id,
            step_id,
            step_type,
            content,
        } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.output_tree
                    .add_step(step_id, StepType::from_str(&step_type), content);
            }
        }
        UiEvent::TaskStepCompleted { task_id, step_id } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.output_tree.complete_step(&step_id);
            }
        }
        _ => {}
    }
}

/// Handle tool call events
fn handle_tool_calls(manager: &mut TaskManager, event: UiEvent) {
    match event {
        UiEvent::ToolCallStarted { task_id, tool, .. } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.output_lines.push(format!("Calling tool: {tool}"));
            }
        }
        UiEvent::ToolCallCompleted {
            task_id, result, ..
        } => {
            if let Some(task) = manager.get_task_mut(task_id) {
                task.output_lines.push(format!("Tool result: {result}"));
            }
        }
        _ => {}
    }
}

/// Handle subtask spawning
fn handle_subtasks(manager: &mut TaskManager, event: UiEvent) {
    if let UiEvent::SubtaskSpawned {
        parent_id,
        child_id,
        description,
    } = event
    {
        let mut task = create_test_task(&description);
        task.parent_id = Some(parent_id);
        manager.add_task(child_id, task);
    }
}

/// Simulates processing a UI event by manually updating the task manager
fn process_event(manager: &mut TaskManager, event: UiEvent) {
    handle_task_lifecycle(manager, event.clone());
    handle_task_updates(manager, event.clone());
    handle_task_steps(manager, event.clone());
    handle_tool_calls(manager, event.clone());
    handle_subtasks(manager, event);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_started_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    let event = UiEvent::TaskStarted {
        task_id,
        description: "New task".to_string(),
        parent_id: None,
    };

    process_event(&mut manager, event);

    assert!(!manager.is_empty());
    assert!(manager.get_task(task_id).is_some());
    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert_eq!(task.description, "New task");
    assert_eq!(task.status, TaskStatus::Running);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_started_with_parent() {
    let mut manager = TaskManager::default();
    let parent_id = TaskId::default();
    let child_id = TaskId::default();

    process_event(
        &mut manager,
        UiEvent::TaskStarted {
            task_id: parent_id,
            description: "Parent".to_string(),
            parent_id: None,
        },
    );

    process_event(
        &mut manager,
        UiEvent::TaskStarted {
            task_id: child_id,
            description: "Child".to_string(),
            parent_id: Some(parent_id),
        },
    );

    assert_eq!(manager.task_order().len(), 2);
    let Some(child_task) = manager.get_task(child_id) else {
        panic!("Child task should exist");
    };
    assert_eq!(child_task.parent_id, Some(parent_id));
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_progress_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    let progress = TaskProgress {
        stage: "Analyzing".to_string(),
        current: 50,
        total: Some(100),
        message: "Halfway done".to_string(),
    };

    process_event(&mut manager, UiEvent::TaskProgress { task_id, progress });

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert!(task.progress.is_some());
    let Some(task_progress) = task.progress.as_ref() else {
        panic!("Progress should exist");
    };
    assert_eq!(task_progress.stage, "Analyzing");
    assert_eq!(task_progress.current, 50);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_output_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    process_event(
        &mut manager,
        UiEvent::TaskOutput {
            task_id,
            output: "First line".to_string(),
        },
    );

    process_event(
        &mut manager,
        UiEvent::TaskOutput {
            task_id,
            output: "Second line".to_string(),
        },
    );

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert_eq!(task.output_lines.len(), 2);
    assert_eq!(task.output_lines[0], "First line");
    assert_eq!(task.output_lines[1], "Second line");
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_completed_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    process_event(
        &mut manager,
        UiEvent::TaskCompleted {
            task_id,
            result: create_test_task_result(task_id, "Success"),
        },
    );

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.end_time.is_some());
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_failed_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    process_event(
        &mut manager,
        UiEvent::TaskFailed {
            task_id,
            error: "Build failed".to_string(),
        },
    );

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert_eq!(task.status, TaskStatus::Failed);
    assert!(task.end_time.is_some());
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_step_started_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    process_event(
        &mut manager,
        UiEvent::TaskStepStarted {
            task_id,
            step_id: "step1".to_string(),
            step_type: "Thinking".to_string(),
            content: "Analyzing code".to_string(),
        },
    );

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    let nodes = task.output_tree.flatten_visible_nodes();
    assert!(!nodes.is_empty());
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_step_completed_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    process_event(
        &mut manager,
        UiEvent::TaskStepStarted {
            task_id,
            step_id: "step1".to_string(),
            step_type: "Thinking".to_string(),
            content: "Analyzing".to_string(),
        },
    );

    process_event(
        &mut manager,
        UiEvent::TaskStepCompleted {
            task_id,
            step_id: "step1".to_string(),
        },
    );

    // Step should be completed (no panic)
    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert!(!task.output_tree.flatten_visible_nodes().is_empty());
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_tool_call_started_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    process_event(
        &mut manager,
        UiEvent::ToolCallStarted {
            task_id,
            tool: "read_file".to_string(),
            args: serde_json::json!({"path": "main.rs"}),
        },
    );

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert!(!task.output_lines.is_empty());
    assert!(task.output_lines[0].contains("read_file"));
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_tool_call_completed_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    process_event(
        &mut manager,
        UiEvent::ToolCallStarted {
            task_id,
            tool: "read_file".to_string(),
            args: serde_json::json!({"path": "main.rs"}),
        },
    );

    process_event(
        &mut manager,
        UiEvent::ToolCallCompleted {
            task_id,
            tool: "read_file".to_string(),
            result: serde_json::json!({"content": "fn main() {}"}),
        },
    );

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert_eq!(task.output_lines.len(), 2);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_thinking_update_event() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    process_event(
        &mut manager,
        UiEvent::ThinkingUpdate {
            task_id,
            content: "Considering approaches...".to_string(),
        },
    );

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert!(!task.output_lines.is_empty());
    assert!(task.output_lines[0].contains("Thinking"));
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_subtask_spawned_event() {
    let mut manager = TaskManager::default();
    let parent_id = TaskId::default();
    let child_id = TaskId::default();

    manager.add_task(parent_id, create_test_task("Parent"));

    process_event(
        &mut manager,
        UiEvent::SubtaskSpawned {
            parent_id,
            child_id,
            description: "Subtask".to_string(),
        },
    );

    assert_eq!(manager.task_order().len(), 2);
    let Some(child_task) = manager.get_task(child_id) else {
        panic!("Child task should exist");
    };
    assert_eq!(child_task.parent_id, Some(parent_id));
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_system_message_info() {
    let mut manager = TaskManager::default();

    let event = UiEvent::SystemMessage {
        level: MessageLevel::Info,
        message: "System ready".to_string(),
    };

    // Should not panic
    process_event(&mut manager, event);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_system_message_warning() {
    let mut manager = TaskManager::default();

    let event = UiEvent::SystemMessage {
        level: MessageLevel::Warning,
        message: "High memory usage".to_string(),
    };

    // Should not panic
    process_event(&mut manager, event);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_system_message_error() {
    let mut manager = TaskManager::default();

    let event = UiEvent::SystemMessage {
        level: MessageLevel::Error,
        message: "Connection lost".to_string(),
    };

    // Should not panic
    process_event(&mut manager, event);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_multiple_events_sequence() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    // Start task
    process_event(
        &mut manager,
        UiEvent::TaskStarted {
            task_id,
            description: "Complex task".to_string(),
            parent_id: None,
        },
    );

    // Add progress
    process_event(
        &mut manager,
        UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage: "Phase 1".to_string(),
                current: 33,
                total: Some(100),
                message: "Working...".to_string(),
            },
        },
    );

    // Add output
    process_event(
        &mut manager,
        UiEvent::TaskOutput {
            task_id,
            output: "Processing file 1".to_string(),
        },
    );

    // Update progress
    process_event(
        &mut manager,
        UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage: "Phase 2".to_string(),
                current: 66,
                total: Some(100),
                message: "Almost done...".to_string(),
            },
        },
    );

    // Complete task
    process_event(
        &mut manager,
        UiEvent::TaskCompleted {
            task_id,
            result: create_test_task_result(task_id, "Done"),
        },
    );

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    assert_eq!(task.status, TaskStatus::Completed);
    let Some(progress) = task.progress.as_ref() else {
        panic!("Progress should exist");
    };
    assert_eq!(progress.stage, "Phase 2");
    assert_eq!(task.output_lines.len(), 1);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_parallel_tasks_events() {
    let mut manager = TaskManager::default();

    let task1 = TaskId::default();
    let task2 = TaskId::default();

    process_event(
        &mut manager,
        UiEvent::TaskStarted {
            task_id: task1,
            description: "Task 1".to_string(),
            parent_id: None,
        },
    );

    process_event(
        &mut manager,
        UiEvent::TaskStarted {
            task_id: task2,
            description: "Task 2".to_string(),
            parent_id: None,
        },
    );

    process_event(
        &mut manager,
        UiEvent::TaskCompleted {
            task_id: task1,
            result: create_test_task_result(task1, "Done"),
        },
    );

    assert_eq!(manager.task_order().len(), 2);
    let Some(task1_ref) = manager.get_task(task1) else {
        panic!("Task 1 should exist");
    };
    assert_eq!(task1_ref.status, TaskStatus::Completed);
    let Some(task2_ref) = manager.get_task(task2) else {
        panic!("Task 2 should exist");
    };
    assert_eq!(task2_ref.status, TaskStatus::Running);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_event_for_nonexistent_task() {
    let mut manager = TaskManager::default();
    let nonexistent_id = TaskId::default();

    // Should not panic, just silently ignore
    process_event(
        &mut manager,
        UiEvent::TaskOutput {
            task_id: nonexistent_id,
            output: "Output".to_string(),
        },
    );

    assert!(manager.is_empty());
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_multiple_progress_updates() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    for index in 1..=10 {
        process_event(
            &mut manager,
            UiEvent::TaskProgress {
                task_id,
                progress: TaskProgress {
                    stage: format!("Stage {index}"),
                    current: index * 10,
                    total: Some(100),
                    message: format!("{index}0% complete"),
                },
            },
        );
    }

    let Some(task) = manager.get_task(task_id) else {
        panic!("Task should exist");
    };
    let Some(progress) = task.progress.as_ref() else {
        panic!("Progress should exist");
    };
    assert_eq!(progress.current, 100);
    assert_eq!(progress.stage, "Stage 10");
}
