//! Task list structure for multi-step workflow execution.
//!
//! This module provides types for structured task planning and tracking
//! that agents create at the start of complex workflows.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of task step in a workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepType {
    /// Debug an issue or investigate a problem
    Debug,
    /// Implement a new feature or functionality
    Feature,
    /// Refactor existing code for better structure
    Refactor,
    /// Verify that changes work correctly
    Verify,
    /// Run tests to ensure no regressions
    Test,
}

impl StepType {
    /// Returns the default exit condition command for this step type.
    ///
    /// These commands must pass (exit code 0) for the step to be considered complete.
    /// Defaults are configured for Rust projects but can be overridden via config.
    #[must_use]
    pub const fn default_exit_command(self) -> &'static str {
        match self {
            Self::Debug | Self::Feature | Self::Verify => "cargo check",
            Self::Refactor => "cargo clippy -- -D warnings",
            Self::Test => "cargo test",
        }
    }
}

impl fmt::Display for StepType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Debug => write!(f, "Debug"),
            Self::Feature => write!(f, "Feature"),
            Self::Refactor => write!(f, "Refactor"),
            Self::Verify => write!(f, "Verify"),
            Self::Test => write!(f, "Test"),
        }
    }
}

/// Status of a task step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step has not been started yet
    Pending,
    /// Step is currently being executed
    InProgress,
    /// Step completed successfully
    Completed,
    /// Step failed and needs attention
    Failed,
    /// Step was skipped (e.g., conditional step not needed)
    Skipped,
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "⏳ Pending"),
            Self::InProgress => write!(f, "🔄 In Progress"),
            Self::Completed => write!(f, "✅ Completed"),
            Self::Failed => write!(f, "❌ Failed"),
            Self::Skipped => write!(f, "⏭️ Skipped"),
        }
    }
}

/// A single step in a task list workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// Unique identifier for this step (e.g., "`step_1`", "`step_2`")
    pub id: String,
    /// Type of step (Debug, Feature, Refactor, Verify, Test)
    pub step_type: StepType,
    /// Human-readable description of what this step does
    pub description: String,
    /// Verification requirement - how to confirm this step succeeded
    pub verification: String,
    /// Current status of this step
    pub status: StepStatus,
    /// Optional error message if step failed
    pub error: Option<String>,
    /// Optional result/output from completing this step
    pub result: Option<String>,
    /// Exit condition command that must pass for step completion.
    /// If None, uses the default command for the step type.
    pub exit_command: Option<String>,
}

impl TaskStep {
    /// Creates a new pending task step with default exit command
    pub fn new(id: String, step_type: StepType, description: String, verification: String) -> Self {
        Self {
            id,
            step_type,
            description,
            verification,
            status: StepStatus::Pending,
            error: None,
            result: None,
            exit_command: None,
        }
    }

    /// Creates a new pending task step with a custom exit command
    pub fn with_exit_command(
        id: String,
        step_type: StepType,
        description: String,
        verification: String,
        exit_command: String,
    ) -> Self {
        Self {
            id,
            step_type,
            description,
            verification,
            status: StepStatus::Pending,
            error: None,
            result: None,
            exit_command: Some(exit_command),
        }
    }

    /// Gets the exit command for this step (custom or default)
    #[must_use]
    pub fn get_exit_command(&self) -> &str {
        self.exit_command
            .as_deref()
            .unwrap_or_else(|| self.step_type.default_exit_command())
    }

    /// Marks the step as in progress
    pub fn start(&mut self) {
        self.status = StepStatus::InProgress;
    }

    /// Marks the step as completed with optional result
    pub fn complete(&mut self, result: Option<String>) {
        self.status = StepStatus::Completed;
        self.result = result;
        self.error = None;
    }

    /// Marks the step as failed with an error message
    pub fn fail(&mut self, error: String) {
        self.status = StepStatus::Failed;
        self.error = Some(error);
    }

    /// Marks the step as skipped
    pub fn skip(&mut self) {
        self.status = StepStatus::Skipped;
    }

    /// Returns true if the step is completed
    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self.status, StepStatus::Completed)
    }

    /// Returns true if the step failed
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self.status, StepStatus::Failed)
    }

    /// Returns true if the step is pending or in progress
    #[must_use]
    pub const fn is_pending_or_in_progress(&self) -> bool {
        matches!(self.status, StepStatus::Pending | StepStatus::InProgress)
    }
}

/// A structured task list for multi-step workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskList {
    /// Unique identifier for this task list
    pub id: String,
    /// Human-readable title describing the overall goal
    pub title: String,
    /// Ordered list of steps to execute
    pub steps: Vec<TaskStep>,
    /// Overall status derived from step statuses
    pub status: TaskListStatus,
}

/// Overall status of a task list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskListStatus {
    /// No steps have been started
    NotStarted,
    /// Some steps are in progress
    InProgress,
    /// All steps completed successfully
    Completed,
    /// One or more steps failed
    Failed,
    /// Partially complete (some steps done, some pending)
    PartiallyComplete,
}

impl fmt::Display for TaskListStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "Not Started"),
            Self::InProgress => write!(f, "In Progress"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::PartiallyComplete => write!(f, "Partially Complete"),
        }
    }
}

impl TaskList {
    /// Creates a new task list
    pub fn new(id: String, title: String, steps: Vec<TaskStep>) -> Self {
        Self {
            id,
            title,
            steps,
            status: TaskListStatus::NotStarted,
        }
    }

