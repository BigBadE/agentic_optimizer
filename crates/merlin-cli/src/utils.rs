//! Utility functions for CLI operations

use merlin_deps::anyhow::{Context as _, Result};
use std::env;
use std::fs;
use std::fs::canonicalize;
use std::iter::Iterator as _;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;

const MAX_TASKS: usize = 50;

/// Get the Merlin folder path, respecting `MERLIN_FOLDER` environment variable
///
/// If `MERLIN_FOLDER` is set, use it. Otherwise default to `project/.merlin`
///
/// # Errors
/// Returns an error if the provided path cannot be canonicalized or accessed.
pub fn get_merlin_folder(project_root: &Path) -> Result<PathBuf> {
    let path =
        env::var("MERLIN_FOLDER").map_or_else(|_| project_root.join(".merlin"), PathBuf::from);
    if let Some(parent) = path.parent() {
        canonicalize(parent).with_context(|| {
            format!(
                "Couldn't create .merlin folder in project or provided MERLIN_FOLDER path \"{}\".\n\
        Make sure you don't have accidental quotes around it",
                path.display()
            )
        })?;
    }
    Ok(path)
}

/// Clean up old task files to prevent disk space waste
///
/// # Errors
/// Returns an error if the tasks directory cannot be read.
pub fn cleanup_old_tasks(merlin_dir: &Path) -> Result<()> {
    let tasks_dir = merlin_dir.join("tasks");
    if !tasks_dir.exists() {
        return Ok(());
    }

    // Get all task files sorted by modification time
    let mut task_files: Vec<_> = fs::read_dir(&tasks_dir)?
        .filter_map(StdResult::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "gz")
        })
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            meta.modified().ok().map(|time| (entry.path(), time))
        })
        .collect();

    // Sort by modification time (newest first)
    task_files.sort_by(|left, right| right.1.cmp(&left.1));

    // Keep only the 50 most recent, delete the rest
    for (path, _) in task_files.iter().skip(MAX_TASKS) {
        if let Err(error) = fs::remove_file(path) {
            merlin_deps::tracing::warn!("failed to remove old task file {:?}: {}", path, error);
        }
    }

    Ok(())
}
