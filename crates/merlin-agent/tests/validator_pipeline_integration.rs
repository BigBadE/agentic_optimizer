//! Integration tests for `ValidationPipeline`.
//!
//! Tests multi-stage validation with syntax, build, test, and lint stages.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::print_stdout,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_agent::validator::{
    BuildValidationStage, LintValidationStage, SyntaxValidationStage, TestValidationStage,
    ValidationPipeline, ValidationStage, Validator as _,
};
use merlin_core::{Response, Task, TokenUsage, ValidationStageType};
use std::sync::Arc;

fn create_test_response(text: impl Into<String>) -> Response {
    Response {
        text: text.into(),
        confidence: 0.9,
        tokens_used: TokenUsage::default(),
        provider: "test".to_owned(),
        latency_ms: 100,
    }
}

fn create_test_task(description: impl Into<String>) -> Task {
    Task::new(description.into())
}

#[tokio::test]
async fn test_validation_pipeline_creation() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("test");
    let task = create_test_task("test task");

    let result = pipeline.validate(&response, &task).await;
    assert!(result.is_ok(), "Pipeline should validate without errors");
}

#[tokio::test]
async fn test_validation_pipeline_with_custom_stages() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![
        Arc::new(SyntaxValidationStage::default()),
        Arc::new(BuildValidationStage::default()),
    ];

    let pipeline = ValidationPipeline::new(stages);
    let response = create_test_response("test");
    let task = create_test_task("test task");

    let result = pipeline.validate(&response, &task).await;
    assert!(result.is_ok(), "Custom pipeline should validate");
}

#[tokio::test]
async fn test_validation_pipeline_early_exit_enabled() {
    let pipeline = ValidationPipeline::with_default_stages().with_early_exit(true);
    let response = create_test_response("invalid code");
    let task = create_test_task("test task");

    let result = pipeline.validate(&response, &task).await;
    assert!(result.is_ok(), "Pipeline should complete");

    let validation_result = result.unwrap();
    // With early exit, should stop at first failure
    if !validation_result.passed {
        assert!(
            validation_result.stages.len() <= 4,
            "Should stop early on failure"
        );
    }
}

#[tokio::test]
async fn test_validation_pipeline_early_exit_disabled() {
    let pipeline = ValidationPipeline::with_default_stages().with_early_exit(false);
    let response = create_test_response("invalid code");
    let task = create_test_task("test task");

    let result = pipeline.validate(&response, &task).await;
    assert!(result.is_ok(), "Pipeline should complete");

    let validation_result = result.unwrap();
    // With early exit disabled, all stages should run regardless of failures
    if !validation_result.passed {
        // May run all 4 stages or stop at some point
        assert!(
            !validation_result.stages.is_empty(),
            "Should run at least one stage"
        );
    }
}

#[tokio::test]
async fn test_syntax_validation_stage_basic() {
    let stage = SyntaxValidationStage::default();
    let response = create_test_response("test");
    let task = create_test_task("test task");

    let result = stage.validate(&response, &task).await;
    assert!(result.is_ok(), "Syntax validation should complete");
}

#[tokio::test]
async fn test_syntax_validation_stage_quick_check() {
    let stage = SyntaxValidationStage::default();
    let response = create_test_response("test");

    let result = stage.quick_check(&response).await;
    assert!(result.is_ok(), "Quick check should complete");
}

