//! Task execution and orchestration logic

use std::io::Write as _;
use std::sync::Arc;

use merlin_agent::RoutingOrchestrator;
use merlin_core::{Message, MessageId, ThreadId, TokenUsage, WorkUnit};
use merlin_deps::ratatui::backend::Backend;
use merlin_routing::{Task, TaskId, UiChannel, UiEvent};
use merlin_tooling::ToolError;
use tokio::spawn;

use super::tui_app::TuiApp;

/// Parameters for thread work completion
struct WorkCompletionParams {
    thread_id: ThreadId,
    message_id: MessageId,
    task_id: TaskId,
    tier_used: String,
    tokens_used: TokenUsage,
    duration_ms: u64,
}

/// Parameters for task execution
pub struct TaskExecutionParams {
    /// Orchestrator for task routing
    pub orchestrator: Arc<RoutingOrchestrator>,
    /// User input text
    pub user_input: String,
    /// Parent task ID if this is a subtask
    pub parent_task_id: Option<TaskId>,
    /// Conversation history
    pub conversation_history: Vec<(String, String)>,
    /// Thread ID for multi-turn conversations
    pub thread_id: Option<ThreadId>,
}

impl<B: Backend> TuiApp<B> {
    /// Spawn task execution in background
    pub(crate) fn spawn_task_execution(&self, params: TaskExecutionParams) {
        let TaskExecutionParams {
            orchestrator,
            user_input,
            parent_task_id,
            conversation_history,
            thread_id,
        } = params;
        let ui_channel = UiChannel::from_sender(self.event_sender.clone());
        let mut log_file = self.log_file.as_ref().and_then(|f| f.try_clone().ok());

        spawn(async move {
            if let Some(ref mut log) = log_file {
                let _ignored = writeln!(log, "User: {user_input}");
            }

            let task = Task::new(user_input.clone());
            let task_id = task.id;

            let (actual_thread_id, message_id) =
                Self::create_or_continue_thread(&orchestrator, &user_input, thread_id);

            ui_channel.task_started_with_thread(
                task_id,
                user_input.clone(),
                parent_task_id,
                actual_thread_id,
            );
            ui_channel.send(UiEvent::TaskOutput {
                task_id,
                output: format!("Prompt: {user_input}\n"),
            });

            let result = if let Some(tid) = actual_thread_id {
                orchestrator
                    .execute_task_in_thread(task, ui_channel.clone(), tid)
                    .await
            } else {
                orchestrator
                    .execute_task_streaming_with_history(
                        task,
                        ui_channel.clone(),
                        conversation_history,
                    )
                    .await
            };

            match result {
                Ok(result_data) => {
                    // Emit the actual execution result as TaskOutput
                    ui_channel.send(UiEvent::TaskOutput {
                        task_id,
                        output: result_data.response.text.clone(),
                    });

                    ui_channel.completed(result_data.task_id, result_data.clone());

                    if let Some(ref mut log) = log_file {
                        let _response_write =
                            writeln!(log, "Response: {}", result_data.response.text);
                        let _metrics_write = writeln!(
                            log,
                            "Tier: {} | Duration: {}ms | Tokens: {}",
                            result_data.tier_used,
                            result_data.duration_ms,
                            result_data.response.tokens_used.total()
                        );
                    }

                    if let (Some(tid), Some(msg_id)) = (actual_thread_id, message_id) {
                        Self::update_thread_work_completed(
                            &orchestrator,
                            WorkCompletionParams {
                                thread_id: tid,
                                message_id: msg_id,
                                task_id,
                                tier_used: result_data.tier_used.clone(),
                                tokens_used: result_data.response.tokens_used.clone(),
                                duration_ms: result_data.duration_ms,
                            },
                        );
                    }
                }
                Err(error) => {
                    ui_channel.failed(task_id, ToolError::ExecutionFailed(error.to_string()));

                    if let (Some(tid), Some(msg_id)) = (actual_thread_id, message_id) {
                        Self::update_thread_work_failed(&orchestrator, tid, msg_id, task_id);
                    }
                }
            }
        });
    }

    fn create_or_continue_thread(
        orchestrator: &RoutingOrchestrator,
        user_input: &str,
        thread_id: Option<ThreadId>,
    ) -> (Option<ThreadId>, Option<MessageId>) {
        let Some(thread_store_arc) = orchestrator.thread_store() else {
            return (None, None);
        };

        let Ok(mut store) = thread_store_arc.lock() else {
            return (None, None);
        };

        let tid = thread_id.unwrap_or_else(|| {
            let thread_name = user_input.chars().take(30).collect::<String>();
            let thread = store.create_thread(thread_name);
            let tid = thread.id;
            if let Err(save_err) = store.save_thread(&thread) {
                merlin_deps::tracing::warn!("Failed to create thread: {save_err}");
            }
            tid
        });

        let message = Message::new(user_input.to_owned());
        let msg_id = message.id;

        let thread_to_save = store.get_thread_mut(tid).map(|thread| {
            thread.add_message(message);
            thread.clone()
        });

        if let Some(thread) = thread_to_save
            && let Err(save_err) = store.save_thread(&thread)
        {
            merlin_deps::tracing::warn!("Failed to save thread message: {save_err}");
        }

        (Some(tid), Some(msg_id))
    }

    fn update_thread_work_completed(
        orchestrator: &RoutingOrchestrator,
        params: WorkCompletionParams,
    ) {
        let Some(thread_store_arc) = orchestrator.thread_store() else {
            return;
        };

        let Ok(mut store) = thread_store_arc.lock() else {
            return;
        };

        let thread_to_save = store.get_thread_mut(params.thread_id).map(|thread| {
            if let Some(msg) = thread
                .messages
                .iter_mut()
                .find(|message| message.id == params.message_id)
            {
                let mut work = WorkUnit::new(params.task_id, params.tier_used);
                work.tokens_used = params.tokens_used;
                work.duration_ms = params.duration_ms;
                work.complete();
                msg.attach_work(work);
            }
            thread.clone()
        });

        if let Some(thread) = thread_to_save
            && let Err(save_err) = store.save_thread(&thread)
        {
            merlin_deps::tracing::warn!("Failed to save thread work completion: {save_err}");
        }
    }

    fn update_thread_work_failed(
        orchestrator: &RoutingOrchestrator,
        thread_id: ThreadId,
        message_id: MessageId,
        task_id: TaskId,
    ) {
        let Some(thread_store_arc) = orchestrator.thread_store() else {
            return;
        };

        let Ok(mut store) = thread_store_arc.lock() else {
            return;
        };

        let thread_to_save = store.get_thread_mut(thread_id).map(|thread| {
            if let Some(msg) = thread
                .messages
                .iter_mut()
                .find(|message| message.id == message_id)
            {
                let mut work = WorkUnit::new(task_id, "unknown".to_string());
                work.fail();
                msg.attach_work(work);
            }
            thread.clone()
        });

        if let Some(thread) = thread_to_save
            && let Err(save_err) = store.save_thread(&thread)
        {
            merlin_deps::tracing::warn!("Failed to save thread work failure: {save_err}");
        }
    }
}
