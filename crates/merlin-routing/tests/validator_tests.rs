//! Comprehensive tests for validation pipeline and stages
#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::min_ident_chars,
    clippy::absolute_paths,
    clippy::single_call_fn,
    clippy::float_cmp,
    clippy::unused_trait_names,
    reason = "Test code prioritizes clarity and uses traits anonymously"
)]

use async_trait::async_trait;
use merlin_core::{Response, TokenUsage};
use merlin_routing::{
    Result, Severity, Task, ValidationStageType as StageType,
    validator::{ValidationPipeline, ValidationStage, Validator, pipeline::StageResult},
};
use std::sync::Arc;

// Mock validation stage for testing
struct MockValidationStage {
    name: &'static str,
    stage_type: StageType,
    should_pass: bool,
    score: f64,
    quick_check_result: bool,
}

impl MockValidationStage {
    fn new(name: &'static str, stage_type: StageType, should_pass: bool) -> Self {
        Self {
            name,
            stage_type,
            should_pass,
            score: if should_pass { 1.0 } else { 0.5 },
            quick_check_result: should_pass,
        }
    }

    fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }

    fn with_quick_check(mut self, result: bool) -> Self {
        self.quick_check_result = result;
        self
    }
}

#[async_trait]
impl ValidationStage for MockValidationStage {
    async fn validate(&self, _response: &Response, _task: &Task) -> Result<StageResult> {
        Ok(StageResult {
            stage: self.stage_type,
            passed: self.should_pass,
            duration_ms: 50,
            details: {
                let status = if self.should_pass { "passed" } else { "failed" };
                format!("{} validation {status}", self.name)
            },
            score: self.score,
        })
    }

    async fn quick_check(&self, _response: &Response) -> Result<bool> {
        Ok(self.quick_check_result)
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn stage_type(&self) -> StageType {
        self.stage_type
    }
}

fn create_test_response() -> Response {
    Response {
        text: "test response".to_owned(),
        confidence: 0.95,
        tokens_used: TokenUsage::default(),
        provider: "test".to_owned(),
        latency_ms: 100,
    }
}

#[tokio::test]
/// # Panics
/// Panics if pipeline with all passing stages fails.
async fn test_pipeline_all_stages_pass() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(MockValidationStage::new("Syntax", StageType::Syntax, true)),
        Arc::new(MockValidationStage::new("Build", StageType::Build, true)),
        Arc::new(MockValidationStage::new("Test", StageType::Test, true)),
        Arc::new(MockValidationStage::new("Lint", StageType::Lint, true)),
    ];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    assert!(result.passed, "All stages passed, result should be passing");
    assert_eq!(result.stages.len(), 4, "Should run all 4 stages");
    assert_eq!(result.score, 1.0, "Perfect score when all pass");
    assert!(result.errors.is_empty(), "No errors when all pass");
}

#[tokio::test]
/// # Panics
/// Panics if pipeline early exit doesn't work correctly.
async fn test_pipeline_early_exit_on_failure() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(MockValidationStage::new("Syntax", StageType::Syntax, true)),
        Arc::new(MockValidationStage::new("Build", StageType::Build, false)),
        Arc::new(MockValidationStage::new("Test", StageType::Test, true)),
        Arc::new(MockValidationStage::new("Lint", StageType::Lint, true)),
    ];

    let pipeline = ValidationPipeline::new(stages).with_early_exit(true);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    assert!(!result.passed, "Should fail when a stage fails");
    assert_eq!(
        result.stages.len(),
        2,
        "Should exit after first failure (2 stages)"
    );
    assert!(
        !result.errors.is_empty(),
        "Should have error for failed stage"
    );
}

#[tokio::test]
/// # Panics
/// Panics if pipeline without early exit doesn't run all stages.
async fn test_pipeline_no_early_exit() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(MockValidationStage::new("Syntax", StageType::Syntax, false)),
        Arc::new(MockValidationStage::new("Build", StageType::Build, false)),
        Arc::new(MockValidationStage::new("Test", StageType::Test, true)),
        Arc::new(MockValidationStage::new("Lint", StageType::Lint, true)),
    ];

    let pipeline = ValidationPipeline::new(stages).with_early_exit(false);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    assert!(!result.passed, "Should fail when stages fail");
    assert_eq!(
        result.stages.len(),
        4,
        "Should run all 4 stages even with failures"
    );
    assert_eq!(
        result.errors.len(),
        2,
        "Should have 2 errors (2 failed stages)"
    );
}

