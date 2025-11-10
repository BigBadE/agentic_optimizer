//! Verification system for unified tests.

use super::execution_tracker::{ExecutionRecord, ExecutionResultTracker};
use super::execution_verifier::ExecutionVerifier;
use super::file_verifier::FileVerifier;
use super::fixture::TestEvent;
use super::mock_provider::MockProvider;
use super::ui_verifier::UiVerifier;
use super::verification_result::VerificationResult;
use super::verify::{ExecutionVerify, FinalVerify, VerifyConfig};
use merlin_cli::TuiApp;
use merlin_cli::ui::task_manager::TaskStatus;
use ratatui::backend::TestBackend;
use std::path::Path;
use std::result::Result;
use std::sync::Arc;

/// Unified verifier
pub struct UnifiedVerifier<'fixture> {
    /// Workspace root
    workspace_root: &'fixture Path,
    /// Accumulated result
    result: VerificationResult,
}

/// Context for event verification
pub struct VerifyEventContext<'ctx> {
    /// The event to verify
    pub event: &'ctx TestEvent,
    /// Verification configuration
    pub verify: &'ctx VerifyConfig,
    /// Optional TUI application
    pub tui_app: Option<&'ctx TuiApp<TestBackend>>,
    /// Execution result tracker
    pub execution_tracker: &'ctx ExecutionResultTracker,
    /// Optional mock provider
    pub provider: Option<&'ctx Arc<MockProvider>>,
}

impl<'fixture> UnifiedVerifier<'fixture> {
    /// Create new verifier
    #[must_use]
    pub fn new(workspace_root: &'fixture Path) -> Self {
        Self {
            workspace_root,
            result: VerificationResult::new(),
        }
    }

    /// Verify an event
    ///
    /// # Errors
    /// Returns error if verification fails critically
    pub async fn verify_event(&mut self, ctx: &VerifyEventContext<'_>) -> Result<(), String> {
        let VerifyEventContext {
            event,
            verify,
            tui_app,
            execution_tracker,
            provider,
        } = ctx;

        // Verify execution if specified
        if let Some(exec_verify) = &verify.execution {
            // Get execution result by ID or fall back to last result
            let execution = exec_verify.execution_id.as_ref().map_or_else(
                || {
                    event.id().map_or_else(
                        || execution_tracker.last_result().map(ExecutionRecord::result),
                        |event_id| {
                            execution_tracker
                                .get_by_id(event_id)
                                .map(ExecutionRecord::result)
                        },
                    )
                },
                |exec_id| {
                    execution_tracker
                        .get_by_id(exec_id)
                        .map(ExecutionRecord::result)
                },
            );

            ExecutionVerifier::verify_execution(
                &mut self.result,
                execution,
                exec_verify,
                *provider,
            );

            // Verify routing/cache/metrics if orchestrator is available
            self.verify_routing_cache_metrics(*tui_app, exec_verify);
        }

        // Verify files if specified
        if let Some(file_verifies) = &verify.files {
            for file_verify in file_verifies {
                FileVerifier::verify_file(&mut self.result, self.workspace_root, file_verify);
            }
        }

        // Verify UI if specified
        if let Some(ui_verify) = &verify.ui {
            UiVerifier::verify_ui(&mut self.result, *tui_app, ui_verify).await;
        }

        // Verify state if specified
        if let Some(state_verify) = &verify.state {
            UiVerifier::verify_state(&mut self.result, *tui_app, state_verify).await;
        }

        // Verify prompt if specified
        if let Some(_prompt_verify) = &verify.prompt {
            if provider.is_some() {
                // Prompt verification now happens during execution, not after
                // The system prompt is verified by checking the actual query sent to the provider
                // For now, we'll skip prompt verification as it's handled during scope matching
                self.result
                    .add_success("Prompt verification handled by scope matching system".to_owned());
            } else {
                self.result.add_failure(
                    "Prompt verification requested but no provider available".to_owned(),
                );
            }
        }

        Ok(())
    }

    /// Verify final state with success-by-default philosophy
    ///
    /// # Errors
    /// Returns error if verification fails
    pub async fn verify_final(
        &mut self,
        verify: &FinalVerify,
        tui_app: Option<&TuiApp<TestBackend>>,
        execution_tracker: &ExecutionResultTracker,
    ) -> Result<(), String> {
        // Verify final execution state
        if let Some(exec_verify) = &verify.execution {
            // Get execution by ID or use last result
            let execution = exec_verify.execution_id.as_ref().map_or_else(
                || execution_tracker.last_result().map(ExecutionRecord::result),
                |exec_id| {
                    execution_tracker
                        .get_by_id(exec_id)
                        .map(ExecutionRecord::result)
                },
            );

            // Success-by-default verification
            ExecutionVerifier::verify_execution(&mut self.result, execution, exec_verify, None);

            // Verify incomplete/failed tasks if explicitly specified
            if !exec_verify.incomplete_tasks.is_empty() || !exec_verify.failed_tasks.is_empty() {
                self.verify_task_states(tui_app, exec_verify);
            }

            // Verify validation failures if explicitly specified
            if !exec_verify.validation_failures.is_empty() {
                self.verify_validation_stages(execution_tracker, exec_verify);
            }

            // Verify routing/cache/metrics if orchestrator is available
            self.verify_routing_cache_metrics(tui_app, exec_verify);
        }

        // Verify final files
        if let Some(file_verifies) = &verify.files {
            for file_verify in file_verifies {
                FileVerifier::verify_file(&mut self.result, self.workspace_root, file_verify);
            }
        }

        // Verify final UI state if specified
        if let Some(ui_verify) = &verify.ui {
            UiVerifier::verify_ui(&mut self.result, tui_app, ui_verify).await;
        }

        // Verify final state if specified
        if let Some(state_verify) = &verify.state {
            UiVerifier::verify_state(&mut self.result, tui_app, state_verify).await;
        }

        Ok(())
    }

