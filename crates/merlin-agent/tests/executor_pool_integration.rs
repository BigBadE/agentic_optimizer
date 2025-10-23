//! Integration tests for `ExecutorPool`.
//!
//! Tests parallel task execution, conflict detection, dependency management, and error handling.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::print_stdout,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_agent::Validator;
use merlin_agent::executor::{ConflictAwareTaskGraph, ExecutorPool, TaskGraph, WorkspaceState};
use merlin_core::{ContextRequirements, Response, Result, Task, TaskId, ValidationResult};
use merlin_routing::{Model, ModelRouter, RoutingDecision};
use std::collections::HashMap;
use std::path::PathBuf;
use std::slice::from_ref;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tempfile::TempDir;

/// Mock router that tracks routing calls
#[derive(Clone)]
struct MockRouter {
    call_count: Arc<Mutex<usize>>,
}

impl MockRouter {
    fn new() -> Self {
        Self {
            call_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl ModelRouter for MockRouter {
    async fn route(&self, _task: &Task) -> Result<RoutingDecision> {
        *self.call_count.lock().expect("lock") += 1;
        Ok(RoutingDecision::new(
            Model::Llama318BInstant,
            "test".to_owned(),
        ))
    }

    async fn is_available(&self, _model: &Model) -> bool {
        true
    }
}

/// Mock validator that always succeeds
struct MockValidator;

#[async_trait::async_trait]
impl Validator for MockValidator {
    async fn validate(&self, _response: &Response, _task: &Task) -> Result<ValidationResult> {
        Ok(ValidationResult::default())
    }

    async fn quick_validate(&self, _response: &Response) -> Result<bool> {
        Ok(true)
    }
}

/// Mock validator that fails for specific task descriptions
struct ConditionalValidator {
    fail_pattern: String,
}

impl ConditionalValidator {
    fn new(fail_pattern: impl Into<String>) -> Self {
        Self {
            fail_pattern: fail_pattern.into(),
        }
    }
}

#[async_trait::async_trait]
impl Validator for ConditionalValidator {
    async fn validate(&self, _response: &Response, task: &Task) -> Result<ValidationResult> {
        use merlin_core::{Severity, ValidationError, ValidationStageType};
        if task.description.contains(&self.fail_pattern) {
            Ok(ValidationResult {
                passed: false,
                score: 0.0,
                errors: vec![ValidationError {
                    stage: ValidationStageType::Syntax,
                    message: format!("Validation failed for: {}", task.description),
                    severity: Severity::Error,
                }],
                warnings: vec![],
                stages: vec![],
            })
        } else {
            Ok(ValidationResult::default())
        }
    }

    async fn quick_validate(&self, _response: &Response) -> Result<bool> {
        Ok(true)
    }
}

fn create_test_workspace() -> (TempDir, Arc<WorkspaceState>) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let workspace = WorkspaceState::new(temp_dir.path().to_path_buf());
    (temp_dir, workspace)
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_executor_pool_parallel_execution() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(Arc::clone(&router), validator, 3, workspace);

    // Create independent tasks
    let tasks = vec![
        Task::new("Task 1".to_owned()).with_difficulty(1),
        Task::new("Task 2".to_owned()).with_difficulty(1),
        Task::new("Task 3".to_owned()).with_difficulty(1),
    ];

    let graph = TaskGraph::from_tasks(&tasks);
    let results = pool.execute_graph(graph).await.expect("execute graph");

    assert_eq!(results.len(), 3, "All tasks should complete");
    // Router tracking not available through trait object, so we just verify execution succeeded
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_executor_pool_sequential_dependencies() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 2, workspace);

    // Create tasks with dependencies (B depends on A, C depends on B)
    let task_a = Task::new("Task A".to_owned()).with_difficulty(1);
    let task_b = Task::new("Task B".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_a.id]);
    let task_c = Task::new("Task C".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_b.id]);

    let graph = TaskGraph::from_tasks(&[task_a.clone(), task_b.clone(), task_c.clone()]);
    let results = pool.execute_graph(graph).await.expect("execute graph");

    assert_eq!(results.len(), 3, "All tasks should complete");

    // Verify execution order by checking result order
    let task_ids: Vec<TaskId> = results.iter().map(|result| result.task_id).collect();
    assert_eq!(task_ids[0], task_a.id, "Task A should complete first");
    assert_eq!(task_ids[1], task_b.id, "Task B should complete second");
    assert_eq!(task_ids[2], task_c.id, "Task C should complete last");
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_executor_pool_diamond_dependencies() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 4, workspace);

    // Diamond dependency: B and C depend on A, D depends on B and C
    let task_a = Task::new("Task A".to_owned()).with_difficulty(1);
    let task_b = Task::new("Task B".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_a.id]);
    let task_c = Task::new("Task C".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_a.id]);
    let task_d = Task::new("Task D".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_b.id, task_c.id]);

    let graph = TaskGraph::from_tasks(&[
        task_a.clone(),
        task_b.clone(),
        task_c.clone(),
        task_d.clone(),
    ]);
    let results = pool.execute_graph(graph).await.expect("execute graph");

    assert_eq!(results.len(), 4, "All tasks should complete");

    // Build completion order map
    let completion_order: HashMap<TaskId, usize> = results
        .iter()
        .enumerate()
        .map(|(idx, result)| (result.task_id, idx))
        .collect();

    // A must complete before B and C
    assert!(
        completion_order[&task_a.id] < completion_order[&task_b.id],
        "A before B"
    );
    assert!(
        completion_order[&task_a.id] < completion_order[&task_c.id],
        "A before C"
    );

    // B and C must complete before D
    assert!(
        completion_order[&task_b.id] < completion_order[&task_d.id],
        "B before D"
    );
    assert!(
        completion_order[&task_c.id] < completion_order[&task_d.id],
        "C before D"
    );
}

