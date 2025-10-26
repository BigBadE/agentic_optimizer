//! Interactive mode functionality - TUI mode

use anyhow::Result;
use merlin_agent::RoutingOrchestrator;
use merlin_core::{Message, MessageId, TaskResult, ThreadId, TokenUsage, WorkUnit};
use merlin_routing::{Task, TaskId};

use crate::ui::{MessageLevel, TuiApp, UiChannel, UiEvent};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::spawn;

use crate::utils::{cleanup_old_tasks, get_merlin_folder, try_write_log};

/// Flags for interactive mode configuration
pub struct InteractiveFlags {
    /// Whether to use local models only
    pub local_only: bool,
}

/// Handle interactive agent session with multi-model routing
///
/// # Errors
/// Returns an error if TUI or IO operations fail, or if the orchestrator returns an error.
pub async fn handle_interactive_agent(
    orchestrator: RoutingOrchestrator,
    project: PathBuf,
    flags: InteractiveFlags,
) -> Result<()> {
    // Run TUI mode (only mode supported)
    run_tui_interactive(orchestrator, project, flags.local_only).await
}

/// Initialize logging for TUI session
///
/// # Errors
/// Returns error if file operations fail
fn init_tui_logging(merlin_dir: &Path, project: &Path, local_only: bool) -> Result<fs::File> {
    let debug_log = merlin_dir.join("debug.log");

    // Open existing debug.log (already created by handle_interactive)
    let mut log_file = fs::OpenOptions::new().append(true).open(&debug_log)?;

    writeln!(
        log_file,
        "=== Session started at {:?} ===",
        SystemTime::now()
    )?;
    writeln!(log_file, "Project: {}", project.display())?;
    writeln!(
        log_file,
        "Mode: {}",
        if local_only {
            "Local Only"
        } else {
            "Multi-Model"
        }
    )?;

    Ok(log_file)
}

/// Parameters for task execution
struct TaskExecutionParams {
    orchestrator: RoutingOrchestrator,
    ui_channel: UiChannel,
    log_file: fs::File,
    user_input: String,
    parent_task_id: Option<TaskId>,
    conversation_history: Vec<(String, String)>,
    thread_id: Option<ThreadId>,
}

/// Creates or continues a thread and adds a message
fn create_or_continue_thread(
    orchestrator: &RoutingOrchestrator,
    user_input: &str,
    thread_id: Option<ThreadId>,
) -> (Option<ThreadId>, Option<MessageId>) {
    orchestrator
        .thread_store()
        .map_or((None, None), |thread_store_arc| {
            thread_store_arc.lock().map_or_else(
                |poison_err| {
                    tracing::error!("Thread store lock poisoned: {poison_err}");
                    (None, None)
                },
                |mut store| {
                    // Get or create thread
                    let tid = thread_id.unwrap_or_else(|| {
                        let thread_name = user_input.chars().take(30).collect::<String>();
                        let thread = store.create_thread(thread_name);
                        let tid = thread.id;
                        if let Err(save_err) = store.save_thread(&thread) {
                            tracing::warn!("Failed to create thread: {save_err}");
                        }
                        tid
                    });

                    // Add message to thread
                    let message = Message::new(user_input.to_owned());
                    let msg_id = message.id;

                    let thread_to_save = store.get_thread_mut(tid).map(|thread| {
                        thread.add_message(message);
                        thread.clone()
                    });

                    // Save thread after message added
                    if let Some(thread) = thread_to_save
                        && let Err(save_err) = store.save_thread(&thread)
                    {
                        tracing::warn!("Failed to save thread message: {save_err}");
                    }

                    (Some(tid), Some(msg_id))
                },
            )
        })
}

/// Work completion information
struct WorkCompletionInfo<'info> {
    thread_id: ThreadId,
    message_id: MessageId,
    task_id: TaskId,
    tier_used: &'info str,
    tokens_used: TokenUsage,
    duration_ms: u64,
}

