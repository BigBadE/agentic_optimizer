//! Sequential task execution with dependency resolution and parallel I/O
//!
//! Steps execute sequentially (to allow `&mut runtime` access), but expensive
//! I/O operations within each step (LLM calls, file ops) execute in parallel
//! on the thread pool via `tokio::spawn`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use merlin_core::{Result, RoutingError, SubtaskStatus, TaskStep, WorkUnit};

use super::step_executor::{
    StepExecutionParams, StepExecutor, StepResult, TaskListExecutionParams,
};

/// Check for deadlock condition
///
/// # Errors
/// Returns an error if deadlock is detected
fn check_for_deadlock(
    params: &TaskListExecutionParams<'_>,
    completed: &HashSet<String>,
) -> Result<StepResult> {
    let remaining: Vec<_> = params
        .task_list
        .steps
        .iter()
        .filter(|task_step| !completed.contains(&task_step.title))
        .map(|task_step| task_step.title.as_str())
        .collect();
    Err(RoutingError::Other(format!(
        "Deadlock detected: {} steps remaining but none can run: {:?}",
        remaining.len(),
        remaining
    )))
}

/// Build final combined output from step results
fn build_combined_output(
    params: &TaskListExecutionParams<'_>,
    results_by_title: &HashMap<String, StepResult>,
) -> String {
    let mut combined_output = Vec::new();
    for (index, step) in params.task_list.steps.iter().enumerate() {
        if let Some(result) = results_by_title.get(&step.title) {
            combined_output.push(format!(
                "## Step {}: {}\n{}",
                index + 1,
                step.title,
                result.text
            ));
        }
    }
    combined_output.join("\n\n")
}

/// Mark subtask as started in `WorkUnit`
async fn mark_subtask_started(work_unit: &Arc<Mutex<WorkUnit>>, index: usize) {
    let mut work_unit_guard = work_unit.lock().await;
    if let Some(subtask) = work_unit_guard.subtasks.get(index) {
        let subtask_id = subtask.id;
        work_unit_guard.start_subtask(subtask_id);
    }
}

/// Update `WorkUnit` when a step completes
async fn update_work_unit_on_completion(
    work_unit: &Arc<Mutex<WorkUnit>>,
    params: &TaskListExecutionParams<'_>,
    step_title: &str,
    step_result: &StepResult,
    completed: &HashSet<String>,
) {
    let mut work_unit_guard = work_unit.lock().await;

    merlin_deps::tracing::debug!(
        "Updating WorkUnit for completed step: '{}' (result length: {})",
        step_title,
        step_result.text.len()
    );

    // Find the subtask index by matching title
    if let Some(step_index) = params
        .task_list
        .steps
        .iter()
        .position(|step| step.title == step_title)
    {
        merlin_deps::tracing::debug!(
            "Found step at index {}, subtask count: {}",
            step_index,
            work_unit_guard.subtasks.len()
        );

        if let Some(subtask) = work_unit_guard.subtasks.get(step_index) {
            let subtask_id = subtask.id;
            merlin_deps::tracing::debug!(
                "Marking subtask {} (id: {:?}, current status: {:?}) as completed",
                step_index,
                subtask_id,
                subtask.status
            );
            work_unit_guard.complete_subtask(subtask_id, Some(step_result.text.clone()));

            // Verify it was actually completed
            if let Some(updated_subtask) = work_unit_guard.subtasks.get(step_index) {
                merlin_deps::tracing::debug!(
                    "After update, subtask {} status: {:?}",
                    step_index,
                    updated_subtask.status
                );
            }
        } else {
            merlin_deps::tracing::warn!(
                "No subtask found at index {} (step_title: '{}')",
                step_index,
                step_title
            );
        }

        // Send progress update to UI
        let completed_count = completed.len() + 1;
        let progress_pct = work_unit_guard.progress_percentage();
        let total_subtasks = work_unit_guard.subtasks.len();
        let completed_subtasks = work_unit_guard
            .subtasks
            .iter()
            .filter(|subtask| matches!(subtask.status, SubtaskStatus::Completed))
            .count();

        merlin_deps::tracing::debug!(
            "WorkUnit progress after step '{}': {:.1}% ({}/{} completed, verified {}/{})",
            step_title,
            progress_pct,
            completed_count,
            total_subtasks,
            completed_subtasks,
            total_subtasks,
        );
    } else {
        merlin_deps::tracing::warn!("Step '{}' not found in task_list.steps", step_title);
    }
}

