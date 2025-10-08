//! Core types for tasks, analysis, validation, and execution.
//!
//! This module defines the fundamental types used throughout the routing system,
//! including task representation, complexity/priority levels, validation results,
//! and execution context.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use merlin_core::{Response, TokenUsage};
use uuid::Uuid;

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(Uuid);

impl TaskId {}

impl Default for TaskId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Immutable task representation with metadata and execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task
    pub id: TaskId,
    /// Human-readable description of what this task should accomplish
    pub description: String,
    /// Estimated complexity level
    pub complexity: Complexity,
    /// Priority for scheduling
    pub priority: Priority,
    /// Other tasks that must complete before this one
    pub dependencies: Vec<TaskId>,
    /// Context and resource requirements
    pub context_needs: ContextRequirements,

    // Self-determinations fields
    /// Current execution state (not serialized)
    #[serde(skip)]
    pub state: TaskState,
    /// History of routing/execution decisions (not serialized)
    #[serde(skip)]
    pub decision_history: Vec<TaskDecision>,
}

impl Task {
    /// Creates a new task with the given description and default settings.
    pub fn new(description: String) -> Self {
        Self {
            id: TaskId::default(),
            description,
            complexity: Complexity::Simple,
            priority: Priority::Medium,
            dependencies: Vec::default(),
            context_needs: ContextRequirements::default(),
            state: TaskState::Created,
            decision_history: Vec::default(),
        }
    }

    /// Sets the complexity level.
    #[must_use]
    pub fn with_complexity(mut self, complexity: Complexity) -> Self {
        self.complexity = complexity;
        self
    }

    /// Sets the priority level.
    #[must_use]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Sets task dependencies.
    #[must_use]
    pub fn with_dependencies(mut self, dependencies: Vec<TaskId>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Sets context requirements.
    #[must_use]
    pub fn with_context(mut self, context_needs: ContextRequirements) -> Self {
        self.context_needs = context_needs;
        self
    }

    /// Checks if this task requires build verification.
    pub fn requires_build_check(&self) -> bool {
        !self.context_needs.required_files.is_empty()
    }
}

/// Task complexity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Complexity {
    /// Very simple task, can be handled by smallest model
    Trivial,
    /// Simple task with straightforward logic
    Simple,
    /// Moderate complexity requiring careful thought
    Medium,
    /// Complex task requiring advanced reasoning
    Complex,
}

/// Task priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority, can be deferred
    Low,
    /// Normal priority
    Medium,
    /// High priority, should be expedited
    High,
    /// Critical priority, requires best available model
    Critical,
}

/// Context requirements for a task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextRequirements {
    /// Estimated number of tokens needed for context
    pub estimated_tokens: usize,
    /// Files that must be included in context
    pub required_files: Vec<PathBuf>,
    /// Whether full codebase context is needed
    pub requires_full_context: bool,
}

impl ContextRequirements {
    /// Creates new context requirements with defaults. Use `Default` instead.
    /// Sets required files.
    #[must_use]
    pub fn with_files(mut self, files: Vec<PathBuf>) -> Self {
        self.required_files = files;
        self
    }

    /// Sets estimated token count.
    #[must_use]
    pub fn with_estimated_tokens(mut self, tokens: usize) -> Self {
        self.estimated_tokens = tokens;
        self
    }

    /// Sets whether full context is required.
    #[must_use]
    pub fn with_full_context(mut self, requires_full: bool) -> Self {
        self.requires_full_context = requires_full;
        self
    }
}

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

/// Result of executing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// ID of the task that was executed
    pub task_id: TaskId,
    /// Response from the model
    pub response: Response,
    /// Name of the model tier that was used
    pub tier_used: String,
    /// Token usage statistics
    pub tokens_used: TokenUsage,
    /// Validation results
    pub validation: ValidationResult,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

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

/// File change operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChange {
    /// Create a new file
    Create {
        /// Path to create
        path: PathBuf,
        /// Initial content
        content: String,
    },
    /// Modify an existing file
    Modify {
        /// Path to modify
        path: PathBuf,
        /// New content
        content: String,
    },
    /// Delete a file
    Delete {
        /// Path to delete
        path: PathBuf,
    },
}

impl FileChange {
    /// Gets the path affected by this file change.
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Create { path, .. } | Self::Modify { path, .. } | Self::Delete { path } => path,
        }
    }
}

/// Execution context accumulated across tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Original user request
    pub original_request: String,
    /// Files that have been read during execution
    pub files_read: HashMap<PathBuf, String>,
    /// Files that have been written during execution
    pub files_written: HashMap<PathBuf, String>,
    /// Commands that have been executed
    pub commands_run: Vec<CommandExecution>,
    /// Key findings discovered during execution
    pub findings: Vec<String>,
    /// Errors encountered during execution
    pub errors: Vec<String>,
}

impl ExecutionContext {
    /// Creates a new execution context for the given request.
    pub fn new(original_request: String) -> Self {
        Self {
            original_request,
            files_read: HashMap::default(),
            files_written: HashMap::default(),
            commands_run: Vec::default(),
            findings: Vec::default(),
            errors: Vec::default(),
        }
    }
}

/// Task lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    /// Task has been created but not started
    Created,
    /// Task is performing self-assessment
    Assessing,
    /// Task is executing
    Executing,
    /// Task is waiting for subtasks to complete
    AwaitingSubtasks,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Created
    }
}

/// Decision made by a task during self-assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDecision {
    /// Action to take
    pub action: TaskAction,
    /// Reasoning for this decision
    pub reasoning: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
}

/// Action a task can decide to take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskAction {
    /// Complete the task immediately with this result
    Complete {
        /// Final result
        result: String,
    },

    /// Decompose into subtasks
    Decompose {
        /// Subtasks to spawn
        subtasks: Vec<SubtaskSpec>,
        /// How to execute the subtasks
        execution_mode: ExecutionMode,
    },

    /// Need more information before proceeding
    GatherContext {
        /// What information is needed
        needs: Vec<String>,
    },
}

/// Specification for a subtask to be spawned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskSpec {
    /// Description of what the subtask should do
    pub description: String,
    /// Estimated complexity
    pub complexity: Complexity,
}

/// How to execute multiple subtasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Execute subtasks one after another
    Sequential,
    /// Execute subtasks in parallel
    Parallel,
}

/// Record of a command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    /// Command that was run
    pub command: String,
    /// Output from the command
    pub output: String,
    /// Exit code returned by the command
    pub exit_code: i32,
    /// When the command was executed
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
}
