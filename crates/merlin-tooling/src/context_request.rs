//! Dynamic context request tool for agents.
//!
//! Allows agents to request additional context files during execution
//! when they need more information to complete a task.

use async_trait::async_trait;
use glob::glob;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(test)]
use tokio::fs::{create_dir, write};
use tokio::fs::{metadata, read_to_string};
use tokio::sync::Mutex;

use crate::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Arguments for context request tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequestArgs {
    /// File pattern to search for (glob pattern or file path)
    pub pattern: String,
    /// Reason for requesting this context
    pub reason: String,
    /// Maximum number of files to return
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_max_files() -> usize {
    5
}

/// Result of context request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequestResult {
    /// Files that were found and added to context
    pub files: Vec<ContextFile>,
    /// Whether the request was successful
    pub success: bool,
    /// Optional message
    pub message: String,
}

/// File added to context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    /// Path to the file
    pub path: PathBuf,
    /// File contents
    pub content: String,
    /// Size in bytes
    pub size: usize,
}

/// Tracker for requested context files
#[derive(Debug, Default, Clone)]
pub struct ContextTracker {
    /// Files requested during conversation
    requested_files: Arc<Mutex<Vec<PathBuf>>>,
}

impl ContextTracker {
    /// Create a new context tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a requested file
    pub async fn add_requested(&self, path: PathBuf) {
        let mut files = self.requested_files.lock().await;
        if !files.contains(&path) {
            files.push(path);
        }
    }

    /// Get all requested files
    pub async fn get_requested(&self) -> Vec<PathBuf> {
        self.requested_files.lock().await.clone()
    }

    /// Clear requested files
    pub async fn clear(&self) {
        self.requested_files.lock().await.clear();
    }
}

/// Dynamic context request tool
pub struct ContextRequestTool {
    /// Project root for file resolution
    project_root: PathBuf,
    /// Context tracker
    tracker: ContextTracker,
    /// Maximum file size to read (bytes)
    max_file_size: usize,
}

impl ContextRequestTool {
    /// Create a new context request tool
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            tracker: ContextTracker::new(),
            max_file_size: 100_000, // 100KB default
        }
    }

    /// Create with custom tracker
    pub fn with_tracker(project_root: PathBuf, tracker: ContextTracker) -> Self {
        Self {
            project_root,
            tracker,
            max_file_size: 100_000,
        }
    }

    /// Set maximum file size
    #[must_use]
    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }

    /// Get the context tracker
    pub fn tracker(&self) -> &ContextTracker {
        &self.tracker
    }

    /// Find files matching pattern
    ///
    /// # Errors
    /// Returns an error if the glob pattern is invalid or file system access fails
    fn find_files(&self, pattern: &str, max_files: usize) -> Result<Vec<PathBuf>, ToolError> {
        // Check if pattern is an exact file path
        let exact_path = self.project_root.join(pattern);
        if exact_path.exists() && exact_path.is_file() {
            return Ok(vec![exact_path]);
        }

        // Use glob pattern matching
        let glob_pattern = if pattern.contains('*') || pattern.contains('?') {
            pattern.to_owned()
        } else {
            // If no wildcards, treat as filename search
            format!("**/{pattern}")
        };

        let full_pattern = self.project_root.join(&glob_pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let mut files = Vec::new();

        for entry in
            glob(&pattern_str).map_err(|err| ToolError::ExecutionFailed(err.to_string()))?
        {
            match entry {
                Ok(path) if path.is_file() => {
                    files.push(path);
                    if files.len() >= max_files {
                        break;
                    }
                }
                _ => {}
            }
        }

        Ok(files)
    }

    /// Read a file's contents
    ///
    /// # Errors
    /// Returns an error if file cannot be read or is too large
    async fn read_file(&self, path: &Path) -> Result<String, ToolError> {
        let file_metadata = metadata(path)
            .await
            .map_err(|err| ToolError::ExecutionFailed(format!("Failed to read metadata: {err}")))?;

        if file_metadata.len() > self.max_file_size as u64 {
            return Err(ToolError::ExecutionFailed(format!(
                "File too large: {} bytes (max: {})",
                file_metadata.len(),
                self.max_file_size
            )));
        }

        read_to_string(path)
            .await
            .map_err(|err| ToolError::ExecutionFailed(format!("Failed to read file: {err}")))
    }
}

