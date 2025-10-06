use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;
use crate::{Result, RoutingError};
use super::Tool;

/// Tool for reading file contents
pub struct ReadFileTool {
    workspace_root: PathBuf,
}

impl ReadFileTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }
    
    fn description(&self) -> &str {
        "Read the contents of a file from the workspace"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file relative to workspace root"
                }
            },
            "required": ["path"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'path' argument".to_owned()))?;
        
        let full_path = self.workspace_root.join(path);
        
        // Security check: ensure path is within workspace
        let canonical_path = full_path.canonicalize()
            .map_err(|e| RoutingError::Other(format!("Invalid path: {}", e)))?;
        let canonical_root = self.workspace_root.canonicalize()
            .map_err(|e| RoutingError::Other(format!("Invalid workspace root: {}", e)))?;
        
        if !canonical_path.starts_with(&canonical_root) {
            return Err(RoutingError::Other("Path outside workspace".to_owned()));
        }
        
        let content = fs::read_to_string(&canonical_path).await
            .map_err(|e| RoutingError::Other(format!("Failed to read file: {}", e)))?;
        
        Ok(json!({
            "content": content,
            "path": path,
            "size": content.len()
        }))
    }
}

/// Tool for writing file contents
pub struct WriteFileTool {
    workspace_root: PathBuf,
}

impl WriteFileTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }
    
    fn description(&self) -> &str {
        "Write content to a file in the workspace, creating it if it doesn't exist"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file relative to workspace root"
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
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'path' argument".to_owned()))?;
        
        let content = args.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'content' argument".to_owned()))?;
        
        let full_path = self.workspace_root.join(path);
        
        // Security check: ensure path is within workspace
        let parent = full_path.parent()
            .ok_or_else(|| RoutingError::Other("Invalid path".to_owned()))?;
        
        // Create parent directories if they don't exist
        fs::create_dir_all(parent).await
            .map_err(|e| RoutingError::Other(format!("Failed to create directories: {}", e)))?;
        
        // Check if path would be within workspace after creation
        let canonical_root = self.workspace_root.canonicalize()
            .map_err(|e| RoutingError::Other(format!("Invalid workspace root: {}", e)))?;
        let canonical_parent = parent.canonicalize()
            .map_err(|e| RoutingError::Other(format!("Invalid parent path: {}", e)))?;
        
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(RoutingError::Other("Path outside workspace".to_owned()));
        }
        
        fs::write(&full_path, content).await
            .map_err(|e| RoutingError::Other(format!("Failed to write file: {}", e)))?;
        
        Ok(json!({
            "path": path,
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
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }
    
    fn description(&self) -> &str {
        "List files and directories in a given path"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory relative to workspace root (empty string for root)"
                }
            },
            "required": ["path"]
        })
    }
    
    async fn execute(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RoutingError::Other("Missing 'path' argument".to_owned()))?;
        
        let full_path = if path.is_empty() {
            self.workspace_root.clone()
        } else {
            self.workspace_root.join(path)
        };
        
        // Security check: ensure path is within workspace
        let canonical_path = full_path.canonicalize()
            .map_err(|e| RoutingError::Other(format!("Invalid path: {}", e)))?;
        let canonical_root = self.workspace_root.canonicalize()
            .map_err(|e| RoutingError::Other(format!("Invalid workspace root: {}", e)))?;
        
        if !canonical_path.starts_with(&canonical_root) {
            return Err(RoutingError::Other("Path outside workspace".to_owned()));
        }
        
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&canonical_path).await
            .map_err(|e| RoutingError::Other(format!("Failed to read directory: {}", e)))?;
        
        while let Some(entry) = read_dir.next_entry().await
            .map_err(|e| RoutingError::Other(format!("Failed to read entry: {}", e)))? {
            
            let metadata = entry.metadata().await
                .map_err(|e| RoutingError::Other(format!("Failed to read metadata: {}", e)))?;
            
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
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "Hello, World!").await.unwrap();
        
        let tool = ReadFileTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "path": "test.txt" })).await.unwrap();
        
        assert_eq!(result["content"], "Hello, World!");
        assert_eq!(result["size"], 13);
    }

    #[tokio::test]
    async fn test_write_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = WriteFileTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({
            "path": "new_file.txt",
            "content": "Test content"
        })).await.unwrap();
        
        assert_eq!(result["path"], "new_file.txt");
        assert_eq!(result["size"], 12);
        
        // Verify file was created
        let content = fs::read_to_string(temp_dir.path().join("new_file.txt")).await.unwrap();
        assert_eq!(content, "Test content");
    }

    #[tokio::test]
    async fn test_list_files_tool() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content1").await.unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content2").await.unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();
        
        let tool = ListFilesTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "path": "" })).await.unwrap();
        
        let entries = result["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn test_security_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = ReadFileTool::new(temp_dir.path().to_path_buf());
        let result = tool.execute(json!({ "path": "../../../etc/passwd" })).await;
        
        assert!(result.is_err());
    }
}
