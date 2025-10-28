//! Workspace setup utilities for tests.
//!
//! This module handles setting up test workspaces, including pre-made workspace
//! resolution, copying, and file creation.

use merlin_core::{Result, RoutingError};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Get test workspace path by name
///
/// # Errors
/// Returns error if workspace doesn't exist
pub fn get_test_workspace_path(workspace_name: &str) -> Result<PathBuf> {
    use std::path::Path as StdPath;

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(StdPath::parent)
        .ok_or_else(|| RoutingError::Other("Failed to find repository root".to_owned()))?
        .to_path_buf();

    let workspace_path = repo_root.join("test-workspaces").join(workspace_name);

    if !workspace_path.exists() {
        return Err(RoutingError::Other(format!(
            "Test workspace not found: {workspace_name} at {}",
            workspace_path.display()
        )));
    }

    Ok(workspace_path)
}

/// Create files in workspace from map
///
/// # Errors
/// Returns error if file creation fails
pub fn create_files(workspace_path: &Path, files: &HashMap<String, String>) -> Result<()> {
    for (path, content) in files {
        let file_path = workspace_path.join(path);
        create_file_with_dirs(&file_path, content)?;
    }
    Ok(())
}

/// Create a file with all necessary parent directories
///
/// # Errors
/// Returns error if directory or file creation fails
fn create_file_with_dirs(file_path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| RoutingError::Other(format!("Failed to create directory: {err}")))?;
    }
    fs::write(file_path, content)
        .map_err(|err| RoutingError::Other(format!("Failed to write file: {err}")))?;
    Ok(())
}
