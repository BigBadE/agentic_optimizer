//! Parallel task execution with dependency resolution

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

use merlin_core::{Result, RoutingError, TaskStep, WorkUnit};

use super::step_executor::{
    StepExecutionParams, StepExecutor, StepResult, TaskListExecutionParams,
};

/// Type alias for parallel execution future return type
type ParallelExecutionFuture<'lifetime> =
    Pin<Box<dyn Future<Output = Result<StepResult>> + Send + 'lifetime>>;

/// Type alias for step result join set
type StepJoinSet = JoinSet<(String, Result<StepResult>)>;

/// Context for spawning step execution tasks
struct StepSpawnContext<'ctx> {
    /// Task list execution parameters
    params: &'ctx TaskListExecutionParams<'ctx>,
    /// Results from completed steps
    results_by_title: &'ctx HashMap<String, StepResult>,
    /// Whether dependencies are being tracked
    has_dependencies: bool,
}

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

/// Spawn a step execution task
fn spawn_step_task(
    join_set: &mut StepJoinSet,
    step: &TaskStep,
    context: &StepSpawnContext<'_>,
    index: usize,
) {
    merlin_deps::tracing::debug!(
        "Starting step {}/{}: {}{}",
        index + 1,
        context.params.task_list.steps.len(),
        step.title,
        if context.has_dependencies {
            " (parallel)"
        } else {
            ""
        }
    );

    // Clone data for the async task
    let step_clone = step.clone();
    let base_context = context.params.base_context.clone();
    let provider = Arc::clone(context.params.provider);
    let tool_registry = Arc::clone(context.params.tool_registry);
    let runtime = Arc::clone(context.params.runtime);
    let task_id = context.params.task_id;
    let ui_channel = context.params.ui_channel.clone();
    let recursion_depth = context.params.recursion_depth;
    let step_title = step.title.clone();

    // Build previous_results as a Vec in step order (for consistency)
    let previous_results: Vec<StepResult> = context
        .params
        .task_list
        .steps
        .iter()
        .filter_map(|task_step| context.results_by_title.get(&task_step.title).cloned())
        .collect();

    join_set.spawn(async move {
        let result = StepExecutor::execute_step_impl(StepExecutionParams {
            step: &step_clone,
            base_context: &base_context,
            previous_results: &previous_results,
            provider: &provider,
            tool_registry: &tool_registry,
            runtime: &runtime,
            task_id,
            ui_channel: &ui_channel,
            recursion_depth,
        })
        .await;
        (step_title, result)
    });
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
            .filter(|s| matches!(s.status, merlin_core::SubtaskStatus::Completed))
            .count();

        merlin_deps::tracing::debug!(
            "WorkUnit progress: {progress_pct}% ({completed_subtasks}/{total_subtasks} subtasks completed)"
        );

        // Send both progress events for compatibility
        params.ui_channel.progress(
            params.task_id,
            format!("step-{completed_count}"),
            format!("Completed: {step_title} ({progress_pct}%)"),
        );

        params.ui_channel.work_unit_progress(
            params.task_id,
            progress_pct,
            completed_subtasks,
            total_subtasks,
        );
    } else {
        merlin_deps::tracing::warn!(
            "Step '{}' not found in task_list.steps",
            step_title
        );
    }
}

/// Execute a task list with dependency-aware parallel execution
///
/// # Errors
/// Returns an error if any step fails or if circular dependencies are detected
pub(super) fn execute_task_list_parallel<'lifetime>(
    params: &'lifetime TaskListExecutionParams<'lifetime>,
) -> ParallelExecutionFuture<'lifetime> {
    Box::pin(execute_task_list_parallel_impl(params))
}

/// Determine concurrency level for execution
fn determine_concurrency(params: &TaskListExecutionParams<'_>) -> (bool, usize) {
    let has_dependencies = params
        .task_list
        .steps
        .iter()
        .any(|step| !step.dependencies.is_empty());

    let max_concurrent = if has_dependencies { 4 } else { 1 };

    if has_dependencies {
        merlin_deps::tracing::info!(
            "Using parallel execution for task list '{}' (dependencies detected)",
            params.task_list.title
        );
    }

    (has_dependencies, max_concurrent)
}

/// Implementation of parallel task list execution
///
/// # Errors
/// Returns an error if any step fails or if circular dependencies are detected
async fn execute_task_list_parallel_impl<'lifetime>(
    params: &'lifetime TaskListExecutionParams<'lifetime>,
) -> Result<StepResult> {
    let start = Instant::now();

    // Store results by step title for dependency resolution
    let mut results_by_title: HashMap<String, StepResult> = HashMap::new();

    // Track completion and execution state
    let mut completed: HashSet<String> = HashSet::new();
    let mut running: HashSet<String> = HashSet::new();
    let mut join_set: StepJoinSet = JoinSet::new();

    merlin_deps::tracing::debug!(
        "Executing task list '{}' with {} steps at depth {}, work_unit tracking: {}",
        params.task_list.title,
        params.task_list.steps.len(),
        params.recursion_depth,
        params.work_unit.is_some()
    );

    let (has_dependencies, max_concurrent) = determine_concurrency(params);

    loop {
        // Find steps ready to execute (dependencies met, not running/completed)
        let ready_steps: Vec<(usize, &TaskStep)> = params
            .task_list
            .steps
            .iter()
            .enumerate()
            .filter(|(_idx, step)| {
                !completed.contains(&step.title)
                    && !running.contains(&step.title)
                    && step.dependencies.iter().all(|dep| completed.contains(dep))
            })
            .collect();

        // Start new tasks up to concurrency limit
        let spawn_context = StepSpawnContext {
            params,
            results_by_title: &results_by_title,
            has_dependencies,
        };

        for (index, step) in ready_steps {
            if running.len() >= max_concurrent {
                break;
            }

            // Mark subtask as started in WorkUnit if tracking
            if let Some(work_unit) = params.work_unit {
                mark_subtask_started(work_unit, index).await;
            }

            running.insert(step.title.clone());
            spawn_step_task(&mut join_set, step, &spawn_context, index);
        }

        // Check if we're done: all steps completed
        if completed.len() == params.task_list.steps.len() {
            break;
        }

        // If nothing is running, something went wrong (circular dependencies?)
        if join_set.is_empty() {
            return check_for_deadlock(params, &completed);
        }

        // Wait for at least one step to complete
        if let Some(joined) = join_set.join_next().await {
            let (step_title, result) =
                joined.map_err(|err| RoutingError::ExecutionFailed(err.to_string()))?;

            let step_result = result?;

            running.remove(&step_title);
            completed.insert(step_title.clone());

            // Mark subtask as completed in WorkUnit if tracking
            if let Some(work_unit) = params.work_unit {
                update_work_unit_on_completion(
                    work_unit,
                    params,
                    &step_title,
                    &step_result,
                    &completed,
                )
                .await;
            }

            results_by_title.insert(step_title, step_result);
        }
    }

    Ok(StepResult {
        text: build_combined_output(params, &results_by_title),
        duration_ms: start.elapsed().as_millis() as u64,
        success: true,
    })
}