    /// Updates the overall status based on step statuses
    pub fn update_status(&mut self) {
        if self.steps.is_empty() {
            self.status = TaskListStatus::NotStarted;
            return;
        }

        let has_failed = self.steps.iter().any(TaskStep::is_failed);
        let all_completed = self.steps.iter().all(TaskStep::is_completed);
        let any_in_progress = self
            .steps
            .iter()
            .any(|step| matches!(step.status, StepStatus::InProgress));
        let any_completed = self.steps.iter().any(TaskStep::is_completed);

        self.status = if has_failed {
            TaskListStatus::Failed
        } else if all_completed {
            TaskListStatus::Completed
        } else if any_in_progress {
            TaskListStatus::InProgress
        } else if any_completed {
            TaskListStatus::PartiallyComplete
        } else {
            TaskListStatus::NotStarted
        };
    }

    /// Gets the next pending step
    #[must_use]
    pub fn next_pending_step(&self) -> Option<&TaskStep> {
        self.steps
            .iter()
            .find(|step| matches!(step.status, StepStatus::Pending))
    }

    /// Gets a mutable reference to a step by ID
    pub fn get_step_mut(&mut self, step_id: &str) -> Option<&mut TaskStep> {
        self.steps.iter_mut().find(|step| step.id == step_id)
    }

    /// Gets a step by ID
    #[must_use]
    pub fn get_step(&self, step_id: &str) -> Option<&TaskStep> {
        self.steps.iter().find(|step| step.id == step_id)
    }

    /// Returns the number of completed steps
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.steps.iter().filter(|step| step.is_completed()).count()
    }

    /// Returns the number of failed steps
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.steps.iter().filter(|step| step.is_failed()).count()
    }

    /// Returns the total number of steps
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.steps.len()
    }

    /// Returns true if all steps are completed
    #[must_use]
    pub fn is_complete(&self) -> bool {
        matches!(self.status, TaskListStatus::Completed)
    }

    /// Returns true if any step has failed
    #[must_use]
    pub fn has_failures(&self) -> bool {
        matches!(self.status, TaskListStatus::Failed)
    }

    /// Returns a progress percentage (0-100)
    #[must_use]
    pub fn progress_percentage(&self) -> u8 {
        if self.steps.is_empty() {
            return 0;
        }
        let completed = self.completed_count();
        ((completed as f64 / self.steps.len() as f64) * 100.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_step_lifecycle() {
        let mut step = TaskStep::new(
            "step_1".to_owned(),
            StepType::Feature,
            "Implement foo".to_owned(),
            "Verify foo compiles".to_owned(),
        );

        assert!(matches!(step.status, StepStatus::Pending));

        step.start();
        assert!(matches!(step.status, StepStatus::InProgress));
        assert!(step.is_pending_or_in_progress());

        step.complete(Some("Success".to_owned()));
        assert!(step.is_completed());
        assert_eq!(step.result, Some("Success".to_owned()));
    }

    #[test]
    fn test_task_step_failure() {
        let mut step = TaskStep::new(
            "step_1".to_owned(),
            StepType::Test,
            "Run tests".to_owned(),
            "All tests pass".to_owned(),
        );

        step.start();
        step.fail("Tests failed".to_owned());

        assert!(step.is_failed());
        assert_eq!(step.error, Some("Tests failed".to_owned()));
    }

    #[test]
    fn test_task_list_status_updates() {
        let steps = vec![
            TaskStep::new(
                "step_1".to_owned(),
                StepType::Feature,
                "Step 1".to_owned(),
                "Verify 1".to_owned(),
            ),
            TaskStep::new(
                "step_2".to_owned(),
                StepType::Test,
                "Step 2".to_owned(),
                "Verify 2".to_owned(),
            ),
        ];

        let mut task_list = TaskList::new("task_1".to_owned(), "My Task".to_owned(), steps);

        task_list.update_status();
        assert!(matches!(task_list.status, TaskListStatus::NotStarted));

        if let Some(step) = task_list.get_step_mut("step_1") {
            step.start();
            step.complete(None);
        }
        task_list.update_status();
        assert!(matches!(
            task_list.status,
            TaskListStatus::PartiallyComplete
        ));

        if let Some(step) = task_list.get_step_mut("step_2") {
            step.start();
            step.complete(None);
        }
        task_list.update_status();
        assert!(task_list.is_complete());
        assert_eq!(task_list.progress_percentage(), 100);
    }

    #[test]
    fn test_task_list_with_failures() {
        let steps = vec![
            TaskStep::new(
                "step_1".to_owned(),
                StepType::Feature,
                "Step 1".to_owned(),
                "Verify 1".to_owned(),
            ),
            TaskStep::new(
                "step_2".to_owned(),
                StepType::Test,
                "Step 2".to_owned(),
                "Verify 2".to_owned(),
            ),
        ];

        let mut task_list = TaskList::new("task_1".to_owned(), "My Task".to_owned(), steps);

        if let Some(step) = task_list.get_step_mut("step_1") {
            step.start();
            step.fail("Error occurred".to_owned());
        }

        task_list.update_status();
        assert!(task_list.has_failures());
        assert_eq!(task_list.failed_count(), 1);
    }

    #[test]
    fn test_next_pending_step() {
        let steps = vec![
            TaskStep::new(
                "step_1".to_owned(),
                StepType::Feature,
                "Step 1".to_owned(),
                "Verify 1".to_owned(),
            ),
            TaskStep::new(
                "step_2".to_owned(),
                StepType::Test,
                "Step 2".to_owned(),
                "Verify 2".to_owned(),
            ),
        ];

        let mut task_list = TaskList::new("task_1".to_owned(), "My Task".to_owned(), steps);

        let first_step = task_list.next_pending_step();
        assert!(first_step.is_some());
        assert_eq!(first_step.unwrap().id, "step_1");

        if let Some(step) = task_list.get_step_mut("step_1") {
            step.complete(None);
        }

        let second_step = task_list.next_pending_step();
        assert!(second_step.is_some());
        assert_eq!(second_step.unwrap().id, "step_2");
    }
}
