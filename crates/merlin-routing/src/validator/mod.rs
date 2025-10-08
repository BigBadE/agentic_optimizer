//! Multi-stage validation pipeline for code generation.
//!
//! This module provides a validation framework with multiple stages
//! (syntax, lint, test, build) that can be run sequentially or with early exit.

/// Validation pipeline implementation
pub mod pipeline;
/// Individual validation stages
pub mod stages;

use crate::{Result, Task, ValidationResult};
use async_trait::async_trait;
use merlin_core::Response;

pub use pipeline::{ValidationPipeline, ValidationStage};
pub use stages::{
    BuildValidationStage, LintValidationStage, SyntaxValidationStage, TestValidationStage,
};

/// Trait for validation strategies
#[async_trait]
pub trait Validator: Send + Sync {
    /// Validate a task response
    async fn validate(&self, response: &Response, task: &Task) -> Result<ValidationResult>;

    /// Quick validation (pre-flight check)
    async fn quick_validate(&self, response: &Response) -> Result<bool>;
}
