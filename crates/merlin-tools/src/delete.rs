use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::from_value;
use tokio::fs::remove_file;
use tracing::{debug, info};

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Parameters describing which file should be deleted.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteParams {
    /// Path to the file that will be deleted.
    file_path: PathBuf,
}

/// Tool that deletes a file from the filesystem.
pub struct DeleteTool;

impl DeleteTool {
    /// Delete the specified file.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` if the file does not exist or deletion fails.
    async fn delete_file(&self, params: DeleteParams) -> ToolResult<ToolOutput> {
        debug!("Deleting file: {:?}", params.file_path);

        if !params.file_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "File does not exist: {}",
                params.file_path.display()
            )));
        }

        if params.file_path.is_dir() {
            return Err(ToolError::InvalidInput(format!(
                "Path is a directory, not a file: {}",
                params.file_path.display()
            )));
        }

        remove_file(&params.file_path).await?;

        info!("Successfully deleted file: {}", params.file_path.display());
        Ok(ToolOutput::success(format!(
            "File deleted successfully: {}",
            params.file_path.display()
        )))
    }
}

impl Default for DeleteTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DeleteTool {
    fn name(&self) -> &'static str {
        "delete"
    }

    fn description(&self) -> &'static str {
        "Delete a file from the filesystem. \
         Parameters: file_path (string)"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let params: DeleteParams = from_value(input.params)?;
        self.delete_file(params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_delete_existing_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let tool = DeleteTool;
        let input = ToolInput {
            params: json!({
                "file_path": path,
            }),
        };

        let result = tool.execute(input).await.unwrap();
        assert!(result.success);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_file() {
        let tool = DeleteTool;
        let input = ToolInput {
            params: json!({
                "file_path": "/nonexistent/file.txt",
            }),
        };

        let result = tool.execute(input).await;
        result.unwrap_err();
    }

    #[tokio::test]
    async fn test_delete_directory_fails() {
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();

        let tool = DeleteTool;
        let input = ToolInput {
            params: json!({
                "file_path": path,
            }),
        };

        let result = tool.execute(input).await;
        result.unwrap_err();
    }
}
