//! Comprehensive tests for task execution and isolation
#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::min_ident_chars,
    clippy::shadow_unrelated,
    clippy::redundant_clone,
    clippy::absolute_paths,
    clippy::ref_as_ptr,
    clippy::cloned_ref_to_slice_refs,
    reason = "Test code prioritizes clarity over efficiency"
)]

use merlin_routing::{
    ContextRequirements, FileChange, Task, TaskId,
    executor::{
        graph::TaskGraph,
        isolation::FileLockManager,
        scheduler::ConflictAwareTaskGraph,
        state::WorkspaceState,
        transaction::{FileState, TaskWorkspace},
    },
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
/// # Panics
/// Panics if task graph construction fails.
fn test_task_graph_creation() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned());
    let task_c = Task::new("Task C".to_owned());

    let graph = TaskGraph::from_tasks(&[task_a, task_b, task_c]);
    assert_eq!(graph.task_count(), 3);
}

#[test]
/// # Panics
/// Panics if ready task detection fails.
fn test_ready_tasks_no_dependencies() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned());

    let graph = TaskGraph::from_tasks(&[task_a.clone(), task_b.clone()]);
    let completed = HashSet::default();

    let ready = graph.ready_tasks(&completed);
    assert_eq!(ready.len(), 2);
}

#[test]
/// # Panics
/// Panics if dependency chain handling fails.
fn test_ready_tasks_linear_dependencies() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned()).with_dependencies(vec![task_a.id]);
    let task_c = Task::new("Task C".to_owned()).with_dependencies(vec![task_b.id]);

    let graph = TaskGraph::from_tasks(&[task_a.clone(), task_b.clone(), task_c.clone()]);
    let mut completed = HashSet::default();

    // Initially, only task_a should be ready
    let ready = graph.ready_tasks(&completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, task_a.id);

    // After completing task_a, task_b should be ready
    completed.insert(task_a.id);
    let ready = graph.ready_tasks(&completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, task_b.id);

    // After completing task_b, task_c should be ready
    completed.insert(task_b.id);
    let ready = graph.ready_tasks(&completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, task_c.id);
}

#[test]
/// # Panics
/// Panics if diamond dependency handling fails.
fn test_ready_tasks_diamond_dependencies() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned()).with_dependencies(vec![task_a.id]);
    let task_c = Task::new("Task C".to_owned()).with_dependencies(vec![task_a.id]);
    let task_d = Task::new("Task D".to_owned()).with_dependencies(vec![task_b.id, task_c.id]);

    let graph = TaskGraph::from_tasks(&[
        task_a.clone(),
        task_b.clone(),
        task_c.clone(),
        task_d.clone(),
    ]);
    let mut completed = HashSet::default();

    // Initially, only task_a should be ready
    let ready = graph.ready_tasks(&completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, task_a.id);

    // After completing task_a, both task_b and task_c should be ready
    completed.insert(task_a.id);
    let ready = graph.ready_tasks(&completed);
    assert_eq!(ready.len(), 2);

    // After completing task_b but not task_c, task_d should not be ready
    completed.insert(task_b.id);
    let ready = graph.ready_tasks(&completed);
    assert!(!ready.iter().any(|t| t.id == task_d.id));

    // After completing both task_b and task_c, task_d should be ready
    completed.insert(task_c.id);
    let ready = graph.ready_tasks(&completed);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, task_d.id);
}

#[test]
/// # Panics
/// Panics if completion detection fails.
fn test_graph_completion() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned());

    let graph = TaskGraph::from_tasks(&[task_a.clone(), task_b.clone()]);
    let mut completed = HashSet::default();

    assert!(!graph.is_complete(&completed));

    completed.insert(task_a.id);
    assert!(!graph.is_complete(&completed));

    completed.insert(task_b.id);
    assert!(graph.is_complete(&completed));
}

#[test]
/// # Panics
/// Panics if cycle detection fails.
fn test_cycle_detection_simple() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned()).with_dependencies(vec![task_a.id]);

    // Non-cyclic graph
    let graph = TaskGraph::from_tasks(&[task_a, task_b]);
    assert!(!graph.has_cycles());
}

