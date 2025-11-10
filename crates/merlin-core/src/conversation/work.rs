//! Work units and subtasks for conversation system.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::ids::{SubtaskId, WorkUnitId};
use crate::{TaskId, TokenUsage};

/// Status of a work unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkStatus {
    /// Work is currently in progress (⏳)
    InProgress,
    /// Work completed successfully (✅)
    Completed,
    /// Work failed (❌)
    Failed,
    /// Work was cancelled by user (⏸️)
    Cancelled,
    /// Work is being retried after failure (🔄)
    Retrying,
}

impl WorkStatus {
    /// Returns the emoji representation of this status
    #[must_use]
    pub const fn emoji(self) -> &'static str {
        match self {
            Self::InProgress => "⏳",
            Self::Completed => "✅",
            Self::Failed => "❌",
            Self::Cancelled => "⏸️",
            Self::Retrying => "🔄",
        }
    }
}

impl fmt::Display for WorkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.emoji())
    }
}

/// Ephemeral work container spawned by a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkUnit {
    /// Unique identifier for this work unit
    pub id: WorkUnitId,
    /// Associated task ID (links to existing task system)
    pub task_id: TaskId,
    /// Current status of the work
    pub status: WorkStatus,
    /// Subtasks decomposed from the original message (empty if single-step)
    pub subtasks: Vec<Subtask>,
    /// Name of the model tier used
    pub tier_used: String,
    /// Token usage statistics
    pub tokens_used: TokenUsage,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Retry count (0 = first attempt, increments on each retry)
    pub retry_count: u32,
}

impl WorkUnit {
    /// Creates a new work unit for a task
    pub fn new(task_id: TaskId, tier_used: String) -> Self {
        Self {
            id: WorkUnitId::new(),
            task_id,
            status: WorkStatus::InProgress,
            subtasks: Vec::new(),
            tier_used,
            tokens_used: TokenUsage::default(),
            duration_ms: 0,
            retry_count: 0,
        }
    }

    /// Adds a subtask to track decomposed work
    pub fn add_subtask(&mut self, description: String, difficulty: u8) -> SubtaskId {
        let subtask = Subtask {
            id: SubtaskId::new(),
            description,
            difficulty,
            status: SubtaskStatus::Pending,
            verification: None,
            error: None,
            result: None,
        };
        let id = subtask.id;
        self.subtasks.push(subtask);
        id
    }

    /// Marks a subtask as in progress by ID
    pub fn start_subtask(&mut self, subtask_id: SubtaskId) {
        if let Some(subtask) = self.subtasks.iter_mut().find(|sub| sub.id == subtask_id) {
            subtask.status = SubtaskStatus::InProgress;
        }
    }

    /// Marks a subtask as completed
    pub fn complete_subtask(&mut self, subtask_id: SubtaskId, result: Option<String>) {
        if let Some(subtask) = self.subtasks.iter_mut().find(|sub| sub.id == subtask_id) {
            tracing::debug!(
                "Completing subtask {:?} ('{}') - previous status: {:?}",
                subtask_id,
                subtask.description,
                subtask.status
            );
            subtask.status = SubtaskStatus::Completed;
            subtask.result = result;
            subtask.error = None;
        } else {
            tracing::warn!(
                "Attempted to complete subtask {:?} but it was not found. Available: {:?}",
                subtask_id,
                self.subtasks
                    .iter()
                    .map(|subtask| (subtask.id, &subtask.description))
                    .collect::<Vec<_>>()
            );
        }
    }

    /// Marks a subtask as failed
    pub fn fail_subtask(&mut self, subtask_id: SubtaskId, error: String) {
        if let Some(subtask) = self.subtasks.iter_mut().find(|sub| sub.id == subtask_id) {
            subtask.status = SubtaskStatus::Failed;
            subtask.error = Some(error);
        }
    }

    /// Returns the next pending subtask
    #[must_use]
    pub fn next_pending_subtask(&self) -> Option<&Subtask> {
        self.subtasks
            .iter()
            .find(|sub| matches!(sub.status, SubtaskStatus::Pending))
    }

    /// Returns progress percentage (0-100) based on completed subtasks
    #[must_use]
    pub fn progress_percentage(&self) -> u8 {
        if self.subtasks.is_empty() {
            return match self.status {
                WorkStatus::Completed => 100,
                WorkStatus::InProgress | WorkStatus::Retrying => 50,
                WorkStatus::Failed | WorkStatus::Cancelled => 0,
            };
        }

        let completed = self
            .subtasks
            .iter()
            .filter(|sub| matches!(sub.status, SubtaskStatus::Completed))
            .count();

        ((completed as f64 / self.subtasks.len() as f64) * 100.0) as u8
    }

    /// Marks the work as completed if all subtasks are already completed
    pub fn complete(&mut self) {
        // Only mark as completed if all subtasks are completed
        let all_completed = self
            .subtasks
            .iter()
            .all(|sub| matches!(sub.status, SubtaskStatus::Completed));

        if all_completed {
            self.status = WorkStatus::Completed;
        } else {
            // If not all subtasks are completed, keep status as InProgress
            // This can happen during async execution when complete() is called before all subtasks finish
            tracing::debug!(
                "WorkUnit.complete() called but only {}/{} subtasks are completed",
                self.subtasks
                    .iter()
                    .filter(|subtask| matches!(subtask.status, SubtaskStatus::Completed))
                    .count(),
                self.subtasks.len()
            );
        }
    }

    /// Marks the work as failed
    pub fn fail(&mut self) {
        self.status = WorkStatus::Failed;
    }

    /// Marks the work as cancelled by user
    pub fn cancel(&mut self) {
        self.status = WorkStatus::Cancelled;
    }

    /// Increments retry count and marks as retrying
    pub fn retry(&mut self) {
        self.retry_count += 1;
        self.status = WorkStatus::Retrying;
    }

    /// Returns true if the work is in a terminal state
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            WorkStatus::Completed | WorkStatus::Failed | WorkStatus::Cancelled
        )
    }
}

/// Status of a subtask
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SubtaskStatus {
    /// Subtask has not been started yet
    #[default]
    Pending,
    /// Subtask is currently being executed
    InProgress,
    /// Subtask completed successfully
    Completed,
    /// Subtask failed
    Failed,
    /// Subtask was skipped (e.g., conditional step not needed)
    Skipped,
}

impl SubtaskStatus {
    /// Returns the emoji representation of this status
    #[must_use]
    pub const fn emoji(self) -> &'static str {
        match self {
            Self::Pending => "⏳",
            Self::InProgress => "🔄",
            Self::Completed => "✅",
            Self::Failed => "❌",
            Self::Skipped => "⏭️",
        }
    }
}

impl fmt::Display for SubtaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.emoji())
    }
}

/// A unified subtask that merges agent decomposition and TypeScript task steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    /// Unique identifier for this subtask
    #[serde(default = "SubtaskId::new")]
    pub id: SubtaskId,
    /// Human-readable description of what this subtask does
    pub description: String,
    /// Difficulty rating from 1 (easiest) to 10 (hardest)
    pub difficulty: u8,
    /// Current status of this subtask
    #[serde(default)]
    pub status: SubtaskStatus,
    /// Optional verification step (command + expected exit code)
    pub verification: Option<VerificationStep>,
    /// Optional error message if subtask failed
    pub error: Option<String>,
    /// Optional result/output from completing this subtask
    pub result: Option<String>,
}

/// Verification step for a subtask (optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStep {
    /// Command to run for verification
    pub command: String,
    /// Expected exit code (typically 0 for success)
    pub expected_exit_code: i32,
}
