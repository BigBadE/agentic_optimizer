//! Integration tests for the routing system
//!
//! These tests verify end-to-end functionality of the routing architecture.
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_routing::{
    ConflictAwareTaskGraph, ContextRequirements, RoutingConfig, RoutingOrchestrator, Task,
    TaskGraph, WorkspaceState,
};
use std::collections::HashSet;
use std::fs::canonicalize;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_orchestrator_basic() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);

    let analysis_result = orchestrator.analyze_request("Add a comment").await;
    assert!(
        analysis_result.is_ok(),
        "analysis error: {:?}",
        analysis_result.as_ref().err()
    );
}

#[tokio::test]
async fn test_task_analysis_decomposition() {
    let config = RoutingConfig::default();
    let orchestrator = RoutingOrchestrator::new(config);

    let simple_result = orchestrator.analyze_request("Add a comment").await;
    assert!(simple_result.is_ok());
    let simple_analysis = simple_result.unwrap();
    assert!(
        !simple_analysis.tasks.is_empty(),
        "Analysis should produce at least one task"
    );

    let complex_result = orchestrator
        .analyze_request("Refactor the entire codebase to use async/await")
        .await;
    assert!(complex_result.is_ok());
    let complex_analysis = complex_result.unwrap();
    assert!(
        !complex_analysis.tasks.is_empty(),
        "Complex analysis should produce tasks"
    );
}

#[tokio::test]
async fn test_task_graph_basic() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned()).with_dependencies(vec![task_a.id]);

    let graph = TaskGraph::from_tasks(&[task_a.clone(), task_b.clone()]);

    let completed = HashSet::new();
    let ready = graph.ready_tasks(&completed);

    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, task_a.id);

    let mut completed_with_a = HashSet::new();
    completed_with_a.insert(task_a.id);
    let ready_after = graph.ready_tasks(&completed_with_a);

    assert_eq!(ready_after.len(), 1);
    assert_eq!(ready_after[0].id, task_b.id);
}

#[tokio::test]
async fn test_conflict_aware_execution() {
    let file = PathBuf::from("test.rs");

    let task_a = Task::new("Task A".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![file.clone()]));

    let task_b = Task::new("Task B".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![file]));

    let graph = ConflictAwareTaskGraph::from_tasks(&[task_a.clone(), task_b]);

    let completed = HashSet::new();
    let running = HashSet::new();

    let ready = graph.ready_non_conflicting_tasks(&completed, &running);
    assert_eq!(ready.len(), 2, "Both tasks ready when nothing running");

    let mut running_with_a = HashSet::new();
    running_with_a.insert(task_a.id);
    let ready_after = graph.ready_non_conflicting_tasks(&completed, &running_with_a);

    assert_eq!(
        ready_after.len(),
        0,
        "Task B blocked due to file conflict with running Task A"
    );
}

#[tokio::test]
async fn test_workspace_state_operations() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let workspace = WorkspaceState::new(temp_dir.path().to_path_buf());

    // Canonicalize to avoid Windows short (8.3) vs long path inconsistencies on CI
    let left = canonicalize(workspace.root_path()).expect("canonicalize left");
    let right = canonicalize(temp_dir.path()).expect("canonicalize right");
    assert_eq!(left, right);

    let test_file = PathBuf::from("test.txt");
    let content = workspace.read_file(&test_file).await;
    assert!(content.is_none(), "Non-existent file should return None");
}

#[tokio::test]
async fn test_task_graph_cycle_detection() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned()).with_dependencies(vec![task_a.id]);
    let task_c = Task::new("Task C".to_owned()).with_dependencies(vec![task_b.id, task_a.id]);

    let graph = TaskGraph::from_tasks(&[task_a, task_b, task_c]);

    assert!(!graph.has_cycles(), "Valid DAG should not have cycles");
}

#[tokio::test]
async fn test_task_graph_completion() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned());

    let graph = TaskGraph::from_tasks(&[task_a.clone(), task_b.clone()]);

    let mut completed = HashSet::new();
    assert!(!graph.is_complete(&completed));

    completed.insert(task_a.id);
    assert!(!graph.is_complete(&completed));

    completed.insert(task_b.id);
    assert!(graph.is_complete(&completed));
}

#[tokio::test]
async fn test_conflict_detection_different_files() {
    let task_a = Task::new("Task A".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![PathBuf::from("a.rs")]));

    let task_b = Task::new("Task B".to_owned())
        .with_context(ContextRequirements::default().with_files(vec![PathBuf::from("b.rs")]));

    let task_b_id = task_b.id;
    let graph = ConflictAwareTaskGraph::from_tasks(&[task_a.clone(), task_b]);

    let completed = HashSet::new();
    let mut running = HashSet::new();
    running.insert(task_a.id);

    let ready = graph.ready_non_conflicting_tasks(&completed, &running);

    assert_eq!(
        ready.len(),
        1,
        "Task B ready - different file, no conflict with running Task A"
    );
    assert_eq!(ready[0].id, task_b_id);
}

#[tokio::test]
async fn test_orchestrator_with_custom_config() {
    let mut config = RoutingConfig::default();
    config.execution.max_concurrent_tasks = 4;
    config.execution.enable_conflict_detection = true;
    config.validation.early_exit = true;

    let orchestrator = RoutingOrchestrator::new(config.clone());

    let analysis = orchestrator.analyze_request("Simple task").await;
    analysis.unwrap();
}

#[tokio::test]
async fn test_task_with_multiple_dependencies() {
    let task_a = Task::new("Task A".to_owned());
    let task_b = Task::new("Task B".to_owned());
    let task_c = Task::new("Task C".to_owned()).with_dependencies(vec![task_a.id, task_b.id]);

    let graph = TaskGraph::from_tasks(&[task_a.clone(), task_b.clone(), task_c.clone()]);

    let completed = HashSet::new();
    let ready = graph.ready_tasks(&completed);

    assert_eq!(ready.len(), 2, "Both A and B should be ready initially");

    let mut completed_with_a = HashSet::new();
    completed_with_a.insert(task_a.id);
    let ready_after_a = graph.ready_tasks(&completed_with_a);

    assert_eq!(
        ready_after_a.len(),
        1,
        "Only B should be ready after A completes"
    );
    assert_eq!(ready_after_a[0].id, task_b.id);

    completed_with_a.insert(task_b.id);
    let ready_after_both = graph.ready_tasks(&completed_with_a);

    assert_eq!(
        ready_after_both.len(),
        1,
        "C should be ready after both A and B complete"
    );
    assert_eq!(ready_after_both[0].id, task_c.id);
}
