//! Task completion and result extraction logic.

use crate::execution_tracker::ExecutionResultTracker;
use crate::tui_test_helpers;
use merlin_cli::TuiApp;
use merlin_core::{Result, RoutingError, TaskResult};
use merlin_deps::ratatui::backend::TestBackend;
use merlin_deps::serde_json::{Value as JsonValue, from_str};
use merlin_deps::tracing;
use merlin_routing::UiEvent;
use merlin_tooling::{ToolError, ToolResult};
use std::result::Result as StdResult;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::{Duration as TokioDuration, timeout};

/// Result type for task completion with captured outputs
pub type TaskCompletionResult = StdResult<(TaskResult, Vec<String>), (ToolError, Vec<String>)>;

/// Pending task result before being added to tracker - can be success or failure
pub type PendingTaskResult = StdResult<(TaskResult, Vec<String>), (ToolError, Vec<String>)>;

/// Timing statistics for task completion
struct CompletionTiming {
    /// Time spent processing UI events
    process_ui: Duration,
    /// Time spent trying to receive events
    try_recv: Duration,
    /// Time spent yielding/sleeping
    yielding: Duration,
}

impl CompletionTiming {
    /// Log completion timing statistics
    fn log_stats(&self, start: Instant, iterations: usize, events_received: usize) {
        let total = start.elapsed().as_secs_f64();
        let process_ui_secs = self.process_ui.as_secs_f64();
        let try_recv_secs = self.try_recv.as_secs_f64();
        let yielding_secs = self.yielding.as_secs_f64();
        let overhead = process_ui_secs + try_recv_secs + yielding_secs;
        let actual_work = total - overhead;
        tracing::debug!(
            "task_completion: {total:.3}s (iter:{iterations}, events:{events_received}) - \
             overhead:{overhead:.3}s (ui:{process_ui_secs:.3}s, recv:{try_recv_secs:.3}s, \
             yield:{yielding_secs:.3}s), work:{actual_work:.3}s"
        );
    }
}

/// Context for processing task events
struct EventContext<'ctx> {
    /// Output strings captured so far
    outputs: &'ctx mut Vec<String>,
    /// Whether task has started
    task_started: &'ctx mut bool,
    /// Timing statistics
    timing: &'ctx CompletionTiming,
    /// Start time
    start: Instant,
    /// Iteration count
    iterations: usize,
    /// Events received count
    events_received: usize,
}

/// Process a task event and update state
///
/// # Errors
/// Returns error if channel disconnected without task completion
fn process_task_event(
    event: UiEvent,
    ctx: &mut EventContext<'_>,
) -> Option<Result<TaskCompletionResult>> {
    match event {
        UiEvent::TaskStarted { .. } => {
            *ctx.task_started = true;
            None
        }
        UiEvent::TaskCompleted { result, .. } => {
            ctx.timing
                .log_stats(ctx.start, ctx.iterations, ctx.events_received);
            Some(Ok(Ok((*result, ctx.outputs.clone()))))
        }
        UiEvent::TaskFailed { error, .. } => {
            ctx.timing
                .log_stats(ctx.start, ctx.iterations, ctx.events_received);
            Some(Ok(Err((error, ctx.outputs.clone()))))
        }
        UiEvent::TaskOutput { output, .. } => {
            ctx.outputs.push(output);
            None
        }
        _ => None, // Ignore other events
    }
}

