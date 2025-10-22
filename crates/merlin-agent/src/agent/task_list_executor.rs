//! Task list execution with step-by-step verification.

use super::{AgentExecutor, CommandResult, CommandRunner};
use merlin_core::{
    Context, Result, RoutingError, Task, TaskId, TaskList, TaskListStatus, TaskListStep,
    TaskResult,
    ui::{TaskProgress, UiChannel, UiEvent},
};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Parameters for executing a single task step
struct StepExecutionParams<'exec> {
    task_list: &'exec mut TaskList,
    step_index: usize,
    total_steps: usize,
    context: &'exec Context,
    ui_channel: &'exec UiChannel,
    task_id: TaskId,
}

/// Trait for executing agent tasks - enables mocking in tests
pub trait AgentExecutorWrapper: Send + Sync {
    /// Execute a task with streaming updates
    ///
    /// # Errors
    /// Returns an error if task execution fails
    fn execute_streaming(
        &self,
        task: Task,
        ui_channel: UiChannel,
    ) -> impl Future<Output = Result<TaskResult>> + Send;
}

/// Wrapper around `AgentExecutor` that implements the trait
pub struct RealAgentExecutorWrapper {
    executor: Arc<Mutex<AgentExecutor>>,
}

impl RealAgentExecutorWrapper {
    /// Create a new wrapper around an `AgentExecutor`
    pub fn new(executor: AgentExecutor) -> Self {
        Self {
            executor: Arc::new(Mutex::new(executor)),
        }
    }
}

impl AgentExecutorWrapper for RealAgentExecutorWrapper {
    async fn execute_streaming(&self, task: Task, ui_channel: UiChannel) -> Result<TaskResult> {
        let mut executor = self.executor.lock().await;
        executor.execute_streaming(task, ui_channel).await
    }
}

/// Result of executing a task list
#[derive(Debug, Clone)]
pub enum TaskListResult {
    /// All steps completed successfully
    Success,
    /// One or more steps failed
    Failed {
        /// ID of the step that failed
        failed_step: String,
    },
}

/// Executor for running task lists step-by-step with verification
pub struct TaskListExecutor<E: AgentExecutorWrapper> {
    /// Agent executor for running individual steps
    agent_executor: Arc<E>,
    /// Command runner for executing exit commands
    command_runner: CommandRunner,
}

impl TaskListExecutor<RealAgentExecutorWrapper> {
    /// Create a new task list executor with a real agent executor
    #[must_use]
    pub fn new(agent_executor: &Arc<AgentExecutor>, working_dir: PathBuf) -> Self {
        let wrapper = RealAgentExecutorWrapper::new((**agent_executor).clone());
        Self {
            agent_executor: Arc::new(wrapper),
            command_runner: CommandRunner::new(working_dir),
        }
    }
}

impl<E: AgentExecutorWrapper> TaskListExecutor<E> {
    /// Create a new task list executor with a custom executor wrapper (for testing)
    pub fn new_with_wrapper(agent_executor: Arc<E>, working_dir: PathBuf) -> Self {
        Self {
            agent_executor,
            command_runner: CommandRunner::new(working_dir),
        }
    }

    /// Execute a task list step-by-step
    ///
    /// # Errors
    /// Returns an error if step execution or verification fails
    pub async fn execute_task_list(
        &self,
        task_list: &mut TaskList,
        context: &Context,
        ui_channel: &UiChannel,
        task_id: TaskId,
    ) -> Result<TaskListResult> {
        tracing::info!(
            "Starting task list execution: {} ({} steps)",
            task_list.title,
            task_list.steps.len()
        );

        task_list.status = TaskListStatus::InProgress;
        Self::send_initial_progress(task_list, ui_channel, task_id);

        let total_steps = task_list.steps.len();
        for step_index in 0..total_steps {
            let params = StepExecutionParams {
                task_list,
                step_index,
                total_steps,
                context,
                ui_channel,
                task_id,
            };

            let result = self.execute_single_step(params).await?;

            if let Some(failed_step) = result {
                task_list.status = TaskListStatus::Failed;
                return Ok(TaskListResult::Failed { failed_step });
            }

            task_list.update_status();
        }

        Self::send_completion_events(task_list, ui_channel, task_id);
        Ok(TaskListResult::Success)
    }

