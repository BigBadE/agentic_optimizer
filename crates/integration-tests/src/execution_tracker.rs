//! Execution result tracking for test fixtures.
//!
//! This module provides comprehensive tracking of all execution results throughout
//! a fixture run, allowing verifiers to reference results from any test in the fixture.

use merlin_core::TaskResult;
use merlin_deps::serde_json::Value;
use merlin_tooling::ToolResult;

/// Record of a single test execution
pub struct ExecutionRecord {
    /// Test index (0-based, incremented for each submit)
    test_index: usize,
    /// Execution result from TypeScript
    result: ToolResult<Value>,
    /// Captured output events
    outputs: Vec<String>,
    /// Task result metadata
    task_result: Box<TaskResult>,
}

impl ExecutionRecord {
    /// Create new execution record
    #[must_use]
    pub fn new(
        test_index: usize,
        result: ToolResult<Value>,
        outputs: Vec<String>,
        task_result: Box<TaskResult>,
    ) -> Self {
        Self {
            test_index,
            result,
            outputs,
            task_result,
        }
    }

    /// Get execution result
    pub fn result(&self) -> &ToolResult<Value> {
        &self.result
    }

    /// Get captured outputs
    #[must_use]
    pub fn outputs(&self) -> &[String] {
        &self.outputs
    }

    /// Get task result
    #[must_use]
    pub fn task_result(&self) -> &TaskResult {
        &self.task_result
    }

    /// Get test index
    #[must_use]
    pub fn test_index(&self) -> usize {
        self.test_index
    }
}

/// Tracks all execution results throughout a fixture run
pub struct ExecutionResultTracker {
    /// All execution records in chronological order
    records: Vec<ExecutionRecord>,
    /// Current test index (incremented after each submit)
    current_test_index: usize,
}

impl ExecutionResultTracker {
    /// Create new tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            current_test_index: 0,
        }
    }

    /// Add execution result for current test
    pub fn add_result(
        &mut self,
        result: ToolResult<Value>,
        outputs: Vec<String>,
        task_result: Box<TaskResult>,
    ) {
        let record = ExecutionRecord::new(self.current_test_index, result, outputs, task_result);
        self.records.push(record);
        self.current_test_index += 1;
    }

    /// Get the most recent execution result
    #[must_use]
    pub fn last_result(&self) -> Option<&ExecutionRecord> {
        self.records.last()
    }

    /// Get execution result by test index
    #[must_use]
    pub fn get_result(&self, index: usize) -> Option<&ExecutionRecord> {
        self.records.get(index)
    }

    /// Get all execution records
    #[must_use]
    pub fn all_results(&self) -> &[ExecutionRecord] {
        &self.records
    }

    /// Get current test index (next test to be executed)
    #[must_use]
    pub fn current_test_index(&self) -> usize {
        self.current_test_index
    }

    /// Get total number of executed tests
    #[must_use]
    pub fn executed_count(&self) -> usize {
        self.records.len()
    }
}

impl Default for ExecutionResultTracker {
    fn default() -> Self {
        Self::new()
    }
}
