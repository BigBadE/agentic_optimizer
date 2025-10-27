//! Unified test fixture format.
//!
//! This module defines the complete fixture format for unified integration tests.
//! All tests use the same format with optional verification layers.

use merlin_deps::serde_json::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete test fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFixture {
    /// Test name
    pub name: String,
    /// Test description
    pub description: String,
    /// Test tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Setup configuration
    #[serde(default)]
    pub setup: SetupConfig,
    /// Event sequence
    pub events: Vec<TestEvent>,
    /// Final verification
    #[serde(default)]
    pub final_verify: FinalVerify,
}

/// Setup configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetupConfig {
    /// Files to create before test
    #[serde(default)]
    pub files: HashMap<String, String>,
    /// Environment variables to set
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    /// Terminal size (width, height)
    pub terminal_size: Option<(u16, u16)>,
}

/// Test event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TestEvent {
    /// User input event
    UserInput(UserInputEvent),
    /// Key press event
    KeyPress(KeyPressEvent),
    /// LLM response event
    LlmResponse(LlmResponseEvent),
    /// Wait event
    Wait(WaitEvent),
}

/// User input event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputEvent {
    /// Event data
    pub data: UserInputData,
    /// Optional verification
    #[serde(default)]
    pub verify: VerifyConfig,
}

/// User input data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputData {
    /// Text to input
    pub text: String,
    /// Whether to submit
    #[serde(default)]
    pub submit: bool,
}

/// Key press event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPressEvent {
    /// Event data
    pub data: KeyPressData,
    /// Optional verification
    #[serde(default)]
    pub verify: VerifyConfig,
}

/// Key press data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPressData {
    /// Key to press
    pub key: String,
}

/// LLM response event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponseEvent {
    /// Trigger configuration
    pub trigger: TriggerConfig,
    /// Response configuration
    pub response: ResponseConfig,
    /// Optional verification
    #[serde(default)]
    pub verify: VerifyConfig,
}

/// Trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Pattern to match
    pub pattern: String,
    /// Match type
    pub match_type: MatchType,
}

/// Match type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    /// Exact string match
    Exact,
    /// Contains substring
    Contains,
    /// Regex match
    Regex,
}

/// Response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseConfig {
    /// TypeScript code lines
    pub typescript: Vec<String>,
}

/// Wait event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitEvent {
    /// Event data
    pub data: WaitData,
}

/// Wait data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitData {
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// What to wait for
    pub wait_for: Option<String>,
}

/// Verification configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifyConfig {
    /// Execution verification
    pub execution: Option<ExecutionVerify>,
    /// File verification
    pub files: Option<Vec<FileVerify>>,
    /// UI verification
    pub ui: Option<UiVerify>,
    /// State verification
    pub state: Option<StateVerify>,
}

/// Execution verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionVerify {
    /// Return type
    pub return_type: Option<String>,
    /// Return value matches exactly (for arrays and primitives)
    pub return_value_matches: Option<Value>,
    /// Return value contains these key-value pairs (for objects)
    pub return_value_contains: Option<Value>,
    /// Error occurred (error message substring expected)
    #[serde(default)]
    pub error_occurred: Option<String>,
    /// All tasks completed
    pub all_tasks_completed: Option<bool>,
    /// Validation passed
    pub validation_passed: Option<bool>,
}

/// File verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVerify {
    /// File path (relative to workspace)
    pub path: String,
    /// File exists
    pub exists: Option<bool>,
    /// File contains patterns
    #[serde(default)]
    pub contains: Vec<String>,
    /// File does not contain patterns
    #[serde(default)]
    pub not_contains: Vec<String>,
    /// Exact file content
    pub exact_content: Option<String>,
    /// File size greater than
    pub size_gt: Option<usize>,
    /// File size less than
    pub size_lt: Option<usize>,
}

