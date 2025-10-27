//! Tests for task coordinator

use super::super::*;
use super::{MAX_CHECKPOINTS, MAX_DECOMPOSITION_DEPTH, MAX_SUBTASKS_PER_TASK};
use merlin_core::{Response, Subtask, Task, TaskId, TaskResult, TokenUsage, ValidationResult};
use tokio::spawn;

/// Helper to create a test `TaskResult`
fn create_test_result(task_id: TaskId) -> TaskResult {
    TaskResult {
        task_id,
        response: Response {
            text: "Test response".to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        },
        tier_used: "test".to_owned(),
        tokens_used: TokenUsage::default(),
        validation: ValidationResult::default(),
        duration_ms: 0,
        work_unit: None,
    }
}

#[tokio::test]
async fn test_coordinator_creation() {
    let coordinator = TaskCoordinator::default();
    let stats = coordinator.get_stats().await;
    assert_eq!(stats.total_tasks, 0);
}

#[tokio::test]
async fn test_register_task() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Test task".to_owned());

    coordinator.register_task(task.clone(), None).await.unwrap();

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.total_tasks, 1);
    assert_eq!(stats.pending, 1);
}

#[tokio::test]
async fn test_max_depth_enforcement() {
    let coordinator = TaskCoordinator::new();
    let mut parent_id = None;

    for depth_level in 0..=(MAX_DECOMPOSITION_DEPTH + 1) {
        let task = Task::new(format!("Task at depth {depth_level}"));
        let task_id = task.id;

        let result = coordinator.register_task(task, parent_id).await;

        if depth_level <= MAX_DECOMPOSITION_DEPTH {
            result.unwrap();
            parent_id = Some(task_id);
        } else {
            assert!(result.is_err());
        }
    }
}

#[tokio::test]
async fn test_max_subtasks_enforcement() {
    let coordinator = TaskCoordinator::new();
    let parent_task = Task::new("Parent task".to_owned());
    let parent_id = parent_task.id;
    coordinator.register_task(parent_task, None).await.unwrap();

    // Add up to MAX_SUBTASKS_PER_TASK
    for idx in 0..MAX_SUBTASKS_PER_TASK {
        let subtask = Task::new(format!("Subtask {idx}"));
        coordinator
            .register_task(subtask, Some(parent_id))
            .await
            .unwrap();
    }

    // Next subtask should fail
    let extra_subtask = Task::new("Extra subtask".to_owned());
    let result = coordinator
        .register_task(extra_subtask, Some(parent_id))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_task_completion() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Test task".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.unwrap();
    coordinator.start_task(task_id).await.unwrap();

    let result = create_test_result(task_id);
    coordinator.complete_subtask(task_id, result).await.unwrap();

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.completed, 1);
}

#[tokio::test]
async fn test_decomposition() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Parent task".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.unwrap();

    let subtask_specs = vec![
        Subtask::new("Subtask 1".to_owned(), 1),
        Subtask::new("Subtask 2".to_owned(), 1),
    ];

    let subtasks = coordinator
        .decompose_task(task_id, subtask_specs)
        .await
        .unwrap();

    assert_eq!(subtasks.len(), 2);

    let progress = coordinator.get_progress(task_id).await.unwrap();
    assert_eq!(progress.total_subtasks, 2);
    assert_eq!(progress.completed_subtasks, 0);
}

#[tokio::test]
async fn test_checkpoint_creation() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Test task".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.unwrap();
    coordinator
        .create_checkpoint(task_id, "Initial checkpoint".to_owned())
        .await
        .unwrap();

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.checkpoints, 1);
}

#[tokio::test]
async fn test_concurrent_task_registration() {
    let coordinator = TaskCoordinator::new();

    let mut handles = vec![];

    for idx in 0..10 {
        let coord = coordinator.clone();
        let handle = spawn(async move {
            let task = Task::new(format!("Concurrent task {idx}"));
            coord.register_task(task, None).await
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.total_tasks, 10);
}

#[tokio::test]
async fn test_progress_tracking() {
    let coordinator = TaskCoordinator::new();
    let parent = Task::new("Parent".to_owned());
    let parent_id = parent.id;

    coordinator.register_task(parent, None).await.unwrap();

    let subtask_specs = vec![
        Subtask::new("Sub 1".to_owned(), 1),
        Subtask::new("Sub 2".to_owned(), 1),
        Subtask::new("Sub 3".to_owned(), 1),
    ];

    let subtasks = coordinator
        .decompose_task(parent_id, subtask_specs)
        .await
        .unwrap();

    for subtask in subtasks {
        coordinator
            .register_task(subtask.clone(), Some(parent_id))
            .await
            .unwrap();
    }

    let initial_progress = coordinator.get_progress(parent_id).await.unwrap();
    assert!(initial_progress.progress_percent.abs() < f32::EPSILON);

    let first_subtask_id = coordinator
        .get_subtasks(parent_id)
        .await
        .unwrap()
        .first()
        .unwrap()
        .id;
    coordinator
        .complete_subtask(first_subtask_id, create_test_result(first_subtask_id))
        .await
        .unwrap();

    let updated_progress = coordinator.get_progress(parent_id).await.unwrap();
    assert!((updated_progress.progress_percent - 33.33).abs() < 0.1);
}

#[tokio::test]
async fn test_cleanup_old_tasks() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Old task".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.unwrap();
    coordinator
        .complete_subtask(task_id, create_test_result(task_id))
        .await
        .unwrap();

    // Tasks completed just now shouldn't be cleaned up
    let count = coordinator.cleanup_old_tasks(60).await.unwrap();
    assert_eq!(count, 0);

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.total_tasks, 1);
}