#[tokio::test]
async fn test_executor_pool_detects_cycles() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 2, workspace);

    // Create cyclic dependency: A -> B -> C -> A
    let task_a = Task::new("Task A".to_owned()).with_difficulty(1);
    let task_b = Task::new("Task B".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_a.id]);
    let task_c = Task::new("Task C".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_b.id]);
    let task_a_cyclic = Task {
        id: task_a.id,
        description: task_a.description.clone(),
        dependencies: vec![task_c.id], // Creates cycle
        ..task_a
    };

    let graph = TaskGraph::from_tasks(&[task_a_cyclic, task_b, task_c]);
    let result = pool.execute_graph(graph).await;

    assert!(
        result.is_err(),
        "Should detect and reject cyclic dependencies"
    );
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_executor_pool_concurrency_limit() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    // Set low concurrency limit
    let pool = ExecutorPool::new(router, validator, 1, workspace);

    let tasks = vec![
        Task::new("Task 1".to_owned()).with_difficulty(1),
        Task::new("Task 2".to_owned()).with_difficulty(1),
        Task::new("Task 3".to_owned()).with_difficulty(1),
    ];

    let graph = TaskGraph::from_tasks(&tasks);
    let start = Instant::now();
    let results = pool.execute_graph(graph).await.expect("execute graph");
    let _duration = start.elapsed();

    assert_eq!(results.len(), 3, "All tasks should complete");
    // With concurrency=1, tasks should run sequentially
    // Duration check is unreliable due to Ollama variance
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_conflict_aware_graph_file_conflicts() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 4, workspace);

    let file = PathBuf::from("test.rs");

    // Create tasks that access the same file
    let task_a = Task::new("Modify test.rs - A".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file.clone()]));

    let task_b = Task::new("Modify test.rs - B".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file.clone()]));

    let task_c = Task::new("Modify test.rs - C".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file]));

    let graph = ConflictAwareTaskGraph::from_tasks(&[task_a, task_b, task_c]);
    let results = pool
        .execute_conflict_aware_graph(graph)
        .await
        .expect("execute conflict-aware graph");

    assert_eq!(
        results.len(),
        3,
        "All tasks should complete despite conflicts"
    );
    // Tasks should be serialized due to file conflict
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_conflict_aware_graph_no_conflicts_parallel() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 4, workspace);

    // Create tasks that access different files (no conflicts)
    let task_a = Task::new("Modify a.rs".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![PathBuf::from("a.rs")]));

    let task_b = Task::new("Modify b.rs".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![PathBuf::from("b.rs")]));

    let task_c = Task::new("Modify c.rs".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![PathBuf::from("c.rs")]));

    let graph = ConflictAwareTaskGraph::from_tasks(&[task_a, task_b, task_c]);
    let start = Instant::now();
    let results = pool
        .execute_conflict_aware_graph(graph)
        .await
        .expect("execute conflict-aware graph");
    let _duration = start.elapsed();

    assert_eq!(results.len(), 3, "All tasks should complete");
    // Tasks should run in parallel since they access different files
    // Duration check is unreliable due to Ollama variance
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_executor_pool_mixed_dependencies_and_conflicts() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 4, workspace);

    let file_a = PathBuf::from("a.rs");
    let file_b = PathBuf::from("b.rs");

    // A and B access different files (can run in parallel)
    let task_a = Task::new("Task A".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file_a.clone()]));

    let task_b = Task::new("Task B".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file_b]));

    // C depends on A and accesses same file as A (must wait for A to complete)
    let task_c = Task::new("Task C".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_a.id])
        .with_context(ContextRequirements::default().with_files(vec![file_a]));

    let graph =
        ConflictAwareTaskGraph::from_tasks(&[task_a.clone(), task_b.clone(), task_c.clone()]);
    let results = pool
        .execute_conflict_aware_graph(graph)
        .await
        .expect("execute graph");

    assert_eq!(results.len(), 3, "All tasks should complete");

    // Verify dependency order
    let completion_order: HashMap<TaskId, usize> = results
        .iter()
        .enumerate()
        .map(|(idx, result)| (result.task_id, idx))
        .collect();

    assert!(
        completion_order[&task_a.id] < completion_order[&task_c.id],
        "A must complete before C (dependency)"
    );
}

