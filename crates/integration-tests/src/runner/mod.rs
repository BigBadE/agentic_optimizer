//! Unified test runner.
//!
//! This module provides the test runner that executes unified test fixtures
//! by running the actual CLI with pattern-based mock LLM responses.

use super::event_source::FixtureEventController;
use super::execution_tracker::ExecutionResultTracker;
use super::fixture::{TestEvent, TestFixture};
use super::mock_provider::MockProvider;
use super::tui_test_helpers;
use super::verification_result::VerificationResult;
use super::verifier::{UnifiedVerifier, VerifyEventContext};
use merlin_cli::TuiApp;
use merlin_core::{Result, RoutingError};
use merlin_deps::ratatui::backend::TestBackend;
use merlin_deps::tempfile::TempDir;
use merlin_deps::tracing;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{Duration as TokioDuration, sleep};

mod runner_setup;
mod task_completion;

use task_completion::{PendingTaskResult, complete_pending_task};

/// Parameters for handling submit input event
struct SubmitInputParams<'event, 'verifier> {
    event: &'event TestEvent,
    input_event: &'event super::fixture::UserInputEvent,
    event_index: usize,
    verifier: &'event mut UnifiedVerifier<'verifier>,
    execution_tracker: &'event ExecutionResultTracker,
}

/// Parameters for handling LLM response event
struct LlmResponseParams<'event, 'verifier> {
    event: &'event TestEvent,
    llm_event: &'event super::fixture::LlmResponseEvent,
    event_index: usize,
    verifier: &'event mut UnifiedVerifier<'verifier>,
    execution_tracker: &'event ExecutionResultTracker,
}

/// Unified test runner
pub struct UnifiedTestRunner {
    /// Test fixture
    fixture: TestFixture,
    /// Temporary workspace (for writable tests, auto-cleanup)
    _workspace_temp: Option<TempDir>,
    /// Workspace path
    workspace_path: PathBuf,
    /// Mock provider
    provider: Arc<MockProvider>,
    /// The actual TUI application under test
    tui_app: TuiApp<TestBackend>,
    /// Fixture event controller
    event_controller: FixtureEventController,
}

impl UnifiedTestRunner {
    /// Create new test runner with auto-managed workspace
    ///
    /// Context fixtures use read-only workspaces with pre-generated embeddings.
    /// Other fixtures use temporary writable workspaces.
    ///
    /// # Errors
    /// Returns error if workspace setup fails
    pub fn new(fixture: TestFixture) -> Result<Self> {
        let components = runner_setup::create_runner_components(&fixture)?;

        Ok(Self {
            fixture,
            _workspace_temp: components.workspace_temp,
            workspace_path: components.workspace_path,
            provider: components.provider,
            tui_app: components.tui_app,
            event_controller: components.event_controller,
        })
    }

    /// Get workspace path
    #[must_use]
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    /// Get mock provider
    #[must_use]
    pub fn provider(&self) -> Arc<MockProvider> {
        Arc::clone(&self.provider)
    }

    /// Get read-only reference to TUI app for verification
    #[must_use]
    pub fn tui_app(&self) -> &TuiApp<TestBackend> {
        &self.tui_app
    }

    /// Process input events from the event source
    ///
    /// # Errors
    /// Returns error if input processing fails
    async fn process_input_events(&mut self) -> Result<()> {
        while let Some(evt) = tui_test_helpers::next_input_event(&mut self.tui_app).await? {
            tui_test_helpers::handle_input(&mut self.tui_app, &evt);
        }
        Ok(())
    }

    /// Process all pending UI events without polling
    ///
    /// Just processes whatever is already in the queue, no sleeping or waiting.
    fn process_pending_ui_events(&mut self) {
        tui_test_helpers::process_ui_events(&mut self.tui_app);
    }

    /// Handle submit user input event
    ///
    /// # Errors
    /// Returns error if input processing or verification fails
    async fn handle_submit_input(
        &mut self,
        params: SubmitInputParams<'_, '_>,
    ) -> Result<PendingTaskResult> {
        let start = Instant::now();

        let process_start = Instant::now();
        self.process_input_events().await?;
        tracing::debug!(
            "  process_input: {:.3}s",
            process_start.elapsed().as_secs_f64()
        );

        let render_start = Instant::now();
        self.tui_app.render()?;
        tracing::debug!("  render: {:.3}s", render_start.elapsed().as_secs_f64());

        let verify_start = Instant::now();
        params
            .verifier
            .verify_event(&VerifyEventContext {
                event: params.event,
                verify: &params.input_event.verify,
                tui_app: Some(&self.tui_app),
                execution_tracker: params.execution_tracker,
                provider: Some(&self.provider),
            })
            .await
            .map_err(RoutingError::ExecutionFailed)?;
        tracing::debug!(
            "  verify_event: {:.3}s",
            verify_start.elapsed().as_secs_f64()
        );

        // Look ahead to find the next LlmResponse event and set it as current
        // This allows LLM queries triggered by submit to find their responses
        for (idx, evt) in self
            .fixture
            .events
            .iter()
            .enumerate()
            .skip(params.event_index + 1)
        {
            if let TestEvent::LlmResponse(llm_event) = evt {
                let event_id = llm_event
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("event_{idx}"));
                self.provider.set_current_event(Some(event_id))?;
                break;
            }
        }

