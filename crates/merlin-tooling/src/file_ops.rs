//! File operation tools for reading, writing, and listing files.
//!
//! These tools provide safe file system access for agents executing in the TypeScript runtime.

use async_trait::async_trait;
use merlin_deps::serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

use crate::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Tool for writing files to the filesystem.
pub struct WriteFileTool {
    /// Root directory to constrain file access (for sandboxing)
    root_dir: PathBuf,
}

impl WriteFileTool {
    /// Create a new `WriteFileTool` with the given root directory.
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

        // Canonicalize root to prevent directory traversal attacks
        let canonical_root = self
            .root_dir
            .canonicalize()
            .map_err(|err| ToolError::InvalidInput(format!("Invalid root directory: {err}")))?;

        // For writing, we need to check if the path would be within the root after resolution
        // We can't canonicalize a non-existent path, so we check the parent directory
        let parent = full_path
            .parent()
            .ok_or_else(|| ToolError::InvalidInput(format!("Invalid path: {path}")))?;

        // Create parent directories if they don't exist
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|err| {
                ToolError::ExecutionFailed(format!("Failed to create parent directories: {err}"))
            })?;
        }

        let canonical_parent = parent.canonicalize().map_err(|err| {
            ToolError::InvalidInput(format!("Invalid parent directory for '{path}': {err}"))
        })?;

        if !canonical_parent.starts_with(&canonical_root) {
            return Err(ToolError::InvalidInput(format!(
                "Path '{path}' is outside the allowed directory"
            )));
        }

        Ok(full_path)
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "writeFile"
    }

    fn typescript_signature(&self) -> &'static str {
        r"/**
 * Writes content to a file in the filesystem.
 * @param path - Path to the file relative to the workspace root
 * @param content - Content to write to the file
 */
declare function writeFile(path: string, content: string): Promise<void>;"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        // Extract parameters - support both object and positional arguments
        let (path, content) = if let Some(obj) = input.params.as_object() {
            let path = obj.get("path").and_then(Value::as_str).ok_or_else(|| {
                ToolError::InvalidInput("writeFile requires a 'path' parameter".to_owned())
            })?;
            let content = obj.get("content").and_then(Value::as_str).ok_or_else(|| {
                ToolError::InvalidInput("writeFile requires a 'content' parameter".to_owned())
            })?;
            (path, content)
        } else {
            return Err(ToolError::InvalidInput(
                "writeFile requires path and content parameters".to_owned(),
            ));
        };

        // Resolve and validate path
        let full_path = self.resolve_path(path)?;

        merlin_deps::tracing::info!(
            "WriteFileTool: writing {} bytes to {:?} (resolved from '{}')",
            content.len(),
            full_path,
            path
        );

        // Write file contents
        fs::write(&full_path, content).map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to write file '{path}': {err}"))
        })?;

        merlin_deps::tracing::info!("WriteFileTool: successfully wrote file {:?}", full_path);

        Ok(ToolOutput::success(format!(
            "Wrote {} bytes to {path}",
            content.len()
        )))
    }
}

/// Tool for reading files from the filesystem.
pub struct ReadFileTool {
    /// Root directory to constrain file access (for sandboxing)
    root_dir: PathBuf,
}

impl ReadFileTool {
    /// Create a new `ReadFileTool` with the given root directory.
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
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "readFile"
    }

    fn typescript_signature(&self) -> &'static str {
        r"/**
 * Reads the contents of a file from the filesystem.
 * @param path - Path to the file relative to the workspace root
 * @returns The contents of the file as a string
 */
declare function readFile(path: string): Promise<string>;"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        // Extract path parameter
        let path = input
            .params
            .as_str()
            .or_else(|| input.params.get("path").and_then(Value::as_str))
            .ok_or_else(|| {
                ToolError::InvalidInput("readFile requires a 'path' parameter".to_owned())
            })?;

        // Resolve and validate path
        let full_path = self.resolve_path(path)?;

        // Read file contents
        let content = fs::read_to_string(&full_path).map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to read file '{path}': {err}"))
        })?;

        Ok(ToolOutput::success_with_data(
            format!("Read {} bytes from {path}", content.len()),
            json!(content),
        ))
    }
}

/// Tool for listing files in a directory.
pub struct ListFilesTool {
    /// Root directory to constrain file access (for sandboxing)
    root_dir: PathBuf,
}

impl ListFilesTool {
    /// Create a new `ListFilesTool` with the given root directory.
    ///
    /// All directory paths will be resolved relative to this root directory.
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
        let full_path = if path.is_empty() || path == "." {
            self.root_dir.clone()
        } else {
            self.root_dir.join(path)
        };

