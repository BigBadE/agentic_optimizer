//! Comprehensive tests for `TaskCoordinator` functionality.

#![cfg_attr(
    test,
    allow(
        clippy::tests_outside_test_module,
        clippy::missing_panics_doc,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::min_ident_chars,
        clippy::absolute_paths,
        clippy::needless_range_loop,
        clippy::float_cmp,
        clippy::assertions_on_result_states,
        clippy::clone_on_ref_ptr,
        reason = "Test file allows"
    )
)]

use merlin_agent::TaskCoordinator;
use merlin_core::{SubtaskSpec, Task};

#[tokio::test]
async fn test_register_and_track_single_task() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Single task".to_owned());
    let task_id = task.id;

    coordinator
        .register_task(task, None)
        .await
        .expect("Failed to register task");

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.total_tasks, 1);
    assert_eq!(stats.pending, 1);

    let is_ready = coordinator.is_ready(task_id).await.expect("Failed");
    assert!(is_ready);
}

#[tokio::test]
async fn test_start_task_changes_status() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Task to start".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.expect("Failed");
    coordinator.start_task(task_id).await.expect("Failed");

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.in_progress, 1);
    assert_eq!(stats.pending, 0);

    let is_ready = coordinator.is_ready(task_id).await.expect("Failed");
    assert!(!is_ready); // No longer ready since in progress
}

#[tokio::test]
async fn test_decompose_task_creates_subtasks() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Parent task".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.expect("Failed");

    let subtask_specs = vec![
        SubtaskSpec {
            description: "Subtask 1".to_owned(),
            difficulty: 5,
        },
        SubtaskSpec {
            description: "Subtask 2".to_owned(),
            difficulty: 3,
        },
    ];

    let subtasks = coordinator
        .decompose_task(task_id, subtask_specs)
        .await
        .expect("Failed to decompose");

    assert_eq!(subtasks.len(), 2);

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.waiting, 1); // Parent task is waiting
}

#[tokio::test]
async fn test_complete_subtask_updates_parent() {
    let coordinator = TaskCoordinator::new();
    let parent_task = Task::new("Parent".to_owned());
    let parent_id = parent_task.id;

    coordinator
        .register_task(parent_task, None)
        .await
        .expect("Failed");

    let subtask_specs = vec![SubtaskSpec {
        description: "Subtask".to_owned(),
        difficulty: 3,
    }];

    let subtasks = coordinator
        .decompose_task(parent_id, subtask_specs)
        .await
        .expect("Failed");
    let subtask_id = subtasks[0].id;

    // Register subtask with parent_id to establish parent link
    coordinator
        .register_task(subtasks[0].clone(), Some(parent_id))
        .await
        .expect("Failed");

    // Complete subtask
    let result = merlin_core::TaskResult {
        task_id: subtask_id,
        response: merlin_core::Response {
            text: "Done".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        },
        tier_used: "test".to_owned(),
        tokens_used: merlin_core::TokenUsage::default(),
        validation: merlin_core::ValidationResult::default(),
        duration_ms: 0,
        task_list: None,
    };

    coordinator
        .complete_subtask(subtask_id, result)
        .await
        .expect("Failed");

    let progress = coordinator.get_progress(parent_id).await.expect("Failed");
    assert_eq!(progress.completed_subtasks, 1);
    assert_eq!(progress.total_subtasks, 1);
    assert_eq!(progress.progress_percent, 100.0);
}

#[tokio::test]
async fn test_max_subtasks_enforcement() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Task with too many subtasks".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.expect("Failed");

    // Try to create 11 subtasks (max is 10)
    let subtask_specs: Vec<SubtaskSpec> = (0..11)
        .map(|i| SubtaskSpec {
            description: format!("Subtask {i}"),
            difficulty: 3,
        })
        .collect();

    let result = coordinator.decompose_task(task_id, subtask_specs).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_max_depth_enforcement() {
    let coordinator = TaskCoordinator::new();
    let mut parent_id = None;

    // Create a chain of nested tasks up to MAX_DECOMPOSITION_DEPTH
    for depth in 0..=6 {
        let task = Task::new(format!("Task at depth {depth}"));
        let task_id = task.id;

        let result = coordinator.register_task(task, parent_id).await;

        if depth <= 5 {
            // Should succeed up to depth 5
            result.expect("Registration should succeed");
            parent_id = Some(task_id);
        } else {
            // Should fail at depth 6 (exceeds MAX_DECOMPOSITION_DEPTH of 5)
            assert!(result.is_err());
        }
    }
}

#[tokio::test]
async fn test_checkpoint_creation_and_limit() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Task with checkpoints".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.expect("Failed");

    // Create 105 checkpoints (max is 100)
    for i in 0..105 {
        coordinator
            .create_checkpoint(task_id, format!("Checkpoint {i}"))
            .await
            .expect("Failed");
    }

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.checkpoints, 100); // Should be capped at 100
}

#[tokio::test]
async fn test_get_progress_for_task_with_no_subtasks() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Simple task".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.expect("Failed");

    let progress = coordinator.get_progress(task_id).await.expect("Failed");
    assert_eq!(progress.total_subtasks, 0);
    assert_eq!(progress.completed_subtasks, 0);
    assert_eq!(progress.progress_percent, 0.0);
}

