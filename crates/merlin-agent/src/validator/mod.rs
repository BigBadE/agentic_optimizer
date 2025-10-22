//! Multi-stage validation pipeline for code generation.
//!
//! This module provides a validation framework with multiple stages
//! (syntax, lint, test, build) that can be run sequentially or with early exit.

/// Citation validation for agent responses
pub mod citations;
/// Validation pipeline implementation
pub mod pipeline;
/// Individual validation stages
pub mod stages;

use async_trait::async_trait;
use merlin_core::Response;
use merlin_core::{Result, Task, ValidationResult};

pub use citations::{Citation, CitationStatistics, CitationValidator};
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
