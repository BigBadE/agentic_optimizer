//! File deletion tool.
//!
//! Provides safe file deletion for agents executing in the TypeScript runtime.

use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Tool for deleting files from the filesystem.
pub struct DeleteFileTool {
    /// Root directory to constrain file access (for sandboxing)
    root_dir: PathBuf,
}

impl DeleteFileTool {
    /// Create a new `DeleteFileTool` with the given root directory.
    ///
    /// All file paths will be resolved relative to this root directory.
    #[must_use]
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    /// Resolve a path relative to the root directory and validate it's within bounds.
    ///
    /// # Errors
    /// Returns error if path escapes the root directory
    fn resolve_path(&self, path: &str) -> ToolResult<PathBuf> {
        let full_path = self.root_dir.join(path);

        // Canonicalize both paths to prevent directory traversal attacks
        let canonical_root = self
            .root_dir
            .canonicalize()
            .map_err(|err| ToolError::InvalidInput(format!("Invalid root directory: {err}")))?;

        if !full_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "File does not exist: {path}"
            )));
        }

        let canonical_path = full_path
            .canonicalize()
            .map_err(|err| ToolError::InvalidInput(format!("Invalid path '{path}': {err}")))?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(ToolError::InvalidInput(format!(
                "Path '{path}' is outside the allowed directory"
            )));
        }

        Ok(canonical_path)
    }
}

#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &'static str {
        "deleteFile"
    }

    fn description(&self) -> &'static str {
        "Deletes a file from the filesystem (directories cannot be deleted)"
    }

    fn typescript_signature(&self) -> &'static str {
        r"/**
 * Deletes a file from the filesystem.
 * @param path - Path to the file relative to the workspace root
 * @throws Error if the path is a directory or file does not exist
 */
declare function deleteFile(path: string): Promise<void>;"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        // Extract path parameter
        let path = input
            .params
            .as_str()
            .or_else(|| input.params.get("path").and_then(Value::as_str))
            .ok_or_else(|| {
                ToolError::InvalidInput("deleteFile requires a 'path' parameter".to_owned())
            })?;

        // Resolve and validate path
        let full_path = self.resolve_path(path)?;

        // Check if path is a directory
        if full_path.is_dir() {
            return Err(ToolError::InvalidInput(format!(
                "Cannot delete directory: {path}. Only files can be deleted."
            )));
        }

        // Delete the file
        fs::remove_file(&full_path).map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to delete file '{path}': {err}"))
        })?;

        Ok(ToolOutput::success(format!("Deleted file: {path}")))
    }
}
