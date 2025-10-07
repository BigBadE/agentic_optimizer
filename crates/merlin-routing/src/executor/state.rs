use crate::{FileChange, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared workspace state (synchronized)
pub struct WorkspaceState {
    files: RwLock<HashMap<PathBuf, String>>,
    root_path: PathBuf,
}

impl WorkspaceState {
    #[must_use]
    pub fn new(root_path: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            files: RwLock::new(HashMap::new()),
            root_path,
        })
    }

    /// Apply file changes atomically
    ///
    /// # Errors
    /// Returns an error if acquiring the write lock fails.
    pub async fn apply_changes(&self, changes: &[FileChange]) -> Result<()> {
        {
            let mut files = self.files.write().await;
            for change in changes {
                match change {
                    FileChange::Create { path, content } | FileChange::Modify { path, content } => {
                        files.insert(path.clone(), content.clone());
                    }
                    FileChange::Delete { path } => {
                        files.remove(path);
                    }
                }
            }
        }
        Ok(())
    }

    /// Read file content (for dependent tasks)
    pub async fn read_file(&self, path: &PathBuf) -> Option<String> {
        let files = self.files.read().await;
        files.get(path).cloned()
    }

    /// Get workspace root path
    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// Create snapshot of specific files
    ///
    /// # Errors
    /// Returns an error if acquiring the read lock fails.
    pub async fn snapshot(&self, files: &[PathBuf]) -> Result<WorkspaceSnapshot> {
        let file_map = self.files.read().await;
        let mut snapshot_files = HashMap::new();

        for path in files {
            if let Some(content) = file_map.get(path) {
                snapshot_files.insert(path.clone(), content.clone());
            }
        }

        Ok(WorkspaceSnapshot {
            files: snapshot_files,
        })
    }
}

/// Immutable snapshot of workspace state
#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    files: HashMap<PathBuf, String>,
}

impl WorkspaceSnapshot {
    #[must_use]
    pub fn get(&self, path: &PathBuf) -> Option<String> {
        self.files.get(path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    /// # Panics
    /// Panics if workspace operations fail in the test harness.
    async fn test_workspace_concurrent_reads() {
        let workspace = WorkspaceState::new(PathBuf::from("/tmp"));

        if let Err(error) = workspace
            .apply_changes(&[FileChange::Create {
                path: PathBuf::from("test.rs"),
                content: "fn main() {}".to_owned(),
            }])
            .await
        {
            panic!("failed to apply initial change: {error}");
        }

        let path = PathBuf::from("test.rs");
        let (content1, content2) =
            tokio::join!(workspace.read_file(&path), workspace.read_file(&path),);

        assert_eq!(content1, content2);
        assert_eq!(content1, Some("fn main() {}".to_owned()));
    }

    #[tokio::test]
    /// # Panics
    /// Panics if workspace operations fail in the test harness.
    async fn test_workspace_snapshot() {
        let workspace = WorkspaceState::new(PathBuf::from("/tmp"));

        if let Err(error) = workspace
            .apply_changes(&[FileChange::Create {
                path: PathBuf::from("test.rs"),
                content: "fn main() {}".to_owned(),
            }])
            .await
        {
            panic!("failed to apply initial change: {error}");
        }

        let snapshot = match workspace.snapshot(&[PathBuf::from("test.rs")]).await {
            Ok(snapshot) => snapshot,
            Err(error) => panic!("failed to create snapshot: {error}"),
        };

        if let Err(error) = workspace
            .apply_changes(&[FileChange::Modify {
                path: PathBuf::from("test.rs"),
                content: "fn main() { println!(\"changed\"); }".to_owned(),
            }])
            .await
        {
            panic!("failed to modify file: {error}");
        }

        assert_eq!(
            snapshot.get(&PathBuf::from("test.rs")),
            Some("fn main() {}".to_owned())
        );
        assert_eq!(
            workspace.read_file(&PathBuf::from("test.rs")).await,
            Some("fn main() { println!(\"changed\"); }".to_owned())
        );
    }
}
