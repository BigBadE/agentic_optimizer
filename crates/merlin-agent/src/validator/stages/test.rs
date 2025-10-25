use async_trait::async_trait;

use merlin_core::Response;

use super::super::pipeline::{StageResult, ValidationStage};
use crate::{IsolatedBuildEnv, WorkspaceState};
use merlin_core::{Result, Task, ValidationStageType as StageType};
use std::sync::Arc;

/// Test validation using isolated cargo test
pub struct TestValidationStage {
    /// Maximum time to allow tests to run before timing out (seconds)
    timeout_seconds: u64,
    /// Optional workspace state used to run tests in isolation
    workspace: Option<Arc<WorkspaceState>>,
    /// Minimum pass rate required to consider the test stage passed (0.0-1.0)
    min_pass_rate: f64,
}

impl TestValidationStage {
    /// Set timeout for test validation
    #[must_use]
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Set workspace for test validation
    #[must_use]
    pub fn with_workspace(mut self, workspace: Arc<WorkspaceState>) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Set minimum pass rate
    #[must_use]
    pub fn with_min_pass_rate(mut self, min_pass_rate: f64) -> Self {
        self.min_pass_rate = min_pass_rate;
        self
    }
}

impl Default for TestValidationStage {
    fn default() -> Self {
        Self {
            timeout_seconds: 300,
            workspace: None,
            min_pass_rate: 1.0,
        }
    }
}

#[async_trait]
impl ValidationStage for TestValidationStage {
    async fn validate(&self, _response: &Response, task: &Task) -> Result<StageResult> {
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

        let build_env = IsolatedBuildEnv::new(workspace.as_ref())?;

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
            format!(
                "Tests passed: {}/{} ({}ms)",
                test_result.passed, total_tests, test_result.duration_ms
            )
        } else {
            format!(
                "Tests failed: {}/{} ({:.1}% pass rate)",
                test_result.failed,
                total_tests,
                pass_rate * 100.0
            )
        };

        Ok(StageResult {
            stage: StageType::Test,
            passed,
            duration_ms: test_result.duration_ms,
            details,
            score,
        })
    }

    async fn quick_check(&self, response: &Response) -> Result<bool> {
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
mod tests {}
