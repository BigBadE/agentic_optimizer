use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use merlin_core::{Result, RoutingError, SubtaskSpec, Task, TaskId, TaskResult};

/// Maximum depth for task decomposition
const MAX_DECOMPOSITION_DEPTH: usize = 5;

/// Maximum number of subtasks per task
const MAX_SUBTASKS_PER_TASK: usize = 10;

/// Maximum number of checkpoints to keep
const MAX_CHECKPOINTS: usize = 100;

/// Coordinates complex multi-step task execution with checkpointing
#[derive(Clone)]
pub struct TaskCoordinator {
    state: Arc<Mutex<CoordinatorState>>,
}

#[derive(Debug)]
struct CoordinatorState {
    active_tasks: HashMap<TaskId, TaskExecution>,
    completed_tasks: HashMap<TaskId, TaskResult>,
    task_hierarchy: HashMap<TaskId, Vec<TaskId>>,
    checkpoints: VecDeque<Checkpoint>,
}

#[derive(Debug, Clone)]
struct TaskExecution {
    task: Task,
    parent_id: Option<TaskId>,
    depth: usize,
    subtasks: Vec<TaskId>,
    completed_subtasks: Vec<TaskId>,
    status: TaskStatus,
    created_at: u64,
    updated_at: u64,
}

/// Status of a task in the coordinator
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is currently being executed
    InProgress,
    /// Task is waiting for subtasks to complete
    WaitingForSubtasks,
    /// Task has completed successfully
    Completed,
    /// Task has failed
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Checkpoint {
    timestamp: u64,
    task_id: TaskId,
    description: String,
    completed_subtasks: Vec<TaskId>,
}

