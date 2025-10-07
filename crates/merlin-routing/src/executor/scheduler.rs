use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use crate::{Task, TaskId};
use super::graph::TaskGraph;

/// Enhanced task graph with file conflict detection
pub struct ConflictAwareTaskGraph {
    graph: TaskGraph,
    file_access_map: HashMap<PathBuf, Vec<TaskId>>,
}

impl ConflictAwareTaskGraph {
    #[must_use]
    pub fn from_tasks(tasks: &[Task]) -> Self {
        let mut file_access_map: HashMap<PathBuf, Vec<TaskId>> = HashMap::new();
        
        for task in tasks {
            for file in &task.context_needs.required_files {
                file_access_map
                    .entry(file.clone())
                    .or_default()
                    .push(task.id);
            }
        }
        
        let graph = TaskGraph::from_tasks(tasks);
        
        Self {
            graph,
            file_access_map,
        }
    }
    
    /// Get ready tasks that don't conflict with running tasks
    #[must_use]
    pub fn ready_non_conflicting_tasks(
        &self,
        completed: &HashSet<TaskId>,
        running: &HashSet<TaskId>,
    ) -> Vec<Task> {
        let base_ready = self.graph.ready_tasks(completed);
        
        base_ready
            .into_iter()
            .filter(|task| {
                !self.conflicts_with_running(task, running)
            })
            .collect()
    }
    
    fn conflicts_with_running(&self, task: &Task, running: &HashSet<TaskId>) -> bool {
        task
            .context_needs
            .required_files
            .iter()
            .any(|file| {
                self.file_access_map
                    .get(file)
                    .is_some_and(|accessing_tasks| {
                        accessing_tasks
                            .iter()
                            .copied()
                            .any(|other_id| running.contains(&other_id) && other_id != task.id)
                    })
            })
    }
    
    /// Check if all tasks completed
    #[must_use]
    pub fn is_complete(&self, completed: &HashSet<TaskId>) -> bool {
        self.graph.is_complete(completed)
    }
    
    /// Detect cycles
    #[must_use]
    pub fn has_cycles(&self) -> bool {
        self.graph.has_cycles()
    }
    
    /// Get total task count
    #[must_use]
    pub fn task_count(&self) -> usize {
        self.graph.task_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContextRequirements, Task};

    #[test]
    /// # Panics
    /// Panics if conflict detection results are not as expected.
    fn test_conflict_detection() {
        let file = PathBuf::from("test.rs");
        
        let task_a = Task::new("Task A".to_owned())
            .with_context(
                ContextRequirements::new()
                    .with_files(vec![file.clone()])
            );
        
        let task_b = Task::new("Task B".to_owned())
            .with_context(
                ContextRequirements::new()
                    .with_files(vec![file])
            );
        
        let graph = ConflictAwareTaskGraph::from_tasks(&[task_a.clone(), task_b]);
        
        let completed = HashSet::new();
        let running = HashSet::new();
        
        // When nothing is running, both tasks are ready (no conflicts with running tasks)
        let ready = graph.ready_non_conflicting_tasks(&completed, &running);
        assert_eq!(ready.len(), 2, "Both tasks ready when nothing running - executor picks one");
        
        // Mark first task as running
        let mut running_after = HashSet::new();
        running_after.insert(task_a.id);
        let ready_after = graph.ready_non_conflicting_tasks(&completed, &running_after);
        // Task A is running, Task B accesses the same file and should be blocked
        // The conflict detection correctly filters out Task B
        assert_eq!(ready_after.len(), 0, "Task B should be blocked due to file conflict with running Task A");
    }
    
    #[test]
    /// # Panics
    /// Panics if non-conflicting tasks are incorrectly blocked.
    fn test_no_conflict_different_files() {
        let task_a = Task::new("Task A".to_owned())
            .with_context(
                ContextRequirements::new()
                    .with_files(vec![PathBuf::from("a.rs")])
            );
        
        let task_b = Task::new("Task B".to_owned())
            .with_context(
                ContextRequirements::new()
                    .with_files(vec![PathBuf::from("b.rs")])
            );
        
        let graph = ConflictAwareTaskGraph::from_tasks(&[task_a.clone(), task_b]);
        
        let completed = HashSet::new();
        let mut running = HashSet::new();
        running.insert(task_a.id);
        
        let ready = graph.ready_non_conflicting_tasks(&completed, &running);
        // Task A is running but Task B uses a different file, so no conflict
        // Task B should be ready to run
        assert_eq!(ready.len(), 1, "Task B ready - different file, no conflict with running Task A");
    }
}