        self.event_controller.advance();

        let await_start = Instant::now();
        let mut task_events = tui_test_helpers::get_task_receiver(&mut self.tui_app)?;
        let result =
            task_completion::await_task_completion(&mut self.tui_app, &mut task_events).await;
        tracing::debug!(
            "  await_completion: {:.3}s",
            await_start.elapsed().as_secs_f64()
        );

        tracing::debug!(
            "handle_submit_input total: {:.3}s",
            start.elapsed().as_secs_f64()
        );
        result
    }

    /// Handle non-submit user input event
    ///
    /// # Errors
    /// Returns error if input processing or verification fails
    async fn handle_user_input(
        &mut self,
        event: &TestEvent,
        input_event: &super::fixture::UserInputEvent,
        verifier: &mut UnifiedVerifier<'_>,
        execution_tracker: &ExecutionResultTracker,
    ) -> Result<()> {
        self.process_input_events().await?;
        self.tui_app.render()?;

        verifier
            .verify_event(&VerifyEventContext {
                event,
                verify: &input_event.verify,
                tui_app: Some(&self.tui_app),
                execution_tracker,
                provider: Some(&self.provider),
            })
            .await
            .map_err(RoutingError::ExecutionFailed)?;

        self.event_controller.advance();
        Ok(())
    }

    /// Handle LLM response event
    ///
    /// # Errors
    /// Returns error if verification fails
    async fn handle_llm_response(&mut self, params: LlmResponseParams<'_, '_>) -> Result<()> {
        let start = Instant::now();

        let verify_before_start = Instant::now();
        params
            .verifier
            .verify_event(&VerifyEventContext {
                event: params.event,
                verify: &params.llm_event.verify_before,
                tui_app: Some(&self.tui_app),
                execution_tracker: params.execution_tracker,
                provider: Some(&self.provider),
            })
            .await
            .map_err(RoutingError::ExecutionFailed)?;
        tracing::debug!(
            "  verify_before: {:.3}s",
            verify_before_start.elapsed().as_secs_f64()
        );

        // Set current event for mock provider (must match registration ID)
        let set_event_start = Instant::now();
        let event_id = params
            .llm_event
            .id
            .clone()
            .unwrap_or_else(|| format!("event_{}", params.event_index));
        self.provider.set_current_event(Some(event_id))?;
        tracing::debug!(
            "  set_event: {:.3}s",
            set_event_start.elapsed().as_secs_f64()
        );

        let process_ui_start = Instant::now();
        self.process_pending_ui_events();
        tracing::debug!(
            "  process_ui: {:.3}s",
            process_ui_start.elapsed().as_secs_f64()
        );

        let render_start = Instant::now();
        self.tui_app.render()?;
        tracing::debug!("  render: {:.3}s", render_start.elapsed().as_secs_f64());

        let verify_config = if params.llm_event.verify_after.is_empty() {
            &params.llm_event.verify
        } else {
            &params.llm_event.verify_after
        };

        let verify_after_start = Instant::now();
        let verify_result = params
            .verifier
            .verify_event(&VerifyEventContext {
                event: params.event,
                verify: verify_config,
                tui_app: Some(&self.tui_app),
                execution_tracker: params.execution_tracker,
                provider: Some(&self.provider),
            })
            .await;
        tracing::debug!(
            "  verify_after: {:.3}s",
            verify_after_start.elapsed().as_secs_f64()
        );

        // Clear current event (even if verification failed)
        let clear_event_start = Instant::now();
        self.provider.set_current_event(None)?;
        tracing::debug!(
            "  clear_event: {:.3}s",
            clear_event_start.elapsed().as_secs_f64()
        );

        // Now check verification result
        verify_result.map_err(RoutingError::ExecutionFailed)?;

        self.event_controller.advance();
        tracing::debug!(
            "handle_llm_response total: {:.3}s",
            start.elapsed().as_secs_f64()
        );
        Ok(())
    }

    /// Run the test
    ///
    /// # Errors
    /// Returns error if test execution or verification fails
    pub async fn run(&mut self) -> Result<VerificationResult> {
        let workspace_path = self.workspace_path.clone();
        let events = self.fixture.events.clone();
        let final_verify = self.fixture.final_verify.clone();
        let mut execution_tracker = ExecutionResultTracker::new();
        let mut verifier = UnifiedVerifier::new(&workspace_path);
        let mut pending_task: Option<(PendingTaskResult, String)> = None;

        for (event_index, event) in events.iter().enumerate() {
            let execution_id = Self::get_execution_id(event, event_index);
            match event {
                TestEvent::UserInput(input_event) if input_event.data.submit => {
                    complete_pending_task(&mut pending_task, &mut execution_tracker);
                    let completion_result = self
                        .handle_submit_input(SubmitInputParams {
                            event,
                            input_event,
                            event_index,
                            verifier: &mut verifier,
                            execution_tracker: &execution_tracker,
                        })
                        .await?;
                    pending_task = Some((completion_result, execution_id));
                }
                TestEvent::UserInput(input_event) => {
                    self.handle_user_input(event, input_event, &mut verifier, &execution_tracker)
                        .await?;
                }
                TestEvent::KeyPress(_) => {
                    self.process_input_events().await?;
                    self.tui_app.render()?;
                    self.event_controller.advance();
                }
                TestEvent::LlmResponse(llm_event) => {
                    complete_pending_task(&mut pending_task, &mut execution_tracker);
                    self.handle_llm_response(LlmResponseParams {
                        event,
                        llm_event: llm_event.as_ref(),
                        event_index,
                        verifier: &mut verifier,
                        execution_tracker: &execution_tracker,
                    })
                    .await?;
                }
                TestEvent::Wait(wait_event) => {
                    sleep(TokioDuration::from_millis(wait_event.data.duration_ms)).await;
                    self.event_controller.advance();
                }
                TestEvent::Verify(verify_event) => {
                    tui_test_helpers::process_ui_events(&mut self.tui_app);
                    self.tui_app.render()?;
                    verifier
                        .verify_event(&VerifyEventContext {
                            event,
                            verify: &verify_event.verify,
                            tui_app: Some(&self.tui_app),
                            execution_tracker: &execution_tracker,
                            provider: Some(&self.provider),
                        })
                        .await
                        .map_err(RoutingError::ExecutionFailed)?;
                    self.event_controller.advance();
                }
            }
        }

        complete_pending_task(&mut pending_task, &mut execution_tracker);

        // Process any pending UI events for final state
        self.process_pending_ui_events();
        self.tui_app.render()?;

        verifier
            .verify_final(&final_verify, Some(&self.tui_app), &execution_tracker)
            .await
            .map_err(RoutingError::ExecutionFailed)?;

        // Clean up test artifacts
        self.cleanup_test_artifacts()?;

        Ok(verifier.result())
    }

    /// Clean up test artifacts (threads and tasks) after test completion
    ///
    /// # Errors
    /// Returns error if cleanup fails
    fn cleanup_test_artifacts(&self) -> Result<()> {
        // Clean up threads directory
        let threads_dir = self.workspace_path.join(".merlin").join("threads");
        if threads_dir.exists() {
            fs::remove_dir_all(&threads_dir).map_err(|err| {
                RoutingError::Other(format!("Failed to cleanup threads directory: {err}"))
            })?;
        }

        // Clean up tasks directory
        let tasks_dir = self.workspace_path.join(".merlin").join("tasks");
        if tasks_dir.exists() {
            fs::remove_dir_all(&tasks_dir).map_err(|err| {
                RoutingError::Other(format!("Failed to cleanup tasks directory: {err}"))
            })?;
        }

        // Clean up any stray thread JSON files in workspace root
        // (from before the path fix - these should no longer be created)
        Self::cleanup_uuid_json_files(&self.workspace_path);

        Ok(())
    }

    /// Check if a filename matches UUID pattern (8-4-4-4-12 hex digits)
    fn is_uuid_filename(stem: &str) -> bool {
        stem.len() == 36
            && stem
                .chars()
                .enumerate()
                .all(|(pos_index, char_value)| match pos_index {
                    8 | 13 | 18 | 23 => char_value == '-',
                    _ => char_value.is_ascii_hexdigit(),
                })
    }

    /// Clean up UUID-pattern JSON files in the given directory
    fn cleanup_uuid_json_files(dir: &Path) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let Some("json") = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };

            let Some(stem) = path.file_stem().and_then(|stem_os| stem_os.to_str()) else {
                continue;
            };

            if Self::is_uuid_filename(stem) {
                drop(fs::remove_file(&path));
            }
        }
    }

    /// Get execution ID for an event
    fn get_execution_id(event: &TestEvent, event_index: usize) -> String {
        event
            .id()
            .map_or_else(|| format!("event_{event_index}"), ToString::to_string)
    }

    /// Load a test fixture from a JSON file
    ///
    /// # Errors
    /// Returns error if file reading or parsing fails
    pub fn load_fixture(path: &Path) -> Result<TestFixture> {
        super::fixture_loader::load_fixture(path)
    }

    /// Discover all fixtures in directory
    ///
    /// # Errors
    /// Returns error if directory reading fails
    pub fn discover_fixtures(dir: &Path) -> Result<Vec<PathBuf>> {
        super::fixture_loader::discover_fixtures(dir)
    }
}
