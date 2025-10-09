use super::state::WorkspaceState;
use crate::{FileChange, Result, RoutingError};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs::{create_dir_all, read_dir, read_to_string, remove_file, write};
use tokio::process::Command;
use tokio::time::timeout;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Isolated build environment for task validation
pub struct IsolatedBuildEnv {
    temp_dir: TempDir,
    _original_workspace: PathBuf,
}

impl IsolatedBuildEnv {
    /// Create isolated build environment
    ///
    /// # Errors
    /// Returns an error if the temporary directory cannot be created or if workspace copying fails.
    pub fn new(workspace: &WorkspaceState) -> Result<Self> {
        let temp_dir = TempDir::new()
            .map_err(|err| RoutingError::Other(format!("Failed to create temp dir: {err}")))?;

        Ok(Self {
            temp_dir,
            _original_workspace: workspace.root_path().clone(),
        })
    }

    /// Copy workspace files to isolated environment
    ///
    /// This copies essential build files (Cargo.toml, Cargo.lock, src/, etc.) to the
    /// isolated environment for full build isolation. Large directories like target/
    /// and .git/ are excluded for performance.
    ///
    /// # Errors
    /// Returns an error if filesystem operations fail during copying.
    pub async fn copy_workspace_files(&self, workspace_root: &Path) -> Result<()> {
        Self::copy_dir_recursive(
            workspace_root.to_path_buf(),
            self.temp_dir.path().to_path_buf(),
            workspace_root.to_path_buf(),
        )
        .await
    }

    fn copy_dir_recursive(
        source: PathBuf,
        dest: PathBuf,
        workspace_root: PathBuf,
    ) -> BoxFuture<Result<()>> {
        Box::pin(async move {
            let mut entries = read_dir(&source).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                if file_name_str.starts_with('.') && file_name_str != ".cargo" {
                    continue;
                }

                if matches!(
                    file_name_str.as_ref(),
                    "target" | "node_modules" | ".git" | ".idea" | ".vscode"
                ) {
                    continue;
                }

                let relative_path = path
                    .strip_prefix(&workspace_root)
                    .map_err(|err| RoutingError::Other(format!("Path strip failed: {err}")))?;
                let dest_path = dest.join(relative_path);

                if path.is_dir() {
                    create_dir_all(&dest_path).await?;
                    Self::copy_dir_recursive(path, dest.clone(), workspace_root.clone()).await?;
                } else if path.is_file() {
                    Self::copy_file(&path, &dest_path).await?;
                }
            }

            Ok(())
        })
    }

    /// Copy a single file from source to destination
    ///
    /// # Errors
    /// Returns an error if filesystem operations fail during copying.
    async fn copy_file(source: &Path, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            create_dir_all(parent).await?;
        }
        let content = read_to_string(source).await.unwrap_or_default();
        write(dest, content).await?;
        Ok(())
    }

    /// Apply changes to isolated environment
    ///
    /// # Errors
    /// Returns an error if filesystem operations fail when applying changes.
    pub async fn apply_changes(&self, changes: &[FileChange]) -> Result<()> {
        for change in changes {
            match change {
                FileChange::Create { path, content } | FileChange::Modify { path, content } => {
                    let full_path = self.temp_dir.path().join(path);

                    if let Some(parent) = full_path.parent() {
                        create_dir_all(parent).await?;
                    }

                    write(full_path, content).await?;
                }
                FileChange::Delete { path } => {
                    let full_path = self.temp_dir.path().join(path);
                    if full_path.exists() {
                        remove_file(full_path).await?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Run build validation in isolation
    ///
    /// # Errors
    /// Returns an error if the cargo command fails to execute.
    pub async fn validate_build(&self) -> Result<BuildResult> {
        let start = Instant::now();

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
    ///
    /// # Errors
    /// Returns an error on timeout or if the cargo test command fails to execute.
    pub async fn run_tests(&self, timeout_secs: u64) -> Result<TestResult> {
        let start = Instant::now();

        let output = timeout(
            Duration::from_secs(timeout_secs),
            Command::new("cargo")
                .arg("test")
                .arg("--all-targets")
                .current_dir(self.temp_dir.path())
                .output(),
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
    ///
    /// # Errors
    /// Returns an error if cargo clippy fails to execute.
    pub async fn run_clippy(&self) -> Result<LintResult> {
        let start = Instant::now();

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

/// Result of a build validation
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// Whether the build succeeded
    pub success: bool,
    /// Standard output from the build command
    pub stdout: String,
    /// Standard error from the build command
    pub stderr: String,
    /// Build duration in milliseconds
    pub duration_ms: u64,
}

/// Result of test execution
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Whether all tests passed
    pub success: bool,
    /// Number of tests that passed
    pub passed: usize,
    /// Number of tests that failed
    pub failed: usize,
    /// Detailed test output
    pub details: String,
    /// Test execution duration in milliseconds
    pub duration_ms: u64,
}

/// Result of clippy lint execution
#[derive(Debug, Clone)]
pub struct LintResult {
    /// Whether clippy found no issues
    pub success: bool,
    /// List of clippy warnings
    pub warnings: Vec<String>,
    /// Lint execution duration in milliseconds
    pub duration_ms: u64,
}

fn parse_test_count(output: &str, status: &str) -> usize {
    output
        .lines()
        .find(|line| line.contains("test result:"))
        .and_then(|line| {
            let (idx, _) = line
                .split_whitespace()
                .enumerate()
                .find(|(_, word)| *word == status)?;
            line.split_whitespace()
                .nth(idx - 1)
                .and_then(|num| num.parse().ok())
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