/// UI verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiVerify {
    /// Input text
    pub input_text: Option<String>,
    /// Input cleared
    pub input_cleared: Option<bool>,
    /// Cursor position
    pub cursor_position: Option<usize>,
    /// Focused pane
    pub focused_pane: Option<String>,
    /// Focus changed
    pub focus_changed: Option<bool>,
    /// Number of tasks displayed
    pub tasks_displayed: Option<usize>,
    /// Task status
    pub task_status: Option<String>,
    /// Task tree expanded
    pub task_tree_expanded: Option<bool>,
    /// Output contains patterns
    #[serde(default)]
    pub output_contains: Vec<String>,
    /// Output does not contain patterns
    #[serde(default)]
    pub output_not_contains: Vec<String>,
    /// Snapshot file path
    pub snapshot: Option<String>,
    /// Final state
    pub final_state: Option<String>,
    /// All tasks completed
    pub all_tasks_completed: Option<bool>,
    /// Task created
    pub task_created: Option<bool>,
    /// Task descriptions that should be visible
    #[serde(default)]
    pub task_descriptions_visible: Vec<String>,
    /// Progress percentage (0-100)
    pub progress_percentage: Option<u8>,
    /// Placeholder text is visible
    pub placeholder_visible: Option<bool>,
    /// Number of pending tasks
    pub pending_tasks_count: Option<usize>,
    /// Number of running tasks
    pub running_tasks_count: Option<usize>,
    /// Number of completed tasks
    pub completed_tasks_count: Option<usize>,
    /// Number of failed tasks
    pub failed_tasks_count: Option<usize>,
    /// Selected task description contains
    pub selected_task_contains: Option<String>,
    /// Thread-specific verification
    /// Number of active threads
    pub thread_count: Option<usize>,
    /// Selected thread ID (if any)
    pub selected_thread_id: Option<String>,
    /// Thread list is visible (side-by-side mode)
    pub thread_list_visible: Option<bool>,
    /// Thread names that should be visible
    #[serde(default)]
    pub thread_names_visible: Vec<String>,
    /// Thread colors (emoji strings) that should be visible
    #[serde(default)]
    pub thread_colors_visible: Vec<String>,
    /// Thread message counts (in order)
    #[serde(default)]
    pub thread_message_counts: Vec<usize>,
    /// Queued input prompt is visible
    pub queued_input_prompt_visible: Option<bool>,
    /// Queued input text matches
    pub queued_input_text: Option<String>,
    /// Cancel is requested
    pub cancel_requested: Option<bool>,
}

/// State verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateVerify {
    /// Conversation count
    pub conversation_count: Option<usize>,
    /// Selected task ID
    pub selected_task: Option<String>,
    /// Vector cache status
    pub vector_cache_status: Option<String>,
}

/// Final verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FinalVerify {
    /// Execution verification
    pub execution: Option<ExecutionVerify>,
    /// File verification
    pub files: Option<Vec<FileVerify>>,
    /// UI verification
    pub ui: Option<UiVerify>,
    /// State verification
    pub state: Option<StateVerify>,
}

/// Event type enum for pattern matching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// User input
    UserInput,
    /// Key press
    KeyPress,
    /// LLM response
    LlmResponse,
    /// Wait
    Wait,
}

impl TestEvent {
    /// Get event type
    #[must_use]
    pub fn event_type(&self) -> EventType {
        match self {
            Self::UserInput(_) => EventType::UserInput,
            Self::KeyPress(_) => EventType::KeyPress,
            Self::LlmResponse(_) => EventType::LlmResponse,
            Self::Wait(_) => EventType::Wait,
        }
    }

    /// Get verification config
    #[must_use]
    pub fn verify_config(&self) -> &VerifyConfig {
        match self {
            Self::UserInput(event) => &event.verify,
            Self::KeyPress(event) => &event.verify,
            Self::LlmResponse(event) => &event.verify,
            Self::Wait(_) => {
                // Wait events don't have verification
                static EMPTY: VerifyConfig = VerifyConfig {
                    execution: None,
                    files: None,
                    ui: None,
                    state: None,
                };
                &EMPTY
            }
        }
    }
}
