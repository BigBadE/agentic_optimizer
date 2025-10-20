//! Execution result types for the TypeScript-based agent system.

use serde::{Deserialize, Serialize};

#[cfg(test)]
use serde_json::{from_str, to_string};

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
mod tests {
    use super::*;

    #[test]
    fn test_done_result() {
        let result = AgentExecutionResult::done("Task completed".to_owned());
        assert!(result.is_done());
        assert_eq!(result.get_result(), Some("Task completed"));
        assert_eq!(result.get_next_task(), None);
    }

    #[test]
    fn test_continue_result() {
        let result = AgentExecutionResult::continue_with("Check error logs".to_owned());
        assert!(!result.is_done());
        assert_eq!(result.get_result(), None);
        assert_eq!(result.get_next_task(), Some("Check error logs"));
    }

    #[test]
    fn test_done_serialization() {
        let result = AgentExecutionResult::done("All tests pass".to_owned());
        let json = to_string(&result).unwrap();
        assert!(json.contains(r#""done":"true"#));
        assert!(json.contains(r#""result":"All tests pass"#));
    }

    #[test]
    fn test_continue_serialization() {
        let result = AgentExecutionResult::continue_with("Run cargo build".to_owned());
        let json = to_string(&result).unwrap();
        assert!(json.contains(r#""done":"false"#));
        assert!(json.contains(r#""continue":"Run cargo build"#));
    }

    #[test]
    fn test_done_deserialization() {
        let json = r#"{"done":"true","result":"Success"}"#;
        let result: AgentExecutionResult = from_str(json).unwrap();
        assert!(result.is_done());
        assert_eq!(result.get_result(), Some("Success"));
    }

    #[test]
    fn test_continue_deserialization() {
        let json = r#"{"done":"false","continue":"Next step"}"#;
        let result: AgentExecutionResult = from_str(json).unwrap();
        assert!(!result.is_done());
        assert_eq!(result.get_next_task(), Some("Next step"));
    }
}
