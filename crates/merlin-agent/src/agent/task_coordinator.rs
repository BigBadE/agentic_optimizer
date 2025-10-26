use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use merlin_core::{Result, RoutingError, Subtask, Task, TaskId, TaskResult};

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
            let parent = state
                .active_tasks
                .get_mut(&parent_id)
                .ok_or_else(|| RoutingError::Other("Parent task not found".to_owned()))?;

            // Check if adding this subtask would exceed the limit
            if parent.subtasks.len() >= MAX_SUBTASKS_PER_TASK {
                return Err(RoutingError::Other(format!(
                    "Maximum subtasks per task ({MAX_SUBTASKS_PER_TASK}) exceeded"
                )));
            }

            // Only add to subtasks if not already present (decompose_task may have added it)
            if !parent.subtasks.contains(&task_id) {
                parent.subtasks.push(task_id);
            }
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
        subtask_specs: Vec<Subtask>,
    ) -> Result<Vec<Task>> {
        if subtask_specs.len() > MAX_SUBTASKS_PER_TASK {
            return Err(RoutingError::Other(format!(
                "Too many subtasks ({} > {MAX_SUBTASKS_PER_TASK})",
                subtask_specs.len()
            )));
        }

        let subtasks: Vec<Task> = subtask_specs
            .into_iter()
            .map(|spec| Task::new(spec.description.clone()).with_difficulty(spec.difficulty))
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

        // Mark the subtask itself as completed
        if let Some(task_exec) = state.active_tasks.get_mut(&task_id) {
            task_exec.status = TaskStatus::Completed;
            task_exec.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs());
        }

        // Update parent's progress
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
#[cfg_attr(
    test,
    allow(clippy::unwrap_used, reason = "Tests use unwrap for clarity")
)]
mod tests {
    use super::*;
    use merlin_core::{Response, TokenUsage, ValidationResult};
    use tokio::spawn;

    /// Helper to create a test `TaskResult`
    fn create_test_result(task_id: TaskId) -> TaskResult {
        TaskResult {
            task_id,
            response: Response {
                text: "Test response".to_owned(),
                confidence: 1.0,
                tokens_used: TokenUsage::default(),
                provider: "test".to_owned(),
                latency_ms: 0,
            },
            tier_used: "test".to_owned(),
            tokens_used: TokenUsage::default(),
            validation: ValidationResult::default(),
            duration_ms: 0,
            work_unit: None,
        }
    }

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

