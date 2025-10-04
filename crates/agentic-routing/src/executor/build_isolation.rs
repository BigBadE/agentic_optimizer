use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::process::Command;
use crate::{FileChange, Result, RoutingError};
use super::state::WorkspaceState;

/// Isolated build environment for task validation
pub struct IsolatedBuildEnv {
    temp_dir: TempDir,
    _original_workspace: PathBuf,
}

impl IsolatedBuildEnv {
    /// Create isolated build environment
    pub async fn new(workspace: &WorkspaceState) -> Result<Self> {
        let temp_dir = TempDir::new()
            .map_err(|e| RoutingError::Other(format!("Failed to create temp dir: {}", e)))?;
        
        // TODO: Copy workspace files for full isolation
        // For now, we just create an empty temp directory
        // In production, this would copy the entire workspace
        
        Ok(Self {
            temp_dir,
            _original_workspace: workspace.root_path().to_path_buf(),
        })
    }
    
    /// Apply changes to isolated environment
    pub async fn apply_changes(&self, changes: &[FileChange]) -> Result<()> {
        for change in changes {
            match change {
                FileChange::Create { path, content } |
                FileChange::Modify { path, content } => {
                    let full_path = self.temp_dir.path().join(path);
                    
                    if let Some(parent) = full_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    
                    tokio::fs::write(full_path, content).await?;
                }
                FileChange::Delete { path } => {
                    let full_path = self.temp_dir.path().join(path);
                    if full_path.exists() {
                        tokio::fs::remove_file(full_path).await.ok();
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Run build validation in isolation
    pub async fn validate_build(&self) -> Result<BuildResult> {
        let start = std::time::Instant::now();
        
        let output = Command::new("cargo")
            .arg("check")
            .arg("--all-targets")
            .current_dir(self.temp_dir.path())
            .output()
            .await?;
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        Ok(BuildResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
        })
    }
    
    /// Run tests in isolation
    pub async fn run_tests(&self, timeout_secs: u64) -> Result<TestResult> {
        let start = std::time::Instant::now();
        
        let output = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            Command::new("cargo")
                .arg("test")
                .arg("--all-targets")
                .current_dir(self.temp_dir.path())
                .output()
        )
        .await
        .map_err(|_| RoutingError::Timeout(timeout_secs * 1000))??;
        
        let duration_ms = start.elapsed().as_millis() as u64;
        let output_str = String::from_utf8_lossy(&output.stdout).to_string();
        
        Ok(TestResult {
            success: output.status.success(),
            passed: parse_test_count(&output_str, "passed"),
            failed: parse_test_count(&output_str, "failed"),
            details: output_str,
            duration_ms,
        })
    }
    
    /// Run clippy in isolation
    pub async fn run_clippy(&self) -> Result<LintResult> {
        let start = std::time::Instant::now();
        
        let output = Command::new("cargo")
            .arg("clippy")
            .arg("--all-targets")
            .arg("--")
            .arg("-D")
            .arg("warnings")
            .current_dir(self.temp_dir.path())
            .output()
            .await?;
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        Ok(LintResult {
            success: output.status.success(),
            warnings: parse_clippy_warnings(&String::from_utf8_lossy(&output.stderr)),
            duration_ms,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub success: bool,
    pub passed: usize,
    pub failed: usize,
    pub details: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct LintResult {
    pub success: bool,
    pub warnings: Vec<String>,
    pub duration_ms: u64,
}

fn parse_test_count(output: &str, status: &str) -> usize {
    output
        .lines()
        .find(|line| line.contains("test result:"))
        .and_then(|line| {
            line.split_whitespace()
                .enumerate()
                .find(|(_, word)| *word == status)
                .and_then(|(idx, _)| {
                    line.split_whitespace()
                        .nth(idx.saturating_sub(1))
                        .and_then(|num| num.parse().ok())
                })
        })
        .unwrap_or(0)
}

fn parse_clippy_warnings(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| line.contains("warning:"))
        .map(String::from)
        .collect()
}
