//! File editing tool for find-and-replace operations.
//!
//! Provides safe text replacement operations within files.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};
use std::fs;
use std::path::PathBuf;

use crate::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Arguments for file editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditFileArgs {
    /// Path to the file to edit
    pub path: String,
    /// Text to find
    pub old_string: String,
    /// Text to replace with
    pub new_string: String,
    /// Whether to replace all occurrences (default: false)
    #[serde(default)]
    pub replace_all: bool,
}

/// Tool for editing files with find-and-replace operations.
pub struct EditFileTool {
    /// Root directory to constrain file access (for sandboxing)
    root_dir: PathBuf,
}

impl EditFileTool {
    /// Create a new `EditFileTool` with the given root directory.
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
    /// Returns error if path escapes the root directory or file doesn't exist
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
impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "editFile"
    }

    fn description(&self) -> &'static str {
        "Edits a file by replacing text. Use replace_all to replace all occurrences, \
         otherwise only the first occurrence is replaced. Returns an error if the \
         old_string appears multiple times and replace_all is false."
    }

    fn typescript_signature(&self) -> &'static str {
        r"/**
 * Edits a file by replacing text.
 * @param path - Path to the file relative to the workspace root
 * @param old_string - Text to find and replace
 * @param new_string - Text to replace with
 * @param options - Optional settings: { replace_all?: boolean }
 */
declare function editFile(path: string, old_string: string, new_string: string, options?: { replace_all?: boolean }): Promise<void>;"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        // Parse arguments from various input formats
        let args: EditFileArgs = if let Some(arr) = input.params.as_array() {
            // Handle positional arguments: [path, old_string, new_string, options?]
            if arr.len() < 3 {
                return Err(ToolError::InvalidInput(
                    "editFile requires at least 3 arguments: path, old_string, new_string"
                        .to_owned(),
                ));
            }

            let path = arr[0].as_str().ok_or_else(|| {
                ToolError::InvalidInput("First argument (path) must be a string".to_owned())
            })?;
            let old_string = arr[1].as_str().ok_or_else(|| {
                ToolError::InvalidInput("Second argument (old_string) must be a string".to_owned())
            })?;
            let new_string = arr[2].as_str().ok_or_else(|| {
                ToolError::InvalidInput("Third argument (new_string) must be a string".to_owned())
            })?;

            let replace_all = if arr.len() > 3 {
                arr[3]
                    .get("replace_all")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            } else {
                false
            };

            EditFileArgs {
                path: path.to_owned(),
                old_string: old_string.to_owned(),
                new_string: new_string.to_owned(),
                replace_all,
            }
        } else {
            // Handle object format
            from_value(input.params)
                .map_err(|err| ToolError::InvalidInput(format!("Invalid arguments: {err}")))?
        };

        // Resolve and validate path
        let full_path = self.resolve_path(&args.path)?;

        // Read file contents
        let content = fs::read_to_string(&full_path).map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to read file '{}': {err}", args.path))
        })?;

        // Check if old_string exists
        if !content.contains(&args.old_string) {
            return Err(ToolError::ExecutionFailed(format!(
                "String '{}' not found in file '{}'",
                args.old_string, args.path
            )));
        }

        // Perform replacement
        let new_content = if args.replace_all {
            content.replace(&args.old_string, &args.new_string)
        } else {
            // Check for multiple occurrences
            let count = content.matches(&args.old_string).count();
            if count > 1 {
                return Err(ToolError::ExecutionFailed(format!(
                    "String '{}' appears {} times in '{}'. Use replace_all: true to replace all occurrences",
                    args.old_string, count, args.path
                )));
            }
            content.replacen(&args.old_string, &args.new_string, 1)
        };

        // Write back to file
        fs::write(&full_path, new_content).map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to write file '{}': {err}", args.path))
        })?;

        let replacement_count = if args.replace_all {
            content.matches(&args.old_string).count()
        } else {
            1
        };

        Ok(ToolOutput::success(format!(
            "Replaced {replacement_count} occurrence(s) in {}",
            args.path
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_edit_file_single_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world").unwrap();

        let tool = EditFileTool::new(temp_dir.path());
        let input = ToolInput {
            params: json!({
                "path": "test.txt",
                "old_string": "world",
                "new_string": "rust"
            }),
        };

        let result = tool.execute(input).await.unwrap();
        assert!(result.success);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello rust");
    }

    #[tokio::test]
    async fn test_edit_file_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "foo bar foo baz foo").unwrap();

        let tool = EditFileTool::new(temp_dir.path());
        let input = ToolInput {
            params: json!({
                "path": "test.txt",
                "old_string": "foo",
                "new_string": "FOO",
                "replace_all": true
            }),
        };

        let result = tool.execute(input).await.unwrap();
        assert!(result.success);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "FOO bar FOO baz FOO");
    }

    #[tokio::test]
    async fn test_edit_file_multiple_without_replace_all_fails() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "foo bar foo baz").unwrap();

        let tool = EditFileTool::new(temp_dir.path());
        let input = ToolInput {
            params: json!({
                "path": "test.txt",
                "old_string": "foo",
                "new_string": "FOO"
            }),
        };

        let result = tool.execute(input).await;
        result.unwrap_err();

        // Content should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "foo bar foo baz");
    }
}