impl TaskCoordinator {
    /// Create a new task coordinator
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(CoordinatorState {
                active_tasks: HashMap::new(),
                completed_tasks: HashMap::new(),
                task_hierarchy: HashMap::new(),
                checkpoints: VecDeque::new(),
            })),
        }
    }

    /// Register a new task for coordination
    ///
    /// # Errors
    /// Returns an error if maximum decomposition depth is exceeded
    pub async fn register_task(&self, task: Task, parent_id: Option<TaskId>) -> Result<()> {
        let mut state = self.state.lock().await;

        let depth = parent_id.map_or(0, |parent| {
            state
                .active_tasks
                .get(&parent)
                .map_or(0, |exec| exec.depth + 1)
        });

        if depth > MAX_DECOMPOSITION_DEPTH {
            return Err(RoutingError::Other(format!(
                "Maximum decomposition depth ({MAX_DECOMPOSITION_DEPTH}) exceeded"
            )));
        }

        let task_id = task.id;
        let execution = TaskExecution {
            task,
            parent_id,
            subtasks: Vec::new(),
            completed_subtasks: Vec::new(),
            status: TaskStatus::Pending,
            depth,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs()),
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs()),
        };

        state.active_tasks.insert(task_id, execution);

        if let Some(parent_id) = parent_id {
            state
                .active_tasks
                .get_mut(&parent_id)
                .ok_or_else(|| RoutingError::Other("Parent task not found".to_owned()))?
                .subtasks
                .push(task_id);
        }
        drop(state);

        Ok(())
    }

    /// Decompose a task into subtasks
    ///
    /// # Errors
    /// Returns an error if task not found or too many subtasks
    pub async fn decompose_task(
        &self,
        task_id: TaskId,
        subtask_specs: Vec<SubtaskSpec>,
    ) -> Result<Vec<Task>> {
        if subtask_specs.len() > MAX_SUBTASKS_PER_TASK {
            return Err(RoutingError::Other(format!(
                "Too many subtasks ({} > {MAX_SUBTASKS_PER_TASK})",
                subtask_specs.len()
            )));
        }

        let subtasks: Vec<Task> = subtask_specs
            .into_iter()
            .map(|spec| Task::new(spec.description).with_complexity(spec.complexity))
            .collect();

        let mut state = self.state.lock().await;

        let execution = state
            .active_tasks
            .get_mut(&task_id)
            .ok_or_else(|| RoutingError::Other("Task not found".to_owned()))?;

        execution.status = TaskStatus::WaitingForSubtasks;
        execution.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        for subtask in &subtasks {
            execution.subtasks.push(subtask.id);
        }
        drop(state);

        Ok(subtasks)
    }

    /// Mark a subtask as completed
    ///
    /// # Errors
    /// Returns an error if task tracking fails
    pub async fn complete_subtask(&self, task_id: TaskId, result: TaskResult) -> Result<()> {
        let mut state = self.state.lock().await;

        state.completed_tasks.insert(task_id, result);

        if let Some(parent_id) = state
            .active_tasks
            .get(&task_id)
            .and_then(|task_exec| task_exec.parent_id)
            && let Some(parent) = state.active_tasks.get_mut(&parent_id)
        {
            parent.completed_subtasks.push(task_id);
            parent.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs());

            if parent.completed_subtasks.len() == parent.subtasks.len() {
                parent.status = TaskStatus::Completed;
            }
        }
        drop(state);

        Ok(())
    }

    /// Create a checkpoint for a task
    ///
    /// # Errors
    /// Returns an error if the task is not found
    pub async fn create_checkpoint(&self, task_id: TaskId, description: String) -> Result<()> {
        let completed_subtasks = {
            let state = self.state.lock().await;
            let execution = state
                .active_tasks
                .get(&task_id)
                .ok_or_else(|| RoutingError::Other("Task not found".to_owned()))?;
            let result = execution.completed_subtasks.clone();
            drop(state);
            result
        };

        let checkpoint = Checkpoint {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs()),
            task_id,
            description,
            completed_subtasks,
        };

        let mut state = self.state.lock().await;
        state.checkpoints.push_back(checkpoint);

        if state.checkpoints.len() > MAX_CHECKPOINTS {
            state.checkpoints.pop_front();
        }
        drop(state);

        Ok(())
    }

    /// Get the progress of a task
    ///
    /// # Errors
    /// Returns an error if the task is not found
    pub async fn get_progress(&self, task_id: TaskId) -> Result<TaskProgress> {
        let state = self.state.lock().await;

        let execution = state
            .active_tasks
            .get(&task_id)
            .ok_or_else(|| RoutingError::Other("Task not found".to_owned()))?;

        let total_subtasks = execution.subtasks.len();
        let completed_subtasks = execution.completed_subtasks.len();
        let progress_percent = if total_subtasks > 0 {
            (completed_subtasks as f32 / total_subtasks as f32) * 100.0
        } else {
            0.0
        };

        let result = TaskProgress {
            task_id,
            total_subtasks,
            completed_subtasks,
            progress_percent,
            status: execution.status.clone(),
            depth: execution.depth,
            created_at: execution.created_at,
        };
        drop(state);
        Ok(result)
    }

    /// Get all subtasks for a task
    ///
    /// # Errors
    /// Returns an error if the task is not found
    pub async fn get_subtasks(&self, task_id: TaskId) -> Result<Vec<Task>> {
        let state = self.state.lock().await;

        let execution = state
            .active_tasks
            .get(&task_id)
            .ok_or_else(|| RoutingError::Other("Task not found".to_owned()))?;

        let subtasks: Vec<Task> = execution
            .subtasks
            .iter()
            .filter_map(|id| state.active_tasks.get(id).map(|exec| exec.task.clone()))
            .collect();

        drop(state);
        Ok(subtasks)
    }

    /// Check if a task is ready to execute (all dependencies met)
    ///
    /// # Errors
    /// Returns an error if the task is not found
    pub async fn is_ready(&self, task_id: TaskId) -> Result<bool> {
        let state = self.state.lock().await;

        let execution = state
            .active_tasks
            .get(&task_id)
            .ok_or_else(|| RoutingError::Other("Task not found".to_owned()))?;

        let result = matches!(execution.status, TaskStatus::Pending);
        drop(state);
        Ok(result)
    }

    /// Mark a task as in progress
    ///
    /// # Errors
    /// Returns an error if the task is not found
    pub async fn start_task(&self, task_id: TaskId) -> Result<()> {
        let mut state = self.state.lock().await;

        let execution = state
            .active_tasks
            .get_mut(&task_id)
            .ok_or_else(|| RoutingError::Other("Task not found".to_owned()))?;

        execution.status = TaskStatus::InProgress;
        execution.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        drop(state);
        Ok(())
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> CoordinatorStats {
        let state = self.state.lock().await;

        let mut pending = 0;
        let mut in_progress = 0;
        let mut waiting = 0;
        let mut completed = 0;
        let mut failed = 0;

        for execution in state.active_tasks.values() {
            match execution.status {
                TaskStatus::Pending => pending += 1,
                TaskStatus::InProgress => in_progress += 1,
                TaskStatus::WaitingForSubtasks => waiting += 1,
                TaskStatus::Completed => completed += 1,
                TaskStatus::Failed => failed += 1,
            }
        }

        CoordinatorStats {
            total_tasks: state.active_tasks.len(),
            pending,
            in_progress,
            waiting,
            completed,
            failed,
            checkpoints: state.checkpoints.len(),
        }
    }

    /// Clear completed tasks older than a threshold
    ///
    /// # Errors
    /// Returns an error if cleanup fails
    pub async fn cleanup_old_tasks(&self, max_age_seconds: u64) -> Result<usize> {
        let mut state = self.state.lock().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        let mut to_remove = Vec::new();

        for (id, execution) in &state.active_tasks {
            if execution.status == TaskStatus::Completed {
                let age = now.saturating_sub(execution.updated_at);
                if age > max_age_seconds {
                    to_remove.push(*id);
                }
            }
        }

        for id in &to_remove {
            state.active_tasks.remove(id);
            state.completed_tasks.remove(id);
            state.task_hierarchy.remove(id);
        }

        let count = to_remove.len();
        drop(state);
        Ok(count)
    }
}

