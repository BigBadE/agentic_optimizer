use super::persistence::TaskPersistence;
use super::state::{ConversationEntry, ConversationRole, UiState};
use super::task_manager::{TaskDisplay, TaskManager, TaskStatus, TaskStepInfo, TaskStepStatus};
use merlin_core::{ThreadId, WorkUnit};
use merlin_deps::serde_json::Value;
use merlin_deps::tracing::warn;
use merlin_routing::{MessageLevel, TaskId, TaskProgress, TaskResult, UiEvent};
use merlin_tooling::ToolError;
use std::sync::Arc;
use tokio::sync::Mutex;

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
                thread_id,
                ..
            } => self.handle_task_started(task_id, description, thread_id),

            UiEvent::TaskProgress { task_id, progress } => {
                self.handle_task_progress(task_id, progress);
            }

            UiEvent::WorkUnitStarted { task_id, work_unit } => {
                self.handle_work_unit_started(task_id, work_unit);
            }

            UiEvent::WorkUnitProgress {
                task_id,
                progress_percentage,
                completed_subtasks,
                total_subtasks,
            } => {
                self.handle_work_unit_progress(
                    task_id,
                    progress_percentage,
                    completed_subtasks,
                    total_subtasks,
                );
            }

            UiEvent::TaskOutput { task_id, output } => self.handle_task_output(task_id, &output),

            UiEvent::TaskCompleted { task_id, result } => {
                self.handle_task_completed(task_id, result);
            }

            UiEvent::TaskFailed { task_id, error } => self.handle_task_failed(task_id, &error),

            UiEvent::TaskRetrying {
                task_id,
                retry_count,
                error,
            } => self.handle_task_retrying(task_id, retry_count, &error),

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

            UiEvent::TaskStepFailed {
                task_id,
                step_id,
                error,
            } => self.handle_task_step_failed(task_id, &step_id, &error),

            UiEvent::ToolCallStarted {
                task_id,
                tool,
                args,
            } => Self::handle_tool_call_started(task_id, tool, args),

            UiEvent::ToolCallCompleted {
                task_id,
                tool,
                result,
            } => Self::handle_tool_call_completed(task_id, &tool, &result),

            UiEvent::ThinkingUpdate { .. } | UiEvent::SubtaskSpawned { .. } => {
                // Deprecated events: functionality now handled by TaskStepStarted
                // These events are kept for backward compatibility with existing tests
                // and will be removed in a future phase when hierarchical task support is added
            }

            UiEvent::EmbeddingProgress { current, total, .. } => {
                // Clear progress when complete (current == total)
                if current >= total {
                    self.state.embedding_progress = None;
                } else {
                    self.state.embedding_progress = Some((current, total));
                }
            }
        }
    }

    // Private event handlers

    fn handle_task_started(
        &mut self,
        task_id: TaskId,
        description: String,
        thread_id: Option<ThreadId>,
    ) {
        let task_display = TaskDisplay {
            description,
            thread_id,
            ..Default::default()
        };

        self.task_manager.add_task(task_id, task_display);
        self.state.active_running_tasks.insert(task_id);

        // Update active_thread_id when a task with a thread starts
        if let Some(tid) = thread_id {
            self.state.active_thread_id = Some(tid);
        }

        self.state.processing_status = None;
        self.select_task(task_id);
    }

    fn handle_task_progress(&mut self, task_id: TaskId, progress: TaskProgress) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.progress = Some(progress);
        }
    }

    fn handle_work_unit_started(&mut self, task_id: TaskId, work_unit: Arc<Mutex<WorkUnit>>) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.work_unit = Some(work_unit);
        }
    }

    fn handle_work_unit_progress(
        &mut self,
        task_id: TaskId,
        _progress_percentage: u8,
        _completed_subtasks: usize,
        _total_subtasks: usize,
    ) {
        // The WorkUnit is already updated by the executor via Arc<Mutex<>>
        // This event is just a signal to re-render the UI
        // The UI will read the latest state from task.work_unit when rendering

        // Trigger a re-render by marking this task as needing update
        if self.state.active_task_id == Some(task_id) {
            // Force output scroll update to trigger re-render
            self.state.auto_scroll_output_to_bottom = false;
        }
    }

    fn handle_task_output(&mut self, task_id: TaskId, output: &str) {
        let Some(task) = self.task_manager.get_task_mut(task_id) else {
            return;
        };

        task.output_lines.push(output.to_string());

        // Filter out "Prompt:" lines and append to output
        for line in output.lines() {
            if line.trim_start().starts_with("Prompt:") {
                continue;
            }

            if !task.output.is_empty() {
                task.output.push('\n');
            }
            task.output.push_str(line);
        }

        // Auto-scroll to bottom if this is the active task
        if self.state.active_task_id == Some(task_id) {
            self.state.auto_scroll_output_to_bottom = true;
        }
    }

    fn handle_task_completed(&mut self, task_id: TaskId, result: Box<TaskResult>) {
        self.state.active_running_tasks.remove(&task_id);

        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.status = TaskStatus::Completed;
            // Clear progress indicator when task completes
            task.progress = None;
        }

        if let Some(persistence) = self.persistence
            && let Some(task) = self.task_manager.get_task(task_id)
            && let Err(save_err) = persistence.save_task(task_id, task)
        {
            warn!("Failed to save completed task {:?}: {}", task_id, save_err);
        }

        self.state.add_conversation_entry(ConversationEntry {
            role: ConversationRole::Assistant,
            text: result.response.text,
        });
    }

    fn handle_task_failed(&mut self, task_id: TaskId, error: &ToolError) {
        self.state.active_running_tasks.remove(&task_id);

        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.status = TaskStatus::Failed;

            let error_msg = format!("Error: {}", error.user_message());
            if !task.output.is_empty() {
                task.output.push('\n');
            }
            task.output.push_str(&error_msg);
        }

        if let Some(persistence) = self.persistence
            && let Some(task) = self.task_manager.get_task(task_id)
            && let Err(save_err) = persistence.save_task(task_id, task)
        {
            warn!("Failed to save failed task {:?}: {}", task_id, save_err);
        }
    }

    fn handle_task_retrying(&mut self, task_id: TaskId, retry_count: u32, _error: &ToolError) {
        // Update retry count in task display
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            task.retry_count = retry_count;
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
            if !task.output.is_empty() {
                task.output.push('\n');
            }
            task.output.push_str(prefix);
            task.output.push(' ');
            task.output.push_str(&message);
        }

        self.state.add_conversation_entry(ConversationEntry {
            role: ConversationRole::System,
            text: message,
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
            let step_info = TaskStepInfo {
                step_id,
                step_type: step_type.to_string(),
                content,
                ..Default::default()
            };

            // Set as current step (replaces previous step)
            task.current_step = Some(step_info.clone());

            // Also keep in history
            task.steps.push(step_info);
        }
    }

    fn handle_task_step_completed(&mut self, task_id: TaskId, step_id: &str) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            // Mark step as completed in history
            if let Some(step) = task.steps.iter_mut().find(|step| step.step_id == step_id) {
                step.status = TaskStepStatus::Completed;
            }

            // Clear current step if it matches
            if task
                .current_step
                .as_ref()
                .is_some_and(|step| step.step_id == step_id)
            {
                task.current_step = None;
            }
        }
    }

    fn handle_task_step_failed(&mut self, task_id: TaskId, step_id: &str, _error: &str) {
        if let Some(task) = self.task_manager.get_task_mut(task_id) {
            // Mark step as failed in history
            if let Some(step) = task.steps.iter_mut().find(|step| step.step_id == step_id) {
                step.status = TaskStepStatus::Failed;
            }

            // Update current step status if it matches
            if let Some(current_step) = &mut task.current_step
                && current_step.step_id == step_id
            {
                current_step.status = TaskStepStatus::Failed;
            }
        }
    }

    fn handle_tool_call_started(_task_id: TaskId, _tool: String, _args: Value) {}

    fn handle_tool_call_completed(_task_id: TaskId, _tool: &str, _result: &Value) {
        // Tool call completion no longer updates output
    }

    fn select_task(&mut self, task_id: TaskId) {
        self.state.active_task_id = Some(task_id);
        // Reset scroll offset when switching tasks
        self.state.output_scroll_offset = 0;
    }
}
