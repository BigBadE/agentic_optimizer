//! Common test utilities and helpers for merlin-routing tests
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

use merlin_core::{Response, TokenUsage};
use merlin_routing::user_interface::output_tree::OutputTree;
use merlin_routing::user_interface::task_manager::{TaskDisplay, TaskStatus};
use merlin_routing::{TaskId, TaskResult, ValidationResult};
use std::time::Instant;

/// Create a basic test task with sensible defaults
pub fn create_test_task(desc: &str) -> TaskDisplay {
    create_test_task_with_time(desc, Instant::now())
}

/// Create a test task with a specific start time
pub fn create_test_task_with_time(desc: &str, start: Instant) -> TaskDisplay {
    TaskDisplay {
        description: desc.to_string(),
        status: TaskStatus::Running,
        start_time: start,
        end_time: None,
        parent_id: None,
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    }
}

/// Create a child task with a parent
pub fn create_child_task(desc: &str, parent_id: TaskId) -> TaskDisplay {
    TaskDisplay {
        description: desc.to_string(),
        status: TaskStatus::Running,
        start_time: Instant::now(),
        end_time: None,
        parent_id: Some(parent_id),
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    }
}

/// Create a completed task
pub fn create_completed_task(desc: &str) -> TaskDisplay {
    let start = Instant::now();
    TaskDisplay {
        description: desc.to_string(),
        status: TaskStatus::Completed,
        start_time: start,
        end_time: Some(start),
        parent_id: None,
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    }
}

/// Create a failed task
pub fn create_failed_task(desc: &str) -> TaskDisplay {
    let start = Instant::now();
    TaskDisplay {
        description: desc.to_string(),
        status: TaskStatus::Failed,
        start_time: start,
        end_time: Some(start),
        parent_id: None,
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    }
}

/// Create a test task result
pub fn create_test_task_result(task_id: TaskId, text: &str) -> TaskResult {
    TaskResult {
        task_id,
        response: Response {
            text: text.to_string(),
            confidence: 0.95,
            tokens_used: TokenUsage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
            },
            provider: "test".to_string(),
            latency_ms: 1000,
        },
        tier_used: "local".to_string(),
        tokens_used: TokenUsage {
            input: 100,
            output: 50,
            cache_read: 0,
            cache_write: 0,
        },
        validation: ValidationResult {
            passed: true,
            score: 1.0,
            errors: vec![],
            warnings: vec![],
            stages: vec![],
        },
        duration_ms: 1000,
    }
}
