//! Unified test runner.
//!
//! This module provides the test runner that executes unified test fixtures
//! by running the actual CLI with pattern-based mock LLM responses.

use super::event_source::{FixtureEventController, FixtureEventSource};
use super::execution_tracker::ExecutionResultTracker;
use super::fixture::{TestEvent, TestFixture};
use super::mock_provider::{MockProvider, MockRouter};
use super::tui_test_helpers;
use super::verification_result::VerificationResult;
use super::verifier::{UnifiedVerifier, VerifyEventContext};
use super::workspace_setup::{create_files, get_test_workspace_path};
use merlin_agent::{RoutingOrchestrator, ThreadStore};
use merlin_cli::TuiApp;
use merlin_core::{ModelProvider, Result, RoutingError, TaskResult};
use merlin_deps::ratatui::backend::TestBackend;
use merlin_deps::serde_json::{Value as JsonValue, from_str};
use merlin_deps::tempfile::TempDir;
use merlin_routing::{Model, ProviderRegistry, RoutingConfig, UiEvent};
use merlin_tooling::ToolError;
use merlin_tooling::ToolResult;
use std::fs;
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::yield_now;
use tokio::time::{Duration as TokioDuration, sleep, timeout};

/// Result type for task completion with captured outputs
type TaskCompletionResult = StdResult<(TaskResult, Vec<String>), (ToolError, Vec<String>)>;

/// Pending task result before being added to tracker - can be success or failure
type PendingTaskResult = StdResult<(TaskResult, Vec<String>), (ToolError, Vec<String>)>;

/// Unified test runner
pub struct UnifiedTestRunner {
    /// Test fixture
    fixture: TestFixture,
    /// Workspace directory (owned `TempDir` for automatic cleanup)
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
    /// # Errors
    /// Returns error if workspace setup fails
    pub async fn new(fixture: TestFixture) -> Result<Self> {
        let workspace = TempDir::new()
            .map_err(|err| RoutingError::Other(format!("Failed to create workspace: {err}")))?;
        let workspace_path = workspace.path().to_path_buf();

        Self::new_internal(fixture, Some(workspace), workspace_path).await
    }

    /// Create new test runner with provided workspace directory
    ///
    /// # Errors
    /// Returns error if workspace setup fails
    pub async fn new_with_workspace(fixture: TestFixture, workspace_path: PathBuf) -> Result<Self> {
        Self::new_internal(fixture, None, workspace_path).await
    }