        // Canonicalize both paths to prevent directory traversal attacks
        let canonical_root = self
            .root_dir
            .canonicalize()
            .map_err(|err| ToolError::InvalidInput(format!("Invalid root directory: {err}")))?;

        if !full_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "Directory does not exist: {path}"
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
impl Tool for ListFilesTool {
    fn name(&self) -> &'static str {
        "listFiles"
    }

    fn typescript_signature(&self) -> &'static str {
        "/**\n * Lists all files in a directory.\n * @param path - Path to the directory relative to the workspace root (optional, defaults to \".\")\n * @returns Array of file names in the directory\n */\ndeclare function listFiles(path?: string): Promise<string[]>;"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        // Extract path parameter (optional, defaults to ".")
        let path = input
            .params
            .as_str()
            .or_else(|| input.params.get("path").and_then(Value::as_str))
            .unwrap_or(".");

        // Resolve and validate path
        let full_path = self.resolve_path(path)?;

        // List directory contents
        let entries = fs::read_dir(&full_path).map_err(|err| {
            ToolError::ExecutionFailed(format!("Failed to list directory '{path}': {err}"))
        })?;

        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| {
                ToolError::ExecutionFailed(format!("Failed to read directory entry: {err}"))
            })?;

            let file_name = entry
                .file_name()
                .to_str()
                .ok_or_else(|| ToolError::ExecutionFailed("Invalid UTF-8 in file name".to_owned()))?
                .to_owned();

            files.push(file_name);
        }

        files.sort();

        Ok(ToolOutput::success_with_data(
            format!("Found {} entries in {path}", files.len()),
            json!(files),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_deps::anyhow::Result;
    use merlin_deps::tempfile::TempDir;

    /// Tests successful file writing.
    ///
    /// # Errors
    /// Returns an error if file operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_write_file_success() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let tool = WriteFileTool::new(temp_dir.path());
        let input = ToolInput {
            params: json!({
                "path": "output.txt",
                "content": "Test content"
            }),
        };

        let result = tool.execute(input).await?;
        assert!(result.success);

        let written_content = fs::read_to_string(temp_dir.path().join("output.txt"))?;
        assert_eq!(written_content, "Test content");
        Ok(())
    }

    /// Tests that writing a file creates parent directories if they don't exist.
    ///
    /// # Errors
    /// Returns an error if file operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_write_file_creates_directories() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let tool = WriteFileTool::new(temp_dir.path());
        let input = ToolInput {
            params: json!({
                "path": "nested/dir/file.txt",
                "content": "Nested content"
            }),
        };

        let result = tool.execute(input).await?;
        assert!(result.success);

        let written_content = fs::read_to_string(temp_dir.path().join("nested/dir/file.txt"))?;
        assert_eq!(written_content, "Nested content");
        Ok(())
    }

    /// Tests listing files in a directory.
    ///
    /// # Errors
    /// Returns an error if file operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_list_files_success() -> Result<()> {
        let temp_dir = TempDir::new()?;
        fs::write(temp_dir.path().join("file1.txt"), "content1")?;
        fs::write(temp_dir.path().join("file2.txt"), "content2")?;
        fs::create_dir(temp_dir.path().join("subdir"))?;

        let tool = ListFilesTool::new(temp_dir.path());
        let input = ToolInput { params: json!(".") };

        let result = tool.execute(input).await?;
        assert!(result.success);

        let files = result
            .data
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected files data"))?;
        let files_array = files
            .as_array()
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected array"))?;
        assert_eq!(files_array.len(), 3);
        assert!(files_array.contains(&json!("file1.txt")));
        assert!(files_array.contains(&json!("file2.txt")));
        assert!(files_array.contains(&json!("subdir")));
        Ok(())
    }

    /// Tests listing files in an empty directory.
    ///
    /// # Errors
    /// Returns an error if file operations fail.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_list_files_empty_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let tool = ListFilesTool::new(temp_dir.path());
        let input = ToolInput { params: json!(".") };

        let result = tool.execute(input).await?;
        assert!(result.success);

        let files = result
            .data
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected files data"))?;
        let files_array = files
            .as_array()
            .ok_or_else(|| merlin_deps::anyhow::anyhow!("Expected array"))?;
        assert_eq!(files_array.len(), 0);
        Ok(())
    }

    /// Tests path traversal attack prevention in file writing.
    ///
    /// # Errors
    /// Returns an error if test setup fails.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn test_path_traversal_prevention_write() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let tool = WriteFileTool::new(temp_dir.path());
        let input = ToolInput {
            params: json!({
                "path": "../../../tmp/malicious.txt",
                "content": "bad"
            }),
        };

        let result = tool.execute(input).await;
        assert!(result.is_err(), "Expected error for path traversal");
        Ok(())
    }
}