impl Default for TaskCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress information for a task
#[derive(Debug, Clone)]
pub struct TaskProgress {
    /// ID of the task
    pub task_id: TaskId,
    /// Total number of subtasks
    pub total_subtasks: usize,
    /// Number of completed subtasks
    pub completed_subtasks: usize,
    /// Progress as a percentage
    pub progress_percent: f32,
    /// Current status of the task
    pub status: TaskStatus,
    /// Depth in the task hierarchy
    pub depth: usize,
    /// Unix timestamp when the task was created
    pub created_at: u64,
}

/// Statistics about the coordinator state
#[derive(Debug, Clone)]
pub struct CoordinatorStats {
    /// Total number of tasks
    pub total_tasks: usize,
    /// Number of pending tasks
    pub pending: usize,
    /// Number of tasks in progress
    pub in_progress: usize,
    /// Number of tasks waiting for subtasks
    pub waiting: usize,
    /// Number of completed tasks
    pub completed: usize,
    /// Number of failed tasks
    pub failed: usize,
    /// Number of checkpoints created
    pub checkpoints: usize,
}
#[cfg(test)]
mod tests {
    use super::*;
    use merlin_core::Complexity;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coordinator = TaskCoordinator::default();
        let stats = coordinator.get_stats().await;
        assert_eq!(stats.total_tasks, 0);
    }

    #[tokio::test]
    async fn test_register_task() {
        let coordinator = TaskCoordinator::new();
        let task = Task::new("Test task".to_owned());

        coordinator.register_task(task.clone(), None).await.unwrap();

        let stats = coordinator.get_stats().await;
        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.pending, 1);
    }

    #[tokio::test]
    async fn test_decompose_task() {
        let coordinator = TaskCoordinator::new();
        let task = Task::new("Parent task".to_owned());
        let task_id = task.id;

        coordinator.register_task(task, None).await.unwrap();

        let subtask_specs = vec![
            SubtaskSpec {
                description: "Subtask 1".to_owned(),
                complexity: Complexity::Simple,
            },
            SubtaskSpec {
                description: "Subtask 2".to_owned(),
                complexity: Complexity::Simple,
            },
        ];

        let subtasks = coordinator
            .decompose_task(task_id, subtask_specs)
            .await
            .unwrap();

        assert_eq!(subtasks.len(), 2);
    }

    #[tokio::test]
    async fn test_max_depth_enforcement() {
        let coordinator = TaskCoordinator::new();
        let mut parent_id = None;

        for depth_level in 0..=(MAX_DECOMPOSITION_DEPTH + 1) {
            let task = Task::new(format!("Task at depth {depth_level}"));
            let task_id = task.id;

            let result = coordinator.register_task(task, parent_id).await;

            if depth_level <= MAX_DECOMPOSITION_DEPTH {
                result.unwrap();
                parent_id = Some(task_id);
            } else {
                assert!(result.is_err());
            }
        }
    }
}