    /// Internal constructor shared by both public constructors
    ///
    /// # Errors
    /// Returns error if workspace setup fails
    async fn new_internal(
        fixture: TestFixture,
        workspace_temp: Option<TempDir>,
        workspace_path: PathBuf,
    ) -> Result<Self> {
        let provider = Arc::new(MockProvider::new("test-mock"));

        // Determine workspace setup strategy
        let (final_workspace_path, workspace_temp, _is_readonly) =
            if let Some(workspace_name) = &fixture.setup.workspace {
                // Use pre-made workspace read-only with pre-generated embeddings
                let premade_path = get_test_workspace_path(workspace_name)?;
                (premade_path, None, true)
            } else {
                // Non-workspace tests: always use temp workspace
                create_files(&workspace_path, &fixture.setup.files)?;
                (workspace_path, workspace_temp, false)
            };

        // Generate embeddings for test workspace if files were created
        // This ensures chunking and scoring code is exercised during test setup
        if !fixture.setup.files.is_empty() {
            use merlin_context::FakeEmbeddingClient;
            use merlin_context::embedding::VectorSearchManager;
            let mut vector_manager =
                VectorSearchManager::with_provider(&final_workspace_path, FakeEmbeddingClient);
            // Synchronously initialize embeddings (this exercises chunking, scoring, etc.)
            vector_manager.initialize().await?;
        }

        // Setup LLM response patterns
        for event in &fixture.events {
            if let TestEvent::LlmResponse(llm_event) = event {
                let typescript = llm_event.response.typescript.join("\n");
                provider.add_response(&llm_event.trigger, typescript)?;
            }
        }

        // Create routing config for test orchestrator
        let mut config = RoutingConfig::default();
        // Disable all real tiers
        config.tiers.local_enabled = false;
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        // Create provider registry and register mock provider
        let mut registry = ProviderRegistry::new(config.clone())?;
        registry.register_provider(
            Model::Qwen25Coder32B,
            Arc::clone(&provider) as Arc<dyn ModelProvider>,
        );

        // Create orchestrator with mock router and provider registry
        let router = Arc::new(MockRouter);

        // Create thread store for conversation management if fixture uses threads
        let needs_threads = fixture.tags.contains(&"threads".to_owned());
        let orchestrator = if needs_threads {
            let store = ThreadStore::new(final_workspace_path.clone())?;
            let thread_store = Arc::new(Mutex::new(store));
            RoutingOrchestrator::new_with_router(config, router, Arc::new(registry))?
                .with_workspace(final_workspace_path.clone())
                .with_thread_store(thread_store)
        } else {
            RoutingOrchestrator::new_with_router(config, router, Arc::new(registry))?
                .with_workspace(final_workspace_path.clone())
        };

        // Create fixture-based event source with controller
        let (event_source, event_controller) = FixtureEventSource::new(&fixture);

        // Create test backend with reasonable size
        let terminal_size = fixture.setup.terminal_size.unwrap_or((80, 24));
        let backend = TestBackend::new(terminal_size.0, terminal_size.1);

        // Create TUI app with test backend, fixture event source, and orchestrator
        let tui_app = tui_test_helpers::new_test_app(
            backend,
            Box::new(event_source),
            Some(final_workspace_path.clone()),
            Some(Arc::new(orchestrator)),
        )
        .await?;

        Ok(Self {
            fixture,
            _workspace_temp: workspace_temp,
            workspace_path: final_workspace_path,
            provider,
            tui_app,
            event_controller,
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
    async fn await_task_completion(
        &mut self,
        task_events: &mut mpsc::Receiver<UiEvent>,
    ) -> Result<TaskCompletionResult> {
        let mut outputs = Vec::new();
        let overall_timeout = TokioDuration::from_secs(5);
        let start = Instant::now();
        let mut events_received = 0;
        let mut last_event_time = Instant::now();
        let mut task_started = false;

        loop {
            // Check overall timeout to prevent infinite hangs
            if start.elapsed() >= overall_timeout {
                let idle_time = last_event_time.elapsed().as_millis();
                return Err(RoutingError::ExecutionFailed(format!(
                    "Task completion timed out after 5 seconds - {events_received} events received, \
                     task_started: {task_started}, idle for {idle_time}ms"
                )));
            }

            // Process any pending UI events first (this broadcasts them)
            tui_test_helpers::process_ui_events(&mut self.tui_app);

            // Try to receive task-specific event with timeout (increased to reduce CPU thrashing)
            let ui_event = timeout(TokioDuration::from_millis(100), task_events.recv()).await;

            match ui_event {
                Ok(Some(event)) => {
                    events_received += 1;
                    last_event_time = Instant::now();

                    match event {
                        UiEvent::TaskStarted { .. } => {
                            task_started = true;
                        }
                        UiEvent::TaskCompleted { result, .. } => {
                            return Ok(Ok((*result, outputs)));
                        }
                        UiEvent::TaskFailed { error, .. } => {
                            return Ok(Err((error, outputs)));
                        }
                        UiEvent::TaskOutput { output, .. } => {
                            outputs.push(output);
                        }
                        _ => {
                            // Ignore other events (TaskProgress, etc.)
                        }
                    }
                }
                Ok(None) => {
                    // Channel closed - check if we got completion
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
                    // Timeout - explicitly yield to allow other tasks to run
                    yield_now().await;
                }
            }
        }
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
                    // Complete any pending task first
                    Self::complete_pending_task(&mut pending_task, &mut execution_tracker);

                    // Process all input events (characters + Enter)
                    self.process_input_events().await?;

                    // Render UI after processing input
                    self.tui_app.render()?;

                    // Verify user input event AFTER submission but BEFORE task completes
                    verifier
                        .verify_event(&VerifyEventContext {
                            event,
                            verify: &input_event.verify,
                            tui_app: Some(&self.tui_app),
                            execution_tracker: &execution_tracker,
                            provider: Some(&self.provider),
                        })
                        .map_err(RoutingError::ExecutionFailed)?;

                    // Advance to next fixture event
                    self.event_controller.advance();

                    // Get the per-task event receiver from spawn_task_execution
                    let mut task_events = tui_test_helpers::get_task_receiver(&mut self.tui_app)?;

                    // Now await task completion using dedicated per-task channel
                    let completion_result = self.await_task_completion(&mut task_events).await?;
                    pending_task = Some((completion_result, execution_id));
                }
                TestEvent::UserInput(input_event) => {
                    // Non-submit input - process all input events
                    self.process_input_events().await?;

                    // Render UI after processing input
                    self.tui_app.render()?;

                    // Verify user input event AFTER processing
                    verifier
                        .verify_event(&VerifyEventContext {
                            event,
                            verify: &input_event.verify,
                            tui_app: Some(&self.tui_app),
                            execution_tracker: &execution_tracker,
                            provider: Some(&self.provider),
                        })
                        .map_err(RoutingError::ExecutionFailed)?;

                    // Advance to next fixture event
                    self.event_controller.advance();
                }
                TestEvent::KeyPress(_) => {
                    self.process_input_events().await?;
                    self.tui_app.render()?;
                    self.event_controller.advance();
                }
                TestEvent::LlmResponse(llm_event) => {
                    Self::complete_pending_task(&mut pending_task, &mut execution_tracker);
                    self.tui_app.render()?;
                    verifier
                        .verify_event(&VerifyEventContext {
                            event,
                            verify: &llm_event.verify,
                            tui_app: Some(&self.tui_app),
                            execution_tracker: &execution_tracker,
                            provider: Some(&self.provider),
                        })
                        .map_err(RoutingError::ExecutionFailed)?;
                    self.event_controller.advance();
                }
                TestEvent::Wait(wait_event) => {
                    sleep(TokioDuration::from_millis(wait_event.data.duration_ms)).await;
                    self.event_controller.advance();
                }
            }
        }

        Self::complete_pending_task(&mut pending_task, &mut execution_tracker);
        self.tui_app.render()?;
        verifier
            .verify_final(&final_verify, Some(&self.tui_app), &execution_tracker)
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

        Ok(())
    }

    /// Get execution ID for an event
    fn get_execution_id(event: &TestEvent, event_index: usize) -> String {
        event
            .id()
            .map_or_else(|| format!("event_{event_index}"), ToString::to_string)
    }

    /// Complete a pending task by adding its result to the tracker
    fn complete_pending_task(
        pending_task: &mut Option<(PendingTaskResult, String)>,
        execution_tracker: &mut ExecutionResultTracker,
    ) {
        if let Some((completion_result, execution_id)) = pending_task.take() {
            match completion_result {
                Ok((task_result, outputs)) => {
                    // Successful task completion
                    let execution_result = Self::extract_execution_result(&task_result, &outputs);
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
    /// Returns error if output parsing fails
    fn extract_execution_result(
        task_result: &TaskResult,
        outputs: &[String],
    ) -> ToolResult<JsonValue> {
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
