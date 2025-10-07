pub mod complexity;
pub mod decompose;
pub mod intent;
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