/// Updates thread with completed work
fn update_thread_work_completed(orchestrator: &RoutingOrchestrator, info: WorkCompletionInfo<'_>) {
    let Some(thread_store_arc) = orchestrator.thread_store() else {
        return;
    };

    let Ok(mut store) = thread_store_arc.lock() else {
        return;
    };

    let thread_to_save = store.get_thread_mut(info.thread_id).map(|thread| {
        if let Some(msg) = thread
            .messages
            .iter_mut()
            .find(|message| message.id == info.message_id)
        {
            let mut work = WorkUnit::new(info.task_id, info.tier_used.to_owned());
            work.tokens_used = info.tokens_used;
            work.duration_ms = info.duration_ms;
            work.complete();
            msg.attach_work(work);
        }
        thread.clone()
    });

    if let Some(thread) = thread_to_save
        && let Err(save_err) = store.save_thread(&thread)
    {
        tracing::warn!("Failed to save thread work completion: {save_err}");
    }
}

/// Updates thread with failed work
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
        tracing::warn!("Failed to save thread work failure: {save_err}");
    }
}

/// Success handling parameters
struct TaskSuccessParams<'info> {
    orchestrator: &'info RoutingOrchestrator,
    ui_channel: &'info UiChannel,
    log_file: &'info mut fs::File,
    result_data: &'info TaskResult,
    actual_thread_id: Option<ThreadId>,
    message_id: Option<MessageId>,
}

/// Handles successful task completion - logging and thread updates
fn handle_task_success(params: TaskSuccessParams<'_>) {
    let TaskSuccessParams {
        orchestrator,
        ui_channel,
        log_file,
        result_data,
        actual_thread_id,
        message_id,
    } = params;
    ui_channel.completed(result_data.task_id, result_data.clone());
    try_write_log(
        ui_channel,
        log_file,
        &format!("Response: {}", result_data.response.text),
    );
    try_write_log(
        ui_channel,
        log_file,
        &format!(
            "Tier: {} | Duration: {}ms | Tokens: {}",
            result_data.tier_used,
            result_data.duration_ms,
            result_data.response.tokens_used.total()
        ),
    );
    try_write_log(ui_channel, log_file, "Task completed successfully.");

    // Update thread with work completion
    if let (Some(tid), Some(msg_id)) = (actual_thread_id, message_id) {
        update_thread_work_completed(
            orchestrator,
            WorkCompletionInfo {
                thread_id: tid,
                message_id: msg_id,
                task_id: result_data.task_id,
                tier_used: &result_data.tier_used,
                tokens_used: result_data.response.tokens_used.clone(),
                duration_ms: result_data.duration_ms,
            },
        );
    }
}

/// Execute a task from user input and handle the result
async fn execute_user_task(params: TaskExecutionParams) {
    let TaskExecutionParams {
        orchestrator,
        ui_channel,
        mut log_file,
        user_input,
        parent_task_id,
        conversation_history,
        thread_id,
    } = params;
    let task = Task::new(user_input.clone());
    let task_id = task.id;

    if let Err(error) = writeln!(log_file, "Created task: {user_input}") {
        let () = ui_channel.send(UiEvent::SystemMessage {
            level: MessageLevel::Warning,
            message: format!("Failed to write to log: {error}"),
        });
    }

    // Create or update thread
    let (actual_thread_id, message_id) =
        create_or_continue_thread(&orchestrator, &user_input, thread_id);

    ui_channel.task_started_with_parent(task_id, user_input.clone(), parent_task_id);

    ui_channel.send(UiEvent::TaskOutput {
        task_id,
        output: format!("Prompt: {user_input}\n"),
    });

    tracing::info!(
        "execute_user_task: Passing {} conversation messages to orchestrator",
        conversation_history.len()
    );

    // Execute task - use thread-aware execution if thread_id is available
    let result = if let Some(tid) = actual_thread_id {
        orchestrator
            .execute_task_in_thread(task, ui_channel.clone(), tid)
            .await
    } else {
        orchestrator
            .execute_task_streaming_with_history(task, ui_channel.clone(), conversation_history)
            .await
    };

    // Handle result and update thread
    match result {
        Ok(result_data) => handle_task_success(TaskSuccessParams {
            orchestrator: &orchestrator,
            ui_channel: &ui_channel,
            log_file: &mut log_file,
            result_data: &result_data,
            actual_thread_id,
            message_id,
        }),
        Err(error) => {
            try_write_log(&ui_channel, &mut log_file, &format!("Error: {error}"));
            ui_channel.failed(task_id, error.to_string());

            if let (Some(tid), Some(msg_id)) = (actual_thread_id, message_id) {
                update_thread_work_failed(&orchestrator, tid, msg_id, task_id);
            }
        }
    }
}

