use async_trait::async_trait;
use std::sync::Arc;

use merlin_core::Response;

use super::super::pipeline::{StageResult, ValidationStage};
use crate::{IsolatedBuildEnv, WorkspaceState};
use merlin_core::{Result, Task, ValidationStageType as StageType};

/// Build validation using isolated cargo check
pub struct BuildValidationStage {
    /// Maximum time to allow the build to run before timing out (seconds)
    timeout_seconds: u64,
    /// Optional workspace state used to run isolated builds
    workspace: Option<Arc<WorkspaceState>>,
}

impl BuildValidationStage {
    /// Set timeout for build validation
    #[must_use]
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Set workspace for build validation
    #[must_use]
    pub fn with_workspace(mut self, workspace: Arc<WorkspaceState>) -> Self {
        self.workspace = Some(workspace);
        self
    }
}

impl Default for BuildValidationStage {
    fn default() -> Self {
        Self {
            timeout_seconds: 60,
            workspace: None,
        }
    }
}

#[async_trait]
impl ValidationStage for BuildValidationStage {
    async fn validate(&self, _response: &Response, task: &Task) -> Result<StageResult> {
        if !task.requires_build_check() {
            return Ok(StageResult {
                stage: StageType::Build,
                passed: true,
                duration_ms: 0,
                details: "Build check skipped (no files modified)".to_owned(),
                score: 1.0,
            });
        }

        let Some(workspace) = &self.workspace else {
            return Ok(StageResult {
                stage: StageType::Build,
                passed: true,
                duration_ms: 0,
                details: "Build check skipped (no workspace)".to_owned(),
                score: 1.0,
            });
        };

        let build_env = IsolatedBuildEnv::new(workspace.as_ref())?;

        let build_result = build_env.validate_build().await?;

        let passed = build_result.success;
        let score = if passed { 1.0 } else { 0.0 };

        let details = if passed {
            format!("Build succeeded ({}ms)", build_result.duration_ms)
        } else {
            format!(
                "Build failed: {}",
                build_result
                    .stderr
                    .lines()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        };

        Ok(StageResult {
            stage: StageType::Build,
            passed,
            duration_ms: build_result.duration_ms,
            details,
            score,
        })
    }

    async fn quick_check(&self, response: &Response) -> Result<bool> {
        let has_build_errors = response.text.contains("error[E")
            || response.text.contains("cannot find")
            || response.text.contains("mismatched types");
        Ok(!has_build_errors)
    }

    fn name(&self) -> &'static str {
        "Build"
    }

    fn stage_type(&self) -> StageType {
        StageType::Build
    }
}

#[cfg(test)]
mod tests {}
