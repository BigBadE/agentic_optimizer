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

    /// Adds a subtask to this work unit
    pub fn add_subtask(&mut self, subtask: Subtask) {
        self.subtasks.push(subtask);
    }

    /// Marks the work as completed
    pub fn complete(&mut self) {
        self.status = WorkStatus::Completed;
    }

    /// Marks the work as failed
    pub fn fail(&mut self) {
        self.status = WorkStatus::Failed;
    }

    /// Marks the work as cancelled
    pub fn cancel(&mut self) {
        self.status = WorkStatus::Cancelled;
    }

    /// Increments retry count and marks as retrying
    pub fn retry(&mut self) {
        self.retry_count += 1;
        self.status = WorkStatus::Retrying;
    }

    /// Returns the next pending subtask
    #[must_use]
    pub fn next_pending_subtask(&self) -> Option<&Subtask> {
        self.subtasks
            .iter()
            .find(|subtask| matches!(subtask.status, SubtaskStatus::Pending))
    }

    /// Returns progress percentage (0-100)
    #[must_use]
    pub fn progress_percentage(&self) -> u8 {
        if self.subtasks.is_empty() {
            return 0;
        }
        let completed = self
            .subtasks
            .iter()
            .filter(|subtask| matches!(subtask.status, SubtaskStatus::Completed))
            .count();
        ((completed as f64 / self.subtasks.len() as f64) * 100.0) as u8
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

impl Subtask {
    /// Creates a new pending subtask
    pub fn new(description: String, difficulty: u8) -> Self {
        Self {
            id: SubtaskId::new(),
            description,
            difficulty,
            status: SubtaskStatus::Pending,
            verification: None,
            error: None,
            result: None,
        }
    }

    /// Creates a new subtask with verification
    pub fn with_verification(
        description: String,
        difficulty: u8,
        verification: VerificationStep,
    ) -> Self {
        Self {
            id: SubtaskId::new(),
            description,
            difficulty,
            status: SubtaskStatus::Pending,
            verification: Some(verification),
            error: None,
            result: None,
        }
    }

    /// Marks the subtask as in progress
    pub fn start(&mut self) {
        self.status = SubtaskStatus::InProgress;
    }

    /// Marks the subtask as completed with optional result
    pub fn complete(&mut self, result: Option<String>) {
        self.status = SubtaskStatus::Completed;
        self.result = result;
        self.error = None;
    }

    /// Marks the subtask as failed with an error message
    pub fn fail(&mut self, error: String) {
        self.status = SubtaskStatus::Failed;
        self.error = Some(error);
    }

    /// Marks the subtask as skipped
    pub fn skip(&mut self) {
        self.status = SubtaskStatus::Skipped;
    }

    /// Returns true if the subtask is completed
    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self.status, SubtaskStatus::Completed)
    }

    /// Returns true if the subtask failed
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self.status, SubtaskStatus::Failed)
    }
}

/// Verification step for a subtask (optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStep {
    /// Command to run for verification
    pub command: String,
    /// Expected exit code (typically 0 for success)
    pub expected_exit_code: i32,
}

impl VerificationStep {
    /// Creates a new verification step expecting exit code 0
    pub fn new(command: String) -> Self {
        Self {
            command,
            expected_exit_code: 0,
        }
    }

    /// Creates a new verification step with custom expected exit code
    pub fn with_exit_code(command: String, expected_exit_code: i32) -> Self {
        Self {
            command,
            expected_exit_code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_unit_progress() {
        let mut work = WorkUnit::new(TaskId::default(), "local".to_owned());

        // No subtasks = 0%
        assert_eq!(work.progress_percentage(), 0);

        // Add 4 subtasks
        work.add_subtask(Subtask::new("Task 1".to_owned(), 5));
        work.add_subtask(Subtask::new("Task 2".to_owned(), 5));
        work.add_subtask(Subtask::new("Task 3".to_owned(), 5));
        work.add_subtask(Subtask::new("Task 4".to_owned(), 5));

        // Complete 2 out of 4 = 50%
        work.subtasks[0].complete(None);
        work.subtasks[1].complete(None);
        assert_eq!(work.progress_percentage(), 50);

        // Complete all = 100%
        work.subtasks[2].complete(None);
        work.subtasks[3].complete(None);
        assert_eq!(work.progress_percentage(), 100);
    }

    #[test]
    fn test_subtask_state_transitions() {
        let mut subtask = Subtask::new("Test".to_owned(), 5);

        assert_eq!(subtask.status, SubtaskStatus::Pending);
        assert!(!subtask.is_completed());
        assert!(!subtask.is_failed());

        subtask.start();
        assert_eq!(subtask.status, SubtaskStatus::InProgress);

        subtask.complete(Some("Done".to_owned()));
        assert_eq!(subtask.status, SubtaskStatus::Completed);
        assert!(subtask.is_completed());
        assert_eq!(subtask.result, Some("Done".to_owned()));
        assert!(subtask.error.is_none());
    }

    #[test]
    fn test_subtask_failure() {
        let mut subtask = Subtask::new("Test".to_owned(), 5);

        subtask.start();
        subtask.fail("Error occurred".to_owned());

        assert_eq!(subtask.status, SubtaskStatus::Failed);
        assert!(subtask.is_failed());
        assert_eq!(subtask.error, Some("Error occurred".to_owned()));
    }

    #[test]
    fn test_work_unit_retry() {
        let mut work = WorkUnit::new(TaskId::default(), "local".to_owned());

        assert_eq!(work.retry_count, 0);
        assert_eq!(work.status, WorkStatus::InProgress);

        work.retry();
        assert_eq!(work.retry_count, 1);
        assert_eq!(work.status, WorkStatus::Retrying);

        work.retry();
        assert_eq!(work.retry_count, 2);
    }

    #[test]
    fn test_verification_step() {
        let verification = VerificationStep::new("cargo test".to_owned());
        assert_eq!(verification.command, "cargo test");
        assert_eq!(verification.expected_exit_code, 0);

        let custom_verification = VerificationStep::with_exit_code("npm run lint".to_owned(), 1);
        assert_eq!(custom_verification.expected_exit_code, 1);
    }

    #[test]
    fn test_subtask_complete_clears_error() {
        let mut subtask = Subtask::new("Test".to_owned(), 5);

        subtask.fail("Initial error".to_owned());
        assert!(subtask.error.is_some());

        subtask.complete(Some("Fixed".to_owned()));
        assert!(subtask.error.is_none());
        assert_eq!(subtask.result, Some("Fixed".to_owned()));
    }
}
