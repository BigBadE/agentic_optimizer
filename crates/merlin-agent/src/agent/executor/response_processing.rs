//! Response processing for agent executor

use std::sync::Arc;

use merlin_core::{
    AgentResponse, Context, ModelProvider, Response, Result, StepType, Task, TaskId, TaskList,
    TaskResult, TokenUsage, ValidationResult, WorkUnit,
};
use merlin_routing::{RoutingDecision, UiChannel};
use merlin_tooling::{PersistentTypeScriptRuntime, ToolRegistry};
use tokio::sync::Mutex;

use super::step_executor::{StepExecutor, TaskListExecutionParams};
use crate::Validator;

/// Parameters for processing agent response
pub struct ResponseProcessingParams<'resp> {
    /// Agent response to process
    pub agent_response: AgentResponse,
    /// Task ID
    pub task_id: TaskId,
    /// Task being executed
    pub task: &'resp Task,
    /// Routing decision
    pub decision: &'resp RoutingDecision,
    /// Context used
    pub context: &'resp Context,
    /// Provider used
    pub provider: &'resp Arc<dyn ModelProvider>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// UI channel for events
    pub ui_channel: &'resp UiChannel,
}

/// Response processor for agent responses
pub struct ResponseProcessor<'proc> {
    /// Validator for responses
    validator: &'proc Arc<dyn Validator>,
    /// Tool registry
    tool_registry: &'proc ToolRegistry,
    /// Persistent TypeScript runtime
    runtime: &'proc mut PersistentTypeScriptRuntime,
}

impl<'proc> ResponseProcessor<'proc> {
    /// Create a new response processor
    pub fn new(
        validator: &'proc Arc<dyn Validator>,
        tool_registry: &'proc ToolRegistry,
        runtime: &'proc mut PersistentTypeScriptRuntime,
    ) -> Self {
        Self {
            validator,
            tool_registry,
            runtime,
        }
    }

    /// Process agent response and create task result
    ///
    /// # Errors
    /// Returns an error if validation or task list execution fails
    pub async fn process_response(
        &mut self,
        params: ResponseProcessingParams<'_>,
    ) -> Result<TaskResult> {
        match params.agent_response {
            AgentResponse::DirectResult(ref result) => {
                self.process_direct_result(result.clone(), params).await
            }
            AgentResponse::TaskList(ref task_list) => {
                self.process_task_list(task_list.clone(), params).await
            }
        }
    }

    /// Process direct result response
    ///
    /// # Errors
    /// Returns an error if validation fails
    async fn process_direct_result(
        &self,
        result: String,
        params: ResponseProcessingParams<'_>,
    ) -> Result<TaskResult> {
        merlin_deps::tracing::debug!("Agent returned DirectResult");
        let response = Response {
            text: result,
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: params.decision.model.to_string(),
            latency_ms: params.duration_ms,
        };

        let validation = self.validate_response(&response, params.task).await?;

        Ok(TaskResult {
            task_id: params.task_id,
            response,
            tier_used: params.decision.model.to_string(),
            tokens_used: TokenUsage::default(),
            validation,
            duration_ms: params.duration_ms,
            work_unit: None,
        })
    }

    /// Process task list response
    ///
    /// # Errors
    /// Returns an error if execution or validation fails
    async fn process_task_list(
        &mut self,
        task_list: TaskList,
        params: ResponseProcessingParams<'_>,
    ) -> Result<TaskResult> {
        merlin_deps::tracing::debug!(
            "Agent returned TaskList with {} steps",
            task_list.steps.len()
        );

        // Create WorkUnit to track decomposed work
        let mut work_unit = WorkUnit::new(params.task_id, params.provider.name().to_owned());

        // Add subtasks for each step in the TaskList
        for step in &task_list.steps {
            let difficulty = Self::estimate_step_difficulty(step.step_type);
            work_unit.add_subtask(step.title.clone(), difficulty);
            merlin_deps::tracing::debug!(
                "Added subtask: {} (difficulty: {})",
                step.title,
                difficulty
            );
        }

        merlin_deps::tracing::debug!(
            "Created WorkUnit with {} subtasks, initial progress: {}%",
            work_unit.subtasks.len(),
            work_unit.progress_percentage()
        );

        // Wrap WorkUnit in Arc<Mutex<>> for shared mutable access during execution
        let work_unit_shared = Arc::new(Mutex::new(work_unit));

        // Send WorkUnit to UI for mid-execution tracking
        params
            .ui_channel
            .work_unit_started(params.task_id, Arc::clone(&work_unit_shared));

        let step_result = StepExecutor::execute_task_list(TaskListExecutionParams {
            task_list: &task_list,
            base_context: params.context,
            provider: params.provider,
            tool_registry: self.tool_registry,
            runtime: self.runtime,
            task_id: params.task_id,
            ui_channel: params.ui_channel,
            recursion_depth: 0,
            work_unit: Some(&work_unit_shared),
        })
        .await?;

        let response = Response {
            text: step_result.text,
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: params.decision.model.to_string(),
            latency_ms: step_result.duration_ms,
        };

        let validation = self.validate_response(&response, params.task).await?;

        // Clone work unit from Arc<Mutex<>> (TUI may still hold a reference)
        let final_work_unit = {
            let mut work_unit_guard = work_unit_shared.lock().await;
            work_unit_guard.duration_ms = step_result.duration_ms;
            work_unit_guard.complete();
            work_unit_guard.clone()
        };

        Ok(TaskResult {
            task_id: params.task_id,
            response,
            tier_used: params.decision.model.to_string(),
            tokens_used: TokenUsage::default(),
            validation,
            duration_ms: step_result.duration_ms,
            work_unit: Some(final_work_unit),
        })
    }

    /// Estimate difficulty for a step type
    const fn estimate_step_difficulty(step_type: StepType) -> u8 {
        match step_type {
            StepType::Research => 3,
            StepType::Planning => 4,
            StepType::Implementation => 7,
            StepType::Validation => 5,
            StepType::Documentation => 2,
        }
    }

    /// Validate response and log failures
    ///
    /// # Errors
    /// Returns an error if validation fails
    async fn validate_response(
        &self,
        response: &Response,
        task: &Task,
    ) -> Result<ValidationResult> {
        self.validator
            .validate(response, task)
            .await
            .map_err(|validation_error| {
                merlin_deps::tracing::info!(
                    "Validation failed. Model response was:\n{}\n\nError: {:?}",
                    response.text,
                    validation_error
                );
                validation_error
            })
    }
}
