//! Integration tests for transaction and state management.
//!
//! Tests `TaskWorkspace`, `WorkspaceState`, `FileLockManager`, and conflict detection.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::print_stdout,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_agent::executor::{FileLockManager, TaskWorkspace, WorkspaceState};
use merlin_core::{FileChange, TaskId};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::spawn;

fn create_test_workspace() -> (TempDir, Arc<WorkspaceState>) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let workspace = WorkspaceState::new(temp_dir.path().to_path_buf());
    (temp_dir, workspace)
}

#[tokio::test]
async fn test_workspace_state_creation() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let temp_path = temp_dir.path().to_path_buf();
    let workspace = WorkspaceState::new(temp_path.clone());

    // Compare canonicalized paths to handle Windows drive letter variations
    let workspace_path = workspace
        .root_path()
        .canonicalize()
        .expect("canonicalize workspace path");
    let expected_path = temp_path
        .canonicalize()
        .expect("canonicalize expected path");
    assert_eq!(workspace_path, expected_path);
}

#[tokio::test]
async fn test_workspace_state_apply_create_change() {
    let (_temp, workspace) = create_test_workspace();

    let changes = vec![FileChange::Create {
        path: PathBuf::from("test.rs"),
        content: "test content".to_owned(),
    }];

    let result = workspace.apply_changes(&changes).await;
    assert!(result.is_ok(), "Should apply create change");

    let content = workspace.read_file(&PathBuf::from("test.rs")).await;
    assert_eq!(content, Some("test content".to_owned()));
}

#[tokio::test]
async fn test_workspace_state_apply_modify_change() {
    let (_temp, workspace) = create_test_workspace();

    // First create the file
    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create file");

    // Then modify it
    let changes = vec![FileChange::Modify {
        path: PathBuf::from("test.rs"),
        content: "modified".to_owned(),
    }];

    let result = workspace.apply_changes(&changes).await;
    assert!(result.is_ok(), "Should apply modify change");

    let content = workspace.read_file(&PathBuf::from("test.rs")).await;
    assert_eq!(content, Some("modified".to_owned()));
}

#[tokio::test]
async fn test_workspace_state_apply_delete_change() {
    let (_temp, workspace) = create_test_workspace();

    // First create the file
    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "content".to_owned(),
        }])
        .await
        .expect("create file");

    // Then delete it
    let changes = vec![FileChange::Delete {
        path: PathBuf::from("test.rs"),
    }];

    let result = workspace.apply_changes(&changes).await;
    assert!(result.is_ok(), "Should apply delete change");

    let content = workspace.read_file(&PathBuf::from("test.rs")).await;
    assert_eq!(content, None, "File should be deleted");
}

#[tokio::test]
async fn test_workspace_state_multiple_changes() {
    let (_temp, workspace) = create_test_workspace();

    let changes = vec![
        FileChange::Create {
            path: PathBuf::from("file1.rs"),
            content: "content 1".to_owned(),
        },
        FileChange::Create {
            path: PathBuf::from("file2.rs"),
            content: "content 2".to_owned(),
        },
        FileChange::Create {
            path: PathBuf::from("file3.rs"),
            content: "content 3".to_owned(),
        },
    ];

    workspace
        .apply_changes(&changes)
        .await
        .expect("apply changes");

    assert_eq!(
        workspace.read_file(&PathBuf::from("file1.rs")).await,
        Some("content 1".to_owned())
    );
    assert_eq!(
        workspace.read_file(&PathBuf::from("file2.rs")).await,
        Some("content 2".to_owned())
    );
    assert_eq!(
        workspace.read_file(&PathBuf::from("file3.rs")).await,
        Some("content 3".to_owned())
    );
}

#[tokio::test]
async fn test_workspace_state_nested_directories() {
    let (_temp, workspace) = create_test_workspace();

    let changes = vec![FileChange::Create {
        path: PathBuf::from("src/module/submodule/deep.rs"),
        content: "deep content".to_owned(),
    }];

    workspace
        .apply_changes(&changes)
        .await
        .expect("apply changes");

    let content = workspace
        .read_file(&PathBuf::from("src/module/submodule/deep.rs"))
        .await;
    assert_eq!(content, Some("deep content".to_owned()));
}

