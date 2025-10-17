//! Tests for self-determining task execution.

#![allow(
    clippy::min_ident_chars,
    clippy::tests_outside_test_module,
    clippy::missing_panics_doc,
    clippy::expect_used,
    clippy::redundant_clone,
    clippy::assertions_on_result_states,
    clippy::unwrap_used,
    missing_docs,
    reason = "Integration tests have different conventions"
)]

use merlin_routing::{
    AgentExecutor, Complexity, ContextFetcher, ExecutionMode, RoutingConfig, StrategyRouter,
    SubtaskSpec, Task, TaskAction, TaskDecision, ToolRegistry, UiChannel, ValidationPipeline,
};
use std::path::PathBuf;
use std::sync::Arc;
#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_simple_task_skips_assessment() {
    let router = Arc::new(StrategyRouter::with_default_strategies());
    let validator = Arc::new(ValidationPipeline::with_default_stages());
    let tool_registry = Arc::new(ToolRegistry::default());
    let context_fetcher = ContextFetcher::new(PathBuf::from("."));
    let config = RoutingConfig::default();

    let mut executor =
        AgentExecutor::new(router, validator, tool_registry, context_fetcher, config);

    let task = Task::new("hi".to_owned());

    // Create a channel for UI events (we'll ignore the receiver)
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(tx);

    // Simple tasks should execute without assessment
    let result = executor.execute_self_determining(task, ui_channel).await;

    result.unwrap();
}

#[tokio::test]
async fn test_task_action_complete() {
    let decision = TaskDecision {
        action: TaskAction::Complete {
            result: "Task completed successfully".to_owned(),
        },
        reasoning: "Simple task".to_owned(),
        confidence: 0.95,
    };

    match decision.action {
        TaskAction::Complete { result } => {
            assert_eq!(result, "Task completed successfully");
        }
        _ => panic!("Expected Complete action"),
    }
}

#[tokio::test]
async fn test_task_action_decompose() {
    let subtasks = vec![
        SubtaskSpec {
            description: "Subtask 1".to_owned(),
            complexity: Complexity::Simple,
        },
        SubtaskSpec {
            description: "Subtask 2".to_owned(),
            complexity: Complexity::Medium,
        },
    ];

    let decision = TaskDecision {
        action: TaskAction::Decompose {
            subtasks,
            execution_mode: ExecutionMode::Sequential,
        },
        reasoning: "Complex task needs decomposition".to_owned(),
        confidence: 0.85,
    };

    match decision.action {
        TaskAction::Decompose {
            subtasks: result_subtasks,
            execution_mode,
        } => {
            assert_eq!(result_subtasks.len(), 2);
            assert_eq!(execution_mode, ExecutionMode::Sequential);
        }
        _ => panic!("Expected Decompose action"),
    }
}

#[tokio::test]
async fn test_task_action_gather_context() {
    let needs = vec!["file: main.rs".to_owned(), "command: cargo test".to_owned()];

    let decision = TaskDecision {
        action: TaskAction::GatherContext { needs },
        reasoning: "Need more information".to_owned(),
        confidence: 0.75,
    };

    match decision.action {
        TaskAction::GatherContext {
            needs: result_needs,
        } => {
            assert_eq!(result_needs.len(), 2);
            assert!(result_needs[0].contains("file"));
            assert!(result_needs[1].contains("command"));
        }
        _ => panic!("Expected GatherContext action"),
    }
}

#[tokio::test]
async fn test_task_decision_history() {
    let mut task = Task::new("Complex task".to_owned());

    let decision1 = TaskDecision {
        action: TaskAction::GatherContext {
            needs: vec!["context".to_owned()],
        },
        reasoning: "Need context".to_owned(),
        confidence: 0.8,
    };

    let decision2 = TaskDecision {
        action: TaskAction::Complete {
            result: "Done".to_owned(),
        },
        reasoning: "Have enough context".to_owned(),
        confidence: 0.9,
    };

    task.decision_history.push(decision1);
    task.decision_history.push(decision2);

    assert_eq!(task.decision_history.len(), 2);
}

#[tokio::test]
async fn test_subtask_spec_creation() {
    let spec = SubtaskSpec {
        description: "Test subtask".to_owned(),
        complexity: Complexity::Medium,
    };

    assert_eq!(spec.description, "Test subtask");
    assert_eq!(spec.complexity, Complexity::Medium);
}

#[tokio::test]
async fn test_execution_mode_variants() {
    let sequential = ExecutionMode::Sequential;
    let parallel = ExecutionMode::Parallel;

    assert_eq!(sequential, ExecutionMode::Sequential);
    assert_eq!(parallel, ExecutionMode::Parallel);
    assert_ne!(sequential, parallel);
}

#[tokio::test]
async fn test_task_with_complexity() {
    let task = Task::new("Test task".to_owned()).with_complexity(Complexity::Complex);

    assert_eq!(task.complexity, Complexity::Complex);
}

#[tokio::test]
async fn test_multiple_subtasks() {
    let subtasks: Vec<SubtaskSpec> = (0..5)
        .map(|i| SubtaskSpec {
            description: format!("Subtask {i}"),
            complexity: Complexity::Simple,
        })
        .collect();

    assert_eq!(subtasks.len(), 5);
    assert_eq!(subtasks[0].description, "Subtask 0");
    assert_eq!(subtasks[4].description, "Subtask 4");
}

#[test]
fn test_task_decision_serialization() {
    let decision = TaskDecision {
        action: TaskAction::Complete {
            result: "Success".to_owned(),
        },
        reasoning: "Test".to_owned(),
        confidence: 0.95,
    };

    let json = serde_json::to_string(&decision).expect("Serialization failed");
    let deserialized: TaskDecision = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.reasoning, "Test");
    assert!((deserialized.confidence - 0.95).abs() < f32::EPSILON);
}

#[test]
fn test_subtask_spec_serialization() {
    let spec = SubtaskSpec {
        description: "Test".to_owned(),
        complexity: Complexity::Medium,
    };

    let json = serde_json::to_string(&spec).expect("Serialization failed");
    let deserialized: SubtaskSpec = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.description, "Test");
    assert_eq!(deserialized.complexity, Complexity::Medium);
}

#[test]
fn test_execution_mode_serialization() {
    let sequential = ExecutionMode::Sequential;
    let parallel = ExecutionMode::Parallel;

    let json_seq = serde_json::to_string(&sequential).expect("Serialization failed");
    let json_par = serde_json::to_string(&parallel).expect("Serialization failed");

    let deser_seq: ExecutionMode = serde_json::from_str(&json_seq).expect("Deserialization failed");
    let deser_par: ExecutionMode = serde_json::from_str(&json_par).expect("Deserialization failed");

    assert_eq!(deser_seq, ExecutionMode::Sequential);
    assert_eq!(deser_par, ExecutionMode::Parallel);
}
