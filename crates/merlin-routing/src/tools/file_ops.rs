use super::Tool;
use crate::{Result, RoutingError};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::fs::{
    create_dir_all as tokio_create_dir_all_async, read_dir as tokio_read_dir_async,
    read_to_string as tokio_read_to_string_async, write as tokio_write_async,
};

/// Tool for reading file contents
pub struct ReadFileTool {
    workspace_root: PathBuf,
}

impl ReadFileTool {
    /// Create a new read file tool
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file from the workspace"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace root, or absolute path within workspace)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'path' argument".to_owned()))?;

        // Handle both absolute and relative paths
        let path_buf = PathBuf::from(path);
        let (full_path, relative_path) = if path_buf.is_absolute() {
            // If absolute path provided, verify it's within workspace and get relative path
            let canonical_root = self
                .workspace_root
                .canonicalize()
                .map_err(|err| RoutingError::Other(format!("Invalid workspace root: {err}")))?;

            let canonical_path = path_buf
                .canonicalize()
                .map_err(|err| RoutingError::Other(format!("Invalid path '{path}': {err}")))?;

            if !canonical_path.starts_with(&canonical_root) {
                return Err(RoutingError::Other(format!(
                    "Path '{path}' is outside workspace"
                )));
            }

            // Get relative path by stripping workspace root
            let rel_path = canonical_path
                .strip_prefix(&canonical_root)
                .map_err(|err| {
                    RoutingError::Other(format!("Failed to create relative path: {err}"))
                })?
                .to_path_buf();

            (canonical_path, rel_path)
        } else {
            // Relative path - proceed as before
            let full = self.workspace_root.join(path);
            let canonical_path = full
                .canonicalize()
                .map_err(|err| RoutingError::Other(format!("Invalid path '{path}': {err}")))?;

            let canonical_root = self
                .workspace_root
                .canonicalize()
                .map_err(|err| RoutingError::Other(format!("Invalid workspace root: {err}")))?;

            if !canonical_path.starts_with(&canonical_root) {
                return Err(RoutingError::Other(format!(
                    "Path '{path}' is outside workspace"
                )));
            }

            (canonical_path, PathBuf::from(path))
        };

        let content = tokio_read_to_string_async(&full_path)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to read file: {err}")))?;

        Ok(json!({
            "content": content,
            "path": relative_path.to_string_lossy().to_string(),
            "size": content.len()
        }))
    }
}

/// Tool for writing file contents
pub struct WriteFileTool {
    workspace_root: PathBuf,
}

impl WriteFileTool {
    /// Create a new write file tool
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Write content to a file in the workspace, creating it if it doesn't exist"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file (relative to workspace root, or absolute path within workspace)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'path' argument".to_owned()))?;

        let content = args
            .get("content")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'content' argument".to_owned()))?;

        // Handle both absolute and relative paths
        let path_buf = PathBuf::from(path);
        let canonical_root = self
            .workspace_root
            .canonicalize()
            .map_err(|err| RoutingError::Other(format!("Invalid workspace root: {err}")))?;

        let (full_path, relative_path) = if path_buf.is_absolute() {
            // Verify absolute path is within workspace
            // For write operations, we can't canonicalize a non-existent path,
            // so we check the parent directory
            let parent = path_buf
                .parent()
                .ok_or_else(|| RoutingError::Other("Invalid path".to_owned()))?;

            // Parent must exist or be within workspace
            if parent.exists() {
                let canonical_parent = parent
                    .canonicalize()
                    .map_err(|err| RoutingError::Other(format!("Invalid parent path: {err}")))?;

                if !canonical_parent.starts_with(&canonical_root) {
                    return Err(RoutingError::Other(format!(
                        "Path '{path}' is outside workspace"
                    )));
                }
            } else {
                // Parent doesn't exist - check if it would be within workspace
                // This is tricky, so we require the parent to exist for absolute paths
                return Err(RoutingError::Other(format!(
                    "Parent directory of '{path}' does not exist"
                )));
            }

            // Get relative path by stripping workspace root
            let rel_path = path_buf
                .strip_prefix(&canonical_root)
                .map_err(|_| RoutingError::Other(format!("Path '{path}' is outside workspace")))?
                .to_path_buf();

            (path_buf, rel_path)
        } else {
            // Relative path - proceed as before
            let full = self.workspace_root.join(path);
            (full, PathBuf::from(path))
        };

        // Security check: ensure path is within workspace
        let parent = full_path
            .parent()
            .ok_or_else(|| RoutingError::Other("Invalid path".to_owned()))?;