#[test]
/// # Panics
/// Panics if file conflict detection fails.
fn test_file_conflict_detection() {
    let file = PathBuf::from("shared.rs");

    let task_a = Task::new("Task A".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![file.clone()]));

    let task_b = Task::new("Task B".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![file]));

    let graph = ConflictAwareTaskGraph::from_tasks(&[task_a.clone(), task_b.clone()]);

    let completed = HashSet::default();
    let mut running = HashSet::default();
    running.insert(task_a.id);

    let ready = graph.ready_non_conflicting_tasks(&completed, &running);

    // Task B should be filtered out due to file conflict
    assert!(!ready.iter().any(|t| t.id == task_b.id));
}

#[test]
/// # Panics
/// Panics if non-conflicting task detection fails.
fn test_no_conflict_separate_files() {
    let task_a = Task::new("Task A".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![PathBuf::from("file_a.rs")]));

    let task_b = Task::new("Task B".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![PathBuf::from("file_b.rs")]));

    let task_c = Task::new("Task C".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![PathBuf::from("file_c.rs")]));

    let graph = ConflictAwareTaskGraph::from_tasks(&[task_a.clone(), task_b, task_c]);

    let completed = HashSet::default();
    let mut running = HashSet::default();
    running.insert(task_a.id);

    let ready = graph.ready_non_conflicting_tasks(&completed, &running);

    // Both task_b and task_c should be ready (different files)
    assert_eq!(ready.len(), 2);
}

#[test]
/// # Panics
/// Panics if empty graph handling fails.
fn test_empty_graph() {
    let graph = TaskGraph::from_tasks(&[]);
    assert_eq!(graph.task_count(), 0);

    let completed = HashSet::default();
    assert!(graph.is_complete(&completed));
    assert!(graph.ready_tasks(&completed).is_empty());
}

#[tokio::test]
/// # Panics
/// Panics if exclusive write lock fails.
async fn test_file_lock_write_exclusivity() {
    let manager = Arc::new(FileLockManager::default());
    let task_a = TaskId::default();
    let task_b = TaskId::default();
    let file = PathBuf::from("test.rs");

    let _guard_a = manager
        .acquire_write_locks(task_a, &[file.clone()])
        .await
        .expect("Task A should acquire write lock");

    let result = manager.acquire_write_locks(task_b, &[file]).await;
    assert!(
        result.is_err(),
        "Task B should not acquire lock while Task A holds it"
    );
}

#[tokio::test]
/// # Panics
/// Panics if shared read locks fail.
async fn test_file_lock_read_sharing() {
    let manager = Arc::new(FileLockManager::default());
    let task_a = TaskId::default();
    let task_b = TaskId::default();
    let task_c = TaskId::default();
    let file = PathBuf::from("test.rs");

    let _guard_a = manager
        .acquire_read_locks(task_a, &[file.clone()])
        .await
        .expect("Task A should acquire read lock");

    let _guard_b = manager
        .acquire_read_locks(task_b, &[file.clone()])
        .await
        .expect("Task B should acquire read lock");

    let _guard_c = manager
        .acquire_read_locks(task_c, &[file])
        .await
        .expect("Task C should acquire read lock");
}

#[tokio::test]
/// # Panics
/// Panics if write-read exclusivity fails.
async fn test_file_lock_write_blocks_readers() {
    let manager = Arc::new(FileLockManager::default());
    let task_a = TaskId::default();
    let task_b = TaskId::default();
    let file = PathBuf::from("test.rs");

    let _guard_a = manager
        .acquire_write_locks(task_a, &[file.clone()])
        .await
        .expect("Task A should acquire write lock");

    let result = manager.acquire_read_locks(task_b, &[file]).await;
    assert!(result.is_err(), "Read lock should be blocked by write lock");
}