#[tokio::test]
async fn test_task_workspace_isolation() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    // Create a file in the global workspace
    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create file");

    // Create isolated task workspace
    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    // Modify file in task workspace
    task_workspace.modify_file(PathBuf::from("test.rs"), "modified in task".to_owned());

    // Read from task workspace - should see modified version
    assert_eq!(
        task_workspace.read_file(&PathBuf::from("test.rs")),
        Some("modified in task".to_owned())
    );

    // Read from global workspace - should still see original
    assert_eq!(
        workspace.read_file(&PathBuf::from("test.rs")).await,
        Some("original".to_owned())
    );
}

#[tokio::test]
async fn test_task_workspace_commit() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create file");

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("test.rs"), "committed change".to_owned());

    let commit_result = task_workspace
        .commit(Arc::clone(&workspace))
        .await
        .expect("commit");
    assert_eq!(commit_result.files_changed, 1);

    assert_eq!(
        workspace.read_file(&PathBuf::from("test.rs")).await,
        Some("committed change".to_owned())
    );
}

#[tokio::test]
async fn test_task_workspace_rollback() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create file");

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("test.rs"), "should be rolled back".to_owned());

    task_workspace.rollback().expect("rollback");

    // Global workspace should still have original content
    assert_eq!(
        workspace.read_file(&PathBuf::from("test.rs")).await,
        Some("original".to_owned())
    );
}

#[tokio::test]
async fn test_task_workspace_create_file() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("new.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.create_file(PathBuf::from("new.rs"), "new file content".to_owned());

    assert_eq!(
        task_workspace.read_file(&PathBuf::from("new.rs")),
        Some("new file content".to_owned())
    );

    task_workspace
        .commit(Arc::clone(&workspace))
        .await
        .expect("commit");

    assert_eq!(
        workspace.read_file(&PathBuf::from("new.rs")).await,
        Some("new file content".to_owned())
    );
}

#[tokio::test]
async fn test_task_workspace_delete_file() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("delete_me.rs"),
            content: "will be deleted".to_owned(),
        }])
        .await
        .expect("create file");

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("delete_me.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.delete_file(PathBuf::from("delete_me.rs"));

    assert_eq!(
        task_workspace.read_file(&PathBuf::from("delete_me.rs")),
        None
    );

    task_workspace
        .commit(Arc::clone(&workspace))
        .await
        .expect("commit");

    assert_eq!(
        workspace.read_file(&PathBuf::from("delete_me.rs")).await,
        None
    );
}

#[tokio::test]
async fn test_task_workspace_multiple_operations() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("existing.rs"),
            content: "existing".to_owned(),
        }])
        .await
        .expect("create file");

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![
            PathBuf::from("existing.rs"),
            PathBuf::from("new.rs"),
            PathBuf::from("deleted.rs"),
        ],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("existing.rs"), "modified".to_owned());
    task_workspace.create_file(PathBuf::from("new.rs"), "new".to_owned());
    task_workspace.delete_file(PathBuf::from("deleted.rs"));

    let commit_result = task_workspace
        .commit(Arc::clone(&workspace))
        .await
        .expect("commit");
    assert_eq!(commit_result.files_changed, 3);

    assert_eq!(
        workspace.read_file(&PathBuf::from("existing.rs")).await,
        Some("modified".to_owned())
    );
    assert_eq!(
        workspace.read_file(&PathBuf::from("new.rs")).await,
        Some("new".to_owned())
    );
    assert_eq!(
        workspace.read_file(&PathBuf::from("deleted.rs")).await,
        None
    );
}

#[tokio::test]
async fn test_file_lock_manager_basic() {
    let lock_manager = Arc::new(FileLockManager::default());
    let task_id = TaskId::default();
    let files = vec![PathBuf::from("test.rs")];

    let lock_guard = lock_manager
        .acquire_write_locks(task_id, &files)
        .await
        .expect("acquire locks");

    // Lock should be held
    drop(lock_guard);
    // Lock should be released after drop
}

#[tokio::test]
async fn test_file_lock_manager_multiple_files() {
    let lock_manager = Arc::new(FileLockManager::default());
    let task_id = TaskId::default();
    let files = vec![
        PathBuf::from("file1.rs"),
        PathBuf::from("file2.rs"),
        PathBuf::from("file3.rs"),
    ];

    let lock_guard = lock_manager
        .acquire_write_locks(task_id, &files)
        .await
        .expect("acquire locks");

    // All locks should be held
    drop(lock_guard);
}