        // Create parent directories if they don't exist
        tokio_create_dir_all_async(parent)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to create directories: {err}")))?;

        // Check if path would be within workspace after creation
        let canonical_parent = parent
            .canonicalize()
            .map_err(|err| RoutingError::Other(format!("Invalid parent path: {err}")))?;

        if !canonical_parent.starts_with(&canonical_root) {
            return Err(RoutingError::Other(format!(
                "Path '{path}' is outside workspace"
            )));
        }

        tokio_write_async(&full_path, content)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to write file: {err}")))?;

        Ok(json!({
            "path": relative_path.to_string_lossy().to_string(),
            "size": content.len(),
            "created": !full_path.exists()
        }))
    }
}

/// Tool for listing files in a directory
pub struct ListFilesTool {
    workspace_root: PathBuf,
}

impl ListFilesTool {
    /// Create a new list files tool
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &'static str {
        "list_files"
    }

    fn description(&self) -> &'static str {
        "List files and directories in a given path"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory (relative to workspace root or absolute within workspace, empty string for root)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'path' argument".to_owned()))?;

        // Handle both absolute and relative paths
        let path_buf = PathBuf::from(path);
        let full_path = if path.is_empty() {
            self.workspace_root.clone()
        } else if path_buf.is_absolute() {
            // Verify absolute path is within workspace
            let canonical_root = self
                .workspace_root
                .canonicalize()
                .map_err(|error| RoutingError::Other(format!("Invalid workspace root: {error}")))?;

            let canonical_path = path_buf
                .canonicalize()
                .map_err(|error| RoutingError::Other(format!("Invalid path '{path}': {error}")))?;

            if !canonical_path.starts_with(&canonical_root) {
                return Err(RoutingError::Other(format!(
                    "Path '{path}' is outside workspace"
                )));
            }

            canonical_path
        } else {
            self.workspace_root.join(path)
        };

        // Security check: ensure path is within workspace
        let canonical_path = full_path
            .canonicalize()
            .map_err(|error| RoutingError::Other(format!("Invalid path '{path}': {error}")))?;
        let canonical_root = self
            .workspace_root
            .canonicalize()
            .map_err(|error| RoutingError::Other(format!("Invalid workspace root: {error}")))?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(RoutingError::Other(format!(
                "Path '{path}' is outside workspace"
            )));
        }

        let mut entries = Vec::default();
        let mut read_dir = tokio_read_dir_async(&canonical_path)
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to read directory: {err}")))?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|err| RoutingError::Other(format!("Failed to read entry: {err}")))?
        {
            let metadata = entry
                .metadata()
                .await
                .map_err(|err| RoutingError::Other(format!("Failed to read metadata: {err}")))?;

            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = metadata.is_dir();
            let size = if is_dir { None } else { Some(metadata.len()) };

            entries.push(json!({
                "name": name,
                "is_dir": is_dir,
                "size": size
            }));
        }

        Ok(json!({
            "path": path,
            "entries": entries
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_read_file_tool() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "Hello, World!").await?;

        let tool = ReadFileTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "path": "test.txt" })).await?;

        assert_eq!(result["content"], "Hello, World!");
        assert_eq!(result["size"], 13);
        Ok(())
    }

    #[tokio::test]
    async fn test_write_file_tool() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let tool = WriteFileTool::new(temp_dir.path().to_path_buf());
        let result = tool
            .execute(json!({
                "path": "new_file.txt",
                "content": "Test content"
            }))
            .await?;

        assert_eq!(result["path"], "new_file.txt");
        assert_eq!(result["size"], 12);

        // Verify file was created
        let content = fs::read_to_string(temp_dir.path().join("new_file.txt")).await?;
        assert_eq!(content, "Test content");
        Ok(())
    }

    #[tokio::test]
    async fn test_list_files_tool() -> Result<()> {
        let temp_dir = TempDir::new()?;
        fs::write(temp_dir.path().join("file1.txt"), "content1").await?;
        fs::write(temp_dir.path().join("file2.txt"), "content2").await?;
        fs::create_dir_all(temp_dir.path().join("subdir")).await?;

        let tool = ListFilesTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "path": "" })).await?;

        if let Some(entries) = result.get("entries").and_then(|value| value.as_array()) {
            assert_eq!(entries.len(), 3);
        } else {
            panic!("entries array missing from result");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_security_path_traversal() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let tool = ReadFileTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "path": "../../../etc/passwd" })).await;
        match result {
            Ok(_) => panic!("expected path traversal to fail"),
            Err(_err) => {}
        }
        Ok(())
    }
}
