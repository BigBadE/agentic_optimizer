//! Verification configuration structures for fixture tests

use merlin_deps::serde_json::Value;
use serde::{Deserialize, Serialize};

/// Verification configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyConfig {
    /// Execution verification
    pub execution: Option<ExecutionVerify>,
    /// File verification
    pub files: Option<Vec<FileVerify>>,
    /// UI verification
    pub ui: Option<UiVerify>,
    /// State verification
    pub state: Option<StateVerify>,
    /// Prompt verification
    pub prompt: Option<PromptVerify>,
    /// Context verification
    pub context: Option<ContextVerify>,
    /// Validation verification
    pub validation: Option<ValidationVerify>,
}

impl VerifyConfig {
    /// Returns true if no verification is specified
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.execution.is_none()
            && self.files.is_none()
            && self.ui.is_none()
            && self.state.is_none()
            && self.prompt.is_none()
            && self.context.is_none()
            && self.validation.is_none()
    }
}

/// Execution verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionVerify {
    /// Execution ID to verify (defaults to current event's ID)
    pub execution_id: Option<String>,
    /// Return type
    pub return_type: Option<String>,
    /// Return value matches exactly (for arrays and primitives)
    pub return_value_matches: Option<Value>,
    /// Return value contains these key-value pairs (for objects)
    pub return_value_contains: Option<Value>,
    /// Expected failure message (if test expects execution to fail)
    #[serde(default)]
    pub expected_failure: Option<String>,
    /// Specific tasks that should have failed (success assumed for all others)
    #[serde(default)]
    pub failed_tasks: Vec<String>,
    /// Specific tasks that should be incomplete (success assumed for all others)
    #[serde(default)]
    pub incomplete_tasks: Vec<String>,
    /// Validation stages that should have failed (success assumed for all others)
    #[serde(default)]
    pub validation_failures: Vec<String>,
    /// Minimum retry attempts expected
    #[serde(default)]
    pub min_retry_attempts: Option<usize>,
    /// Maximum retry attempts expected
    #[serde(default)]
    pub max_retry_attempts: Option<usize>,
    /// Whether model tier escalation occurred
    #[serde(default)]
    pub escalation_occurred: Option<bool>,
    /// Whether parallel execution was used
    #[serde(default)]
    pub parallel_execution: Option<bool>,
    /// Whether conflict detection triggered
    #[serde(default)]
    pub conflict_detected: Option<bool>,
}

/// File verification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// `WorkUnit` verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkUnitVerify {
    /// `WorkUnit` exists on the message
    pub exists: Option<bool>,
    /// `WorkUnit` status (`in_progress`, completed, failed, cancelled, retrying)
    pub status: Option<String>,
    /// Number of subtasks
    pub subtask_count: Option<usize>,
    /// Progress percentage (0-100)
    pub progress_percentage: Option<u8>,
    /// Retry count
    pub retry_count: Option<u32>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Model tier used
    pub tier_used: Option<String>,
    /// Whether `WorkUnit` is in a terminal state (completed, failed, cancelled)
    pub is_terminal: Option<bool>,
    /// Subtask titles that should exist
    #[serde(default)]
    pub subtask_titles: Vec<String>,
    /// Number of completed subtasks
    pub completed_subtasks: Option<usize>,
    /// Number of pending subtasks
    pub pending_subtasks: Option<usize>,
    /// Number of in-progress subtasks
    pub in_progress_subtasks: Option<usize>,
    /// Number of failed subtasks
    pub failed_subtasks: Option<usize>,
}

/// UI verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Rendered buffer contains these patterns
    #[serde(default)]
    pub rendered_buffer_contains: Vec<String>,
    /// Rendered buffer does not contain these patterns
    #[serde(default)]
    pub rendered_buffer_not_contains: Vec<String>,
    /// Rendered buffer region/box checks
    #[serde(default)]
    pub rendered_buffer_regions: Vec<RenderedRegionVerify>,
    /// `WorkUnit` verification for the last completed task in the current thread
    pub work_unit: Option<WorkUnitVerify>,
}

/// Rendered region/box verification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderedRegionVerify {
    /// Region name (for error messages): `tasks`, `output`, `input`, `threads`
    pub region: String,
    /// Patterns that must appear in this region
    #[serde(default)]
    pub contains: Vec<String>,
    /// Patterns that must NOT appear in this region
    #[serde(default)]
    pub not_contains: Vec<String>,
    /// Exact line sequences that must appear (in order, but not necessarily consecutive)
    #[serde(default)]
    pub lines_in_order: Vec<String>,
}

/// State verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateVerify {
    /// Conversation count
    pub conversation_count: Option<usize>,
    /// Selected task ID
    pub selected_task: Option<String>,
    /// Vector cache status
    pub vector_cache_status: Option<String>,
    /// `WorkUnit` verification
    pub work_unit: Option<WorkUnitVerify>,
}

/// Prompt verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptVerify {
    /// Prompt file name (e.g., `task_assessment`, `typescript_agent`)
    /// Will check that the captured prompt matches the header from `prompts/{prompt_file}.md`
    pub prompt_file: Option<String>,
    /// Patterns that should be in the prompt
    #[serde(default)]
    pub contains: Vec<String>,
    /// Patterns that should NOT be in the prompt
    #[serde(default)]
    pub not_contains: Vec<String>,
    /// Tool signatures that should be present
    #[serde(default)]
    pub has_tool_signatures: Vec<String>,
    /// Type definitions that should be present
    #[serde(default)]
    pub has_type_definitions: Vec<String>,
}

/// Context verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextVerify {
    /// Files that should be included in context
    #[serde(default)]
    pub included_files: Vec<String>,
    /// Files that should be excluded from context
    #[serde(default)]
    pub excluded_files: Vec<String>,
    /// Minimum number of files in context
    pub min_files: Option<usize>,
    /// Maximum number of files in context
    pub max_files: Option<usize>,
    /// Chunking was performed
    pub chunking_performed: Option<bool>,
    /// Semantic search was used
    pub semantic_search_used: Option<bool>,
    /// Token limit was enforced
    pub token_limit_enforced: Option<bool>,
}

/// Validation verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationVerify {
    /// Validation stages that should have run
    #[serde(default)]
    pub stages_run: Vec<String>,
    /// Validation stages that should have passed
    #[serde(default)]
    pub stages_passed: Vec<String>,
    /// Validation stages that should have failed
    #[serde(default)]
    pub stages_failed: Vec<String>,
    /// Citations were checked
    pub citations_checked: Option<bool>,
    /// Citation warnings were issued
    pub citation_warnings: Option<bool>,
    /// Syntax validation was performed
    pub syntax_validated: Option<bool>,
    /// Build validation was performed
    pub build_validated: Option<bool>,
    /// Early exit occurred
    pub early_exit: Option<bool>,
}

/// Final verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