#[tokio::test]
/// # Panics
/// Panics if read-write exclusivity fails.
async fn test_file_lock_readers_block_writer() {
    let manager = Arc::new(FileLockManager::default());
    let task_a = TaskId::default();
    let task_b = TaskId::default();
    let file = PathBuf::from("test.rs");

    let _guard_a = manager
        .acquire_read_locks(task_a, &[file.clone()])
        .await
        .expect("Task A should acquire read lock");

    let result = manager.acquire_write_locks(task_b, &[file]).await;
    assert!(
        result.is_err(),
        "Write lock should be blocked by read locks"
    );
}

#[tokio::test]
/// # Panics
/// Panics if lock release fails.
async fn test_file_lock_release() {
    let manager = Arc::new(FileLockManager::default());
    let task_a = TaskId::default();
    let task_b = TaskId::default();
    let file = PathBuf::from("test.rs");

    {
        let _guard_a = manager
            .acquire_write_locks(task_a, &[file.clone()])
            .await
            .expect("Task A should acquire write lock");

        // Guard dropped here, releasing lock
    }

    // Small delay to allow async drop to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Task B should now be able to acquire the lock
    let _guard_b = manager
        .acquire_write_locks(task_b, &[file])
        .await
        .expect("Task B should acquire lock after Task A releases");
}

#[tokio::test]
/// # Panics
/// Panics if multi-file locking fails.
async fn test_file_lock_multiple_files() {
    let manager = Arc::new(FileLockManager::default());
    let task_a = TaskId::default();
    let files = vec![
        PathBuf::from("file1.rs"),
        PathBuf::from("file2.rs"),
        PathBuf::from("file3.rs"),
    ];

    let _guard = manager
        .acquire_write_locks(task_a, &files)
        .await
        .expect("Should acquire locks on multiple files");
}

#[tokio::test]
/// # Panics
/// Panics if workspace isolation fails.
async fn test_workspace_isolation() {
    let tmp_dir = TempDir::new().expect("create temp dir");
    let workspace = Arc::new(WorkspaceState::new(tmp_dir.path().to_path_buf()));
    let lock_manager = Arc::new(FileLockManager::default());
    let task_id = TaskId::default();

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original content".to_owned(),
        }])
        .await
        .expect("create initial file");

    let mut task_workspace = TaskWorkspace::new(
        task_id,
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("test.rs"), "modified content".to_owned());

    // Task workspace should see modified content
    assert_eq!(
        task_workspace.read_file(&PathBuf::from("test.rs")),
        Some("modified content".to_owned())
    );

    // Global workspace should still see original content
    assert_eq!(
        workspace.read_file(&PathBuf::from("test.rs")).await,
        Some("original content".to_owned())
    );
}

#[tokio::test]
/// # Panics
/// Panics if workspace commit fails.
async fn test_workspace_commit() {
    let tmp_dir = TempDir::new().expect("create temp dir");
    let workspace = Arc::new(WorkspaceState::new(tmp_dir.path().to_path_buf()));
    let lock_manager = Arc::new(FileLockManager::default());
    let task_id = TaskId::default();

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create initial file");

    let mut task_workspace = TaskWorkspace::new(
        task_id,
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("test.rs"), "committed".to_owned());

    let result = task_workspace
        .commit(Arc::clone(&workspace))
        .await
        .expect("commit changes");

    assert_eq!(result.files_changed, 1);
    assert_eq!(
        workspace.read_file(&PathBuf::from("test.rs")).await,
        Some("committed".to_owned())
    );
}

