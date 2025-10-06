use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use merlin_core::{Response, TokenUsage};
use uuid::Uuid;

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(Uuid);

impl TaskId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub description: String,
    pub complexity: Complexity,
    pub priority: Priority,
    pub dependencies: Vec<TaskId>,
    pub context_needs: ContextRequirements,
    
    // Self-determinations fields
    #[serde(skip)]
    pub state: TaskState,
    #[serde(skip)]
    pub decision_history: Vec<TaskDecision>,
}

impl Task {
    #[must_use]
    pub fn new(description: String) -> Self {
        Self {
            id: TaskId::new(),
            description,
            complexity: Complexity::Simple,
            priority: Priority::Medium,
            dependencies: Vec::new(),
            context_needs: ContextRequirements::default(),
            state: TaskState::Created,
            decision_history: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_complexity(mut self, complexity: Complexity) -> Self {
        self.complexity = complexity;
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_dependencies(mut self, dependencies: Vec<TaskId>) -> Self {
        self.dependencies = dependencies;
        self
    }

    #[must_use]
    pub fn with_context(mut self, context_needs: ContextRequirements) -> Self {
        self.context_needs = context_needs;
        self
    }

    #[must_use]
    pub fn requires_build_check(&self) -> bool {
        !self.context_needs.required_files.is_empty()
    }
}

/// Task complexity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Complexity {
    Trivial,
    Simple,
    Medium,
    Complex,
}

/// Task priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Context requirements for a task
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextRequirements {
    pub estimated_tokens: usize,
    pub required_files: Vec<PathBuf>,
    pub requires_full_context: bool,
}

impl ContextRequirements {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_files(mut self, files: Vec<PathBuf>) -> Self {
        self.required_files = files;
        self
    }

    #[must_use]
    pub fn with_estimated_tokens(mut self, tokens: usize) -> Self {
        self.estimated_tokens = tokens;
        self
    }

    #[must_use]
    pub fn with_full_context(mut self, requires_full: bool) -> Self {
        self.requires_full_context = requires_full;
        self
    }
}

/// Analysis result containing decomposed tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    pub tasks: Vec<Task>,
    pub execution_strategy: ExecutionStrategy,
}

/// Execution strategy for tasks
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Sequential,
    Parallel { max_concurrent: usize },
    Pipeline,
}

impl Default for ExecutionStrategy {
    fn default() -> Self {
        Self::Parallel { max_concurrent: 4 }
    }
}

/// Result of executing a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub response: Response,
    pub tier_used: String,
    pub tokens_used: TokenUsage,
    pub validation: ValidationResult,
    pub duration_ms: u64,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub score: f64,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub stages: Vec<StageResult>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            passed: true,
            score: 1.0,
            errors: Vec::new(),
            warnings: Vec::new(),
            stages: Vec::new(),
        }
    }
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub stage: ValidationStage,
    pub message: String,
    pub severity: Severity,
}

/// Validation stage identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStage {
    Syntax,
    Build,
    Test,
    Lint,
}

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Result of a validation stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: ValidationStage,
    pub passed: bool,
    pub duration_ms: u64,
    pub details: String,
    pub score: f64,
}

/// File change operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChange {
    Create { path: PathBuf, content: String },
    Modify { path: PathBuf, content: String },
    Delete { path: PathBuf },
}

impl FileChange {
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Create { path, .. } | Self::Modify { path, .. } | Self::Delete { path } => path,
        }
    }
}

/// Execution context accumulated across tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub original_request: String,
    pub files_read: std::collections::HashMap<PathBuf, String>,
    pub files_written: std::collections::HashMap<PathBuf, String>,
    pub commands_run: Vec<CommandExecution>,
    pub findings: Vec<String>,
    pub errors: Vec<String>,
}

impl ExecutionContext {
    #[must_use]
    pub fn new(original_request: String) -> Self {
        Self {
            original_request,
            files_read: std::collections::HashMap::new(),
            files_written: std::collections::HashMap::new(),
            commands_run: Vec::new(),
            findings: Vec::new(),
            errors: Vec::new(),
        }
    }
}

/// Task lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Created,
    Assessing,
    Executing,
    AwaitingSubtasks,
    Completed,
    Failed,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Created
    }
}

/// Decision made by a task during self-assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDecision {
    pub action: TaskAction,
    pub reasoning: String,
    pub confidence: f32,
}

/// Action a task can decide to take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskAction {
    /// Complete the task immediately with this result
    Complete { result: String },
    
    /// Decompose into subtasks
    Decompose {
        subtasks: Vec<SubtaskSpec>,
        execution_mode: ExecutionMode,
    },
    
    /// Need more information before proceeding
    GatherContext { needs: Vec<String> },
}

/// Specification for a subtask to be spawned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskSpec {
    pub description: String,
    pub complexity: Complexity,
}

/// How to execute multiple subtasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Sequential,
    Parallel,
}

/// Record of a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    pub command: String,
    pub output: String,
    pub exit_code: i32,
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: std::time::Instant,
}
