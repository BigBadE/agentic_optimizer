//! Execution result tracking for test fixtures.
//!
//! This module provides comprehensive tracking of all execution results throughout
//! a fixture run, allowing verifiers to reference results from any test in the fixture.

use merlin_core::TaskResult;
use merlin_tooling::{ToolError, ToolResult};
use serde_json::Value;
use std::collections::HashMap;

/// Record of a single test execution
pub struct ExecutionRecord {
    /// Execution ID (from event ID, or generated)
    execution_id: String,
    /// Test index (0-based, incremented for each submit)
    test_index: usize,
    /// Execution result from TypeScript - Ok for success, Err for `TaskFailed`
    result: Result<ToolResult<Value>, ToolError>,
    /// Captured output events
    outputs: Vec<String>,
    /// Task result metadata (only present for successful completions)
    task_result: Option<Box<TaskResult>>,
}

impl ExecutionRecord {
    /// Create new execution record from successful task completion
    #[must_use]
    pub fn success(
        execution_id: String,
        test_index: usize,
        result: ToolResult<Value>,
        outputs: Vec<String>,
        task_result: Box<TaskResult>,
    ) -> Self {
        Self {
            execution_id,
            test_index,
            result: Ok(result),
            outputs,
            task_result: Some(task_result),
        }
    }

    /// Create new execution record from task failure
    #[must_use]
    pub fn failure(
        execution_id: String,
        test_index: usize,
        error: ToolError,
        outputs: Vec<String>,
    ) -> Self {
        Self {
            execution_id,
            test_index,
            result: Err(error),
            outputs,
            task_result: None,
        }
    }

    /// Get execution ID
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// Get execution result
    pub fn result(&self) -> &Result<ToolResult<Value>, ToolError> {
        &self.result
    }

    /// Get the execution result as a `ToolResult` for compatibility
    ///
    /// Converts task failures into `ToolError::ExecutionFailed`
    ///
    /// # Errors
    /// Returns `ToolError` if the task failed or tool execution failed
    pub fn as_tool_result(&self) -> ToolResult<&Value> {
        match &self.result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(tool_err)) | Err(tool_err) => Err(tool_err.clone()),
        }
    }

    /// Get captured outputs
    #[must_use]
    pub fn outputs(&self) -> &[String] {
        &self.outputs
    }

    /// Get task result (only available for successful completions)
    #[must_use]
    pub fn task_result(&self) -> Option<&TaskResult> {
        self.task_result.as_deref()
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
    /// ID-based lookup for executions
    records_by_id: HashMap<String, usize>,
    /// Current test index (incremented after each submit)
    current_test_index: usize,
}

impl ExecutionResultTracker {
    /// Create new tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            records_by_id: HashMap::new(),
            current_test_index: 0,
        }
    }

    /// Add successful execution result for current test
    pub fn add_success(
        &mut self,
        execution_id: String,
        result: ToolResult<Value>,
        outputs: Vec<String>,
        task_result: Box<TaskResult>,
    ) {
        let record = ExecutionRecord::success(
            execution_id.clone(),
            self.current_test_index,
            result,
            outputs,
            task_result,
        );
        let index = self.records.len();
        self.records_by_id.insert(execution_id, index);
        self.records.push(record);
        self.current_test_index += 1;
    }

    /// Add failed execution result for current test
    pub fn add_failure(&mut self, execution_id: String, error: ToolError, outputs: Vec<String>) {
        let record = ExecutionRecord::failure(
            execution_id.clone(),
            self.current_test_index,
            error,
            outputs,
        );
        let index = self.records.len();
        self.records_by_id.insert(execution_id, index);
        self.records.push(record);
        self.current_test_index += 1;
    }

    /// Get execution result by ID
    #[must_use]
    pub fn get_by_id(&self, execution_id: &str) -> Option<&ExecutionRecord> {
        let &index = self.records_by_id.get(execution_id)?;
        self.records.get(index)
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