#[tokio::test]
/// # Panics
/// Panics if conflict detection fails.
async fn test_workspace_conflict_detection() {
    let tmp_dir = TempDir::new().expect("create temp dir");
    let workspace = Arc::new(WorkspaceState::new(tmp_dir.path().to_path_buf()));
    let lock_manager = Arc::new(FileLockManager::default());
    let task_id = TaskId::default();

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create initial file");

    let mut task_workspace = TaskWorkspace::new(
        task_id,
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    // Modify file in task workspace
    task_workspace.modify_file(PathBuf::from("test.rs"), "task version".to_owned());

    // Meanwhile, another process modifies the file globally
    workspace
        .apply_changes(&[FileChange::Modify {
            path: PathBuf::from("test.rs"),
            content: "global version".to_owned(),
        }])
        .await
        .expect("apply global change");

    // Attempting to commit should detect conflict
    let result = task_workspace.commit(Arc::clone(&workspace)).await;
    assert!(result.is_err(), "Commit should fail due to conflict");
}

#[tokio::test]
/// # Panics
/// Panics if file creation in workspace fails.
async fn test_workspace_file_creation() {
    let tmp_dir = TempDir::new().expect("create temp dir");
    let workspace = Arc::new(WorkspaceState::new(tmp_dir.path().to_path_buf()));
    let lock_manager = Arc::new(FileLockManager::default());
    let task_id = TaskId::default();

    let mut task_workspace = TaskWorkspace::new(
        task_id,
        vec![PathBuf::from("new.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.create_file(PathBuf::from("new.rs"), "new content".to_owned());

    assert_eq!(
        task_workspace.read_file(&PathBuf::from("new.rs")),
        Some("new content".to_owned())
    );

    task_workspace
        .commit(Arc::clone(&workspace))
        .await
        .expect("commit new file");

    assert_eq!(
        workspace.read_file(&PathBuf::from("new.rs")).await,
        Some("new content".to_owned())
    );
}

#[tokio::test]
/// # Panics
/// Panics if file deletion in workspace fails.
async fn test_workspace_file_deletion() {
    let tmp_dir = TempDir::new().expect("create temp dir");
    let workspace = Arc::new(WorkspaceState::new(tmp_dir.path().to_path_buf()));
    let lock_manager = Arc::new(FileLockManager::default());
    let task_id = TaskId::default();

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("delete.rs"),
            content: "to be deleted".to_owned(),
        }])
        .await
        .expect("create file to delete");

    let mut task_workspace = TaskWorkspace::new(
        task_id,
        vec![PathBuf::from("delete.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.delete_file(PathBuf::from("delete.rs"));

    assert_eq!(task_workspace.read_file(&PathBuf::from("delete.rs")), None);

    task_workspace
        .commit(Arc::clone(&workspace))
        .await
        .expect("commit deletion");

    assert_eq!(workspace.read_file(&PathBuf::from("delete.rs")).await, None);
}

#[tokio::test]
/// # Panics
/// Panics if workspace rollback fails.
async fn test_workspace_rollback() {
    let tmp_dir = TempDir::new().expect("create temp dir");
    let workspace = Arc::new(WorkspaceState::new(tmp_dir.path().to_path_buf()));
    let lock_manager = Arc::new(FileLockManager::default());
    let task_id = TaskId::default();

    workspace
        .apply_changes(&[FileChange::Create {
            path: PathBuf::from("test.rs"),
            content: "original".to_owned(),
        }])
        .await
        .expect("create initial file");

    let mut task_workspace = TaskWorkspace::new(
        task_id,
        vec![PathBuf::from("test.rs")],
        Arc::clone(&workspace),
        lock_manager,
    )
    .await
    .expect("create task workspace");

    task_workspace.modify_file(PathBuf::from("test.rs"), "modified".to_owned());

    task_workspace.rollback().expect("rollback changes");

    // Global workspace should still have original content
    assert_eq!(
        workspace.read_file(&PathBuf::from("test.rs")).await,
        Some("original".to_owned())
    );
}

#[test]
/// # Panics
/// Panics if file state handling fails.
fn test_file_state_variants() {
    let created = FileState::Created("content".to_owned());
    let modified = FileState::Modified("content".to_owned());
    let deleted = FileState::Deleted;

    match created {
        FileState::Created(_) => {}
        _ => panic!("Expected Created variant"),
    }

    match modified {
        FileState::Modified(_) => {}
        _ => panic!("Expected Modified variant"),
    }

    match deleted {
        FileState::Deleted => {}
        _ => panic!("Expected Deleted variant"),
    }
}