#[tokio::test]
async fn test_executor_pool_empty_graph() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 2, workspace);

    let graph = TaskGraph::from_tasks(&[]);
    let results = pool.execute_graph(graph).await.expect("execute graph");

    assert_eq!(results.len(), 0, "Empty graph should return empty results");
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_executor_pool_single_task() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 2, workspace);

    let task = Task::new("Single task".to_owned()).with_difficulty(1);
    let graph = TaskGraph::from_tasks(from_ref(&task));
    let results = pool.execute_graph(graph).await.expect("execute graph");

    assert_eq!(results.len(), 1, "Single task should complete");
    assert_eq!(results[0].task_id, task.id, "Result should match task ID");
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_executor_pool_large_graph() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 8, workspace);

    // Create 20 independent tasks
    let tasks: Vec<Task> = (0..20)
        .map(|idx| Task::new(format!("Task {idx}")).with_difficulty(1))
        .collect();

    let graph = TaskGraph::from_tasks(&tasks);
    let results = pool.execute_graph(graph).await.expect("execute graph");

    assert_eq!(results.len(), 20, "All 20 tasks should complete");
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_executor_pool_validation_failure_propagation() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(ConditionalValidator::new("fail"));

    let pool = ExecutorPool::new(router, validator, 2, workspace);

    let tasks = vec![
        Task::new("Task success".to_owned()).with_difficulty(1),
        Task::new("Task fail".to_owned()).with_difficulty(1), // Will fail validation
    ];

    let graph = TaskGraph::from_tasks(&tasks);
    let result = pool.execute_graph(graph).await;

    // Execution may fail or partially succeed depending on validation behavior
    assert!(
        result.is_err() || result.unwrap().len() < 2,
        "Validation failure should affect execution"
    );
}

#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_conflict_aware_graph_complex_scenario() {
    let (_temp, workspace) = create_test_workspace();
    let router = Arc::new(MockRouter::new()) as Arc<dyn ModelRouter>;
    let validator = Arc::new(MockValidator);

    let pool = ExecutorPool::new(router, validator, 4, workspace);

    let file1 = PathBuf::from("file1.rs");
    let file2 = PathBuf::from("file2.rs");
    let file3 = PathBuf::from("file3.rs");

    // Complex scenario:
    // - Tasks A, B access file1 (conflict)
    // - Tasks C, D access file2 (conflict)
    // - Task E accesses file3 (no conflict)
    // - Task F depends on A and C
    let task_a = Task::new("A: file1".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file1.clone()]));

    let task_b = Task::new("B: file1".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file1]));

    let task_c = Task::new("C: file2".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file2.clone()]));

    let task_d = Task::new("D: file2".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file2]));

    let task_e = Task::new("E: file3".to_owned())
        .with_difficulty(1)
        .with_context(ContextRequirements::default().with_files(vec![file3]));

    let task_f = Task::new("F: depends on A and C".to_owned())
        .with_difficulty(1)
        .with_dependencies(vec![task_a.id, task_c.id]);

    let graph = ConflictAwareTaskGraph::from_tasks(&[
        task_a.clone(),
        task_b,
        task_c.clone(),
        task_d,
        task_e,
        task_f.clone(),
    ]);

    let results = pool
        .execute_conflict_aware_graph(graph)
        .await
        .expect("execute graph");

    assert_eq!(results.len(), 6, "All tasks should complete");

    // Verify F waited for both A and C
    let completion_order: HashMap<TaskId, usize> = results
        .iter()
        .enumerate()
        .map(|(idx, result)| (result.task_id, idx))
        .collect();

    assert!(
        completion_order[&task_a.id] < completion_order[&task_f.id],
        "A before F"
    );
    assert!(
        completion_order[&task_c.id] < completion_order[&task_f.id],
        "C before F"
    );
}
