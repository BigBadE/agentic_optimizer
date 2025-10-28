use merlin_core::ThreadId;
use merlin_routing::TaskId;
use merlin_routing::TaskProgress;
use std::collections::HashMap;
use std::time::{Instant, SystemTime};

/// Status of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "Pending status will be used when dependency tracking is implemented"
)]
pub enum TaskStatus {
    /// Task is pending (waiting for dependencies or resources)
    Pending,
    /// Task is currently running
    Running,
    /// Task has completed successfully
    Completed,
    /// Task has failed
    Failed,
}

/// Task display information
#[derive(Clone)]
pub struct TaskDisplay {
    /// Description of the task
    pub description: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// Optional progress information
    pub progress: Option<TaskProgress>,
    /// Output lines from the task
    pub output_lines: Vec<String>,
    /// When the task was created (persists across program runs)
    pub created_at: SystemTime,
    /// When the task was created/started (monotonic clock, used for sorting within a program run)
    pub timestamp: Instant,
    /// Thread this task belongs to
    pub thread_id: Option<ThreadId>,
    /// Plain text output
    pub output: String,
    /// List of task steps
    pub steps: Vec<TaskStepInfo>,
    /// Currently active step (shown as visual subtask in UI)
    pub current_step: Option<TaskStepInfo>,
    /// Retry count (0 = first attempt, increments on each retry)
    pub retry_count: u32,
}

/// Status of a task step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStepStatus {
    /// Step is currently running
    Running,
    /// Step has completed successfully
    Completed,
    /// Step has failed
    Failed,
}

/// Task step information
#[derive(Clone)]
pub struct TaskStepInfo {
    /// Unique identifier for this step
    pub step_id: String,
    /// Type of step (e.g., `thinking`, `tool_call`, `validation`)
    ///
    /// Used for differentiated UI rendering of step types.
    pub step_type: String,
    /// Content of the step
    pub content: String,
    /// Status of the step
    pub status: TaskStepStatus,
}

/// Manages task storage and ordering
#[derive(Default)]
pub struct TaskManager {
    tasks: HashMap<TaskId, TaskDisplay>,
    task_order: Vec<TaskId>,
}

impl TaskManager {
    /// Adds a task to the manager
    pub fn add_task(&mut self, task_id: TaskId, task: TaskDisplay) {
        self.tasks.insert(task_id, task);
        self.task_order.push(task_id);
    }

    /// Inserts a task into the `HashMap` only, without updating `task_order`
    /// Used during bulk loading - call `rebuild_order()` after all tasks are inserted
    pub fn insert_task_for_load(&mut self, task_id: TaskId, task: TaskDisplay) {
        self.tasks.insert(task_id, task);
    }

    /// Removes a task, returns list of removed IDs
    pub fn remove_task(&mut self, task_id: TaskId) -> Vec<TaskId> {
        self.tasks.remove(&task_id);
        self.task_order.retain(|id| *id != task_id);
        vec![task_id]
    }

    /// Gets a task by ID
    pub fn get_task(&self, task_id: TaskId) -> Option<&TaskDisplay> {
        self.tasks.get(&task_id)
    }

    /// Gets a mutable task by ID
    pub fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut TaskDisplay> {
        self.tasks.get_mut(&task_id)
    }

    /// Rebuilds task order from timestamps
    ///
    /// Orders tasks by creation time (oldest first). Uses `SystemTime` which persists
    /// across program runs, unlike `Instant` which is relative to program start.
    pub fn rebuild_order(&mut self) {
        self.task_order.clear();

        // Collect all task IDs with their creation times
        let mut all_tasks: Vec<(TaskId, SystemTime)> = self
            .tasks
            .iter()
            .map(|(&id, task)| (id, task.created_at))
            .collect();

        // Sort by creation time ascending (oldest first)
        all_tasks.sort_by_key(|(_, created_at)| *created_at);

        for (task_id, _) in all_tasks {
            self.task_order.push(task_id);
        }
    }

    /// Iterates over all tasks
    #[allow(
        dead_code,
        reason = "Kept for future use when iteration over all tasks is needed"
    )]
    pub fn iter_tasks(&self) -> impl Iterator<Item = (TaskId, &TaskDisplay)> {
        self.tasks.iter().map(|(&id, task)| (id, task))
    }

    /// Gets the task order
    pub fn task_order(&self) -> &[TaskId] {
        &self.task_order
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_task(description: &str, time_offset_secs: u64) -> TaskDisplay {
        let created_at = SystemTime::now()
            .checked_sub(Duration::from_secs(time_offset_secs))
            .unwrap_or_else(SystemTime::now);

        TaskDisplay {
            description: description.to_owned(),
            status: TaskStatus::Running,
            progress: None,
            output_lines: vec![],
            created_at,
            timestamp: Instant::now()
                .checked_sub(Duration::from_secs(time_offset_secs))
                .unwrap_or_else(Instant::now),
            thread_id: None,
            output: String::default(),
            steps: vec![],
            current_step: None,
            retry_count: 0,
        }
    }

    /// Test task ordering after loading from persistence
    ///
    /// # Panics
    /// Panics if test assertions fail
    #[test]
    fn test_task_order_after_loading() {
        let mut manager = TaskManager::default();

        let task1 = create_task("Oldest task", 300); // Created 5 minutes ago
        let task2 = create_task("Middle task", 180); // Created 3 minutes ago
        let task3 = create_task("Recent task", 60); // Created 1 minute ago
        let task4 = create_task("Newest task", 10); // Created 10 seconds ago

        let id1 = TaskId::default();
        let id2 = TaskId::default();
        let id3 = TaskId::default();
        let id4 = TaskId::default();

        // Insert in random order
        manager.insert_task_for_load(id3, task3);
        manager.insert_task_for_load(id1, task1);
        manager.insert_task_for_load(id4, task4);
        manager.insert_task_for_load(id2, task2);

        // Rebuild order
        manager.rebuild_order();

        // Verify tasks are ordered by timestamp (oldest first)
        let order = manager.task_order();
        assert_eq!(order.len(), 4, "Should have 4 tasks");
        assert_eq!(order[0], id1, "Oldest task should be first");
        assert_eq!(order[1], id2, "Middle task should be second");
        assert_eq!(order[2], id3, "Recent task should be third");
        assert_eq!(order[3], id4, "Newest task should be fourth");
    }
}