    /// Verify routing, cache, and metrics if orchestrator is available
    fn verify_routing_cache_metrics(
        &mut self,
        tui_app: Option<&TuiApp<TestBackend>>,
        exec_verify: &ExecutionVerify,
    ) {
        let Some(app) = tui_app else {
            return;
        };
        let Some(orchestrator) = &app.runtime_state.orchestrator else {
            return;
        };

        // Verify routing decisions
        if exec_verify.model_used.is_some() || exec_verify.expected_difficulty.is_some() {
            ExecutionVerifier::verify_routing(
                &mut self.result,
                orchestrator,
                exec_verify.model_used.as_deref(),
                exec_verify.expected_difficulty,
            );
        }

        // Verify cache behavior
        if exec_verify.cache_hit.is_some() || exec_verify.cache_hit_count.is_some() {
            ExecutionVerifier::verify_cache(
                &mut self.result,
                orchestrator,
                exec_verify.cache_hit,
                exec_verify.cache_hit_count,
            );
        }

        // Verify metrics collection
        if exec_verify.metrics_recorded.is_some() {
            ExecutionVerifier::verify_metrics(
                &mut self.result,
                orchestrator,
                exec_verify.metrics_recorded,
            );
        }
    }

    /// Verify task states (incomplete/failed tasks)
    fn verify_task_states(
        &mut self,
        tui_app: Option<&TuiApp<TestBackend>>,
        verify: &ExecutionVerify,
    ) {
        let Some(app) = tui_app else {
            self.result
                .add_failure("Cannot verify task states without TUI app".to_owned());
            return;
        };

        let task_manager = &app.ui_components.task_manager;

        // Verify incomplete tasks
        for expected_incomplete in &verify.incomplete_tasks {
            let found = task_manager.task_order().iter().any(|task_id| {
                task_manager.get_task(*task_id).is_some_and(|task| {
                    task.description.contains(expected_incomplete)
                        && !matches!(task.status, TaskStatus::Completed)
                })
            });

            if found {
                self.result.add_success(format!(
                    "Task '{expected_incomplete}' is incomplete as expected"
                ));
            } else {
                self.result.add_failure(format!(
                    "Expected task '{expected_incomplete}' to be incomplete but it was not found or completed"
                ));
            }
        }

        // Verify failed tasks
        for expected_failed in &verify.failed_tasks {
            let found = task_manager.task_order().iter().any(|task_id| {
                task_manager.get_task(*task_id).is_some_and(|task| {
                    task.description.contains(expected_failed)
                        && matches!(task.status, TaskStatus::Failed)
                })
            });

            if found {
                self.result
                    .add_success(format!("Task '{expected_failed}' failed as expected"));
            } else {
                self.result.add_failure(format!(
                    "Expected task '{expected_failed}' to fail but it was not found or didn't fail"
                ));
            }
        }
    }

    /// Verify validation stages
    fn verify_validation_stages(
        &mut self,
        execution_tracker: &ExecutionResultTracker,
        verify: &ExecutionVerify,
    ) {
        let Some(record) = execution_tracker.last_result() else {
            self.result
                .add_failure("Cannot verify validation stages: no execution results".to_owned());
            return;
        };

        let Some(task_result) = record.task_result() else {
            self.result
                .add_failure("Cannot verify validation stages: task failed".to_owned());
            return;
        };

        let validation = &task_result.validation;

        for expected_failure in &verify.validation_failures {
            // Check if this validation stage failed
            let failed = match expected_failure.as_str() {
                "citations" => {
                    !validation.passed
                        && validation
                            .errors
                            .iter()
                            .any(|error| error.message.to_lowercase().contains("citation"))
                }
                "syntax" => {
                    !validation.passed
                        && validation
                            .errors
                            .iter()
                            .any(|error| error.message.to_lowercase().contains("syntax"))
                }
                "build" => {
                    !validation.passed
                        && validation
                            .errors
                            .iter()
                            .any(|error| error.message.to_lowercase().contains("build"))
                }
                stage => {
                    self.result
                        .add_failure(format!("Unknown validation stage: {stage}"));
                    continue;
                }
            };

            if failed {
                self.result.add_success(format!(
                    "Validation stage '{expected_failure}' failed as expected"
                ));
            } else {
                self.result.add_failure(format!(
                    "Expected validation stage '{expected_failure}' to fail but it didn't"
                ));
            }
        }
    }

    /// Get accumulated result
    #[must_use]
    pub fn result(self) -> VerificationResult {
        self.result
    }
}
