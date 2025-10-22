use merlin_routing::TaskId;
use merlin_routing::TaskProgress;
use std::collections::HashMap;
use std::time::{Instant, SystemTime};

/// Status of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
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
    /// ID of the parent task (if this is a subtask)
    pub parent_id: Option<TaskId>,
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

/// Manages task storage, ordering, and hierarchy
#[derive(Default)]
pub struct TaskManager {
    tasks: HashMap<TaskId, TaskDisplay>,
    task_order: Vec<TaskId>,
}

impl TaskManager {
    /// Adds a task to the manager with proper hierarchical positioning
    pub fn add_task(&mut self, task_id: TaskId, task: TaskDisplay) {
        let parent_id = task.parent_id;
        self.tasks.insert(task_id, task);

        // Insert task in correct hierarchical position
        if let Some(parent_id) = parent_id {
            self.insert_child_task(task_id, parent_id);
        } else {
            // Root task, append to end
            self.task_order.push(task_id);
        }
    }

    /// Inserts a task into the `HashMap` only, without updating `task_order`
    /// Used during bulk loading - call `rebuild_order()` after all tasks are inserted
    pub fn insert_task_for_load(&mut self, task_id: TaskId, task: TaskDisplay) {
        self.tasks.insert(task_id, task);
    }

    /// Removes a task and all its descendants, returns list of removed IDs
    pub fn remove_task(&mut self, task_id: TaskId) -> Vec<TaskId> {
        let to_delete = self.collect_descendants(task_id);

        for id in &to_delete {
            self.tasks.remove(id);
        }

        self.task_order.retain(|id| !to_delete.contains(id));
        to_delete
    }

    /// Gets a task by ID
    pub fn get_task(&self, task_id: TaskId) -> Option<&TaskDisplay> {
        self.tasks.get(&task_id)
    }

    /// Gets a mutable task by ID
    pub fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut TaskDisplay> {
        self.tasks.get_mut(&task_id)
    }

    /// Rebuilds task order from parent relationships
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

        // Add root tasks first, then recursively add their children
        for (task_id, _) in &all_tasks {
            let is_root = self.is_root_task(*task_id);

            if is_root && !self.task_order.contains(task_id) {
                self.add_task_and_descendants(*task_id);
            }
        }

