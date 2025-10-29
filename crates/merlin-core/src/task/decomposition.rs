//! Types for task decomposition and step-based execution

use merlin_deps::serde_json::Value as JsonValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent response type - either a direct result or a task list to decompose
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentResponse {
    /// Direct string result - task completed
    DirectResult(String),
    /// Task list requiring decomposition
    TaskList(TaskList),
}

/// Task list with ordered steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskList {
    /// Overall objective of this task list
    pub title: String,
    /// Ordered steps to execute
    pub steps: Vec<TaskStep>,
}

/// Individual step in a task list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// Short step name
    pub title: String,
    /// Detailed description of what to do
    pub description: String,
    /// Type of work for this step
    pub step_type: StepType,
    /// Validation criteria for completion
    pub exit_requirement: ExitRequirement,
    /// Optional context specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextSpec>,
}

/// Type of work being performed in a step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    /// Information gathering, reading files
    Research,
    /// Design and architecture planning
    Planning,
    /// Code writing and modification
    Implementation,
    /// Testing and verification
    Validation,
    /// Documentation and comments
    Documentation,
}

/// Exit requirement for step validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExitRequirement {
    /// Built-in callback function validation
    Callback {
        /// Function name (e.g., `file_exists`)
        function_name: String,
        /// Arguments for the function
        #[serde(default)]
        args: HashMap<String, JsonValue>,
    },
    /// Pattern matching validation
    Pattern {
        /// Regex pattern to match
        pattern: String,
    },
    /// Named validator from pipeline
    Validation {
        /// Validator name
        validator: String,
    },
}

/// Context specification for a step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextSpec {
    /// File patterns to include
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<FilePattern>>,
    /// Indices of previous steps to include results from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_steps: Option<Vec<usize>>,
    /// Explicit content to inject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_content: Option<String>,
}

/// File pattern for context inclusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePattern {
    /// Glob pattern
    pub pattern: String,
    /// Whether to recurse directories
    #[serde(default)]
    pub recursive: bool,
}

/// Error type for exit requirement validation
#[derive(Debug, Clone)]
pub enum ValidationErrorType {
    /// Hard error - requires escalation to higher tier
    Hard(String),
    /// Soft error - can retry with feedback
    Soft(String),
}