#[tokio::test]
/// # Panics
/// Panics if score calculation is incorrect.
async fn test_pipeline_score_calculation() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(MockValidationStage::new("Syntax", StageType::Syntax, true).with_score(1.0)),
        Arc::new(MockValidationStage::new("Build", StageType::Build, true).with_score(0.9)),
        Arc::new(MockValidationStage::new("Test", StageType::Test, true).with_score(0.8)),
    ];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    // Score should be product of all stage scores: 1.0 * 0.9 * 0.8 = 0.72
    assert!((result.score - 0.72).abs() < 0.01, "Score should be ~0.72");
}

#[tokio::test]
/// # Panics
/// Panics if quick validation doesn't use first stage.
async fn test_pipeline_quick_validate() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(
            MockValidationStage::new("Syntax", StageType::Syntax, true).with_quick_check(true),
        ),
        Arc::new(MockValidationStage::new("Build", StageType::Build, false)),
    ];

    let pipeline = ValidationPipeline::new(stages);
    let response = create_test_response();

    let result = pipeline
        .quick_validate(&response)
        .await
        .expect("quick validation should succeed");

    assert!(result, "Quick check should pass based on first stage");
}

#[tokio::test]
/// # Panics
/// Panics if quick validation with failing first stage doesn't fail.
async fn test_pipeline_quick_validate_fails() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(
            MockValidationStage::new("Syntax", StageType::Syntax, false).with_quick_check(false),
        ),
        Arc::new(MockValidationStage::new("Build", StageType::Build, true)),
    ];

    let pipeline = ValidationPipeline::new(stages);
    let response = create_test_response();

    let result = pipeline
        .quick_validate(&response)
        .await
        .expect("quick validation should succeed");

    assert!(!result, "Quick check should fail based on first stage");
}

#[tokio::test]
/// # Panics
/// Panics if empty pipeline doesn't handle gracefully.
async fn test_pipeline_empty_stages() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    assert!(result.passed, "Empty pipeline should pass");
    assert_eq!(result.stages.len(), 0, "No stages should run");
    assert_eq!(
        result.score, 1.0,
        "Empty pipeline should have perfect score"
    );
}

#[tokio::test]
/// # Panics
/// Panics if single stage pipeline doesn't work.
async fn test_pipeline_single_stage() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![Arc::new(MockValidationStage::new(
        "Syntax",
        StageType::Syntax,
        true,
    ))];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    assert!(result.passed);
    assert_eq!(result.stages.len(), 1);
}

#[tokio::test]
/// # Panics
/// Panics if validation result structure is incorrect.
async fn test_validation_result_structure() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![Arc::new(MockValidationStage::new(
        "Syntax",
        StageType::Syntax,
        false,
    ))];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    // Check structure
    assert!(!result.passed);
    assert_eq!(result.stages.len(), 1);
    assert_eq!(result.errors.len(), 1);

    let error = &result.errors[0];
    assert_eq!(error.stage, StageType::Syntax);
    assert_eq!(error.severity, Severity::Error);
    assert!(!error.message.is_empty());
}

#[tokio::test]
/// # Panics
/// Panics if stage details aren't preserved.
async fn test_stage_details_preservation() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![Arc::new(MockValidationStage::new(
        "CustomStage",
        StageType::Syntax,
        true,
    ))];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    assert_eq!(result.stages.len(), 1);
    let stage_result = &result.stages[0];

    assert!(stage_result.passed);
    assert!(stage_result.details.contains("CustomStage"));
    assert!(stage_result.details.contains("passed"));
}

#[tokio::test]
/// # Panics
/// Panics if default pipeline creation fails.
async fn test_default_pipeline_creation() {
    let pipeline = ValidationPipeline::with_default_stages();
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    // Should not panic
    let _result = pipeline.validate(&response, &task).await;
}