/// Await task completion by listening to dedicated task-specific UI events
///
/// Uses a per-task event channel to receive only events for this specific task,
/// preventing event mixing from concurrent tasks.
///
/// We capture ALL `TaskOutput` events during this wait to ensure we capture outputs
/// from the main task and any subtasks (like TypeScript tool executions).
///
/// Returns the task result and any captured output from `TaskOutput` events.
///
/// # Errors
/// Returns error if task completion fails or times out
pub async fn await_task_completion(
    tui_app: &mut TuiApp<TestBackend>,
    task_events: &mut mpsc::Receiver<UiEvent>,
) -> Result<TaskCompletionResult> {
    let mut outputs = Vec::new();
    let overall_timeout = TokioDuration::from_secs(15);
    let start = Instant::now();
    let mut events_received = 0;
    let mut last_event_time = Instant::now();
    let mut task_started = false;
    let mut iterations = 0;
    let mut timing = CompletionTiming {
        process_ui: Duration::ZERO,
        try_recv: Duration::ZERO,
        yielding: Duration::ZERO,
    };

    loop {
        iterations += 1;
        // Check overall timeout to prevent infinite hangs
        if start.elapsed() >= overall_timeout {
            let idle_time = last_event_time.elapsed().as_millis();
            return Err(RoutingError::ExecutionFailed(format!(
                "Task completion timed out after 15 seconds - {events_received} events received, \
                 task_started: {task_started}, idle for {idle_time}ms"
            )));
        }

        // Process any pending UI events first (this broadcasts them)
        let ui_start = Instant::now();
        tui_test_helpers::process_ui_events(tui_app);
        timing.process_ui += ui_start.elapsed();

        // Block on receive with timeout (for periodic UI processing)
        // Wakes immediately when event arrives, or after 10ms for UI updates
        let recv_start = Instant::now();
        match timeout(TokioDuration::from_millis(10), task_events.recv()).await {
            Ok(Some(event)) => {
                // Event received - process immediately
                timing.try_recv += recv_start.elapsed();
                events_received += 1;
                last_event_time = Instant::now();

                if let Some(result) = process_task_event(
                    event,
                    &mut EventContext {
                        outputs: &mut outputs,
                        task_started: &mut task_started,
                        timing: &timing,
                        start,
                        iterations,
                        events_received,
                    },
                ) {
                    return result;
                }
            }
            Ok(None) => {
                // Channel closed - check if we got completion
                timing.try_recv += recv_start.elapsed();
                if task_started {
                    return Err(RoutingError::ExecutionFailed(format!(
                        "Task event channel closed after {events_received} events \
                         without TaskCompleted/TaskFailed"
                    )));
                }
                return Err(RoutingError::ExecutionFailed(
                    "Task event channel closed before task started".to_owned(),
                ));
            }
            Err(_) => {
                // Timeout - continue loop to process UI events again
                let elapsed = recv_start.elapsed();
                timing.try_recv += elapsed;
                timing.yielding += elapsed;
            }
        }
    }
}

/// Complete a pending task by adding its result to the tracker
pub fn complete_pending_task(
    pending_task: &mut Option<(PendingTaskResult, String)>,
    execution_tracker: &mut ExecutionResultTracker,
) {
    if let Some((completion_result, execution_id)) = pending_task.take() {
        match completion_result {
            Ok((task_result, outputs)) => {
                // Successful task completion
                let execution_result = extract_execution_result(&task_result, &outputs);
                execution_tracker.add_success(
                    execution_id,
                    execution_result,
                    outputs,
                    Box::new(task_result),
                );
            }
            Err((error, outputs)) => {
                // Task failed
                execution_tracker.add_failure(execution_id, error, outputs);
            }
        }
    }
}

/// Extract execution result from `TaskResult` and captured outputs
///
/// The TypeScript execution result is sent via `TaskOutput` events during execution.
/// We capture these outputs and parse them to extract the actual execution result.
///
/// The last output typically contains the result returned by the TypeScript code.
///
/// # Errors
/// Returns error if JSON parsing of outputs fails (though this is currently handled gracefully)
fn extract_execution_result(task_result: &TaskResult, outputs: &[String]) -> ToolResult<JsonValue> {
    let response_text = &task_result.response.text;

    // Check if there were any outputs from TypeScript execution
    outputs.last().map_or_else(
        || {
            // No outputs captured - this could mean:
            // 1. No TypeScript was executed
            // 2. TypeScript executed but didn't produce output
            // Return the response text as fallback
            Ok(JsonValue::String(response_text.clone()))
        },
        |last_output| {
            // Try to parse the output as JSON first
            from_str::<JsonValue>(last_output).map_or_else(
                |_| {
                    // If not valid JSON, return as string
                    Ok(JsonValue::String(last_output.clone()))
                },
                Ok,
            )
        },
    )
}
