use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use crate::{FileChange, Result, RoutingError, TaskId};
use crate::error::ConflictReport as ErrorConflictReport;
use super::isolation::{FileLockManager, WriteLockGuard};
use super::state::{WorkspaceState, WorkspaceSnapshot};

/// Isolated workspace for a single task
pub struct TaskWorkspace {
    base_snapshot: Arc<WorkspaceSnapshot>,
    pending_changes: HashMap<PathBuf, FileState>,
    _lock_guard: WriteLockGuard,
}

#[derive(Debug, Clone)]
pub enum FileState {
    Created(String),
    Modified(String),
    Deleted,
}

impl TaskWorkspace {
    /// Create isolated workspace for task
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
            pending_changes: HashMap::new(),
            _lock_guard: lock_guard,
        })
    }
    
    /// Modify file in isolated workspace
    pub fn modify_file(&mut self, path: PathBuf, content: String) {
        self.pending_changes.insert(path, FileState::Modified(content));
    }
    
    /// Create file in isolated workspace
    pub fn create_file(&mut self, path: PathBuf, content: String) {
        self.pending_changes.insert(path, FileState::Created(content));
    }
    
    /// Delete file in isolated workspace
    pub fn delete_file(&mut self, path: PathBuf) {
        self.pending_changes.insert(path, FileState::Deleted);
    }
    
    /// Read file (sees pending changes + base snapshot)
    #[must_use]
    pub fn read_file(&self, path: &PathBuf) -> Option<String> {
        if let Some(state) = self.pending_changes.get(path) {
            return match state {
                FileState::Created(content) | FileState::Modified(content) => {
                    Some(content.clone())
                }
                FileState::Deleted => None,
            };
        }
        
        self.base_snapshot.get(path)
    }
    
    /// Validate changes don't conflict with current global state
    pub async fn check_conflicts(
        &self,
        global_state: Arc<WorkspaceState>,
    ) -> Result<ConflictReport> {
        let mut conflicts = Vec::new();
        
        for path in self.pending_changes.keys() {
            let base_version = self.base_snapshot.get(path);
            let current_version = global_state.read_file(path).await;
            
            if base_version != current_version {
                conflicts.push(FileConflict {
                    path: path.clone(),
                    base_hash: hash_content(&base_version),
                    current_hash: hash_content(&current_version),
                });
            }
        }
        
        Ok(ConflictReport { conflicts })
    }
    
    /// Commit changes to global state (atomic)
    pub async fn commit(
        self,
        global_state: Arc<WorkspaceState>,
    ) -> Result<CommitResult> {
        let conflict_report = self.check_conflicts(global_state.clone()).await?;
        
        if !conflict_report.conflicts.is_empty() {
            let error_report = ErrorConflictReport {
                conflicts: conflict_report.conflicts.iter().map(|c| {
                    crate::error::FileConflict {
                        path: c.path.clone(),
                        base_hash: c.base_hash,
                        current_hash: c.current_hash,
                    }
                }).collect(),
            };
            return Err(RoutingError::ConflictDetected(error_report));
        }
        
        let changes: Vec<FileChange> = self.pending_changes
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
    pub async fn rollback(self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CommitResult {
    pub files_changed: usize,
}

#[derive(Debug, Clone)]
pub struct ConflictReport {
    pub conflicts: Vec<FileConflict>,
}

#[derive(Debug, Clone)]
pub struct FileConflict {
    pub path: PathBuf,
    pub base_hash: u64,
    pub current_hash: u64,
}

fn hash_content(content: &Option<String>) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash as _, Hasher as _};
    
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_workspace_isolation() {
        let workspace = WorkspaceState::new(PathBuf::from("/tmp"));
        let lock_manager = FileLockManager::new();
        let task_id = TaskId::new();
        
        workspace.apply_changes(&[
            FileChange::Create {
                path: PathBuf::from("test.rs"),
                content: "original".to_owned(),
            }
        ]).await.unwrap();
        
        let mut task_workspace = TaskWorkspace::new(
            task_id,
            vec![PathBuf::from("test.rs")],
            workspace.clone(),
            lock_manager,
        ).await.unwrap();
        
        task_workspace.modify_file(PathBuf::from("test.rs"), "modified".to_owned());
        
        assert_eq!(
            task_workspace.read_file(&PathBuf::from("test.rs")),
            Some("modified".to_owned())
        );
        
        assert_eq!(
            workspace.read_file(&PathBuf::from("test.rs")).await,
            Some("original".to_owned())
        );
    }
    
    #[tokio::test]
    async fn test_task_workspace_commit() {
        let workspace = WorkspaceState::new(PathBuf::from("/tmp"));
        let lock_manager = FileLockManager::new();
        let task_id = TaskId::new();
        
        workspace.apply_changes(&[
            FileChange::Create {
                path: PathBuf::from("test.rs"),
                content: "original".to_owned(),
            }
        ]).await.unwrap();
        
        let mut task_workspace = TaskWorkspace::new(
            task_id,
            vec![PathBuf::from("test.rs")],
            workspace.clone(),
            lock_manager,
        ).await.unwrap();
        
        task_workspace.modify_file(PathBuf::from("test.rs"), "modified".to_owned());
        
        task_workspace.commit(workspace.clone()).await.unwrap();
        
        assert_eq!(
            workspace.read_file(&PathBuf::from("test.rs")).await,
            Some("modified".to_owned())
        );
    }
}