        // Handle orphaned tasks (parent doesn't exist)
        for (task_id, _) in all_tasks {
            if !self.task_order.contains(&task_id) {
                self.task_order.push(task_id);
            }
        }
    }

    /// Checks if a task is a descendant of another
    pub fn is_descendant_of(&self, task_id: TaskId, ancestor_id: TaskId) -> bool {
        let mut current_parent = self.get_parent_id(task_id);

        while let Some(parent_id) = current_parent {
            if parent_id == ancestor_id {
                return true;
            }
            current_parent = self.get_parent_id(parent_id);
        }

        false
    }

    /// Iterates over all tasks
    pub fn iter_tasks(&self) -> impl Iterator<Item = (TaskId, &TaskDisplay)> {
        self.tasks.iter().map(|(&id, task)| (id, task))
    }

    /// Gets the task order
    pub fn task_order(&self) -> &[TaskId] {
        &self.task_order
    }

    // Private helper methods

    fn insert_child_task(&mut self, task_id: TaskId, parent_id: TaskId) {
        if let Some(parent_pos) = self.task_order.iter().position(|&id| id == parent_id) {
            let insert_pos = self.find_last_descendant_position(parent_id, parent_pos);
            self.task_order.insert(insert_pos, task_id);
        } else {
            // Parent not found, append to end
            self.task_order.push(task_id);
        }
    }

    fn find_last_descendant_position(&self, parent_id: TaskId, parent_pos: usize) -> usize {
        let mut insert_pos = parent_pos + 1;
        while insert_pos < self.task_order.len() {
            let current_id = self.task_order[insert_pos];
            if self.is_descendant_of(current_id, parent_id) {
                insert_pos += 1;
            } else {
                break;
            }
        }
        insert_pos
    }

    fn collect_descendants(&self, task_id: TaskId) -> Vec<TaskId> {
        let mut to_delete = vec![task_id];
        let mut index = 0;
        while index < to_delete.len() {
            let current = to_delete[index];
            let children: Vec<TaskId> = self
                .task_order
                .iter()
                .filter(|&&id| {
                    self.tasks
                        .get(&id)
                        .is_some_and(|task| task.parent_id == Some(current))
                })
                .copied()
                .collect();
            to_delete.extend(children);
            index += 1;
        }
        to_delete
    }

    fn add_task_and_descendants(&mut self, task_id: TaskId) {
        self.task_order.push(task_id);

        // Collect children with their timestamps
        let mut children: Vec<(TaskId, Instant)> = self
            .tasks
            .iter()
            .filter(|(_, task)| task.parent_id == Some(task_id))
            .map(|(&id, task)| (id, task.timestamp))
            .collect();

        // Sort children by timestamp ascending (oldest first)
        children.sort_by_key(|(_, timestamp)| *timestamp);

        for (child_id, _) in children {
            if !self.task_order.contains(&child_id) {
                self.add_task_and_descendants(child_id);
            }
        }
    }

    fn is_root_task(&self, task_id: TaskId) -> bool {
        self.tasks
            .get(&task_id)
            .is_some_and(|task| task.parent_id.is_none())
    }

    fn get_parent_id(&self, task_id: TaskId) -> Option<TaskId> {
        let task = self.tasks.get(&task_id)?;
        task.parent_id
    }

    /// Checks if there are any tasks with active progress indicators
    /// Used to determine if UI should force periodic updates
    pub fn has_tasks_with_progress(&self) -> bool {
        self.tasks.values().any(|task| task.progress.is_some())
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
            parent_id: None,
            output: String::default(),
            steps: vec![],
            current_step: None,
            retry_count: 0,
        }
    }

    #[test]
    fn test_task_order_after_loading_simulates_insert_for_load() {
        let mut manager = TaskManager::default();

        // Simulate loading tasks from disk
        // This is the ACTUAL flow used in app.rs:
        // 1. Call insert_task_for_load for each task (doesn't update order)
        // 2. Call rebuild_order once at the end
        //
        // Tasks are ordered by timestamp (oldest first).
        // Higher time_offset_secs = older task (created further in the past).

        let task1 = create_task("Oldest task", 300); // Created 5 minutes ago
        let task2 = create_task("Middle task", 180); // Created 3 minutes ago
        let task3 = create_task("Recent task", 60); // Created 1 minute ago
        let task4 = create_task("Newest task", 10); // Created 10 seconds ago

        let id1 = TaskId::default();
        let id2 = TaskId::default();
        let id3 = TaskId::default();
        let id4 = TaskId::default();

        // Use insert_task_for_load in any order - rebuild_order will sort by timestamp
        manager.insert_task_for_load(id3, task3);
        manager.insert_task_for_load(id1, task1);
        manager.insert_task_for_load(id4, task4);
        manager.insert_task_for_load(id2, task2);

        // Rebuild order (this is what app.rs does after loading)
        manager.rebuild_order();

        // Verify tasks are ordered by timestamp (oldest first)
        let order = manager.task_order();
        assert_eq!(order.len(), 4, "Should have 4 tasks");

        // Oldest task should be first
        assert_eq!(order[0], id1, "Oldest task should be first");
        assert_eq!(order[1], id2, "Middle task should be second");
        assert_eq!(order[2], id3, "Recent task should be third");
        assert_eq!(order[3], id4, "Newest task should be fourth");
    }

    #[test]
    fn test_task_order_with_children() {
        let mut manager = TaskManager::default();

        let id1 = TaskId::default();
        let id2 = TaskId::default();
        let id3 = TaskId::default();
        let id4 = TaskId::default();

        // Parent task
        let parent = create_task("Parent task", 300);
        manager.add_task(id1, parent);

        // Child tasks of parent (added in order)
        let mut child1 = create_task("First child", 200);
        child1.parent_id = Some(id1);
        let mut child2 = create_task("Second child", 100);
        child2.parent_id = Some(id1);

        manager.add_task(id2, child1);
        manager.add_task(id3, child2);

        // Another root task
        let task2 = create_task("Second root", 150);
        manager.add_task(id4, task2);

        manager.rebuild_order();

        let order = manager.task_order();
        assert_eq!(order.len(), 4);

        // Parent should come first
        assert_eq!(order[0], id1, "Parent should be first");

        // Children should follow parent in insertion order
        assert_eq!(order[1], id2, "First child should come first");
        assert_eq!(order[2], id3, "Second child should come second");

        // Second root task should come last
        assert_eq!(order[3], id4, "Second root task should be last");
    }
}
