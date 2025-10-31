//! Task execution and orchestration logic

use std::fs::File;
use std::io::Write as _;
use std::sync::Arc;

use merlin_agent::RoutingOrchestrator;
use merlin_core::{Message, MessageId, TaskResult, ThreadId, TokenUsage, WorkUnit};
use merlin_deps::ratatui::backend::Backend;
use merlin_routing::{RoutingError, Task, TaskId, UiChannel, UiEvent};
use merlin_tooling::ToolError;
use tokio::spawn;
use tokio::sync::{mpsc, oneshot};

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

/// Context for task result handling
struct TaskResultContext<'ctx> {
    task_id: TaskId,
    ui_channel: &'ctx UiChannel,
    log_file: &'ctx mut Option<File>,
    orchestrator: &'ctx RoutingOrchestrator,
    actual_thread_id: Option<ThreadId>,
    message_id: Option<MessageId>,
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

/// Internal parameters for task execution including runtime state
struct InternalExecutionParams {
    orchestrator: Arc<RoutingOrchestrator>,
    user_input: String,
    parent_task_id: Option<TaskId>,
    conversation_history: Vec<(String, String)>,
    thread_id: Option<ThreadId>,
    ui_channel: UiChannel,
    log_file: Option<File>,
    forwarder_done_rx: oneshot::Receiver<()>,
}

/// Execute a task and handle its result
async fn execute_and_handle_task(params: InternalExecutionParams) {
    let InternalExecutionParams {
        orchestrator,
        user_input,
        parent_task_id,
        conversation_history,
        thread_id,
        ui_channel,
        mut log_file,
        forwarder_done_rx,
    } = params;
    if let Some(ref mut log) = log_file {
        let _ignored = writeln!(log, "User: {user_input}");
    }

    let task = Task::new(user_input.clone());
    let task_id = task.id;

    let (actual_thread_id, message_id) =
        create_or_continue_thread(&orchestrator, &user_input, thread_id);

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
            .execute_task_streaming_with_history(task, ui_channel.clone(), conversation_history)
            .await
    };

    // Handle result in a scope to ensure ctx is dropped before we drop ui_channel
    {
        let mut ctx = TaskResultContext {
            task_id,
            ui_channel: &ui_channel,
            log_file: &mut log_file,
            orchestrator: &orchestrator,
            actual_thread_id,
            message_id,
        };

        match result {
            Ok(ref result_data) => {
                handle_task_success(result_data, &mut ctx);
            }
            Err(ref error) => {
                handle_task_failure(error, &ctx);
            }
        }
    }

    // Drop ui_channel to close internal_tx, signaling forwarder to finish
    drop(ui_channel);

    // Wait for forwarder to finish processing all events
    if forwarder_done_rx.await.is_err() {
        merlin_deps::tracing::warn!(
            "Forwarder completion signal sender was dropped before signaling"
        );
    }
}

/// Handle successful task completion
fn handle_task_success(result_data: &TaskResult, ctx: &mut TaskResultContext<'_>) {
    // Emit the actual execution result as TaskOutput
    ctx.ui_channel.send(UiEvent::TaskOutput {
        task_id: ctx.task_id,
        output: result_data.response.text.clone(),
    });

    ctx.ui_channel
        .completed(result_data.task_id, result_data.clone());

    if let Some(log) = ctx.log_file {
        let _response_write = writeln!(log, "Response: {}", result_data.response.text);
        let _metrics_write = writeln!(
            log,
            "Tier: {} | Duration: {}ms | Tokens: {}",
            result_data.tier_used,
            result_data.duration_ms,
            result_data.response.tokens_used.total()
        );
    }

    if let (Some(tid), Some(msg_id)) = (ctx.actual_thread_id, ctx.message_id) {
        update_thread_work_completed(
            ctx.orchestrator,
            WorkCompletionParams {
                thread_id: tid,
                message_id: msg_id,
                task_id: ctx.task_id,
                tier_used: result_data.tier_used.clone(),
                tokens_used: result_data.response.tokens_used.clone(),
                duration_ms: result_data.duration_ms,
            },
        );
    }
}

/// Handle task execution failure
fn handle_task_failure(error: &RoutingError, ctx: &TaskResultContext<'_>) {
    ctx.ui_channel
        .failed(ctx.task_id, ToolError::ExecutionFailed(error.to_string()));

    if let (Some(tid), Some(msg_id)) = (ctx.actual_thread_id, ctx.message_id) {
        update_thread_work_failed(ctx.orchestrator, tid, msg_id, ctx.task_id);
    }
}

/// Create or continue a thread for multi-turn conversations
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

/// Update thread work completion status
fn update_thread_work_completed(orchestrator: &RoutingOrchestrator, params: WorkCompletionParams) {
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

/// Update thread work failed status
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

impl<B: Backend> TuiApp<B> {
    /// Spawns an event forwarder that duplicates events to both task-specific and global channels
    fn spawn_event_forwarder(
        mut internal_rx: mpsc::Receiver<UiEvent>,
        task_event_tx: mpsc::Sender<UiEvent>,
        global_ui_sender: mpsc::UnboundedSender<UiEvent>,
        forwarder_done_tx: oneshot::Sender<()>,
    ) {
        spawn(async move {
            while let Some(event) = internal_rx.recv().await {
                // Send to task-specific channel (test waits on this) - now bounded
                if task_event_tx.send(event.clone()).await.is_err() {
                    // Task channel closed, continue draining to global channel
                    merlin_deps::tracing::debug!("Task event channel closed, draining to global");
                }
                // Send to global UI channel (UI updates from this) - still unbounded
                if global_ui_sender.send(event).is_err() {
                    break; // Global channel closed, stop forwarding
                }
            }
            // Signal that forwarding is complete AFTER all events processed
            if forwarder_done_tx.send(()).is_err() {
                merlin_deps::tracing::warn!("Forwarder completion signal receiver was dropped");
            }
        });
    }

    /// Spawn task execution in background
    ///
    /// In test mode, stores a receiver for task-specific events in `last_task_receiver`.
    pub(crate) fn spawn_task_execution(&mut self, params: TaskExecutionParams) {
        let TaskExecutionParams {
            orchestrator,
            user_input,
            parent_task_id,
            conversation_history,
            thread_id,
        } = params;

        // Create per-task event channel for isolated event delivery (bounded with backpressure)
        let (task_event_tx, task_event_rx) = mpsc::channel(128);

        // Clone global UI channel for broadcasting to UI
        let global_ui_sender = self.event_sender.clone();

        // Create internal channel for task execution (bounded with backpressure)
        let (internal_tx, internal_rx) = mpsc::channel::<UiEvent>(128);

        // Create oneshot channel to signal when forwarder is done
        let (forwarder_done_tx, forwarder_done_rx) = oneshot::channel();

        // Spawn forwarder that duplicates events to both channels
        Self::spawn_event_forwarder(
            internal_rx,
            task_event_tx,
            global_ui_sender,
            forwarder_done_tx,
        );

        let ui_channel = UiChannel::from_sender(internal_tx);
        let log_file = self.log_file.as_ref().and_then(|f| f.try_clone().ok());

        spawn(execute_and_handle_task(InternalExecutionParams {
            orchestrator,
            user_input,
            parent_task_id,
            conversation_history,
            thread_id,
            ui_channel,
            log_file,
            forwarder_done_rx,
        }));

        // Store receiver for test access
        self.last_task_receiver = Some(task_event_rx);
    }
}
