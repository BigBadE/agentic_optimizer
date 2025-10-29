//! Validation types and results

use serde::{Deserialize, Serialize};

/// Validation result with pass/fail status and detailed feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed overall
    pub passed: bool,
    /// Overall quality score (0.0 to 1.0)
    pub score: f64,
    /// Validation errors that were found
    pub errors: Vec<ValidationError>,
    /// Non-blocking warnings
    pub warnings: Vec<String>,
    /// Results from individual validation stages
    pub stages: Vec<StageResult>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            passed: true,
            score: 1.0,
            errors: Vec::default(),
            warnings: Vec::default(),
            stages: Vec::default(),
        }
    }
}

/// Validation error from a specific stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Which validation stage produced this error
    pub stage: ValidationStage,
    /// Error message
    pub message: String,
    /// Severity level
    pub severity: Severity,
}

/// Validation stage identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStage {
    /// Syntax validation
    Syntax,
    /// Build validation
    Build,
    /// Test execution
    Test,
    /// Linting checks
    Lint,
}

/// Error severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning that should be addressed
    Warning,
    /// Error that should block acceptance
    Error,
    /// Critical error requiring immediate attention
    Critical,
}

/// Result of a validation stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    /// Which validation stage this result is for
    pub stage: ValidationStage,
    /// Whether this stage passed
    pub passed: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Detailed information about the result
    pub details: String,
    /// Quality score for this stage (0.0 to 1.0)
    pub score: f64,
}
