//! Types for E2E scenario testing

use crate::serde_json::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete E2E test scenario
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scenario {
    /// Scenario name
    pub name: String,
    /// Description of what this scenario tests
    pub description: String,
    /// Initial setup before scenario runs
    #[serde(default)]
    pub setup: ScenarioSetup,
    /// Ordered list of steps to execute
    pub steps: Vec<ScenarioStep>,
}

/// Initial setup for a scenario
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ScenarioSetup {
    /// Files to create in the workspace before test
    #[serde(default)]
    pub workspace_files: Vec<WorkspaceFile>,
    /// Environment variables to set
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// Terminal dimensions [width, height]
    #[serde(default = "default_terminal_size")]
    pub terminal_size: [u16; 2],
}

const fn default_terminal_size() -> [u16; 2] {
    [80, 24]
}

/// A file to create in the test workspace
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkspaceFile {
    /// Relative path from workspace root
    pub path: String,
    /// File content
    pub content: String,
}

/// A single step in the scenario
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioStep {
    /// Step number (for readability)
    #[serde(default)]
    pub step: usize,
    /// Action to perform
    pub action: StepAction,
    /// Expected outcomes after action
    #[serde(default)]
    pub expectations: StepExpectations,
}

/// Actions that can be performed in a step
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepAction {
    /// Type text into the TUI input
    UserInput {
        /// Data for user input
        data: UserInputData,
    },
    /// Wait for a specific condition
    Wait {
        /// Wait configuration
        data: WaitData,
    },
    /// Inject a mock agent response
    MockAgentResponse {
        /// Mock response data
        data: MockResponseData,
    },
    /// Send a key event to the TUI
    KeyPress {
        /// Key event data
        data: KeyPressData,
    },
    /// Wait for background tasks to complete
    WaitForTasks {
        /// Task wait configuration
        data: WaitForTasksData,
    },
}

/// Data for user input action
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserInputData {
    /// Text to type
    pub text: String,
    /// Whether to submit (press Enter)
    #[serde(default)]
    pub submit: bool,
}

/// Data for wait action
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WaitData {
    /// Milliseconds to wait
    #[serde(default)]
    pub duration_ms: u64,
    /// Alternative: wait for specific condition
    #[serde(default)]
    pub condition: Option<WaitCondition>,
}

/// Conditions to wait for
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitCondition {
    /// Wait for task count to reach value
    TaskCount(usize),
    /// Wait for specific task status
    TaskStatus {
        /// Task index
        task_index: usize,
        /// Expected status
        status: String,
    },
    /// Wait for UI to update
    UiUpdate,
}

/// Mock agent response data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MockResponseData {
    /// Pattern to match in query
    pub pattern: String,
    /// Response text or TypeScript code
    pub response: ResponseContent,
}

/// Response content types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ResponseContent {
    /// Plain text response
    Text(String),
    /// Array of lines (joined with newlines)
    Lines(Vec<String>),
}

impl ResponseContent {
    /// Convert to string representation
    #[must_use]
    pub fn as_string(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Lines(lines) => lines.join("\n"),
        }
    }
}

/// Key press data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyPressData {
    /// Key code
    pub key: String,
    /// Modifiers
    #[serde(default)]
    pub modifiers: Vec<String>,
}

/// Wait for tasks data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WaitForTasksData {
    /// Maximum time to wait in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Expected final task count
    #[serde(default)]
    pub expected_count: Option<usize>,
}

const fn default_timeout() -> u64 {
    5000
}

/// Expected outcomes after a step
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct StepExpectations {
    /// Expected UI state
    #[serde(default)]
    pub ui_state: Option<UiExpectations>,
    /// Expected task state
    #[serde(default)]
    pub task_state: Option<TaskExpectations>,
    /// Expected events
    #[serde(default)]
    pub events: Vec<EventExpectation>,
}

/// UI state expectations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UiExpectations {
    /// Expected snapshot file name
    #[serde(default)]
    pub snapshot: Option<String>,
    /// Expected task count
    #[serde(default)]
    pub task_count: Option<usize>,
    /// Expected input text
    #[serde(default)]
    pub input_text: Option<String>,
    /// Expected focused element
    #[serde(default)]
    pub focused: Option<FocusedElement>,
}

/// Focusable UI elements
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusedElement {
    /// Input area is focused
    Input,
    /// Output area is focused
    Output,
    /// Task tree is focused
    Tasks,
}

/// Task state expectations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskExpectations {
    /// Total number of tasks
    #[serde(default)]
    pub total: Option<usize>,
    /// Task at specific index
    #[serde(default)]
    pub tasks: Vec<TaskExpectation>,
}

/// Expectation for a specific task
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskExpectation {
    /// Task index in tree
    pub index: usize,
    /// Expected description
    #[serde(default)]
    pub description: Option<String>,
    /// Expected status
    #[serde(default)]
    pub status: Option<String>,
    /// Expected child count
    #[serde(default)]
    pub child_count: Option<usize>,
}

/// Expected event
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventExpectation {
    /// Event type
    #[serde(rename = "type")]
    pub event_type: String,
    /// Additional data to match
    #[serde(default)]
    pub data: HashMap<String, Value>,
}