#[async_trait]
impl Tool for ContextRequestTool {
    fn name(&self) -> &'static str {
        "requestContext"
    }

    fn description(&self) -> &'static str {
        "Request additional context files during task execution. \
         Use this when you need more information that wasn't in the initial context. \
         Provide a file pattern (glob or path) and a reason for the request."
    }

    fn typescript_signature(&self) -> &'static str {
        r"/**
 * Request additional context files during task execution.
 * @param pattern - File pattern (glob or path) to search for
 * @param reason - Reason for requesting this context
 * @param max_files - Maximum number of files to return (default: 5)
 * @returns Promise<{ files: { path: string, content: string, size: number }[], success: boolean, message: string }>
 */
declare function requestContext(pattern: string, reason: string, max_files?: number): Promise<{ files: { path: string, content: string, size: number }[], success: boolean, message: string }>"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let args: ContextRequestArgs = from_value(input.params)
            .map_err(|err| ToolError::InvalidInput(format!("Invalid arguments: {err}")))?;

        tracing::info!(
            "Context request: pattern='{}', reason='{}'",
            args.pattern,
            args.reason
        );

        // Find matching files
        let file_paths = self.find_files(&args.pattern, args.max_files)?;

        if file_paths.is_empty() {
            return Ok(ToolOutput::error(format!(
                "No files found matching pattern: {}",
                args.pattern
            )));
        }

        // Read file contents
        let mut context_files = Vec::new();

        for path in &file_paths {
            match self.read_file(path).await {
                Ok(content) => {
                    let size = content.len();

                    // Track this request
                    self.tracker.add_requested(path.clone()).await;

                    context_files.push(ContextFile {
                        path: path.clone(),
                        content,
                        size,
                    });
                }
                Err(err) => {
                    tracing::warn!("Failed to read {}: {}", path.display(), err);
                }
            }
        }

        let message = if context_files.is_empty() {
            "Failed to read any files".to_owned()
        } else {
            format!("Added {} files to context", context_files.len())
        };

        tracing::info!(
            "Context request result: {} files added",
            context_files.len()
        );

        let result = ContextRequestResult {
            files: context_files.clone(),
            success: !context_files.is_empty(),
            message: message.clone(),
        };

        let data = to_value(result).map_err(ToolError::Serialization)?;

        // If no files were successfully read, return an error ToolOutput
        if context_files.is_empty() {
            Ok(ToolOutput::error(message))
        } else {
            Ok(ToolOutput::success_with_data(message, data))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_context_request_exact_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("test.rs");

        write(&test_file, "fn main() {}")
            .await
            .expect("Failed to write test file");

        let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

        let input = ToolInput {
            params: serde_json::json!({
                "pattern": "test.rs",
                "reason": "Testing exact file match"
            }),
        };

        let output = tool.execute(input).await.expect("Tool execution failed");
        assert!(output.success);

        let result: ContextRequestResult = from_value(output.data.expect("No data in output"))
            .expect("Failed to deserialize result");

        assert!(result.success);
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].content, "fn main() {}");
    }

    #[tokio::test]
    async fn test_context_request_glob_pattern() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let src_dir = temp_dir.path().join("src");
        create_dir(&src_dir)
            .await
            .expect("Failed to create src dir");

        write(src_dir.join("lib.rs"), "pub fn foo() {}")
            .await
            .expect("Failed to write lib.rs");
        write(src_dir.join("main.rs"), "fn main() {}")
            .await
            .expect("Failed to write main.rs");

        let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

        let input = ToolInput {
            params: serde_json::json!({
                "pattern": "**/*.rs",
                "reason": "Testing glob pattern",
                "max_files": 10
            }),
        };

        let output = tool.execute(input).await.expect("Tool execution failed");
        assert!(output.success);

        let result: ContextRequestResult = from_value(output.data.expect("No data in output"))
            .expect("Failed to deserialize result");

        assert!(result.success);
        assert!(result.files.len() >= 2);
    }

    #[tokio::test]
    async fn test_context_tracker() {
        let tracker = ContextTracker::new();

        tracker.add_requested(PathBuf::from("file1.rs")).await;
        tracker.add_requested(PathBuf::from("file2.rs")).await;
        tracker.add_requested(PathBuf::from("file1.rs")).await; // Duplicate

        let requested = tracker.get_requested().await;
        assert_eq!(requested.len(), 2); // No duplicates

        tracker.clear().await;
        let requested_after_clear = tracker.get_requested().await;
        assert_eq!(requested_after_clear.len(), 0);
    }
}