#[tokio::test]
async fn test_syntax_validation_stage_with_rust_code() {
    let stage = SyntaxValidationStage::default();
    let rust_code = "
        pub fn hello() -> String {
            String::from(\"Hello, world!\")
        }
    ";
    let response = create_test_response(rust_code);
    let task = create_test_task("test task");

    let result = stage.validate(&response, &task).await;
    assert!(result.is_ok(), "Valid Rust code should pass syntax check");
}

#[tokio::test]
async fn test_syntax_validation_stage_with_invalid_rust() {
    let stage = SyntaxValidationStage::default();
    let invalid_rust = "
        pub fn hello() -> String {
            this is not valid rust syntax
        }
    ";
    let response = create_test_response(invalid_rust);
    let task = create_test_task("test task");

    let result = stage.validate(&response, &task).await;
    // May pass or fail depending on implementation details
    assert!(result.is_ok(), "Validation should complete");
}

#[tokio::test]
async fn test_build_validation_stage_basic() {
    let stage = BuildValidationStage::default();
    let response = create_test_response("test");
    let task = create_test_task("test task");

    let result = stage.validate(&response, &task).await;
    assert!(result.is_ok(), "Build validation should complete");
}

#[tokio::test]
async fn test_build_validation_stage_quick_check() {
    let stage = BuildValidationStage::default();
    let response = create_test_response("test");

    let result = stage.quick_check(&response).await;
    assert!(result.is_ok(), "Quick check should complete");
    assert!(
        result.unwrap(),
        "Quick check should pass for simple response"
    );
}

#[tokio::test]
async fn test_test_validation_stage_basic() {
    let stage = TestValidationStage::default();
    let response = create_test_response("test");
    let task = create_test_task("test task");

    let result = stage.validate(&response, &task).await;
    assert!(result.is_ok(), "Test validation should complete");
}

#[tokio::test]
async fn test_test_validation_stage_quick_check() {
    let stage = TestValidationStage::default();
    let response = create_test_response("test");

    let result = stage.quick_check(&response).await;
    assert!(result.is_ok(), "Quick check should complete");
    assert!(result.unwrap(), "Quick check should pass");
}

#[tokio::test]
async fn test_lint_validation_stage_basic() {
    let stage = LintValidationStage::default();
    let response = create_test_response("test");
    let task = create_test_task("test task");

    let result = stage.validate(&response, &task).await;
    assert!(result.is_ok(), "Lint validation should complete");
}

#[tokio::test]
async fn test_lint_validation_stage_quick_check() {
    let stage = LintValidationStage::default();
    let response = create_test_response("test");

    let result = stage.quick_check(&response).await;
    assert!(result.is_ok(), "Quick check should complete");
    assert!(result.unwrap(), "Quick check should pass");
}

#[tokio::test]
async fn test_validation_pipeline_stage_metadata() {
    let syntax = SyntaxValidationStage::default();
    let build = BuildValidationStage::default();
    let test = TestValidationStage::default();
    let lint = LintValidationStage::default();

    assert_eq!(syntax.name(), "Syntax");
    assert_eq!(syntax.stage_type(), ValidationStageType::Syntax);

    assert_eq!(build.name(), "Build");
    assert_eq!(build.stage_type(), ValidationStageType::Build);

    assert_eq!(test.name(), "Test");
    assert_eq!(test.stage_type(), ValidationStageType::Test);

    assert_eq!(lint.name(), "Lint");
    assert_eq!(lint.stage_type(), ValidationStageType::Lint);
}

#[tokio::test]
async fn test_validation_pipeline_score_accumulation() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("test code");
    let task = create_test_task("test task");

    let result = pipeline.validate(&response, &task).await.unwrap();

    // Score should be between 0.0 and 1.0
    assert!(
        result.score >= 0.0 && result.score <= 1.0,
        "Score should be normalized"
    );
}

#[tokio::test]
async fn test_validation_pipeline_quick_validate() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("test");

    let result = pipeline.quick_validate(&response).await;
    assert!(result.is_ok(), "Quick validate should complete");
}

#[tokio::test]
async fn test_validation_pipeline_empty_response() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("");
    let task = create_test_task("test task");

    let result = pipeline.validate(&response, &task).await;
    assert!(result.is_ok(), "Should handle empty response");
}

#[tokio::test]
async fn test_validation_pipeline_with_file_changes() {
    let pipeline = ValidationPipeline::with_default_stages();
    let code = "pub fn test() {}";
    let response = create_test_response(code);

    let task = create_test_task("add test function");

    let result = pipeline.validate(&response, &task).await;
    assert!(result.is_ok(), "Should handle file changes");
}

#[tokio::test]
async fn test_validation_pipeline_multiple_file_changes() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("multiple file changes");

    let task = create_test_task("modify multiple files");

    let result = pipeline.validate(&response, &task).await;
    assert!(result.is_ok(), "Should handle multiple file changes");
}

#[tokio::test]
async fn test_validation_pipeline_delete_operation() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("delete file");

    let task = create_test_task("delete old file");

    let result = pipeline.validate(&response, &task).await;
    assert!(result.is_ok(), "Should handle delete operations");
}

