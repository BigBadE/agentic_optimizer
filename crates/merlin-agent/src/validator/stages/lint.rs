use async_trait::async_trait;
use std::sync::Arc;

use merlin_core::Response;

use super::super::pipeline::{StageResult, ValidationStage};
use crate::{IsolatedBuildEnv, WorkspaceState};
use merlin_core::{Result, Task, ValidationStageType as StageType};

/// Lint validation using clippy
pub struct LintValidationStage {
    /// Optional workspace state used to run clippy in isolation
    workspace: Option<Arc<WorkspaceState>>,
    /// Maximum number of warnings allowed before failing the stage
    max_warnings: usize,
}

impl LintValidationStage {
    /// Set workspace for lint validation
    #[must_use]
    pub fn with_workspace(mut self, workspace: Arc<WorkspaceState>) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Set maximum warnings allowed
    #[must_use]
    pub fn with_max_warnings(mut self, max_warnings: usize) -> Self {
        self.max_warnings = max_warnings;
        self
    }
}

impl Default for LintValidationStage {
    fn default() -> Self {
        Self {
            workspace: None,
            max_warnings: 10,
        }
    }
}

#[async_trait]
impl ValidationStage for LintValidationStage {
    async fn validate(&self, _response: &Response, task: &Task) -> Result<StageResult> {
        if !task.requires_build_check() {
            return Ok(StageResult {
                stage: StageType::Lint,
                passed: true,
                duration_ms: 0,
                details: "Lint check skipped (no files modified)".to_owned(),
                score: 1.0,
            });
        }

        let Some(workspace) = &self.workspace else {
            return Ok(StageResult {
                stage: StageType::Lint,
                passed: true,
                duration_ms: 0,
                details: "Lint check skipped (no workspace)".to_owned(),
                score: 1.0,
            });
        };

        let build_env = IsolatedBuildEnv::new(workspace.as_ref())?;

        let lint_result = build_env.run_clippy().await?;

        let warning_count = lint_result.warnings.len();
        let passed = lint_result.success && warning_count <= self.max_warnings;

        let score = if warning_count == 0 {
            1.0
        } else if warning_count <= self.max_warnings {
            1.0 - (warning_count as f64 / (self.max_warnings * 2) as f64)
        } else {
            0.5
        };

        let details = if passed {
            if warning_count == 0 {
                format!(
                    "Clippy passed with no warnings ({}ms)",
                    lint_result.duration_ms
                )
            } else {
                format!(
                    "Clippy passed with {} warnings ({}ms)",
                    warning_count, lint_result.duration_ms
                )
            }
        } else {
            format!(
                "Clippy found {warning_count} warnings (max: {})",
                self.max_warnings
            )
        };

        Ok(StageResult {
            stage: StageType::Lint,
            passed,
            duration_ms: lint_result.duration_ms,
            details,
            score,
        })
    }

    async fn quick_check(&self, response: &Response) -> Result<bool> {
        let has_lint_issues = response.text.contains("warning:")
            && (response.text.contains("clippy") || response.text.contains("lint"));
        Ok(!has_lint_issues)
    }

    fn name(&self) -> &'static str {
        "Lint"
    }

    fn stage_type(&self) -> StageType {
        StageType::Lint
    }
}

#[cfg(test)]
mod tests {}
