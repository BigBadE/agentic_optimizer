use async_trait::async_trait;
use crate::{IsolatedBuildEnv, Result, Task, ValidationStageType as StageType, WorkspaceState};
use super::super::pipeline::{StageResult, ValidationStage};
use std::sync::Arc;

/// Build validation using isolated cargo check
pub struct BuildValidationStage {
    timeout_seconds: u64,
    workspace: Option<Arc<WorkspaceState>>,
}

impl BuildValidationStage {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            timeout_seconds: 60,
            workspace: None,
        }
    }
    
    #[must_use] 
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
    
    pub fn with_workspace(mut self, workspace: Arc<WorkspaceState>) -> Self {
        self.workspace = Some(workspace);
        self
    }
}

impl Default for BuildValidationStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationStage for BuildValidationStage {
    async fn validate(&self, _response: &merlin_core::Response, task: &Task) -> Result<StageResult> {
        if !task.requires_build_check() {
            return Ok(StageResult {
                stage: StageType::Build,
                passed: true,
                duration_ms: 0,
                details: "Build check skipped (no files modified)".to_string(),
                score: 1.0,
            });
        }
        
        let Some(workspace) = &self.workspace else {
            return Ok(StageResult {
                stage: StageType::Build,
                passed: true,
                duration_ms: 0,
                details: "Build check skipped (no workspace)".to_string(),
                score: 1.0,
            });
        };
        
        let build_env = IsolatedBuildEnv::new(workspace.as_ref()).await?;
        
        let build_result = build_env.validate_build().await?;
        
        let passed = build_result.success;
        let score = if passed { 1.0 } else { 0.0 };
        
        let details = if passed {
            format!("Build succeeded ({}ms)", build_result.duration_ms)
        } else {
            format!("Build failed: {}", 
                build_result.stderr.lines().take(3).collect::<Vec<_>>().join("; "))
        };
        
        Ok(StageResult {
            stage: StageType::Build,
            passed,
            duration_ms: build_result.duration_ms,
            details,
            score,
        })
    }
    
    async fn quick_check(&self, response: &merlin_core::Response) -> Result<bool> {
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_build_validation_skip_no_files() {
        let stage = BuildValidationStage::new();
        let response = merlin_core::Response {
            text: "fn main() {}".to_string(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_string(),
            latency_ms: 0,
        };
        let task = Task::new("Test".to_string());
        
        let result = stage.validate(&response, &task).await.unwrap();
        assert!(result.passed);
        assert!(result.details.contains("skipped"));
    }
    
    #[tokio::test]
    async fn test_quick_check() {
        let stage = BuildValidationStage::new();
        
        let good_response = merlin_core::Response {
            text: "fn main() {}".to_string(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_string(),
            latency_ms: 0,
        };
        assert!(stage.quick_check(&good_response).await.unwrap());
        
        let bad_response = merlin_core::Response {
            text: "error[E0425]: cannot find value".to_string(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_string(),
            latency_ms: 0,
        };
        assert!(!stage.quick_check(&bad_response).await.unwrap());
    }
}
