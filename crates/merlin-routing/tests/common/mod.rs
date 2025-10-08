#![allow(dead_code, reason = "Test utilities used across multiple test files")]
//! Common test utilities and helpers for merlin-routing tests

use merlin_routing::TaskId;
use merlin_routing::user_interface::output_tree::OutputTree;
use merlin_routing::user_interface::task_manager::{TaskDisplay, TaskStatus};
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
