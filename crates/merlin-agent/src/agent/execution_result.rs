//! Execution result types for the TypeScript-based agent system.

use serde::{Deserialize, Serialize};

/// Result of executing agent TypeScript code
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "done", rename_all = "camelCase")]
pub enum AgentExecutionResult {
    /// Agent has completed the task
    #[serde(rename = "true")]
    Done {
        /// Final result to show the user
        result: String,
    },
    /// Agent wants to continue with another task
    #[serde(rename = "false")]
    Continue {
        /// Description of the next step/task to execute
        #[serde(rename = "continue")]
        next_task: String,
    },
}

impl AgentExecutionResult {
    /// Create a Done result
    pub fn done(result: String) -> Self {
        Self::Done { result }
    }

    /// Create a Continue result
    pub fn continue_with(next_task: String) -> Self {
        Self::Continue { next_task }
    }

    /// Check if this is a Done result
    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done { .. })
    }

    /// Get the final result if this is Done
    pub fn get_result(&self) -> Option<&str> {
        match self {
            Self::Done { result } => Some(result.as_str()),
            Self::Continue { .. } => None,
        }
    }

    /// Get the next task if this is Continue
    pub fn get_next_task(&self) -> Option<&str> {
        match self {
            Self::Done { .. } => None,
            Self::Continue { next_task } => Some(next_task.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {}