#[tokio::test]
async fn test_subtask_hierarchy() {
    let coordinator = TaskCoordinator::new();
    let parent = Task::new("Parent".to_owned());
    let parent_id = parent.id;
    coordinator.register_task(parent, None).await.unwrap();

    let child1 = Task::new("Child 1".to_owned());
    let child1_id = child1.id;
    coordinator
        .register_task(child1, Some(parent_id))
        .await
        .unwrap();

    let child2 = Task::new("Child 2".to_owned());
    coordinator
        .register_task(child2, Some(parent_id))
        .await
        .unwrap();

    let subtasks = coordinator.get_subtasks(parent_id).await.unwrap();
    assert_eq!(subtasks.len(), 2);

    let grandchild = Task::new("Grandchild".to_owned());
    coordinator
        .register_task(grandchild, Some(child1_id))
        .await
        .unwrap();

    let child1_subtasks = coordinator.get_subtasks(child1_id).await.unwrap();
    assert_eq!(child1_subtasks.len(), 1);
}

#[tokio::test]
async fn test_is_ready() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Test task".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.unwrap();

    assert!(coordinator.is_ready(task_id).await.unwrap());

    coordinator.start_task(task_id).await.unwrap();
    assert!(!coordinator.is_ready(task_id).await.unwrap());
}

#[tokio::test]
async fn test_parent_completion_when_all_subtasks_complete() {
    let coordinator = TaskCoordinator::new();
    let parent = Task::new("Parent".to_owned());
    let parent_id = parent.id;
    coordinator.register_task(parent, None).await.unwrap();

    let child1 = Task::new("Child 1".to_owned());
    let child1_id = child1.id;
    coordinator
        .register_task(child1, Some(parent_id))
        .await
        .unwrap();

    let child2 = Task::new("Child 2".to_owned());
    let child2_id = child2.id;
    coordinator
        .register_task(child2, Some(parent_id))
        .await
        .unwrap();

    coordinator
        .complete_subtask(child1_id, create_test_result(child1_id))
        .await
        .unwrap();
    coordinator
        .complete_subtask(child2_id, create_test_result(child2_id))
        .await
        .unwrap();

    let progress = coordinator.get_progress(parent_id).await.unwrap();
    assert_eq!(progress.status, TaskStatus::Completed);
}

#[tokio::test]
async fn test_max_checkpoints() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Test".to_owned());
    let task_id = task.id;
    coordinator.register_task(task, None).await.unwrap();

    for idx in 0..=MAX_CHECKPOINTS {
        coordinator
            .create_checkpoint(task_id, format!("Checkpoint {idx}"))
            .await
            .unwrap();
    }

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.checkpoints, MAX_CHECKPOINTS);
}

#[tokio::test]
async fn test_coordinator_stats() {
    let coordinator = TaskCoordinator::new();

    let task1 = Task::new("Task 1".to_owned());
    let task2 = Task::new("Task 2".to_owned());
    let task3 = Task::new("Task 3".to_owned());

    coordinator
        .register_task(task1.clone(), None)
        .await
        .unwrap();
    coordinator
        .register_task(task2.clone(), None)
        .await
        .unwrap();
    coordinator
        .register_task(task3.clone(), None)
        .await
        .unwrap();

    coordinator.start_task(task2.id).await.unwrap();
    coordinator
        .complete_subtask(task3.id, create_test_result(task3.id))
        .await
        .unwrap();

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.total_tasks, 3);
    assert_eq!(stats.pending, 1);
    assert_eq!(stats.in_progress, 1);
    assert_eq!(stats.completed, 1);
}

#[tokio::test]
async fn test_decompose_too_many_subtasks() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Parent".to_owned());
    let task_id = task.id;
    coordinator.register_task(task, None).await.unwrap();

    let too_many: Vec<Subtask> = (0..=MAX_SUBTASKS_PER_TASK)
        .map(|idx| Subtask::new(format!("Subtask {idx}"), 1))
        .collect();

    let result = coordinator.decompose_task(task_id, too_many).await;
    result.unwrap_err();
}

#[tokio::test]
async fn test_task_not_found_errors() {
    let coordinator = TaskCoordinator::new();
    let fake_id = TaskId::default();

    coordinator.get_progress(fake_id).await.unwrap_err();
    coordinator.get_subtasks(fake_id).await.unwrap_err();
    coordinator.is_ready(fake_id).await.unwrap_err();
    coordinator.start_task(fake_id).await.unwrap_err();
    coordinator
        .create_checkpoint(fake_id, "test".to_owned())
        .await
        .unwrap_err();
}
