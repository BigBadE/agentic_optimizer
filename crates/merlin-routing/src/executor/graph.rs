use crate::{Task, TaskId};
use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef as _;
use petgraph::{Direction, algo};
use std::collections::{HashMap, HashSet};

/// Immutable task dependency graph
#[derive(Debug, Clone)]
pub struct TaskGraph {
    graph: DiGraph<Task, ()>,
}

impl TaskGraph {
    #[must_use]
    pub fn from_tasks(tasks: &[Task]) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        for task in tasks {
            let node = graph.add_node(task.clone());
            node_map.insert(task.id, node);
        }

        for task in tasks {
            let task_node = node_map[&task.id];
            for dep_id in &task.dependencies {
                if let Some(&dep_node) = node_map.get(dep_id) {
                    graph.add_edge(dep_node, task_node, ());
                }
            }
        }

        Self { graph }
    }

    /// Get tasks ready to execute (no pending dependencies)
    #[must_use]
    pub fn ready_tasks(&self, completed: &HashSet<TaskId>) -> Vec<Task> {
        self.graph
            .node_indices()
            .filter_map(|node| {
                let task = &self.graph[node];

                if completed.contains(&task.id) {
                    return None;
                }

                let deps_satisfied =
                    self.graph
                        .edges_directed(node, Direction::Incoming)
                        .all(|edge| {
                            let dep_task = &self.graph[edge.source()];
                            completed.contains(&dep_task.id)
                        });

                deps_satisfied.then(|| task.clone())
            })
            .collect()
    }

    /// Check if all tasks completed
    #[must_use]
    pub fn is_complete(&self, completed: &HashSet<TaskId>) -> bool {
        self.graph.node_count() == completed.len()
    }

    /// Detect cycles (invalid graph)
    #[must_use]
    pub fn has_cycles(&self) -> bool {
        algo::is_cyclic_directed(&self.graph)
    }

    /// Get total task count
    #[must_use]
    pub fn task_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get all tasks
    #[must_use]
    pub fn tasks(&self) -> Vec<Task> {
        self.graph.node_weights().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Task;

    #[test]
    /// # Panics
    ///
    /// Panics if assertions about ready task counts or IDs fail.
    fn test_task_graph_ready_tasks() {
        let task_a = Task::new("Task A".to_owned());
        let task_b = Task::new("Task B".to_owned()).with_dependencies(vec![task_a.id]);

        let graph = TaskGraph::from_tasks(&[task_a.clone(), task_b.clone()]);
        let mut completed = HashSet::new();

        let ready = graph.ready_tasks(&completed);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, task_a.id);

        completed.insert(task_a.id);
        let ready_after = graph.ready_tasks(&completed);
        assert_eq!(ready_after.len(), 1);
        assert_eq!(ready_after[0].id, task_b.id);
    }

    #[test]
    /// # Panics
    ///
    /// Panics if the constructed graph incorrectly reports cycles.
    fn test_task_graph_cycle_detection() {
        let task_a = Task::new("Task A".to_owned());
        let task_b = Task::new("Task B".to_owned()).with_dependencies(vec![task_a.id]);

        let graph = TaskGraph::from_tasks(&[task_a, task_b]);
        assert!(!graph.has_cycles());
    }
}
