//! Task analysis and difficulty estimation.
//!
//! This module provides tools for analyzing user requests, estimating difficulty,
//! extracting intent, and decomposing tasks into subtasks.

/// Task decomposition into subtasks
pub mod decompose;
/// Intent extraction from user requests
pub mod intent;
/// Local task analyzer implementation
pub mod local;

use crate::{Result, TaskAnalysis};
use async_trait::async_trait;

pub use decompose::TaskDecomposer;
pub use intent::{Action, Intent, IntentExtractor, Scope};
pub use local::LocalTaskAnalyzer;

/// Trait for task analysis strategies
#[async_trait]
pub trait TaskAnalyzer: Send + Sync {
    /// Analyze a user request and decompose into tasks
    async fn analyze(&self, request: &str) -> Result<TaskAnalysis>;

    /// Estimate difficulty without full analysis (fast path, 1-10 scale)
    fn estimate_difficulty(&self, request: &str) -> u8;
}
