//! Task analysis and complexity estimation.
//!
//! This module provides tools for analyzing user requests, estimating complexity,
//! extracting intent, and decomposing tasks into subtasks.

/// Complexity estimation for tasks
pub mod complexity;
/// Task decomposition into subtasks
pub mod decompose;
/// Intent extraction from user requests
pub mod intent;
/// Local task analyzer implementation
pub mod local;

use crate::{Complexity, Result, TaskAnalysis};
use async_trait::async_trait;

pub use complexity::ComplexityEstimator;
pub use decompose::TaskDecomposer;
pub use intent::{Action, Intent, IntentExtractor, Scope};
pub use local::LocalTaskAnalyzer;

/// Trait for task analysis strategies
#[async_trait]
pub trait TaskAnalyzer: Send + Sync {
    /// Analyze a user request and decompose into tasks
    async fn analyze(&self, request: &str) -> Result<TaskAnalysis>;

    /// Estimate complexity without full analysis (fast path)
    fn estimate_complexity(&self, request: &str) -> Complexity;
}
