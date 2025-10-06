use async_trait::async_trait;
use crate::{IsolatedBuildEnv, Result, Task, ValidationStageType as StageType, WorkspaceState};
use super::super::pipeline::{StageResult, ValidationStage};
use std::sync::Arc;

/// Lint validation using clippy
pub struct LintValidationStage {
    workspace: Option<Arc<WorkspaceState>>,
    max_warnings: usize,
}

impl LintValidationStage {
    #[must_use]
    pub fn new() -> Self {
        Self {
            workspace: None,
            max_warnings: 10,
        }
    }
    
    #[must_use]
    pub fn with_workspace(mut self, workspace: Arc<WorkspaceState>) -> Self {
        self.workspace = Some(workspace);
        self
    }
    
    #[must_use]
    pub fn with_max_warnings(mut self, max_warnings: usize) -> Self {
        self.max_warnings = max_warnings;
        self
    }
}

impl Default for LintValidationStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationStage for LintValidationStage {
    async fn validate(&self, _response: &merlin_core::Response, task: &Task) -> Result<StageResult> {
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
        
        let build_env = IsolatedBuildEnv::new(workspace.as_ref()).await?;
        
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
                format!("Clippy passed with no warnings ({}ms)", lint_result.duration_ms)
            } else {
                format!("Clippy passed with {} warnings ({}ms)", 
                    warning_count, lint_result.duration_ms)
            }
        } else {
            format!("Clippy found {} warnings (max: {})", warning_count, self.max_warnings)
        };
        
        Ok(StageResult {
            stage: StageType::Lint,
            passed,
            duration_ms: lint_result.duration_ms,
            details,
            score,
        })
    }
    
    async fn quick_check(&self, response: &merlin_core::Response) -> Result<bool> {
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lint_validation_skip_no_files() {
        let stage = LintValidationStage::new();
        let response = merlin_core::Response {
            text: "test".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        let task = Task::new("Test".to_owned());
        
        let result = stage.validate(&response, &task).await.unwrap();
        assert!(result.passed);
        assert!(result.details.contains("skipped"));
    }
    
    #[tokio::test]
    async fn test_quick_check() {
        let stage = LintValidationStage::new();
        
        let good_response = merlin_core::Response {
            text: "Finished dev [unoptimized + debuginfo]".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        assert!(stage.quick_check(&good_response).await.unwrap());
        
        let bad_response = merlin_core::Response {
            text: "warning: unused variable - clippy::unused_variable".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        assert!(!stage.quick_check(&bad_response).await.unwrap());
    }
}

