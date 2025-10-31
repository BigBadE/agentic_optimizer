use super::isolation::{FileLockManager, WriteLockGuard};
use super::state::{WorkspaceSnapshot, WorkspaceState};
use merlin_core::routing_error::{
    ConflictReport as ErrorConflictReport, FileConflict as ErrorFileConflict,
};
use merlin_core::{FileChange, Result, RoutingError, TaskId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Isolated workspace for a single task
pub struct TaskWorkspace {
    base_snapshot: Arc<WorkspaceSnapshot>,
    pending_changes: HashMap<PathBuf, FileState>,
    _lock_guard: WriteLockGuard,
}

/// State of a file in the task workspace
#[derive(Debug, Clone)]
pub enum FileState {
    /// File was created with content
    Created(String),
    /// File was modified with new content
    Modified(String),
    /// File was deleted
    Deleted,
}

impl TaskWorkspace {
    /// Create isolated workspace for task
    ///
    /// # Errors
    /// Returns an error if acquiring file locks or creating the base snapshot fails.
    pub async fn new(
        task_id: TaskId,
        files_to_modify: Vec<PathBuf>,
        global_state: Arc<WorkspaceState>,
        lock_manager: Arc<FileLockManager>,
    ) -> Result<Self> {
        let lock_guard = lock_manager
            .acquire_write_locks(task_id, &files_to_modify)
            .await?;

        let base_snapshot = Arc::new(global_state.snapshot(&files_to_modify).await?);

        Ok(Self {
            base_snapshot,
            pending_changes: HashMap::default(),
            _lock_guard: lock_guard,
        })
    }

    /// Modify file in isolated workspace
    pub fn modify_file(&mut self, path: PathBuf, content: String) {
        self.pending_changes
            .insert(path, FileState::Modified(content));
    }

    /// Create file in isolated workspace
    pub fn create_file(&mut self, path: PathBuf, content: String) {
        self.pending_changes
            .insert(path, FileState::Created(content));
    }

    /// Delete file in isolated workspace
    pub fn delete_file(&mut self, path: PathBuf) {
        self.pending_changes.insert(path, FileState::Deleted);
    }

    /// Read file (sees pending changes + base snapshot)
    pub fn read_file(&self, path: &PathBuf) -> Option<String> {
        if let Some(state) = self.pending_changes.get(path) {
            return match state {
                FileState::Created(content) | FileState::Modified(content) => Some(content.clone()),
                FileState::Deleted => None,
            };
        }

        self.base_snapshot.get(path)
    }

    /// Validate changes don't conflict with current global state
    ///
    /// # Errors
    /// Returns an error if reading files or building the conflict report fails.
    pub async fn check_conflicts(
        &self,
        global_state: Arc<WorkspaceState>,
    ) -> Result<ConflictReport> {
        let mut conflicts = Vec::default();

        for path in self.pending_changes.keys() {
            let base_version = self.base_snapshot.get(path);
            let current_version = global_state.read_file(path).await;

            if base_version != current_version {
                conflicts.push(FileConflict {
                    path: path.clone(),
                    base_hash: hash_content(base_version.as_ref()),
                    current_hash: hash_content(current_version.as_ref()),
                });
            }
        }

        Ok(ConflictReport { conflicts })
    }

    /// Commit changes to global state (atomic)
    ///
    /// # Errors
    /// Returns an error if conflicts are detected or applying changes fails
    pub async fn commit(self, global_state: Arc<WorkspaceState>) -> Result<CommitResult> {
        let conflict_report = self.check_conflicts(Arc::clone(&global_state)).await?;

        if !conflict_report.conflicts.is_empty() {
            let error_report = ErrorConflictReport {
                conflicts: conflict_report
                    .conflicts
                    .iter()
                    .map(|conflict| ErrorFileConflict {
                        path: conflict.path.clone(),
                        base_hash: conflict.base_hash,
                        current_hash: conflict.current_hash,
                    })
                    .collect(),
            };
            return Err(RoutingError::ConflictDetected(error_report));
        }

        let changes: Vec<FileChange> = self
            .pending_changes
            .into_iter()
            .map(|(path, state)| match state {
                FileState::Created(content) => FileChange::Create { path, content },
                FileState::Modified(content) => FileChange::Modify { path, content },
                FileState::Deleted => FileChange::Delete { path },
            })
            .collect();

        let files_changed = changes.len();
        global_state.apply_changes(&changes).await?;

        Ok(CommitResult { files_changed })
    }

    /// Abort changes (rollback)
    ///
    /// # Errors
    /// Returns an error if rollback logic fails (currently infallible)
    pub fn rollback(self) -> Result<()> {
        Ok(())
    }
}

/// Result of committing changes to global state
#[derive(Debug, Clone)]
pub struct CommitResult {
    /// Number of files that were changed
    pub files_changed: usize,
}

/// Report of file conflicts detected during validation
#[derive(Debug, Clone)]
pub struct ConflictReport {
    /// List of files with conflicts
    pub conflicts: Vec<FileConflict>,
}

/// Information about a single file conflict
#[derive(Debug, Clone)]
pub struct FileConflict {
    /// Path to the conflicting file
    pub path: PathBuf,
    /// Hash of the base version
    pub base_hash: u64,
    /// Hash of the current version
    pub current_hash: u64,
}

fn hash_content(content: Option<&String>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash as _, Hasher as _};

    let mut hasher = DefaultHasher::default();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_deps::tempfile::TempDir;

    /// Tests task workspace isolation from global workspace.
    ///
    /// # Errors
    /// Returns an error if workspace operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_task_workspace_isolation() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let workspace = WorkspaceState::new(tmp_dir.path().to_path_buf());
        let lock_manager = Arc::new(FileLockManager::default());
        let task_id = TaskId::default();

        workspace
            .apply_changes(&[FileChange::Create {
                path: PathBuf::from("test.rs"),
                content: "original".to_owned(),
            }])
            .await?;

        let mut task_workspace = TaskWorkspace::new(
            task_id,
            vec![PathBuf::from("test.rs")],
            Arc::clone(&workspace),
            lock_manager,
        )
        .await?;

        task_workspace.modify_file(PathBuf::from("test.rs"), "modified".to_owned());

        assert_eq!(
            task_workspace.read_file(&PathBuf::from("test.rs")),
            Some("modified".to_owned())
        );

        assert_eq!(
            workspace.read_file(&PathBuf::from("test.rs")).await,
            Some("original".to_owned())
        );
        Ok(())
    }

    /// Tests task workspace commit to global workspace.
    ///
    /// # Errors
    /// Returns an error if workspace operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_task_workspace_commit() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let workspace = WorkspaceState::new(tmp_dir.path().to_path_buf());
        let lock_manager = Arc::new(FileLockManager::default());
        let task_id = TaskId::default();

        workspace
            .apply_changes(&[FileChange::Create {
                path: PathBuf::from("test.rs"),
                content: "original".to_owned(),
            }])
            .await?;

        let mut task_workspace = TaskWorkspace::new(
            task_id,
            vec![PathBuf::from("test.rs")],
            Arc::clone(&workspace),
            lock_manager,
        )
        .await?;

        task_workspace.modify_file(PathBuf::from("test.rs"), "modified".to_owned());

        task_workspace.commit(Arc::clone(&workspace)).await?;

        assert_eq!(
            workspace.read_file(&PathBuf::from("test.rs")).await,
            Some("modified".to_owned())
        );
        Ok(())
    }
}