    #[tokio::test]
    async fn test_max_subtasks_enforcement() {
        let coordinator = TaskCoordinator::new();
        let parent_task = Task::new("Parent task".to_owned());
        let parent_id = parent_task.id;
        coordinator.register_task(parent_task, None).await.unwrap();

        // Add up to MAX_SUBTASKS_PER_TASK
        for idx in 0..MAX_SUBTASKS_PER_TASK {
            let subtask = Task::new(format!("Subtask {idx}"));
            coordinator
                .register_task(subtask, Some(parent_id))
                .await
                .unwrap();
        }

        // Next subtask should fail
        let extra_subtask = Task::new("Extra subtask".to_owned());
        let result = coordinator
            .register_task(extra_subtask, Some(parent_id))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_task_completion() {
        let coordinator = TaskCoordinator::new();
        let task = Task::new("Test task".to_owned());
        let task_id = task.id;

        coordinator.register_task(task, None).await.unwrap();
        coordinator.start_task(task_id).await.unwrap();

        let result = create_test_result(task_id);

        coordinator
            .complete_subtask(task_id, result.clone())
            .await
            .unwrap();

        // Verify task is marked as completed
        let execution = coordinator
            .state
            .lock()
            .await
            .active_tasks
            .get(&task_id)
            .unwrap()
            .clone();
        assert_eq!(execution.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_task_hierarchy() {
        let coordinator = TaskCoordinator::new();

        let parent = Task::new("Parent".to_owned());
        let parent_id = parent.id;
        coordinator.register_task(parent, None).await.unwrap();

        let child1 = Task::new("Child 1".to_owned());
        let child1_id = child1.id;
        coordinator
            .register_task(child1, Some(parent_id))
            .await
            .unwrap();

        let child2 = Task::new("Child 2".to_owned());
        let child2_id = child2.id;
        coordinator
            .register_task(child2, Some(parent_id))
            .await
            .unwrap();

        let parent_exec = coordinator
            .state
            .lock()
            .await
            .active_tasks
            .get(&parent_id)
            .unwrap()
            .clone();
        assert_eq!(parent_exec.subtasks.len(), 2);
        assert!(parent_exec.subtasks.contains(&child1_id));
        assert!(parent_exec.subtasks.contains(&child2_id));
    }

    #[tokio::test]
    async fn test_checkpoint_creation() {
        let coordinator = TaskCoordinator::new();
        let task = Task::new("Test task".to_owned());
        let task_id = task.id;
        coordinator.register_task(task, None).await.unwrap();

        coordinator
            .create_checkpoint(task_id, "test checkpoint".to_owned())
            .await
            .unwrap();

        let checkpoints = coordinator.state.lock().await.checkpoints.clone();
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].description, "test checkpoint");
    }

    #[tokio::test]
    async fn test_checkpoint_limit() {
        let coordinator = TaskCoordinator::new();
        let task = Task::new("Test task".to_owned());
        let task_id = task.id;
        coordinator.register_task(task, None).await.unwrap();

        // Create more than MAX_CHECKPOINTS
        for idx in 0..=(MAX_CHECKPOINTS + 5) {
            coordinator
                .create_checkpoint(task_id, format!("checkpoint {idx}"))
                .await
                .unwrap();
        }

        let checkpoint_count = coordinator.state.lock().await.checkpoints.len();
        assert_eq!(checkpoint_count, MAX_CHECKPOINTS);
    }

    #[tokio::test]
    async fn test_get_task_status() {
        let coordinator = TaskCoordinator::new();
        let task = Task::new("Test task".to_owned());
        let task_id = task.id;

        coordinator.register_task(task, None).await.unwrap();

        let pending_status = coordinator
            .state
            .lock()
            .await
            .active_tasks
            .get(&task_id)
            .unwrap()
            .status
            .clone();
        assert_eq!(pending_status, TaskStatus::Pending);

        coordinator.start_task(task_id).await.unwrap();

        let in_progress_status = coordinator
            .state
            .lock()
            .await
            .active_tasks
            .get(&task_id)
            .unwrap()
            .status
            .clone();
        assert_eq!(in_progress_status, TaskStatus::InProgress);
    }

    #[tokio::test]
    async fn test_nonexistent_task() {
        let coordinator = TaskCoordinator::new();
        let fake_id = TaskId::default();

        let result = coordinator.start_task(fake_id).await;
        assert!(result.is_err());

        let _result = coordinator
            .complete_subtask(fake_id, create_test_result(fake_id))
            .await;
        // This won't error because complete_subtask just inserts into completed_tasks
        // But we can verify the task isn't in active_tasks
        assert!(
            !coordinator
                .state
                .lock()
                .await
                .active_tasks
                .contains_key(&fake_id)
        );
    }

    #[tokio::test]
    async fn test_task_stats() {
        let coordinator = TaskCoordinator::new();

        // Create various tasks
        let task1 = Task::new("Task 1".to_owned());
        let task1_id = task1.id;
        coordinator.register_task(task1, None).await.unwrap();

        let task2 = Task::new("Task 2".to_owned());
        let task2_id = task2.id;
        coordinator.register_task(task2, None).await.unwrap();

        let task3 = Task::new("Task 3".to_owned());
        coordinator.register_task(task3, None).await.unwrap();

        // Start some tasks
        coordinator.start_task(task1_id).await.unwrap();
        coordinator.start_task(task2_id).await.unwrap();

        // Complete one task
        coordinator
            .complete_subtask(task1_id, create_test_result(task1_id))
            .await
            .unwrap();

        let stats = coordinator.get_stats().await;
        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 0);
    }

    #[tokio::test]
    async fn test_decompose_task() {
        let coordinator = TaskCoordinator::new();
        let parent_task = Task::new("Parent task".to_owned());
        let parent_id = parent_task.id;

        coordinator
            .register_task(parent_task, None)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register parent task");

        let subtask_specs = vec![
            Subtask::new("Subtask 1".to_owned(), 1),
            Subtask::new("Subtask 2".to_owned(), 2),
        ];

        let subtasks = coordinator
            .decompose_task(parent_id, subtask_specs)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to decompose task");

        assert_eq!(subtasks.len(), 2);

        // Verify parent status changed to Waiting
        let parent_exec = coordinator
            .state
            .lock()
            .await
            .active_tasks
            .get(&parent_id)
            .expect("Parent task not found")
            .clone();
        assert_eq!(parent_exec.status, TaskStatus::WaitingForSubtasks);
        assert_eq!(parent_exec.subtasks.len(), 2);
    }

    #[tokio::test]
    async fn test_decompose_task_not_found() {
        let coordinator = TaskCoordinator::new();
        let fake_id = TaskId::default();

        let subtask_specs = vec![Subtask::new("Subtask".to_owned(), 5)];

        let result = coordinator.decompose_task(fake_id, subtask_specs).await;
        result.unwrap_err();
    }

