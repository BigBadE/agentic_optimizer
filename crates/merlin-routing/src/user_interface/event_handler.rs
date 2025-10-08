use super::events::{MessageLevel, UiEvent};
use super::output_tree::StepType;
use super::persistence::TaskPersistence;
use super::state::{ConversationEntry, ConversationRole, UiState};
use super::task_manager::{TaskDisplay, TaskManager, TaskStatus, TaskStepInfo};
use crate::{TaskId, TaskResult};
use serde_json::Value;
use std::time::Instant;

/// Handles UI events and updates task manager and state
pub struct EventHandler<'handler> {
    task_manager: &'handler mut TaskManager,
    state: &'handler mut UiState,
    persistence: Option<&'handler TaskPersistence>,
}

impl<'handler> EventHandler<'handler> {
    /// Creates a new `EventHandler`
    pub fn new(
        task_manager: &'handler mut TaskManager,
        state: &'handler mut UiState,
        persistence: Option<&'handler TaskPersistence>,
    ) -> Self {
        Self {
            task_manager,
            state,
            persistence,
        }
    }

    /// Handles a UI event
    pub fn handle_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::TaskStarted {
                task_id,
                description,
                parent_id,
            } => self.handle_task_started(task_id, description, parent_id),

            UiEvent::TaskProgress { task_id, progress } => {
                self.handle_task_progress(task_id, progress);
            }

            UiEvent::TaskOutput { task_id, output } => self.handle_task_output(task_id, output),

            UiEvent::TaskCompleted { task_id, result } => {
                self.handle_task_completed(task_id, result);
            }

            UiEvent::TaskFailed { task_id, error } => self.handle_task_failed(task_id, &error),

            UiEvent::SystemMessage { level, message } => {
                self.handle_system_message(level, message);
            }

            UiEvent::TaskStepStarted {
                task_id,
                step_id,
                step_type,
                content,
            } => self.handle_task_step_started(task_id, step_id, &step_type, content),

            UiEvent::TaskStepCompleted { task_id, step_id } => {
                self.handle_task_step_completed(task_id, &step_id);
            }

            UiEvent::ToolCallStarted {
                task_id,
                tool,
                args,
            } => Self::handle_tool_call_started(task_id, tool, args),

            UiEvent::ToolCallCompleted {
                task_id,
                tool,
                result,
            } => self.handle_tool_call_completed(task_id, &tool, &result),

            UiEvent::ThinkingUpdate { .. } | UiEvent::SubtaskSpawned { .. } => {
                // Deprecated: handled by TaskStepStarted
                // TODO: Phase 5 - Handle hierarchical tasks
            }
        }
    }

    // Private event handlers

    fn handle_task_started(
        &mut self,
        task_id: TaskId,
        description: String,
        parent_id: Option<TaskId>,
    ) {
        let task_display = TaskDisplay {
            description,
            status: TaskStatus::Running,
            progress: None,
            output_lines: Vec::default(),
            start_time: Instant::now(),
            end_time: None,
            parent_id,
            output_tree: super::output_tree::OutputTree::default(),
            steps: Vec::default(),
        };

        self.task_manager.add_task(task_id, task_display);
        self.state.active_running_tasks.insert(task_id);

        // Ensure parent is not collapsed
        if let Some(parent_id) = parent_id {
            self.task_manager.expand_task(parent_id);
        }

        // Select the newly spawned task
        self.select_task(task_id);
    }

    fn handle_task_progress(&mut self, task_id: TaskId, progress: super::events::TaskProgress) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.progress = Some(progress);
        }
    }

    fn handle_task_output(&mut self, task_id: TaskId, output: String) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.output_lines.push(output.clone());
            task.output_tree.add_text(output);
        }
    }

    fn handle_task_completed(&mut self, task_id: TaskId, result: TaskResult) {
        self.state.active_running_tasks.remove(&task_id);

        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.status = TaskStatus::Completed;
            task.end_time = Some(Instant::now());
        }

        if let Some(persistence) = self.persistence
            && let Some(task) = self.task_manager.get_task(task_id)
        {
            drop(persistence.save_task(task_id, task));
        }

        self.state.conversation_history.push(ConversationEntry {
            role: ConversationRole::Assistant,
            text: result.response.text,
            timestamp: Instant::now(),
        });
    }

    fn handle_task_failed(&mut self, task_id: TaskId, error: &str) {
        self.state.active_running_tasks.remove(&task_id);

        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.status = TaskStatus::Failed;
            task.end_time = Some(Instant::now());

            let error_msg = format!("Error: {error}");
            task.output_tree.add_text(error_msg);
        }

        if let Some(persistence) = self.persistence
            && let Some(task) = self.task_manager.get_task(task_id)
        {
            drop(persistence.save_task(task_id, task));
        }
    }

    fn handle_system_message(&mut self, level: MessageLevel, message: String) {
        let prefix = match level {
            MessageLevel::Info => "[i]",
            MessageLevel::Warning => "[!]",
            MessageLevel::Error => "[X]",
            MessageLevel::Success => "[+]",
        };

        // Send to active task
        if let Some(task_id) = self.state.active_task_id
            && let Some(task) = self.task_manager.get_task_mut(task_id)
        {
            task.output_tree.add_text(format!("{prefix} {message}"));
        }

        self.state.conversation_history.push(ConversationEntry {
            role: ConversationRole::System,
            text: message,
            timestamp: Instant::now(),
        });
    }

    fn handle_task_step_started(
        &mut self,
        task_id: TaskId,
        step_id: String,
        step_type: &str,
        content: String,
    ) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.steps.push(TaskStepInfo {
                step_id: step_id.clone(),
                step_type: step_type.to_string(),
                content: content.clone(),
                timestamp: Instant::now(),
            });

            let step_type_enum = StepType::from_str(step_type);
            task.output_tree.add_step(step_id, step_type_enum, content);
        }
    }

    fn handle_task_step_completed(&mut self, task_id: TaskId, step_id: &str) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.output_tree.complete_step(step_id);
        }
    }

    fn handle_tool_call_started(_task_id: TaskId, _tool: String, _args: Value) {}

    fn handle_tool_call_completed(&mut self, task_id: TaskId, tool: &str, result: &Value) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.output_tree.complete_tool_call(tool, result);
        }
    }

    fn select_task(&mut self, task_id: TaskId) {
        if let Some(pos) = self
            .task_manager
            .task_order()
            .iter()
            .position(|&id| id == task_id)
        {
            self.state.selected_task_index = pos;
        }
        self.state.active_task_id = Some(task_id);
    }
}
