use super::events::TaskProgress;
use crate::TaskId;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

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
    /// When the task started
    pub start_time: Instant,
    /// When the task ended (if completed)
    pub end_time: Option<Instant>,
    /// ID of the parent task (if this is a subtask)
    pub parent_id: Option<TaskId>,
    /// Plain text output
    pub output: String,
    /// List of task steps
    pub steps: Vec<TaskStepInfo>,
    /// Currently active step (shown as visual subtask in UI)
    pub current_step: Option<TaskStepInfo>,
}

/// Task step information
#[derive(Clone)]
pub struct TaskStepInfo {
    /// Unique identifier for this step
    pub step_id: String,
    /// Type of step (e.g., `thinking`, `tool_call`)
    pub step_type: String,
    /// Content of the step
    pub content: String,
    /// When this step occurred
    pub timestamp: Instant,
}

impl TaskStepInfo {
    /// Access `step_id`
    pub fn step_id(&self) -> &str {
        &self.step_id
    }

    /// Access `step_type`
    pub fn step_type(&self) -> &str {
        &self.step_type
    }

    /// Access `content`
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Access `timestamp`
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

/// Manages task storage, ordering, and hierarchy
#[derive(Default)]
pub struct TaskManager {
    tasks: HashMap<TaskId, TaskDisplay>,
    task_order: Vec<TaskId>,
    collapsed_tasks: HashSet<TaskId>,
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
            self.collapsed_tasks.remove(id);
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
    pub fn rebuild_order(&mut self) {
        self.task_order.clear();

        let mut all_tasks: Vec<(TaskId, Instant)> = self
            .tasks
            .iter()
            .map(|(&id, task)| (id, task.start_time))
            .collect();
        all_tasks.sort_by_key(|(_, time)| Reverse(*time));

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

    /// Gets all visible tasks (not hidden by collapsed parents)
    pub fn get_visible_tasks(&self) -> Vec<TaskId> {
        self.task_order
            .iter()
            .copied()
            .filter(|&task_id| !self.is_hidden_by_collapse(task_id))
            .collect()
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

    /// Collapses a task
    pub fn collapse_task(&mut self, task_id: TaskId) {
        self.collapsed_tasks.insert(task_id);
    }

    /// Expands a task
    pub fn expand_task(&mut self, task_id: TaskId) {
        self.collapsed_tasks.remove(&task_id);
    }

    /// Toggles collapse state of a task
    pub fn toggle_collapse(&mut self, task_id: TaskId) {
        if self.collapsed_tasks.contains(&task_id) {
            self.expand_task(task_id);
        } else {
            self.collapse_task(task_id);
        }
    }

    /// Checks if a task is collapsed
    pub fn is_collapsed(&self, task_id: TaskId) -> bool {
        self.collapsed_tasks.contains(&task_id)
    }

    /// Checks if a task has children
    pub fn has_children(&self, task_id: TaskId) -> bool {
        self.task_order.iter().any(|id| {
            self.tasks
                .get(id)
                .is_some_and(|task| task.parent_id == Some(task_id))
        })
    }

    /// Iterates over all tasks
    pub fn iter_tasks(&self) -> impl Iterator<Item = (TaskId, &TaskDisplay)> {
        self.tasks.iter().map(|(&id, task)| (id, task))
    }

    /// Gets the task order
    pub fn task_order(&self) -> &[TaskId] {
        &self.task_order
    }

    /// Checks if task manager is empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
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

        let mut children: Vec<(TaskId, Instant)> = self
            .tasks
            .iter()
            .filter(|(_, task)| task.parent_id == Some(task_id))
            .map(|(&id, task)| (id, task.start_time))
            .collect();
        children.sort_by_key(|(_, time)| Reverse(*time));

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

    fn is_hidden_by_collapse(&self, task_id: TaskId) -> bool {
        let mut current_parent = self.get_parent_id(task_id);

        while let Some(parent_id) = current_parent {
            if self.collapsed_tasks.contains(&parent_id) {
                return true;
            }
            current_parent = self.get_parent_id(parent_id);
        }
        false
    }

    /// Checks if there are any tasks with active progress indicators
    /// Used to determine if UI should force periodic updates
    pub fn has_tasks_with_progress(&self) -> bool {
        self.tasks.values().any(|task| task.progress.is_some())
    }

    /// Updates an existing task with new data
    pub fn update_task(&mut self, task_id: TaskId, task: TaskDisplay) {
        self.tasks.insert(task_id, task);
    }
}
