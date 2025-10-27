//! File scanning utilities for discovering source files in a project.

use merlin_deps::walkdir::{DirEntry, WalkDir};
use std::path::Path;

use core::result::Result as CoreResult;
use merlin_core::FileContext;

use crate::fs_utils::is_source_file;

/// Directories ignored during project scan.
pub const IGNORED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    ".git",
    ".idea",
    ".vscode",
];

/// Check if a directory entry should be ignored
pub fn is_ignored(entry: &DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();

    // Don't filter the root directory itself (depth 0)
    if entry.depth() == 0 {
        return false;
    }

    if file_name.starts_with('.') {
        return true;
    }

    if entry.file_type().is_dir() && IGNORED_DIRS.contains(&file_name.as_ref()) {
        return true;
    }

    false
}

/// Collect a list of readable code files under the project root.
pub fn collect_all_files(
    project_root: &Path,
    max_files: usize,
    max_file_size: usize,
) -> Vec<FileContext> {
    let mut files = Vec::new();

    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|entry_var| !is_ignored(entry_var))
        .filter_map(CoreResult::ok)
    {
        if entry.file_type().is_dir() {
            continue;
        }

        if !is_source_file(entry.path()) {
            continue;
        }

        if let Ok(metadata) = entry.metadata()
            && metadata.len() > max_file_size as u64
        {
            continue;
        }

        if let Ok(file_context) = FileContext::from_path(&entry.path().to_path_buf()) {
            files.push(file_context);
        }

        if files.len() >= max_files {
            break;
        }
    }

    files
}
