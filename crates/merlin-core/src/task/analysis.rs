//! Task analysis and execution strategy types

use serde::{Deserialize, Serialize};

use super::core::Task;

/// Analysis result containing decomposed tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    /// Decomposed tasks to be executed
    pub tasks: Vec<Task>,
    /// Strategy for executing the tasks
    pub execution_strategy: ExecutionStrategy,
}

/// Execution strategy for tasks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    /// Execute tasks one after another
    Sequential,
    /// Execute tasks in parallel up to max concurrent limit
    Parallel {
        /// Maximum number of concurrent tasks
        max_concurrent: usize,
    },
    /// Execute tasks in pipeline fashion
    Pipeline,
}

impl Default for ExecutionStrategy {
    fn default() -> Self {
        Self::Parallel { max_concurrent: 4 }
    }
}
