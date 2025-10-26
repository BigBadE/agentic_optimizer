use merlin_core::ThreadId;
use merlin_routing::TaskId;
use std::collections::{HashMap, HashSet};

/// Maximum number of conversation entries to retain
const MAX_CONVERSATION_HISTORY: usize = 50;

/// Main UI state
#[derive(Default)]
pub struct UiState {
    /// Active task identifier
    pub active_task_id: Option<TaskId>,
    /// Set of currently running tasks
    pub active_running_tasks: HashSet<TaskId>,
    /// Task pending deletion
    pub pending_delete_task_id: Option<TaskId>,
    /// Whether tasks are currently loading
    pub loading_tasks: bool,
    /// History of conversation entries (limited to `MAX_CONVERSATION_HISTORY`)
    pub conversation_history: Vec<ConversationEntry>,
    /// Status message to display when processing input
    pub processing_status: Option<String>,
    /// Vertical scroll offset for the output pane
    pub output_scroll_offset: u16,
    /// Background embedding index progress (current, total)
    pub embedding_progress: Option<(u64, u64)>,
    /// Task ID to continue conversation from (when submitting with a task selected)
    pub continuing_conversation_from: Option<TaskId>,
    /// Scroll offset for the task list (0 = bottom/newest, higher = scroll up to older)
    pub task_list_scroll_offset: usize,
    /// Set of expanded conversation IDs (showing child messages)
    pub expanded_conversations: HashSet<TaskId>,
    /// Set of task IDs with expanded steps (showing step details)
    pub expanded_steps: HashSet<TaskId>,
    /// Per-task output scroll positions for preserving scroll when switching tasks
    pub task_output_scroll: HashMap<TaskId, u16>,
    /// Flag to auto-scroll output to bottom on next render
    pub auto_scroll_output_to_bottom: bool,
    /// Currently active thread
    #[allow(dead_code, reason = "Will be used in Phase 5")]
    pub active_thread_id: Option<ThreadId>,
    /// Thread list scroll offset
    #[allow(dead_code, reason = "Will be used in Phase 5")]
    pub thread_list_scroll_offset: usize,
    /// Which panel is focused (thread list vs work details)
    #[allow(dead_code, reason = "Will be used in Phase 5")]
    pub focused_panel: PanelFocus,
}

impl UiState {
    /// Add a conversation entry and trim history if needed
    pub fn add_conversation_entry(&mut self, entry: ConversationEntry) {
        tracing::info!(
            "UiState::add_conversation_entry() - role: {:?}, text length: {}",
            entry.role,
            entry.text.len()
        );
        self.conversation_history.push(entry);

        // Trim oldest entries if we exceed the limit
        if self.conversation_history.len() > MAX_CONVERSATION_HISTORY {
            let excess = self.conversation_history.len() - MAX_CONVERSATION_HISTORY;
            self.conversation_history.drain(0..excess);
        }

        tracing::info!(
            "UiState now has {} conversation entries",
            self.conversation_history.len()
        );
    }
}

/// Which panel has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code, reason = "Will be used in Phase 5")]
pub enum PanelFocus {
    /// Thread list panel
    ThreadList,
    /// Work details panel
    #[default]
    WorkDetails,
}

/// Conversation entry
#[derive(Clone)]
pub struct ConversationEntry {
    /// Role of the speaker
    pub role: ConversationRole,
    /// Text content of the message
    pub text: String,
}

/// Conversation role
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConversationRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// System message
    System,
}