    #[tokio::test]
    async fn test_concurrent_subtask_completion() {
        let coordinator = TaskCoordinator::new();
        let parent_task = Task::new("Parent".to_owned());
        let parent_id = parent_task.id;

        coordinator
            .register_task(parent_task, None)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register parent");

        // Create 3 subtasks
        let child1 = Task::new("Child 1".to_owned());
        let child1_id = child1.id;
        let child2 = Task::new("Child 2".to_owned());
        let child2_id = child2.id;
        let child3 = Task::new("Child 3".to_owned());
        let child3_id = child3.id;

        coordinator
            .register_task(child1, Some(parent_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register child1");
        coordinator
            .register_task(child2, Some(parent_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register child2");
        coordinator
            .register_task(child3, Some(parent_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register child3");

        // Complete them concurrently
        let coord1 = coordinator.clone();
        let coord2 = coordinator.clone();
        let coord3 = coordinator.clone();

        let handle1 = spawn(async move {
            coord1
                .complete_subtask(child1_id, create_test_result(child1_id))
                .await
        });
        let handle2 = spawn(async move {
            coord2
                .complete_subtask(child2_id, create_test_result(child2_id))
                .await
        });
        let handle3 = spawn(async move {
            coord3
                .complete_subtask(child3_id, create_test_result(child3_id))
                .await
        });

        handle1
            .await
            .map_err(|err| err.to_string())
            .expect("Task 1 panicked")
            .map_err(|err| err.to_string())
            .expect("Task 1 failed");
        handle2
            .await
            .map_err(|err| err.to_string())
            .expect("Task 2 panicked")
            .map_err(|err| err.to_string())
            .expect("Task 2 failed");
        handle3
            .await
            .map_err(|err| err.to_string())
            .expect("Task 3 panicked")
            .map_err(|err| err.to_string())
            .expect("Task 3 failed");

        // Verify parent is now completed
        let parent_exec = coordinator
            .state
            .lock()
            .await
            .active_tasks
            .get(&parent_id)
            .expect("Parent not found")
            .clone();
        assert_eq!(parent_exec.status, TaskStatus::Completed);
        assert_eq!(parent_exec.completed_subtasks.len(), 3);
    }

    #[tokio::test]
    async fn test_get_subtasks() {
        let coordinator = TaskCoordinator::new();
        let parent = Task::new("Parent".to_owned());
        let parent_id = parent.id;

        coordinator
            .register_task(parent, None)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register parent");

        let child1 = Task::new("Child 1".to_owned());
        let child2 = Task::new("Child 2".to_owned());

        coordinator
            .register_task(child1.clone(), Some(parent_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register child1");
        coordinator
            .register_task(child2.clone(), Some(parent_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register child2");

        let subtasks = coordinator
            .get_subtasks(parent_id)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to get subtasks");
        assert_eq!(subtasks.len(), 2);
    }

    #[tokio::test]
    async fn test_get_progress() {
        let coordinator = TaskCoordinator::new();
        let parent = Task::new("Parent".to_owned());
        let parent_id = parent.id;

        coordinator
            .register_task(parent, None)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register parent");

        let child1 = Task::new("Child 1".to_owned());
        let child1_id = child1.id;
        let child2 = Task::new("Child 2".to_owned());

        coordinator
            .register_task(child1, Some(parent_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register child1");
        coordinator
            .register_task(child2, Some(parent_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register child2");

        // Initial progress should be 0%
        let progress = coordinator
            .get_progress(parent_id)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to get progress");
        assert_eq!(progress.total_subtasks, 2);
        assert_eq!(progress.completed_subtasks, 0);
        assert!((progress.progress_percent - 0.0).abs() < f32::EPSILON);

        // Complete one subtask
        coordinator
            .complete_subtask(child1_id, create_test_result(child1_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to complete subtask");

        // Progress should be 50%
        let final_progress = coordinator
            .get_progress(parent_id)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to get progress");
        assert_eq!(final_progress.completed_subtasks, 1);
        assert!((final_progress.progress_percent - 50.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_is_ready() {
        let coordinator = TaskCoordinator::new();
        let task = Task::new("Test task".to_owned());
        let task_id = task.id;

        coordinator
            .register_task(task, None)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register task");

        // Should be ready when pending
        let ready = coordinator
            .is_ready(task_id)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to check if ready");
        assert!(ready);

        // Should not be ready when in progress
        coordinator
            .start_task(task_id)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to start task");
        let ready_in_progress = coordinator
            .is_ready(task_id)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to check if ready");
        assert!(!ready_in_progress);
    }

    #[tokio::test]
    async fn test_cleanup_old_tasks() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let coordinator = TaskCoordinator::new();

        let task1 = Task::new("Task 1".to_owned());
        let task1_id = task1.id;
        coordinator
            .register_task(task1, None)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register task1");

        coordinator
            .start_task(task1_id)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to start task1");
        coordinator
            .complete_subtask(task1_id, create_test_result(task1_id))
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to complete task1");

        // Manually set task1 to be old by backdating its timestamp
        {
            let mut state = coordinator.state.lock().await;
            if let Some(execution) = state.active_tasks.get_mut(&task1_id) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |duration| duration.as_secs());
                // Set timestamp to 2 seconds ago
                execution.updated_at = now.saturating_sub(2);
            }
        }

        let task2 = Task::new("Task 2".to_owned());
        let task2_id = task2.id;
        coordinator
            .register_task(task2, None)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to register task2");

        // Cleanup tasks older than 1 second (should remove task1 but not task2)
        let removed = coordinator
            .cleanup_old_tasks(1)
            .await
            .map_err(|err| err.to_string())
            .expect("Failed to cleanup");

        // task1 should be removed
        assert_eq!(removed, 1);

        {
            let state = coordinator.state.lock().await;
            assert!(!state.active_tasks.contains_key(&task1_id));
            assert!(state.active_tasks.contains_key(&task2_id));
            drop(state);
        }
    }
}
