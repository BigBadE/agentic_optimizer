pub mod pipeline;
pub mod stages;

use async_trait::async_trait;
use crate::{Result, Task, ValidationResult};
use merlin_core::Response;

pub use pipeline::{ValidationPipeline, ValidationStage};
pub use stages::{BuildValidationStage, LintValidationStage, SyntaxValidationStage, TestValidationStage};

/// Trait for validation strategies
#[async_trait]
pub trait Validator: Send + Sync {
    /// Validate a task response
    async fn validate(&self, response: &Response, task: &Task) -> Result<ValidationResult>;
    
    /// Quick validation (pre-flight check)
    async fn quick_validate(&self, response: &Response) -> Result<bool>;
}

