use merlin_core::{FileChange, Result, RoutingError};
use std::collections::HashMap;
use std::fs as stdfs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::fs as tfs;
use tokio::sync::RwLock;

/// Shared workspace state (synchronized)
pub struct WorkspaceState {
    files: RwLock<HashMap<PathBuf, String>>,
    root_path: PathBuf,
}

impl WorkspaceState {
    /// Create a new workspace state
    pub fn new(root_path: PathBuf) -> Arc<Self> {
        let canonical = stdfs::canonicalize(&root_path).unwrap_or(root_path);
        let canonical_root = Self::normalize_root(&canonical);

        Arc::new(Self {
            files: RwLock::new(HashMap::default()),
            root_path: canonical_root,
        })
    }

    /// Normalize root path for platform specifics.
    /// On Windows, strip the verbatim prefix (\\?\) so snapshots/tests match expected paths.
    #[cfg(windows)]
    fn normalize_root(path: &Path) -> PathBuf {
        let path_string = path.display().to_string();
        let normalized: String = path_string
            .strip_prefix(r"\\?\")
            .map_or_else(|| path_string.clone(), ToString::to_string);
        PathBuf::from(normalized)
    }

    /// No-op normalization on non-Windows platforms.
    #[cfg(not(windows))]
    fn normalize_root(path: &Path) -> PathBuf {
        path.to_path_buf()
    }

    /// Apply file changes to in-memory state and persist them to disk
    ///
    /// - Paths are normalized relative to the workspace root and validated
    /// - Parents are created as needed
    /// - Deletes are safe and ignore missing files
    ///
    /// # Errors
    /// Returns an error if applying or persisting changes fails.
    pub async fn apply_changes(&self, changes: &[FileChange]) -> Result<()> {
        // 1) Normalize and validate all paths first
        let mut normalized: Vec<FileChange> = Vec::with_capacity(changes.len());
        for change in changes {
            match change {
                FileChange::Create { path, content } | FileChange::Modify { path, content } => {
                    let rel = Self::normalize_to_root(self.root_path.as_path(), path.as_path())?;
                    if !Self::is_safe_relative(&rel) {
                        return Err(RoutingError::Other(format!(
                            "Unsafe relative path: {}",
                            rel.display()
                        )));
                    }
                    normalized.push(FileChange::Modify {
                        path: rel,
                        content: content.clone(),
                    });
                }
                FileChange::Delete { path } => {
                    let rel = Self::normalize_to_root(self.root_path.as_path(), path.as_path())?;
                    if !Self::is_safe_relative(&rel) {
                        return Err(RoutingError::Other(format!(
                            "Unsafe relative path: {}",
                            rel.display()
                        )));
                    }
                    normalized.push(FileChange::Delete { path: rel });
                }
            }
        }

        // 2) Persist changes to disk (no locks held during awaits)
        for change in &normalized {
            match change {
                FileChange::Modify { path, content } => {
                    let abs_path = self.root_path.join(path);
                    if let Some(parent) = abs_path.parent() {
                        tfs::create_dir_all(parent).await?;
                    }
                    tfs::write(&abs_path, content).await?;
                }
                FileChange::Delete { path } => {
                    let abs_path = self.root_path.join(path);
                    Self::delete_file_if_exists(&abs_path).await?;
                }
                // Create is treated equivalently to Modify after normalization
                FileChange::Create { .. } => {}
            }
        }

        // 3) Update in-memory state
        {
            let mut files = self.files.write().await;
            for change in normalized {
                match change {
                    FileChange::Modify { path, content } => {
                        files.insert(path, content);
                    }
                    FileChange::Delete { path } => {
                        files.remove(&path);
                    }
                    FileChange::Create { .. } => {}
                }
            }
        }

        Ok(())
    }

    /// Ensure the given path is a safe relative path (no absolute, no parent components)
    fn is_safe_relative(path: &Path) -> bool {
        if path.is_absolute() {
            return false;
        }
        for component in path.components() {
            if matches!(component, Component::ParentDir) {
                return false;
            }
        }
        true
    }

    /// Convert an input path to a path relative to `root` if it is inside `root`.
    /// Rejects any absolute path outside the root.
    ///
    /// # Errors
    /// Returns an error with a descriptive message when the absolute `input` is outside `root`.
    fn normalize_to_root(root: &Path, input: &Path) -> Result<PathBuf> {
        if input.is_absolute() {
            if let Ok(stripped) = input.strip_prefix(root) {
                return Ok(stripped.to_path_buf());
            }
            return Err(RoutingError::Other(format!(
                "Absolute path outside workspace root: {} (root: {})",
                input.display(),
                root.display()
            )));
        }
        Ok(input.to_path_buf())
    }

    /// Delete the file at `abs_path` if it exists and is a file. Ignore missing files.
    ///
    /// # Errors
    /// Returns an error if filesystem metadata lookup or file removal fails with an error
    /// other than "not found".
    async fn delete_file_if_exists(abs_path: &Path) -> Result<()> {
        match tfs::metadata(abs_path).await {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Ok(());
                }
                tfs::remove_file(abs_path).await?;
                Ok(())
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    return Ok(());
                }
                Err(err.into())
            }
        }
    }

    /// Read file content (for dependent tasks)
    ///
    /// Tries in-memory state first, then falls back to disk under the workspace root.
    pub async fn read_file(&self, path: &PathBuf) -> Option<String> {
        {
            let files = self.files.read().await;
            if let Some(content) = files.get(path) {
                return Some(content.clone());
            }
        }

        // Fallback to disk
        let abs_path = if path.is_absolute() {
            PathBuf::from(path)
        } else {
            self.root_path.join(path)
        };
        match tfs::read_to_string(abs_path).await {
            Ok(content) => Some(content),
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    return None;
                }
                None
            }
        }
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
        let mut snapshot_files = HashMap::default();

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
    /// Get file content from snapshot
    pub fn get(&self, path: &PathBuf) -> Option<String> {
        self.files.get(path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_deps::tempfile::TempDir;

    #[tokio::test]
    async fn test_workspace_concurrent_reads() {
        let tmp_dir = TempDir::new().expect("create temp dir");
        let workspace = WorkspaceState::new(tmp_dir.path().to_path_buf());

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
            tokio::join!(workspace.read_file(&path), workspace.read_file(&path));

        assert_eq!(content1, content2);
        assert_eq!(content1, Some("fn main() {}".to_owned()));
    }

    #[tokio::test]
    async fn test_workspace_snapshot() {
        let tmp_dir = TempDir::new().expect("create temp dir");
        let workspace = WorkspaceState::new(tmp_dir.path().to_path_buf());

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
