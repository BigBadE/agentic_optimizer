//! Types for task decomposition and step-based execution

use serde::{Deserialize, Serialize};

/// Handle to a JavaScript value stored in the persistent runtime
///
/// This is a lightweight handle that references a JavaScript value
/// kept alive in the TypeScript runtime's storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsValueHandle {
    /// Unique identifier for this value in the runtime's storage
    pub(crate) id: String,
}

impl JsValueHandle {
    /// Create a new handle with the given ID
    #[must_use]
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Get the identifier for this handle
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

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
#[serde(deny_unknown_fields)]
pub struct TaskList {
    /// Overall objective of this task list
    pub title: String,
    /// Ordered steps to execute
    pub steps: Vec<TaskStep>,
}

/// Individual step in a task list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskStep {
    /// Short step name
    pub title: String,
    /// Detailed description of what to do
    pub description: String,
    /// Type of work for this step
    pub step_type: StepType,
    /// Optional JavaScript function reference for validation
    /// Function is kept alive in persistent runtime and called when step completes
    /// Function signature: () => Promise<void>
    /// Throws error if validation fails
    /// Has full access to all tools (readFile, writeFile, bash, etc.)
    /// Can capture variables from outer scope
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_requirement: Option<JsValueHandle>,
    /// Optional context specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextSpec>,
    /// Step titles this step depends on (for parallel execution)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
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
