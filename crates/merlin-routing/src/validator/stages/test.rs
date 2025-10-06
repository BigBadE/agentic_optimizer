use async_trait::async_trait;
use crate::{IsolatedBuildEnv, Result, Task, ValidationStageType as StageType, WorkspaceState};
use super::super::pipeline::{StageResult, ValidationStage};
use std::sync::Arc;

/// Test validation using isolated cargo test
pub struct TestValidationStage {
    timeout_seconds: u64,
    workspace: Option<Arc<WorkspaceState>>,
    min_pass_rate: f64,
}

impl TestValidationStage {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            timeout_seconds: 300,
            workspace: None,
            min_pass_rate: 1.0,
        }
    }
    
    #[must_use]
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
    
    #[must_use]
    pub fn with_workspace(mut self, workspace: Arc<WorkspaceState>) -> Self {
        self.workspace = Some(workspace);
        self
    }
    
    #[must_use]
    pub fn with_min_pass_rate(mut self, min_pass_rate: f64) -> Self {
        self.min_pass_rate = min_pass_rate;
        self
    }
}

impl Default for TestValidationStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationStage for TestValidationStage {
    async fn validate(&self, _response: &merlin_core::Response, task: &Task) -> Result<StageResult> {
        if !task.requires_build_check() {
            return Ok(StageResult {
                stage: StageType::Test,
                passed: true,
                duration_ms: 0,
                details: "Test check skipped (no files modified)".to_owned(),
                score: 1.0,
            });
        }
        
        let Some(workspace) = &self.workspace else {
            return Ok(StageResult {
                stage: StageType::Test,
                passed: true,
                duration_ms: 0,
                details: "Test check skipped (no workspace)".to_owned(),
                score: 1.0,
            });
        };
        
        let build_env = IsolatedBuildEnv::new(workspace.as_ref()).await?;
        
        let test_result = build_env.run_tests(self.timeout_seconds).await?;
        
        let total_tests = test_result.passed + test_result.failed;
        let pass_rate = if total_tests > 0 {
            test_result.passed as f64 / total_tests as f64
        } else {
            1.0
        };
        
        let passed = test_result.success && pass_rate >= self.min_pass_rate;
        let score = pass_rate;
        
        let details = if passed {
            format!("Tests passed: {}/{} ({}ms)", 
                test_result.passed, total_tests, test_result.duration_ms)
        } else {
            format!("Tests failed: {}/{} ({:.1}% pass rate)", 
                test_result.failed, total_tests, pass_rate * 100.0)
        };
        
        Ok(StageResult {
            stage: StageType::Test,
            passed,
            duration_ms: test_result.duration_ms,
            details,
            score,
        })
    }
    
    async fn quick_check(&self, response: &merlin_core::Response) -> Result<bool> {
        let has_test_failures = response.text.contains("test result: FAILED")
            || response.text.contains("assertion failed");
        Ok(!has_test_failures)
    }
    
    fn name(&self) -> &'static str {
        "Test"
    }
    
    fn stage_type(&self) -> StageType {
        StageType::Test
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_skip_no_files() {
        let stage = TestValidationStage::new();
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
        let stage = TestValidationStage::new();
        
        let good_response = merlin_core::Response {
            text: "test result: ok. 5 passed".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        assert!(stage.quick_check(&good_response).await.unwrap());
        
        let bad_response = merlin_core::Response {
            text: "test result: FAILED. 2 passed; 3 failed".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        assert!(!stage.quick_check(&bad_response).await.unwrap());
    }
}