#[tokio::test]
/// # Panics
/// Panics if mixed stage results aren't handled correctly.
async fn test_mixed_stage_results() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(MockValidationStage::new("Stage1", StageType::Syntax, true).with_score(1.0)),
        Arc::new(MockValidationStage::new("Stage2", StageType::Build, true).with_score(0.95)),
        Arc::new(MockValidationStage::new("Stage3", StageType::Test, true).with_score(0.85)),
        Arc::new(MockValidationStage::new("Stage4", StageType::Lint, true).with_score(0.75)),
    ];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    assert!(result.passed, "All stages passed");
    assert!(result.score < 1.0, "Score should be reduced");
    assert!(result.score > 0.6, "Score shouldn't be too low");
}

#[tokio::test]
/// # Panics
/// Panics if warnings tracking fails.
async fn test_validation_warnings() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![Arc::new(MockValidationStage::new(
        "Syntax",
        StageType::Syntax,
        true,
    ))];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    // Default mock stages don't generate warnings, just verify structure
    assert!(result.warnings.is_empty());
}

#[tokio::test]
/// # Panics
/// Panics if concurrent validation fails.
async fn test_concurrent_validation() {
    let pipeline = Arc::new(ValidationPipeline::with_default_stages());

    let mut handles = vec![];

    for i in 0..10 {
        let pipeline_clone = Arc::clone(&pipeline);
        let handle = tokio::spawn(async move {
            let task = Task::new(format!("Task {i}"));
            let response = create_test_response();
            pipeline_clone.validate(&response, &task).await
        });
        handles.push(handle);
    }

    for handle in handles {
        drop(handle.await.expect("task should complete"));
    }
}

#[test]
/// # Panics
/// Panics if stage type enum doesn't work correctly.
fn test_stage_type_enum() {
    let syntax = StageType::Syntax;
    let build = StageType::Build;
    let test = StageType::Test;
    let lint = StageType::Lint;

    assert_ne!(syntax, build);
    assert_ne!(build, test);
    assert_ne!(test, lint);
}

#[test]
/// # Panics
/// Panics if severity enum doesn't work correctly.
fn test_severity_enum() {
    let error = Severity::Error;
    let warning = Severity::Warning;

    assert_ne!(error, warning);
}

#[tokio::test]
/// # Panics
/// Panics if large response validation fails.
async fn test_large_response_validation() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![Arc::new(MockValidationStage::new(
        "Syntax",
        StageType::Syntax,
        true,
    ))];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let large_response = Response {
        text: "x".repeat(100_000),
        confidence: 0.95,
        tokens_used: TokenUsage::default(),
        provider: "test".to_owned(),
        latency_ms: 100,
    };

    let _result = pipeline
        .validate(&large_response, &task)
        .await
        .expect("validation should handle large responses");
}

#[tokio::test]
/// # Panics
/// Panics if empty response validation fails.
async fn test_empty_response_validation() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![Arc::new(MockValidationStage::new(
        "Syntax",
        StageType::Syntax,
        true,
    ))];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let empty_response = Response {
        text: String::new(),
        confidence: 0.0,
        tokens_used: TokenUsage::default(),
        provider: "test".to_owned(),
        latency_ms: 0,
    };

    let _result = pipeline
        .validate(&empty_response, &task)
        .await
        .expect("validation should handle empty responses");
}

#[tokio::test]
/// # Panics
/// Panics if stage ordering isn't preserved.
async fn test_stage_execution_order() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(MockValidationStage::new("First", StageType::Syntax, true)),
        Arc::new(MockValidationStage::new("Second", StageType::Build, true)),
        Arc::new(MockValidationStage::new("Third", StageType::Test, true)),
    ];

    let pipeline = ValidationPipeline::new(stages);
    let task = Task::new("Test task".to_owned());
    let response = create_test_response();

    let result = pipeline
        .validate(&response, &task)
        .await
        .expect("validation should succeed");

    assert_eq!(result.stages.len(), 3);
    assert_eq!(result.stages[0].stage, StageType::Syntax);
    assert_eq!(result.stages[1].stage, StageType::Build);
    assert_eq!(result.stages[2].stage, StageType::Test);
}