#[tokio::test]
async fn test_conflict_detection_no_conflicts() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create file");

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("test.rs"), "modified".to_owned());

    let conflict_report = task_workspace
        .check_conflicts(Arc::clone(&workspace))
        .await
        .expect("check conflicts");

    assert!(
        conflict_report.conflicts.is_empty(),
        "Should have no conflicts"
    );
}

#[tokio::test]
async fn test_conflict_detection_with_conflict() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create file");

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        Arc::clone(&lock_manager),
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("test.rs"), "task modification".to_owned());

    // Simulate another task modifying the same file in global workspace
    // (This would normally be prevented by locks, but we're testing conflict detection)
    workspace
        .apply_changes(&[FileChange::Modify {
            path: PathBuf::from("test.rs"),
            content: "concurrent modification".to_owned(),
        }])
        .await
        .expect("concurrent modification");

    let conflict_report = task_workspace
        .check_conflicts(Arc::clone(&workspace))
        .await
        .expect("check conflicts");

    assert!(
        !conflict_report.conflicts.is_empty(),
        "Should detect conflict"
    );
    assert_eq!(conflict_report.conflicts.len(), 1);
    assert_eq!(conflict_report.conflicts[0].path, PathBuf::from("test.rs"));
}

#[tokio::test]
async fn test_commit_fails_with_conflicts() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create file");

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("test.rs"), "task modification".to_owned());

    // Create conflict
    workspace
        .apply_changes(&[FileChange::Modify {
            path: PathBuf::from("test.rs"),
            content: "concurrent modification".to_owned(),
        }])
        .await
        .expect("concurrent modification");

    let commit_result = task_workspace.commit(Arc::clone(&workspace)).await;

    assert!(commit_result.is_err(), "Commit should fail due to conflict");
}

#[tokio::test]
async fn test_workspace_snapshot_consistency() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "v1".to_owned(),
        }])
        .await
        .expect("create file");

    let task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    // Modify global workspace after snapshot
    workspace
        .apply_changes(&[FileChange::Modify {
            path: PathBuf::from("test.rs"),
            content: "v2".to_owned(),
        }])
        .await
        .expect("modify file");

    // Task workspace should still see v1 (snapshot isolation)
    assert_eq!(
        task_workspace.read_file(&PathBuf::from("test.rs")),
        Some("v1".to_owned())
    );
}

#[tokio::test]
async fn test_task_workspace_read_nonexistent_file() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    let task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("nonexistent.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    assert_eq!(
        task_workspace.read_file(&PathBuf::from("nonexistent.rs")),
        None
    );
}

#[tokio::test]
async fn test_workspace_state_concurrent_reads() {
    let (_temp, workspace) = create_test_workspace();

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("shared.rs"),
            content: "shared content".to_owned(),
        }])
        .await
        .expect("create file");

    // Simulate concurrent reads
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let workspace = Arc::clone(&workspace);
            spawn(async move { workspace.read_file(&PathBuf::from("shared.rs")).await })
        })
        .collect();

    for handle in handles {
        let content = handle.await.expect("task should complete");
        assert_eq!(content, Some("shared content".to_owned()));
    }
}

#[tokio::test]
async fn test_workspace_state_empty_file_content() {
    let (_temp, workspace) = create_test_workspace();

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("empty.rs"),
            content: String::new(),
        }])
        .await
        .expect("create empty file");

    assert_eq!(
        workspace.read_file(&PathBuf::from("empty.rs")).await,
        Some(String::new())
    );
}

#[tokio::test]
async fn test_task_workspace_overwrite_operations() {
    let (_temp, workspace) = create_test_workspace();
    let lock_manager = Arc::new(FileLockManager::default());

    let mut task_workspace = TaskWorkspace::new(
        TaskId::default(),
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    // Create, then modify, then delete the same file
    task_workspace.create_file(PathBuf::from("test.rs"), "created".to_owned());
    assert_eq!(
        task_workspace.read_file(&PathBuf::from("test.rs")),
        Some("created".to_owned())
    );

    task_workspace.modify_file(PathBuf::from("test.rs"), "modified".to_owned());
    assert_eq!(
        task_workspace.read_file(&PathBuf::from("test.rs")),
        Some("modified".to_owned())
    );

    task_workspace.delete_file(PathBuf::from("test.rs"));
    assert_eq!(task_workspace.read_file(&PathBuf::from("test.rs")), None);
}
