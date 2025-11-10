//! Core task types and basic structures

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::conversation::Subtask;

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(Uuid);

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
    /// Difficulty rating from 1 (easiest) to 10 (hardest)
    pub difficulty: u8,
    /// Priority for scheduling
    pub priority: Priority,
    /// Other tasks that must complete before this one
    pub dependencies: Vec<TaskId>,
    /// Context and resource requirements
    pub context_needs: ContextRequirements,

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
            difficulty: 5,
            priority: Priority::Medium,
            dependencies: Vec::default(),
            context_needs: ContextRequirements::default(),
            state: TaskState::Created,
            decision_history: Vec::default(),
        }
    }

    /// Creates a task with a pre-existing ID (for UI synchronization).
    pub fn from_id(id: TaskId, description: String) -> Self {
        Self {
            id,
            description,
            difficulty: 5,
            priority: Priority::Medium,
            dependencies: Vec::default(),
            context_needs: ContextRequirements::default(),
            state: TaskState::Created,
            decision_history: Vec::default(),
        }
    }

    /// Sets the difficulty level (1-10).
    ///
    /// # Panics
    /// Panics if difficulty is not in range 1-10.
    #[must_use]
    pub fn with_difficulty(mut self, difficulty: u8) -> Self {
        assert!(
            (1..=10).contains(&difficulty),
            "Difficulty must be between 1 and 10, got {difficulty}"
        );
        self.difficulty = difficulty;
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

/// Task lifecycle state.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    /// Task has been created but not started
    #[default]
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
        subtasks: Vec<Subtask>,
        /// How to execute the subtasks
        execution_mode: ExecutionMode,
    },

    /// Need more information before proceeding
    GatherContext {
        /// What information is needed
        needs: Vec<String>,
    },
}

/// How to execute multiple subtasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Execute subtasks one after another
    Sequential,
    /// Execute subtasks in parallel
    Parallel,
}
