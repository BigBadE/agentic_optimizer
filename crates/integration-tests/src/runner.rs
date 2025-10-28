//! Unified test runner.
//!
//! This module provides the test runner that executes unified test fixtures
//! by running the actual CLI with pattern-based mock LLM responses.

use super::event_source::FixtureEventSource;
use super::execution_tracker::ExecutionResultTracker;
use super::fixture::{TestEvent, TestFixture};
use super::mock_provider::{MockRouter, PatternMockProvider};
use super::verification_result::VerificationResult;
use super::verifier::UnifiedVerifier;
use merlin_agent::RoutingOrchestrator;
use merlin_cli::TuiApp;
use merlin_core::{
    ModelProvider, Response, Result, RoutingError, TaskResult, TokenUsage, ValidationResult,
};
use merlin_deps::ratatui::backend::TestBackend;
use merlin_deps::serde_json::{Value as JsonValue, from_str};
use merlin_deps::tempfile::TempDir;
use merlin_routing::{Model, ProviderRegistry, RoutingConfig, UiEvent};
use merlin_tooling::{ToolError, ToolResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{self as tokio_time, Duration as TokioDuration, timeout};

/// Result type for task completion with captured outputs
type TaskCompletionResult = (Box<TaskResult>, Vec<String>);

/// Unified test runner
pub struct UnifiedTestRunner {
    /// Test fixture
    fixture: TestFixture,
    /// Workspace directory (owned TempDir for automatic cleanup)
    _workspace_temp: Option<TempDir>,
    /// Workspace path
    workspace_path: PathBuf,
    /// Mock provider
    provider: Arc<PatternMockProvider>,
    /// The actual TUI application under test
    tui_app: TuiApp<TestBackend>,
    /// Event tap receiver for listening to task completions
    event_receiver: mpsc::UnboundedReceiver<UiEvent>,
}

impl UnifiedTestRunner {
    /// Create new test runner with auto-managed workspace
    ///
    /// # Errors
    /// Returns error if workspace setup fails
    pub fn new(fixture: TestFixture) -> Result<Self> {
        let workspace = TempDir::new()
            .map_err(|err| RoutingError::Other(format!("Failed to create workspace: {err}")))?;
        let workspace_path = workspace.path().to_path_buf();

        Self::new_internal(fixture, Some(workspace), workspace_path)
    }

    /// Create new test runner with provided workspace directory
    ///
    /// # Errors
    /// Returns error if workspace setup fails
    pub fn new_with_workspace(fixture: TestFixture, workspace_path: PathBuf) -> Result<Self> {
        Self::new_internal(fixture, None, workspace_path)
    }

    /// Internal constructor shared by both public constructors
    fn new_internal(
        fixture: TestFixture,
        workspace_temp: Option<TempDir>,
        workspace_path: PathBuf,
    ) -> Result<Self> {
        let provider = Arc::new(PatternMockProvider::new("test-mock"));

        // Setup files
        for (path, content) in &fixture.setup.files {
            let file_path = workspace_path.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    RoutingError::Other(format!("Failed to create directory: {err}"))
                })?;
            }
            fs::write(&file_path, content)
                .map_err(|err| RoutingError::Other(format!("Failed to write file: {err}")))?;
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
        config.workspace.root_path = workspace_path.clone();
        config.execution.max_concurrent_tasks = 4;
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
        let orchestrator =
            RoutingOrchestrator::new_with_router(config, router, Arc::new(registry))?;

        // Create fixture-based event source
        let event_source = Box::new(FixtureEventSource::new(&fixture));

        // Create test backend with reasonable size
        let terminal_size = fixture.setup.terminal_size.unwrap_or((80, 24));
        let backend = TestBackend::new(terminal_size.0, terminal_size.1);

        // Create TUI app with test backend, fixture event source, and orchestrator
        let mut tui_app = TuiApp::new_for_test(
            backend,
            event_source,
            Some(workspace_path.clone()),
            Some(Arc::new(orchestrator)),
        )?;

        // Set up event tap for task completion monitoring
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        tui_app.test_set_event_tap(event_tx);

        Ok(Self {
            fixture,
            _workspace_temp: workspace_temp,
            workspace_path,
            provider,
            tui_app,
            event_receiver: event_rx,
        })
    }

    /// Get workspace path
    #[must_use]
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    /// Get mock provider
    #[must_use]
    pub fn provider(&self) -> Arc<PatternMockProvider> {
        Arc::clone(&self.provider)
    }

    /// Get read-only reference to TUI app for verification
    #[must_use]
    pub fn tui_app(&self) -> &TuiApp<TestBackend> {
        &self.tui_app
    }

    /// Process a submit event and await completion
    ///
    /// Since we process one submit at a time sequentially, we wait for the next
    /// `TaskCompleted` or `TaskFailed` event. We drain ALL `TaskOutput` events
    /// from the channel during this wait to ensure we capture outputs from the
    /// main task and any subtasks (like TypeScript tool executions).
    ///
    /// Returns the task result and any captured output from `TaskOutput` events.
    ///
    /// # Errors
    /// Returns error if submission fails
    async fn process_submit_and_await(&mut self) -> Result<TaskCompletionResult> {
        // Tick once to trigger submission
        self.tui_app.tick()?;

        // Wait for the task to complete (10 second timeout)
        let result = timeout(TokioDuration::from_secs(10), async {
            let mut outputs = Vec::new();

            loop {
                // Tick to process any pending work
                self.tui_app.tick()?;

                // Event-driven wait: block until we receive an event from the channel
                // This eliminates arbitrary polling intervals
                let event = tokio::select! {
                    // Wait for event from channel
                    event = self.event_receiver.recv() => {
                        match event {
                            Some(e) => e,
                            None => {
                                // Channel closed unexpectedly
                                return Err(RoutingError::ExecutionFailed(
                                    "Event channel closed before task completion".to_owned()
                                ));
                            }
                        }
                    }
                    // Also allow timeout for ticking (to process UI updates)
                    _ = tokio_time::sleep(TokioDuration::from_millis(50)) => {
                        continue; // No event, just tick again
                    }
                };

                // Process the received event
                match event {
                    UiEvent::TaskCompleted { result, .. } => {
                        // Drain any remaining output events
                        while let Ok(UiEvent::TaskOutput { output, .. }) =
                            self.event_receiver.try_recv()
                        {
                            outputs.push(output);
                        }
                        return Ok((result, outputs));
                    }
                    UiEvent::TaskFailed { error, task_id } => {
                        // Drain any remaining output events
                        while let Ok(UiEvent::TaskOutput { output, .. }) =
                            self.event_receiver.try_recv()
                        {
                            outputs.push(output);
                        }

                        // Create a failure result instead of returning an error
                        // This allows fixtures to verify that expected errors occurred
                        let task_result = TaskResult {
                            task_id,
                            response: Response {
                                text: error,
                                confidence: 0.0,
                                tokens_used: TokenUsage::default(),
                                provider: "mock".to_owned(),
                                latency_ms: 0,
                            },
                            tier_used: "default".to_owned(),
                            tokens_used: TokenUsage::default(),
                            validation: ValidationResult::default(),
                            duration_ms: 0,
                            work_unit: None,
                        };
                        return Ok((Box::new(task_result), outputs));
                    }
                    UiEvent::TaskOutput { output, .. } => {
                        // Capture output and continue waiting
                        outputs.push(output);
                    }
                    _ => {
                        // Ignore other events (TaskStarted, TaskProgress, etc.)
                    }
                }
            }
        })
        .await;

        result.map_err(|_| RoutingError::ExecutionFailed("Task execution timeout".to_owned()))?
    }

    /// Run the test
    ///
    /// # Errors
    /// Returns error if test execution or verification fails
    pub async fn run(&mut self) -> Result<VerificationResult> {
        let workspace_path = self.workspace_path.clone();

        // Clone the events and final_verify to avoid borrowing issues
        let events = self.fixture.events.clone();
        let final_verify = self.fixture.final_verify.clone();

        // Create single execution tracker for entire fixture run
        let mut execution_tracker = ExecutionResultTracker::new();

        // Create single verifier instance that will use the tracker
        let mut verifier = UnifiedVerifier::new(&workspace_path);

        for event in &events {
            match event {
                TestEvent::UserInput(input_event) if input_event.data.submit => {
                    // Process submit and await completion with result capture
                    let (task_result, outputs) = self.process_submit_and_await().await?;

                    // Extract execution result and add to tracker
                    let execution_result = Self::extract_execution_result(&task_result, &outputs);
                    execution_tracker.add_result(execution_result, outputs, task_result);
                }
                TestEvent::UserInput(_) | TestEvent::KeyPress(_) => {
                    // Non-submit input or key press
                    self.tui_app.tick()?;
                }
                TestEvent::LlmResponse(llm_event) => {
                    // LLM response events are handled by the orchestrator internally
                    // Just verify the results after processing
                    verifier
                        .verify_event(
                            event,
                            &llm_event.verify,
                            Some(&self.tui_app),
                            &execution_tracker,
                        )
                        .map_err(RoutingError::ExecutionFailed)?;
                }
                TestEvent::Wait(wait_event) => {
                    // Sleep for specified duration
                    tokio_time::sleep(TokioDuration::from_millis(wait_event.data.duration_ms))
                        .await;

                    // Process any pending UI events during wait
                    self.tui_app.tick()?;
                }
            }
        }

        // Verify final state (pass TUI app reference and execution tracker)
        verifier
            .verify_final(&final_verify, Some(&self.tui_app), &execution_tracker)
            .map_err(RoutingError::ExecutionFailed)?;

        Ok(verifier.result())
    }

    /// Extract execution result from `TaskResult` and captured outputs
    ///
    /// The TypeScript execution result is sent via `TaskOutput` events during execution.
    /// We capture these outputs and parse them to extract the actual execution result.
    ///
    /// The last output typically contains the result returned by the TypeScript code.
    ///
    /// # Errors
    /// Returns error if task execution failed - this is expected and allows verification
    fn extract_execution_result(
        task_result: &TaskResult,
        outputs: &[String],
    ) -> ToolResult<JsonValue> {
        // Check if the task result contains an error (from TaskFailed event)
        // These errors are captured from TypeScript execution failures or other task errors
        let response_text = &task_result.response.text;

        // If response contains error indicators, return as error
        if response_text.contains("TypeScript execution failed")
            || response_text.contains("Tool execution failed")
            || response_text.contains("Task execution failed")
        {
            return Err(ToolError::ExecutionFailed(response_text.clone()));
        }

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

    /// Load fixture from file
    ///
    /// # Errors
    /// Returns error if file reading or parsing fails
    pub fn load_fixture(path: &Path) -> Result<TestFixture> {
        let content = fs::read_to_string(path)
            .map_err(|err| RoutingError::Other(format!("Failed to read fixture: {err}")))?;
        from_str(&content)
            .map_err(|err| RoutingError::Other(format!("Failed to parse fixture: {err}")))
    }

    /// Discover all fixtures in directory
    ///
    /// # Errors
    /// Returns error if directory reading fails
    pub fn discover_fixtures(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut fixtures = Vec::new();

        if !dir.exists() {
            return Ok(fixtures);
        }

        let entries = fs::read_dir(dir)
            .map_err(|err| RoutingError::Other(format!("Failed to read directory: {err}")))?;

        for entry in entries {
            let entry =
                entry.map_err(|err| RoutingError::Other(format!("Failed to read entry: {err}")))?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                fixtures.push(path);
            } else if path.is_dir() {
                // Recurse into subdirectories
                let mut sub_fixtures = Self::discover_fixtures(&path)?;
                fixtures.append(&mut sub_fixtures);
            }
        }

        Ok(fixtures)
    }
}