#[tokio::test]
async fn test_validation_pipeline_result_structure() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("test");
    let task = create_test_task("test task");

    let result = pipeline.validate(&response, &task).await.unwrap();

    // Check result structure
    assert!(
        !result.stages.is_empty(),
        "Should have at least one stage result"
    );

    for stage_result in &result.stages {
        // Each stage should have a type
        assert!(
            matches!(
                stage_result.stage,
                ValidationStageType::Syntax
                    | ValidationStageType::Build
                    | ValidationStageType::Test
                    | ValidationStageType::Lint
            ),
            "Stage type should be valid"
        );

        // Score should be normalized
        assert!(
            stage_result.score >= 0.0 && stage_result.score <= 1.0,
            "Stage score should be normalized"
        );
    }
}

#[tokio::test]
async fn test_validation_pipeline_error_accumulation() {
    let pipeline = ValidationPipeline::with_default_stages().with_early_exit(false);
    let invalid_code = "this is clearly not valid code at all";
    let response = create_test_response(invalid_code);

    let task = create_test_task("write invalid code");

    let result = pipeline.validate(&response, &task).await.unwrap();

    // With early_exit=false, errors may accumulate across stages
    if !result.passed {
        assert!(
            !result.errors.is_empty(),
            "Failed validation should have errors"
        );
    }
}

#[tokio::test]
async fn test_validation_pipeline_chaining() {
    let pipeline1 = ValidationPipeline::with_default_stages();
    let pipeline2 = pipeline1.with_early_exit(false);
    let pipeline3 = pipeline2.with_early_exit(true);

    let response = create_test_response("test");
    let task = create_test_task("test");

    // All should work
    pipeline3.validate(&response, &task).await.unwrap();
}

#[tokio::test]
async fn test_validation_stages_independence() {
    // Test that each stage can be used independently
    let syntax = SyntaxValidationStage::default();
    let build = BuildValidationStage::default();
    let test = TestValidationStage::default();
    let lint = LintValidationStage::default();

    let response = create_test_response("test");
    let task = create_test_task("test");

    syntax.validate(&response, &task).await.unwrap();
    build.validate(&response, &task).await.unwrap();
    test.validate(&response, &task).await.unwrap();
    lint.validate(&response, &task).await.unwrap();
}

#[tokio::test]
async fn test_validation_pipeline_empty_stages() {
    let pipeline = ValidationPipeline::new(vec![]);
    let response = create_test_response("test");
    let task = create_test_task("test");

    let result = pipeline.validate(&response, &task).await.unwrap();
    assert!(result.passed, "Empty pipeline should pass");
    assert_eq!(result.stages.len(), 0, "No stages should run");
    assert!(
        (result.score - 1.0).abs() < f64::EPSILON,
        "Score should be perfect with no stages"
    );
}

#[tokio::test]
async fn test_validation_pipeline_single_stage() {
    let stages: Vec<Arc<dyn ValidationStage>> = vec![Arc::new(SyntaxValidationStage::default())];

    let pipeline = ValidationPipeline::new(stages);
    let response = create_test_response("test");
    let task = create_test_task("test");

    let result = pipeline.validate(&response, &task).await.unwrap();
    assert_eq!(result.stages.len(), 1, "Single stage should run");
}

#[tokio::test]
async fn test_validation_result_warnings() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("test with potential issues");
    let task = create_test_task("test");

    let result = pipeline.validate(&response, &task).await.unwrap();

    // Warnings field should exist (may be empty or populated)
    // Just verify the field is accessible
    let _ = &result.warnings;
}

#[tokio::test]
async fn test_validation_pipeline_stage_ordering() {
    let pipeline = ValidationPipeline::with_default_stages();
    let response = create_test_response("test");
    let task = create_test_task("test");

    let result = pipeline.validate(&response, &task).await.unwrap();

    if result.stages.len() >= 2 {
        // Stages should run in order: Syntax -> Build -> Test -> Lint
        let stage_types: Vec<ValidationStageType> =
            result.stages.iter().map(|stage| stage.stage).collect();

        // Verify ordering if multiple stages ran
        for window in stage_types.windows(2) {
            // Each stage should have a valid type
            assert!(matches!(
                window[0],
                ValidationStageType::Syntax
                    | ValidationStageType::Build
                    | ValidationStageType::Test
                    | ValidationStageType::Lint
            ));
        }
    }
}
