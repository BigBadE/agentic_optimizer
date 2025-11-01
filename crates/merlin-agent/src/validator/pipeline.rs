use super::Validator;
use async_trait::async_trait;
use merlin_core::Response;
use merlin_core::{
    Result, Severity, StageResult as PublicStageResult, Task, ValidationError, ValidationResult,
    ValidationStageType as StageType,
};
use std::sync::Arc;
use std::time::Instant;

/// Individual validation stage trait.
#[async_trait]
pub trait ValidationStage: Send + Sync {
    /// Validates a response against a task.
    ///
    /// # Errors
    /// Returns an error if validation cannot be performed.
    async fn validate(&self, response: &Response, task: &Task) -> Result<StageResult>;

    /// Performs a quick pre-flight check of the response.
    ///
    /// # Errors
    /// Returns an error if the quick check cannot be performed.
    async fn quick_check(&self, response: &Response) -> Result<bool>;

    /// Returns the human-readable name of this stage.
    fn name(&self) -> &'static str;

    /// Returns the stage type identifier.
    fn stage_type(&self) -> StageType;
}

/// Internal validation stage result (different from public `StageResult`).
#[derive(Debug, Clone)]
pub struct StageResult {
    /// Which validation stage this result is for
    pub stage: StageType,
    /// Whether this stage passed
    pub passed: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Detailed information about the result
    pub details: String,
    /// Quality score for this stage (0.0 to 1.0)
    pub score: f64,
}

/// Multi-stage validation pipeline
pub struct ValidationPipeline {
    /// Ordered collection of validation stages to execute
    stages: Vec<Arc<dyn ValidationStage>>,
    /// If true, stops running further stages after the first failure
    early_exit: bool,
}

impl ValidationPipeline {
    /// Creates a new validation pipeline with the given stages.
    pub fn new(stages: Vec<Arc<dyn ValidationStage>>) -> Self {
        Self {
            stages,
            early_exit: true,
        }
    }

    /// Configures whether to exit early on first failure.
    #[must_use]
    pub fn with_early_exit(mut self, early_exit: bool) -> Self {
        self.early_exit = early_exit;
        self
    }

    /// Creates a pipeline with the default validation stages.
    ///
    /// Currently includes only: Syntax validation.
    pub fn with_default_stages() -> Self {
        use super::stages::SyntaxValidationStage;

        let stages: Vec<Arc<dyn ValidationStage>> =
            vec![Arc::new(SyntaxValidationStage::default())];

        Self::new(stages)
    }
}

#[async_trait]
impl Validator for ValidationPipeline {
    async fn validate(&self, response: &Response, task: &Task) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            passed: true,
            score: 1.0,
            errors: Vec::default(),
            warnings: Vec::default(),
            stages: Vec::default(),
        };

        for stage in &self.stages {
            let start = Instant::now();
            let stage_result = stage.validate(response, task).await?;

            result.stages.push(PublicStageResult {
                stage: stage_result.stage,
                passed: stage_result.passed,
                duration_ms: start.elapsed().as_millis() as u64,
                details: stage_result.details.clone(),
                score: stage_result.score,
            });

            result.score *= stage_result.score;
            result.passed &= stage_result.passed;

            if !stage_result.passed {
                result.errors.push(ValidationError {
                    stage: stage_result.stage,
                    message: stage_result.details,
                    severity: Severity::Error,
                });

                if self.early_exit {
                    break;
                }
            }
        }

        Ok(result)
    }

    async fn quick_validate(&self, response: &Response) -> Result<bool> {
        if let Some(syntax_stage) = self.stages.first() {
            syntax_stage.quick_check(response).await
        } else {
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_core::{Response, TokenUsage};
    use merlin_core::{Result, Task, ValidationStageType as StageType};

    struct MockStage {
        name: &'static str,
        should_pass: bool,
    }
    #[async_trait]
    impl ValidationStage for MockStage {
        async fn validate(&self, _response: &Response, _task: &Task) -> Result<StageResult> {
            Ok(StageResult {
                stage: StageType::Syntax,
                passed: self.should_pass,
                duration_ms: 10,
                details: format!("{} result", self.name),
                score: if self.should_pass { 1.0 } else { 0.0 },
            })
        }

        async fn quick_check(&self, _response: &Response) -> Result<bool> {
            Ok(self.should_pass)
        }

        fn name(&self) -> &'static str {
            self.name
        }

        fn stage_type(&self) -> StageType {
            StageType::Syntax
        }
    }

    /// Tests that pipeline passes when all stages pass.
    ///
    /// # Errors
    /// Returns an error if validation fails.
    ///
    /// # Panics
    /// Panics if validation results don't match expected behavior.
    #[tokio::test]
    async fn test_pipeline_all_pass() -> Result<()> {
        let stages: Vec<Arc<dyn ValidationStage>> = vec![
            Arc::new(MockStage {
                name: "Stage1",
                should_pass: true,
            }),
            Arc::new(MockStage {
                name: "Stage2",
                should_pass: true,
            }),
        ];

        let pipeline = ValidationPipeline::new(stages);
        let task = Task::new("Test".to_owned());
        let response = Response {
            text: "test".to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };

        let result = pipeline.validate(&response, &task).await?;
        assert!(result.passed);
        assert_eq!(result.stages.len(), 2);
        Ok(())
    }

    /// Tests that pipeline exits early when a stage fails.
    ///
    /// # Errors
    /// Returns an error if validation fails.
    ///
    /// # Panics
    /// Panics if validation results don't match expected behavior.
    #[tokio::test]
    async fn test_pipeline_early_exit() -> Result<()> {
        let stages: Vec<Arc<dyn ValidationStage>> = vec![
            Arc::new(MockStage {
                name: "Stage1",
                should_pass: false,
            }),
            Arc::new(MockStage {
                name: "Stage2",
                should_pass: true,
            }),
        ];

        let pipeline = ValidationPipeline::new(stages).with_early_exit(true);
        let task = Task::new("Test".to_owned());
        let response = Response {
            text: "test".to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };

        let result = pipeline.validate(&response, &task).await?;
        assert!(!result.passed);
        assert_eq!(result.stages.len(), 1);
        Ok(())
    }
}