/// Initialize vector embeddings in background
async fn initialize_embeddings_background(ui_channel: UiChannel, project: PathBuf) {
    use merlin_context::VectorSearchManager;
    use std::sync::Arc;

    tracing::info!("Starting background embedding initialization...");

    let ui_channel_progress = ui_channel.clone();
    let progress_callback = Arc::new(move |stage: &str, current: u64, total: Option<u64>| {
        if let Some(total_count) = total {
            tracing::debug!("Embedding progress: {stage} {current}/{total_count}");
            ui_channel_progress.send(UiEvent::EmbeddingProgress {
                current,
                total: total_count,
                stage: stage.to_owned(),
            });
        }
    });

    let mut manager = VectorSearchManager::new(project).with_progress_callback(progress_callback);

    match manager.initialize().await {
        Ok(()) => {
            tracing::info!("Embedding initialization completed successfully");
            // Don't send UI message - keeps output clean
        }
        Err(error) => {
            tracing::warn!("Embedding initialization failed: {error}");
            // Don't send UI message - keeps output clean, logged to debug.log
        }
    }
}

/// Run fully self-contained TUI interactive session
///
/// # Errors
/// Returns an error if filesystem, TUI, or async operations fail.
async fn run_tui_interactive(
    orchestrator: RoutingOrchestrator,
    project: PathBuf,
    local_only: bool,
) -> Result<()> {
    // Create merlin directory for logs and task storage (respects MERLIN_FOLDER)
    let merlin_dir = get_merlin_folder(&project)?;
    fs::create_dir_all(&merlin_dir)?;

    let mut log_file = init_tui_logging(&merlin_dir, &project, local_only)?;

    // Clean up old task files (keep last 50 tasks)
    cleanup_old_tasks(&merlin_dir)?;

    // Create TUI with task storage
    let tasks_dir = merlin_dir.join("tasks");
    fs::create_dir_all(&tasks_dir)?;
    let (mut tui_app, ui_channel) = TuiApp::new_with_storage(tasks_dir.clone())?;

    // Enable raw mode before loading
    TuiApp::enable_raw_mode()?;

    // Load tasks and threads
    tui_app.load_tasks_async().await;
    if let Err(err) = tui_app.load_threads() {
        tracing::warn!("Failed to load threads: {err}");
    }

    // Start embedding initialization in background
    spawn(initialize_embeddings_background(
        ui_channel.clone(),
        project.clone(),
    ));

    // Log how many task files exist on disk and how many were parsed
    let disk_task_files = fs::read_dir(&tasks_dir).map_or(0, |read_dir| {
        read_dir
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext_str| ext_str == "gz")
            })
            .count()
    });
    let parsed_tasks = tui_app.loaded_task_count();
    writeln!(
        log_file,
        "Found {} .gz file(s), parsed {} task(s) from {}",
        disk_task_files,
        parsed_tasks,
        tasks_dir.display(),
    )?;

    // Main event loop - event-driven
    loop {
        // Tick the TUI (handles rendering and input)
        let should_quit = tui_app.tick()?;
        if should_quit {
            break;
        }

        // Check if user submitted input
        if let Some(user_input) = tui_app.take_pending_input() {
            writeln!(log_file, "User: {user_input}")?;

            // Get conversation history BEFORE taking continuing_from (which clears the state)
            let conversation_history = tui_app.get_conversation_history();

            // Check if we're continuing a conversation
            let continuing_from = tui_app.take_continuing_conversation_from();

            // Get parent task ID - use continuing_from if set, otherwise use selected task
            let parent_task_id = continuing_from.or_else(|| tui_app.get_selected_task_id());
            tracing::info!(
                "interactive.rs: Extracted {} conversation messages for task execution{}",
                conversation_history.len(),
                if continuing_from.is_some() {
                    " (continuing conversation)"
                } else {
                    ""
                }
            );
            let log_clone = log_file.try_clone()?;

            spawn(execute_user_task(TaskExecutionParams {
                orchestrator: orchestrator.clone(),
                ui_channel: ui_channel.clone(),
                log_file: log_clone,
                user_input,
                parent_task_id,
                conversation_history,
                thread_id: None, // Thread will be created automatically
            }));
        }
    }

    // Disable raw mode and clean up
    tui_app.disable_raw_mode()?;
    writeln!(log_file, "=== Session ended ===")?;

    Ok(())
}