    /// Send initial progress event
    fn send_initial_progress(task_list: &TaskList, ui_channel: &UiChannel, task_id: TaskId) {
        ui_channel.send(UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage: "Task List Execution".to_owned(),
                current: 0,
                total: Some(task_list.steps.len() as u64),
                message: format!("Starting: {title}", title = task_list.title),
            },
        });
    }

    /// Execute a single step from the task list
    ///
    /// # Errors
    /// Returns an error if step execution or verification fails
    ///
    /// # Returns
    /// Returns `Ok(Some(step_id))` if the step failed, `Ok(None)` if it succeeded
    async fn execute_single_step(&self, params: StepExecutionParams<'_>) -> Result<Option<String>> {
        let StepExecutionParams {
            task_list,
            step_index,
            total_steps,
            context,
            ui_channel,
            task_id,
        } = params;

        let step = &mut task_list.steps[step_index];
        step.start();

        tracing::info!(
            "Executing step {}/{}: {}",
            step_index + 1,
            total_steps,
            step.description
        );

        Self::send_step_events(step, step_index, total_steps, ui_channel, task_id);

        let step_result = self.execute_step(step, context, ui_channel, task_id).await;

        match step_result {
            Ok(()) => {
                let verification_passed =
                    self.verify_and_fix_step(step, ui_channel, task_id).await?;
                if verification_passed {
                    Ok(None)
                } else {
                    Ok(Some(step.id.clone()))
                }
            }
            Err(err) => {
                Self::handle_step_failure(step, &err, ui_channel, task_id);
                Ok(Some(step.id.clone()))
            }
        }
    }

    /// Send step started and progress events
    fn send_step_events(
        step: &TaskListStep,
        step_index: usize,
        total_steps: usize,
        ui_channel: &UiChannel,
        task_id: TaskId,
    ) {
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: step.id.clone(),
            step_type: step.step_type.to_string(),
            content: step.description.clone(),
        });

        ui_channel.send(UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage: "Task List Execution".to_owned(),
                current: step_index as u64,
                total: Some(total_steps as u64),
                message: format!(
                    "Step {current}/{total}: {desc}",
                    current = step_index + 1,
                    total = total_steps,
                    desc = step.description
                ),
            },
        });
    }

    /// Handle step execution failure
    fn handle_step_failure(
        step: &mut TaskListStep,
        err: &RoutingError,
        ui_channel: &UiChannel,
        task_id: TaskId,
    ) {
        let error_msg = format!("Step execution failed: {err}");
        step.fail(error_msg.clone());

        ui_channel.send(UiEvent::TaskStepFailed {
            task_id,
            step_id: step.id.clone(),
            error: error_msg,
        });

        tracing::error!("Step execution failed: {} - {}", step.description, err);
    }

    /// Send completion events
    fn send_completion_events(task_list: &mut TaskList, ui_channel: &UiChannel, task_id: TaskId) {
        task_list.status = TaskListStatus::Completed;

        ui_channel.send(UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage: "Task List Execution".to_owned(),
                current: task_list.steps.len() as u64,
                total: Some(task_list.steps.len() as u64),
                message: "All steps completed".to_owned(),
            },
        });

        ui_channel.send(UiEvent::TaskOutput {
            task_id,
            output: format!("✅ Task list completed: {}", task_list.title),
        });

        tracing::info!(
            "Task list execution completed successfully: {}",
            task_list.title
        );
    }

    /// Execute a single task step using the agent
    ///
    /// # Errors
    /// Returns an error if the agent execution fails
    async fn execute_step(
        &self,
        step: &TaskListStep,
        _context: &Context,
        ui_channel: &UiChannel,
        _task_id: TaskId,
    ) -> Result<()> {
        // Generate prompt for this specific step
        let step_prompt = format!(
            "Execute the following task step:\n\n\
            Type: {}\n\
            Description: {}\n\
            Verification: {}\n\n\
            Complete this step and ensure it meets the verification criteria.",
            step.step_type, step.description, step.verification
        );

        tracing::debug!("Executing step with agent: {}", step.description);

        // Create a task for this step
        let step_task = Task::new(step_prompt);

        // Execute with the agent (streaming) using the trait method
        let _result = self
            .agent_executor
            .execute_streaming(step_task, ui_channel.clone())
            .await?;

        tracing::debug!("Step execution completed: {}", step.description);

        Ok(())
    }

    /// Attempt to auto-fix a failed step using the agent
    ///
    /// # Errors
    /// Returns an error if the fix attempt fails
    async fn attempt_fix(
        &self,
        step: &TaskListStep,
        verification: &super::CommandResult,
        ui_channel: &UiChannel,
        task_id: TaskId,
    ) -> Result<bool> {
        tracing::info!("Attempting auto-fix for step: {}", step.description);

        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: format!("{}_fix", &step.id),
            step_type: "fix".to_owned(),
            content: "Analyzing failure and attempting fix".to_owned(),
        });

        // Generate fix prompt with detailed error information
        let fix_prompt = format!(
            "A task step has failed. Please analyze the error and fix the issue.\n\n\
            Step Type: {}\n\
            Step Description: {}\n\
            Verification Requirement: {}\n\
            Exit Command: {}\n\
            Exit Code: {}\n\n\
            Error Output:\n{}\n\n\
            Please fix the issue that caused this failure.",
            step.step_type,
            step.description,
            step.verification,
            step.get_exit_command(),
            verification.exit_code,
            verification.error_message()
        );

        tracing::debug!("Fix prompt: {}", fix_prompt);

        // Create a task for the fix
        let fix_task = Task::new(fix_prompt);

        // Execute fix with the agent using the trait method
        let fix_result = self
            .agent_executor
            .execute_streaming(fix_task, ui_channel.clone())
            .await;

        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: format!("{}_fix", &step.id),
        });

        match fix_result {
            Ok(_) => {
                tracing::info!("Auto-fix completed, will re-verify");
                Ok(true)
            }
            Err(err) => {
                tracing::error!("Auto-fix failed: {}", err);
                Ok(false)
            }
        }
    }

    /// Verify a step and attempt to fix if verification fails
    ///
    /// # Errors
    /// Returns an error if command execution fails
    ///
    /// # Returns
    /// Returns `Ok(true)` if verification passed, `Ok(false)` otherwise
    async fn verify_and_fix_step(
        &self,
        step: &mut TaskListStep,
        ui_channel: &UiChannel,
        task_id: TaskId,
    ) -> Result<bool> {
        let exit_cmd = step.get_exit_command().to_owned();
        tracing::debug!("Running exit command: {exit_cmd}");

        let verification = self.command_runner.run(&exit_cmd)?;

        if verification.success {
            // Exit command passed
            let result_msg = format!("✅ {exit_cmd}");
            step.complete(Some(result_msg));

            ui_channel.send(UiEvent::TaskStepCompleted {
                task_id,
                step_id: step.id.clone(),
            });

            ui_channel.send(UiEvent::TaskOutput {
                task_id,
                output: format!("Step completed: {}", step.description),
            });

            return Ok(true);
        }

        // Verification failed - attempt fix
        let error_msg = format!(
            "❌ Exit command failed: {exit_cmd}\n{}",
            verification.error_message()
        );
        step.fail(error_msg.clone());

        ui_channel.send(UiEvent::TaskStepFailed {
            task_id,
            step_id: step.id.clone(),
            error: error_msg.clone(),
        });

        tracing::warn!(
            "Step failed verification: {} - {error_msg}",
            step.description
        );

        // Attempt auto-fix and retry
        self.attempt_fix_and_retry(step, &verification, ui_channel, task_id)
            .await
    }

    /// Attempt to fix a failed step and retry verification
    ///
    /// # Errors
    /// Returns an error if command execution fails
    ///
    /// # Returns
    /// Returns `Ok(true)` if fix succeeded and verification passed, `Ok(false)` otherwise
    async fn attempt_fix_and_retry(
        &self,
        step: &mut TaskListStep,
        verification: &CommandResult,
        ui_channel: &UiChannel,
        task_id: TaskId,
    ) -> Result<bool> {
        let fix_succeeded = self
            .attempt_fix(step, verification, ui_channel, task_id)
            .await?;

        if !fix_succeeded {
            return Ok(false);
        }

        // Fix succeeded, retry verification
        let exit_cmd_for_retry = step.get_exit_command().to_owned();
        let recheck = self.command_runner.run(&exit_cmd_for_retry)?;

        if !recheck.success {
            // Fix didn't work
            return Ok(false);
        }

        // Success after fix
        let result_msg = format!("✅ {exit_cmd_for_retry} (after fix)");
        step.complete(Some(result_msg));

        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: step.id.clone(),
        });

        ui_channel.send(UiEvent::TaskOutput {
            task_id,
            output: format!("Step completed after fix: {}", step.description),
        });

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValidationPipeline;
    use merlin_context::ContextFetcher;
    use merlin_core::{Response, RoutingConfig, TaskStepType, TokenUsage, ValidationResult};
    use merlin_routing::StrategyRouter;
    use merlin_tooling::ToolRegistry;
    use std::path::PathBuf;
    use tokio::sync::Mutex as TokioMutex;

    /// Mock agent executor wrapper for testing
    struct MockAgentExecutorWrapper {
        should_succeed: Arc<TokioMutex<bool>>,
    }

    impl MockAgentExecutorWrapper {
        fn new(should_succeed: bool) -> Self {
            Self {
                should_succeed: Arc::new(TokioMutex::new(should_succeed)),
            }
        }
    }

    impl AgentExecutorWrapper for MockAgentExecutorWrapper {
        async fn execute_streaming(&self, task: Task, ui_channel: UiChannel) -> Result<TaskResult> {
            let should_succeed = *self.should_succeed.lock().await;

            // Send a mock output event
            ui_channel.send(UiEvent::TaskOutput {
                task_id: task.id,
                output: "Mock agent executing task".to_owned(),
            });

            if should_succeed {
                Ok(TaskResult {
                    task_id: task.id,
                    response: Response {
                        text: "Mock response".to_owned(),
                        confidence: 1.0,
                        tokens_used: TokenUsage::default(),
                        provider: "mock".to_owned(),
                        latency_ms: 0,
                    },
                    tier_used: "mock".to_owned(),
                    tokens_used: TokenUsage::default(),
                    validation: ValidationResult::default(),
                    duration_ms: 0,
                    task_list: None,
                })
            } else {
                Err(RoutingError::Other("Mock failure".to_owned()))
            }
        }
    }

    /// Create a real agent executor for non-mocked tests
    fn create_real_executor() -> Option<Arc<AgentExecutor>> {
        let router = StrategyRouter::with_default_strategies().ok()?;
        let router = Arc::new(router);
        let validator = Arc::new(ValidationPipeline::with_default_stages());
        let tool_registry = Arc::new(ToolRegistry::default());
        let context_fetcher = ContextFetcher::new(PathBuf::from("."));
        let config = RoutingConfig::default();

        let executor =
            AgentExecutor::new(router, validator, tool_registry, context_fetcher, &config).ok()?;
        Some(Arc::new(executor))
    }

    #[tokio::test]
    async fn test_task_list_executor_creation() {
        if let Some(agent_executor) = create_real_executor() {
            let _executor = TaskListExecutor::new(&agent_executor, PathBuf::from("."));
            // Just verify it was created successfully
        }
        // Test passes even if provider initialization fails
    }

    #[tokio::test]
    async fn test_execute_simple_task_list_with_mock_success() {
        use tokio::sync::mpsc;

        // Create a mock that always succeeds
        let mock_executor = Arc::new(MockAgentExecutorWrapper::new(true));
        let executor = TaskListExecutor::new_with_wrapper(mock_executor, PathBuf::from("."));

        // Create a simple task list with a step that uses a passing exit command
        let mut task_list = TaskList::new(
            "test_list".to_owned(),
            "Test Task List".to_owned(),
            vec![TaskListStep::with_exit_command(
                "step_1".to_owned(),
                TaskStepType::Feature,
                "Test step".to_owned(),
                "Should succeed".to_owned(),
                "true".to_owned(),
            )],
        );

        let context = Context::new("test context");
        let (sender_tx, _rx) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(sender_tx);
        let task_id = TaskId::default();

        // Execute the task list
        let result = executor
            .execute_task_list(&mut task_list, &context, &ui_channel, task_id)
            .await
            .unwrap();

        // Verify success
        assert!(matches!(result, TaskListResult::Success));
        assert!(task_list.is_complete());
    }

    #[tokio::test]
    async fn test_execute_task_list_with_mock_agent_failure() {
        use tokio::sync::mpsc;

        // Create a mock that always fails
        let mock_executor = Arc::new(MockAgentExecutorWrapper::new(false));
        let executor = TaskListExecutor::new_with_wrapper(mock_executor, PathBuf::from("."));

        // Create a task list with a step
        let mut task_list = TaskList::new(
            "test_list".to_owned(),
            "Test Task List".to_owned(),
            vec![TaskListStep::with_exit_command(
                "step_1".to_owned(),
                TaskStepType::Feature,
                "This will fail in agent execution".to_owned(),
                "Should fail".to_owned(),
                "true".to_owned(),
            )],
        );

        let context = Context::new("test context");
        let (sender_tx, _rx) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(sender_tx);
        let task_id = TaskId::default();

        // Execute - should fail at agent execution
        let result = executor
            .execute_task_list(&mut task_list, &context, &ui_channel, task_id)
            .await
            .unwrap();

        // Verify failure
        assert!(matches!(result, TaskListResult::Failed { .. }));
    }

    #[tokio::test]
    async fn test_execute_task_list_exit_command_failure() {
        use tokio::sync::mpsc;

        // Create a mock that succeeds, but use a failing exit command
        let mock_executor = Arc::new(MockAgentExecutorWrapper::new(true));
        let executor = TaskListExecutor::new_with_wrapper(mock_executor, PathBuf::from("."));

        // Create a task list with an exit command that will definitely fail
        let mut task_list = TaskList::new(
            "test_list".to_owned(),
            "Test Task List".to_owned(),
            vec![TaskListStep::with_exit_command(
                "step_1".to_owned(),
                TaskStepType::Feature,
                "This step has a failing exit command".to_owned(),
                "Should fail verification".to_owned(),
                "false".to_owned(),
            )],
        );

        let context = Context::new("test context");
        let (sender_tx, _rx) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(sender_tx);
        let task_id = TaskId::default();

        // Execute - should fail at exit command verification
        let result = executor
            .execute_task_list(&mut task_list, &context, &ui_channel, task_id)
            .await
            .unwrap();

        // Verify failure due to exit command
        assert!(matches!(result, TaskListResult::Failed { .. }));
    }

    #[tokio::test]
    async fn test_execute_task_list_multiple_steps() {
        use tokio::sync::mpsc;

        // Create a mock that always succeeds
        let mock_executor = Arc::new(MockAgentExecutorWrapper::new(true));
        let executor = TaskListExecutor::new_with_wrapper(mock_executor, PathBuf::from("."));

        // Create a task list with multiple steps
        let mut task_list = TaskList::new(
            "test_list".to_owned(),
            "Multi-step Test".to_owned(),
            vec![
                TaskListStep::with_exit_command(
                    "step_1".to_owned(),
                    TaskStepType::Feature,
                    "First step".to_owned(),
                    "Step 1 verification".to_owned(),
                    "true".to_owned(),
                ),
                TaskListStep::with_exit_command(
                    "step_2".to_owned(),
                    TaskStepType::Test,
                    "Second step".to_owned(),
                    "Step 2 verification".to_owned(),
                    "true".to_owned(),
                ),
            ],
        );

        let context = Context::new("test context");
        let (sender_tx, _rx) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(sender_tx);
        let task_id = TaskId::default();

        // Execute all steps
        let result = executor
            .execute_task_list(&mut task_list, &context, &ui_channel, task_id)
            .await
            .unwrap();

        // Verify all steps completed
        assert!(matches!(result, TaskListResult::Success));
        assert!(task_list.is_complete());
        assert_eq!(task_list.steps.len(), 2);
        assert!(task_list.steps[0].is_completed());
        assert!(task_list.steps[1].is_completed());
    }
}