/// Execute a single step and update tracking state
///
/// # Errors
/// Returns an error if step execution fails
async fn execute_and_track_step(
    params: &mut TaskListExecutionParams<'_>,
    index: usize,
    step: &TaskStep,
    results_by_title: &HashMap<String, StepResult>,
    completed: &mut HashSet<String>,
) -> Result<StepResult> {
    merlin_deps::tracing::debug!(
        "Executing step {}/{}: {}",
        index + 1,
        params.task_list.steps.len(),
        step.title
    );

    // Mark subtask as started in WorkUnit if tracking
    if let Some(work_unit) = params.work_unit {
        mark_subtask_started(work_unit, index).await;
    }

    // Build previous_results as a Vec in step order (for consistency)
    let previous_results: Vec<StepResult> = params
        .task_list
        .steps
        .iter()
        .filter_map(|task_step| results_by_title.get(&task_step.title).cloned())
        .collect();

    // Execute step - this is where parallel I/O escapes happen
    let step_result = StepExecutor::execute_step_impl(StepExecutionParams {
        step,
        base_context: params.base_context,
        previous_results: &previous_results,
        provider: params.provider,
        tool_registry: params.tool_registry,
        runtime: params.runtime,
        task_id: params.task_id,
        ui_channel: params.ui_channel,
        recursion_depth: params.recursion_depth,
        retry_attempt: 0,
        previous_result: None,
    })
    .await?;

    completed.insert(step.title.clone());

    // Mark subtask as completed in WorkUnit if tracking
    if let Some(work_unit) = params.work_unit {
        update_work_unit_on_completion(work_unit, params, &step.title, &step_result, completed)
            .await;
    }

    Ok(step_result)
}

/// Execute a task list with dependency-aware sequential execution
///
/// Steps execute sequentially (to access `&mut runtime`), but each step
/// spawns parallel work for I/O operations (LLM calls, file validation).
///
/// # Errors
/// Returns an error if any step fails or if circular dependencies are detected
pub(super) async fn execute_task_list_parallel(
    params: &mut TaskListExecutionParams<'_>,
) -> Result<StepResult> {
    let start = Instant::now();

    // Store results by step title for dependency resolution
    let mut results_by_title: HashMap<String, StepResult> = HashMap::new();

    // Track completion
    let mut completed: HashSet<String> = HashSet::new();

    merlin_deps::tracing::debug!(
        "Executing task list '{}' with {} steps at depth {}, work_unit tracking: {}",
        params.task_list.title,
        params.task_list.steps.len(),
        params.recursion_depth,
        params.work_unit.is_some()
    );

    let has_dependencies = params
        .task_list
        .steps
        .iter()
        .any(|step| !step.dependencies.is_empty());

    if has_dependencies {
        merlin_deps::tracing::info!(
            "Using dependency-aware execution for task list '{}'",
            params.task_list.title
        );
    }

    loop {
        // Find next step ready to execute (dependencies met, not completed)
        let next_step = params
            .task_list
            .steps
            .iter()
            .enumerate()
            .find(|(_idx, step)| {
                !completed.contains(&step.title)
                    && step.dependencies.iter().all(|dep| completed.contains(dep))
            });

        let Some((index, step)) = next_step else {
            // Check if we're done: all steps completed
            if completed.len() == params.task_list.steps.len() {
                break;
            }

            // Otherwise, we have a deadlock
            return check_for_deadlock(params, &completed);
        };

        let step_result =
            execute_and_track_step(params, index, step, &results_by_title, &mut completed).await?;
        results_by_title.insert(step.title.clone(), step_result);
    }

    Ok(StepResult {
        text: build_combined_output(params, &results_by_title),
        duration_ms: start.elapsed().as_millis() as u64,
        success: true,
    })
}
