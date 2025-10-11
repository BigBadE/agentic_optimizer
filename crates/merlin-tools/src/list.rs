use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json};
use tokio::fs::read_dir;
use tracing::debug;

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Parameters describing which directory should be listed.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListParams {
    /// Path to the directory that will be listed.
    directory_path: PathBuf,
    #[serde(default)]
    /// Whether to include hidden files (starting with .)
    include_hidden: bool,
}

/// Tool that lists files and directories in a given path.
pub struct ListTool;

impl ListTool {
    /// List files and directories in the specified path.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` if the directory does not exist or cannot be read.
    async fn list_directory(&self, params: ListParams) -> ToolResult<ToolOutput> {
        debug!("Listing directory: {:?}", params.directory_path);

        if !params.directory_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "Directory does not exist: {}",
                params.directory_path.display()
            )));
        }

        if !params.directory_path.is_dir() {
            return Err(ToolError::InvalidInput(format!(
                "Path is not a directory: {}",
                params.directory_path.display()
            )));
        }

        let mut entries = read_dir(&params.directory_path).await?;
        let mut files = Vec::new();
        let mut directories = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy().to_string();

            // Skip hidden files if not included
            if !params.include_hidden && name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                directories.push(name);
            } else {
                files.push(name);
            }
        }

        // Sort for consistent output
        files.sort();
        directories.sort();

        let data = json!({
            "files": files,
            "directories": directories,
        });

        Ok(ToolOutput::success_with_data(
            format!(
                "Found {} files and {} directories in {}",
                files.len(),
                directories.len(),
                params.directory_path.display()
            ),
            data,
        ))
    }
}

impl Default for ListTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ListTool {
    fn name(&self) -> &'static str {
        "list"
    }

    fn description(&self) -> &'static str {
        "List files and directories in a given directory. \
         Parameters: directory_path (string), include_hidden (bool, optional, default: false)"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let params: ListParams = from_value(input.params)?;
        self.list_directory(params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::fs::{create_dir, write};

    #[tokio::test]
    async fn test_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_path_buf();

        // Create some test files and directories
        write(dir_path.join("file1.txt"), "content")
            .await
            .expect("write failed");
        write(dir_path.join("file2.txt"), "content")
            .await
            .expect("write failed");
        create_dir(dir_path.join("subdir"))
            .await
            .expect("create_dir failed");

        let tool = ListTool;
        let input = ToolInput {
            params: json!({
                "directory_path": dir_path,
            }),
        };

        let result = tool.execute(input).await.unwrap();
        assert!(result.success);

        let data = result.data.unwrap();
        let files = data["files"].as_array().unwrap();
        let dirs = data["directories"].as_array().unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(dirs.len(), 1);
    }

    #[tokio::test]
    async fn test_list_nonexistent_directory() {
        let tool = ListTool;
        let input = ToolInput {
            params: json!({
                "directory_path": "/nonexistent/directory",
            }),
        };

        let result = tool.execute(input).await;
        result.unwrap_err();
    }

    #[tokio::test]
    async fn test_list_file_not_directory() {
        use tempfile::NamedTempFile;
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let tool = ListTool;
        let input = ToolInput {
            params: json!({
                "directory_path": path,
            }),
        };

        let result = tool.execute(input).await;
        result.unwrap_err();
    }

    #[tokio::test]
    async fn test_list_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_path_buf();

        // Create visible and hidden files
        write(dir_path.join("visible.txt"), "content")
            .await
            .expect("write failed");
        write(dir_path.join(".hidden.txt"), "content")
            .await
            .expect("write failed");

        let tool = ListTool;

        // Test without include_hidden
        let input_no_hidden = ToolInput {
            params: json!({
                "directory_path": dir_path.clone(),
                "include_hidden": false,
            }),
        };
        let result_no_hidden = tool.execute(input_no_hidden).await.unwrap();
        let data_no_hidden = result_no_hidden.data.unwrap();
        let files_no_hidden = data_no_hidden["files"].as_array().unwrap();
        assert_eq!(files_no_hidden.len(), 1);

        // Test with include_hidden
        let input_with_hidden = ToolInput {
            params: json!({
                "directory_path": dir_path,
                "include_hidden": true,
            }),
        };
        let result_with_hidden = tool.execute(input_with_hidden).await.unwrap();
        let data_with_hidden = result_with_hidden.data.unwrap();
        let files_with_hidden = data_with_hidden["files"].as_array().unwrap();
        assert_eq!(files_with_hidden.len(), 2);
    }
}