#[tokio::test]
async fn test_cleanup_old_tasks() {
    let coordinator = TaskCoordinator::new();
    let task = Task::new("Old task".to_owned());
    let task_id = task.id;

    coordinator.register_task(task, None).await.expect("Failed");
    coordinator.start_task(task_id).await.expect("Failed");

    // Complete via decomposing into 1 subtask and completing that subtask
    let subtask_specs = vec![SubtaskSpec {
        description: "Subtask".to_owned(),
        difficulty: 1,
    }];

    let subtasks = coordinator
        .decompose_task(task_id, subtask_specs)
        .await
        .expect("Failed");
    let subtask_id = subtasks[0].id;

    coordinator
        .register_task(subtasks[0].clone(), Some(task_id))
        .await
        .expect("Failed");

    // Complete the subtask, which should mark parent as completed
    let result = merlin_core::TaskResult {
        task_id: subtask_id,
        response: merlin_core::Response {
            text: "Done".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        },
        tier_used: "test".to_owned(),
        tokens_used: merlin_core::TokenUsage::default(),
        validation: merlin_core::ValidationResult::default(),
        duration_ms: 0,
        task_list: None,
    };

    coordinator
        .complete_subtask(subtask_id, result)
        .await
        .expect("Failed");

    // Wait to ensure age > 0 seconds (timestamps are in seconds, not milliseconds)
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Cleanup tasks older than 0 seconds - should remove both parent and subtask
    let removed = coordinator.cleanup_old_tasks(0).await.expect("Failed");
    assert_eq!(removed, 2); // Parent + subtask

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.total_tasks, 0);
}

#[tokio::test]
async fn test_concurrent_task_registration() {
    let coordinator = TaskCoordinator::new();
    let coordinator = std::sync::Arc::new(coordinator);

    let mut handles = vec![];

    // Spawn 10 concurrent task registrations
    for i in 0..10 {
        let coord = coordinator.clone();
        let handle = tokio::spawn(async move {
            let task = Task::new(format!("Concurrent task {i}"));
            coord.register_task(task, None).await.expect("Failed");
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let stats = coordinator.get_stats().await;
    assert_eq!(stats.total_tasks, 10);
}

#[tokio::test]
async fn test_get_subtasks_returns_registered_subtasks() {
    let coordinator = TaskCoordinator::new();
    let parent = Task::new("Parent".to_owned());
    let parent_id = parent.id;

    coordinator
        .register_task(parent, None)
        .await
        .expect("Failed");

    let subtask_specs = vec![
        SubtaskSpec {
            description: "Sub 1".to_owned(),
            difficulty: 3,
        },
        SubtaskSpec {
            description: "Sub 2".to_owned(),
            difficulty: 5,
        },
    ];

    let subtasks = coordinator
        .decompose_task(parent_id, subtask_specs)
        .await
        .expect("Failed");

    // Register the subtasks with parent_id to establish parent link
    for subtask in &subtasks {
        coordinator
            .register_task(subtask.clone(), Some(parent_id))
            .await
            .expect("Failed");
    }

    // Get subtasks
    let retrieved = coordinator.get_subtasks(parent_id).await.expect("Failed");
    assert_eq!(retrieved.len(), 2);
}

#[tokio::test]
async fn test_progress_percent_calculation() {
    let coordinator = TaskCoordinator::new();
    let parent = Task::new("Parent".to_owned());
    let parent_id = parent.id;

    coordinator
        .register_task(parent, None)
        .await
        .expect("Failed");

    let subtask_specs = vec![
        SubtaskSpec {
            description: "Sub 1".to_owned(),
            difficulty: 3,
        },
        SubtaskSpec {
            description: "Sub 2".to_owned(),
            difficulty: 3,
        },
        SubtaskSpec {
            description: "Sub 3".to_owned(),
            difficulty: 3,
        },
        SubtaskSpec {
            description: "Sub 4".to_owned(),
            difficulty: 3,
        },
    ];

    let subtasks = coordinator
        .decompose_task(parent_id, subtask_specs)
        .await
        .expect("Failed");

    // Register all subtasks with parent_id to establish parent link
    for subtask in &subtasks {
        coordinator
            .register_task(subtask.clone(), Some(parent_id))
            .await
            .expect("Failed");
    }

    // Complete 2 out of 4
    for i in 0..2 {
        let result = merlin_core::TaskResult {
            task_id: subtasks[i].id,
            response: merlin_core::Response {
                text: "Done".to_owned(),
                confidence: 1.0,
                tokens_used: merlin_core::TokenUsage::default(),
                provider: "test".to_owned(),
                latency_ms: 0,
            },
            tier_used: "test".to_owned(),
            tokens_used: merlin_core::TokenUsage::default(),
            validation: merlin_core::ValidationResult::default(),
            duration_ms: 0,
            task_list: None,
        };

        coordinator
            .complete_subtask(subtasks[i].id, result)
            .await
            .expect("Failed");
    }

    let progress = coordinator.get_progress(parent_id).await.expect("Failed");
    assert_eq!(progress.completed_subtasks, 2);
    assert_eq!(progress.total_subtasks, 4);
    assert_eq!(progress.progress_percent, 50.0);
}
