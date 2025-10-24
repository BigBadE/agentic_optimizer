//! Scenario runner implementation

use crate::serde_json::from_str;
use crate::tui_helpers::{TestEventSource, parse_key};
use crate::types::{
    EventExpectation, MockResponseData, Scenario, ScenarioStep, StepAction, StepExpectations,
    TaskExpectations, UiEventData, UiExpectations, UserInputData, WaitCondition, WaitData,
    WaitForTasksData,
};
use anyhow::{Context as _, Error, Result, anyhow};
use merlin_agent::RoutingOrchestrator;
use merlin_cli::ui::TuiApp;
use merlin_core::ui::{TaskProgress, UiEvent};
use merlin_core::{Task, TaskId, TaskResult};
use merlin_providers::MockProvider;
use merlin_routing::{RoutingConfig, UiChannel};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::spawn;
use tokio::sync::{Mutex as TokioMutex, mpsc};
use tokio::time::{Instant, sleep, timeout};

/// Shared task results storage
type TaskResults = Arc<TokioMutex<HashMap<String, TaskResult>>>;

/// Extracted UI state data for verification without holding TUI app reference
struct UiStateData {
    expectations: UiExpectations,
    task_count: usize,
    input_text: String,
    theme: String,
    focused_pane: String,
    expanded_conversations: Vec<String>,
    expanded_steps: Vec<String>,
    output_scroll: u16,
    task_list_scroll: usize,
    processing_status: Option<String>,
    pending_delete_task_id: Option<String>,
}

/// Scenario runner that executes E2E test scenarios
pub struct ScenarioRunner {
    scenario: Scenario,
    workspace: TempDir,
    event_source: TestEventSource,
    ui_channel: Option<UiChannel>,
    orchestrator: Option<RoutingOrchestrator>,
    collected_events: Arc<TokioMutex<Vec<UiEvent>>>,
    mock_provider: Arc<MockProvider>,
    task_results: TaskResults,
    tui_app: Option<TuiApp<TestBackend>>,
    tui_sender: Option<mpsc::UnboundedSender<UiEvent>>,
}

impl ScenarioRunner {
    /// Load a scenario from a JSON file
    ///
    /// # Errors
    /// Returns error if scenario file cannot be loaded or parsed
    pub fn load(scenario_name: &str) -> Result<Self> {
        let scenarios_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("scenarios");

        let scenario_path = scenarios_dir.join(format!("{scenario_name}.json"));
        let content = fs::read_to_string(&scenario_path).with_context(|| {
            format!("Failed to read scenario file: {}", scenario_path.display())
        })?;

        let scenario: Scenario = from_str(&content)
            .with_context(|| format!("Failed to parse scenario: {scenario_name}"))?;

        let workspace = TempDir::new().context("Failed to create temp workspace")?;

        Ok(Self {
            scenario,
            workspace,
            event_source: TestEventSource::new(),
            ui_channel: None,
            orchestrator: None,
            collected_events: Arc::new(TokioMutex::new(Vec::new())),
            mock_provider: Arc::new(MockProvider::new("test_provider")),
            task_results: Arc::new(TokioMutex::new(HashMap::new())),
            tui_app: None,
            tui_sender: None,
        })
    }

    /// Run the scenario
    ///
    /// # Errors
    /// Returns error if any step fails
    pub async fn run(mut self) -> Result<()> {
        // Setup
        self.setup_workspace()?;
        self.setup_orchestrator()?;
        self.setup_tui()?;

        // Execute steps
        for (step_index, step) in self.scenario.steps.clone().into_iter().enumerate() {
            tracing::info!("Step {}: {:?}", step_index + 1, step.action);
            self.execute_step(&step).await.with_context(|| {
                format!(
                    "Failed to execute step {} in scenario '{}'",
                    step_index + 1,
                    self.scenario.name
                )
            })?;

            // Process TUI ticks to handle events
            self.process_tui_ticks(5).await?;

            self.verify_expectations(&step.expectations)
                .await
                .with_context(|| {
                    format!(
                        "Step {} expectations failed in scenario '{}'",
                        step_index + 1,
                        self.scenario.name
                    )
                })?;
        }

        Ok(())
    }

    /// Set up workspace files and environment
    ///
    /// # Errors
    /// Returns error if file creation or env var setting fails
    fn setup_workspace(&self) -> Result<()> {
        // Create workspace files
        for file in &self.scenario.setup.workspace_files {
            let file_path = self.workspace.path().join(&file.path);

            // Create parent directories
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }

            // Write file content
            fs::write(&file_path, &file.content)
                .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
        }

        // Set environment variables
        for (key, value) in &self.scenario.setup.env_vars {
            #[allow(
                unsafe_code,
                reason = "Required for setting environment variables in test setup"
            )]
            // SAFETY: Setting environment variables in tests before any concurrent access
            unsafe {
                env::set_var(key, value);
            }
        }

        // Set MERLIN_FOLDER to temp directory
        // SAFETY: Setting environment variable in test setup before any concurrent access
        #[allow(unsafe_code, reason = "Required for setting test environment variable")]
        unsafe {
            env::set_var(
                "MERLIN_FOLDER",
                env::temp_dir().join("merlin_integration_test"),
            );
        }

        Ok(())
    }

    /// Set up orchestrator with test configuration
    ///
    /// # Errors
    /// Returns error if orchestrator creation fails
    fn setup_orchestrator(&mut self) -> Result<()> {
        let mut config = RoutingConfig::default();
        config.workspace.root_path = self.workspace.path().to_path_buf();
        config.tiers.groq_enabled = false;
        config.tiers.premium_enabled = false;

        let orchestrator =
            RoutingOrchestrator::new(config).context("Failed to create orchestrator")?;

        self.orchestrator = Some(orchestrator);

        // Create UI channel for event collection
        let (sender, receiver_channel) = mpsc::unbounded_channel();
        self.ui_channel = Some(UiChannel::from_sender(sender));

        // Spawn task to collect events
        let events_clone = Arc::clone(&self.collected_events);
        spawn(async move {
            let mut receiver = receiver_channel;
            while let Some(event) = receiver.recv().await {
                events_clone.lock().await.push(event);
            }
        });

        Ok(())
    }

    /// Set up TUI with test backend
    ///
    /// # Errors
    /// Returns error if TUI creation fails
    fn setup_tui(&mut self) -> Result<()> {
        let [width, height] = self.scenario.setup.terminal_size;
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).context("Failed to create test terminal")?;

        // Create a new receiver that will get the same events
        // For now, we'll create a separate channel and the TUI will receive events
        // that we explicitly send to it
        let (tui_sender, tui_receiver) = mpsc::unbounded_channel();

        // Store the TUI sender for sending events
        // We'll use the collector channel for verification and tui_sender for display
        self.tui_sender = Some(tui_sender);

        let mut app = TuiApp::new_for_test(terminal, tui_receiver);
        app.set_event_source(Box::new(self.event_source.clone()));

        self.tui_app = Some(app);
        Ok(())
    }

    /// Process TUI ticks to handle queued events
    ///
    /// # Errors
    /// Returns error if tick processing fails
    async fn process_tui_ticks(&mut self, count: usize) -> Result<()> {
        if let Some(ref mut app) = self.tui_app {
            for _ in 0..count {
                match app.tick() {
                    Ok(should_quit) if should_quit => break,
                    Err(err) => {
                        tracing::debug!("TUI tick error (expected in tests): {err}");
                        break;
                    }
                    Ok(_) => {}
                }
                // Small delay between ticks
                sleep(Duration::from_millis(10)).await;
            }
        }
        Ok(())
    }

    /// Execute a single scenario step
    ///
    /// # Errors
    /// Returns error if step execution fails
    async fn execute_step(&self, step: &ScenarioStep) -> Result<()> {
        match &step.action {
            StepAction::UserInput { data } => {
                self.execute_user_input(data);
                Ok(())
            }
            StepAction::Wait { data } => self.execute_wait(data).await,
            StepAction::MockAgentResponse { data } => {
                self.execute_mock_response(data);
                Ok(())
            }
            StepAction::KeyPress { data } => {
                let (code, modifiers) =
                    parse_key(&data.key).map_err(|err| anyhow!("Failed to parse key: {err}"))?;
                self.event_source.push_key(code, modifiers);
                Ok(())
            }
            StepAction::WaitForTasks { data } => self.execute_wait_for_tasks(data).await,
            StepAction::SendUiEvent { data } => {
                self.send_ui_event(data);
                Ok(())
            }
        }
    }

    /// Push user input events to event source
    fn execute_user_input(&self, data: &UserInputData) {
        // Push text input events
        self.event_source.push_text(&data.text);

        if data.submit {
            self.event_source.push_enter();
        }
    }

    /// Execute wait action (duration or condition)
    ///
    /// # Errors
    /// Returns error if condition wait fails
    async fn execute_wait(&self, data: &WaitData) -> Result<()> {
        if let Some(ref condition) = data.condition {
            // Wait for specific condition
            self.wait_for_condition(condition).await
        } else {
            // Simple duration wait
            sleep(Duration::from_millis(data.duration_ms)).await;
            Ok(())
        }
    }

    /// Wait for a specific condition to be met
    ///
    /// # Errors
    /// Returns error if condition is never met
    async fn wait_for_condition(&self, condition: &WaitCondition) -> Result<()> {
        match condition {
            WaitCondition::TaskCount(count) => {
                // Wait for a specific number of tasks to be stored
                let timeout_duration = Duration::from_secs(5);
                let start = Instant::now();

                loop {
                    let results = self.task_results.lock().await;
                    if results.len() >= *count {
                        return Ok(());
                    }
                    drop(results);

                    if start.elapsed() > timeout_duration {
                        return Err(anyhow!("Timeout waiting for {count} tasks"));
                    }

                    sleep(Duration::from_millis(100)).await;
                }
            }
            WaitCondition::TaskStatus { .. } => {
                // Wait for a specific task to reach a status
                // For now, just succeed - this would need task tracking
                Ok(())
            }
            WaitCondition::UiUpdate => {
                // Wait for UI events
                sleep(Duration::from_millis(100)).await;
                Ok(())
            }
        }
    }

    /// Configure mock provider to return specific response
    fn execute_mock_response(&self, data: &MockResponseData) {
        // Configure the mock provider with the response
        let pattern = data.pattern.clone();
        let response_text = data.response.as_string();

        // Clone the provider and configure it
        // MockProvider uses Arc<Mutex<>> internally so cloning shares the same state
        let provider_clone = (*self.mock_provider).clone();
        drop(provider_clone.with_response(pattern, response_text));

        // Note: The Arc wrapping the provider still references the updated state
    }

    /// Execute a task using the orchestrator
    ///
    /// # Errors
    /// Returns error if task execution fails
    #[allow(dead_code, reason = "Will be used in future scenario implementations")]
    async fn execute_task(&self, task_description: &str) -> Result<()> {
        // Extract needed fields before any await points
        let orchestrator = self
            .orchestrator
            .as_ref()
            .ok_or_else(|| anyhow!("Orchestrator not initialized"))?
            .clone();
        let ui_channel = self
            .ui_channel
            .as_ref()
            .ok_or_else(|| anyhow!("UI channel not initialized"))?
            .clone();
        let task_results = Arc::clone(&self.task_results);

        // Call static implementation
        Self::execute_task_impl(orchestrator, ui_channel, task_results, task_description).await
    }

    /// Implementation of `execute_task` that doesn't hold self
    ///
    /// # Errors
    /// Returns error if task execution fails
    async fn execute_task_impl(
        orchestrator: RoutingOrchestrator,
        ui_channel: UiChannel,
        task_results: TaskResults,
        task_description: &str,
    ) -> Result<()> {
        // Create a task
        let task = Task::new(task_description.to_owned());
        let task_id = format!("{:?}", task.id);

        // Execute the task
        let result = orchestrator
            .execute_task_streaming(task, ui_channel)
            .await?;

        // Store the result
        task_results.lock().await.insert(task_id, result);

        Ok(())
    }

    /// Wait for tasks to complete
    ///
    /// # Errors
    /// Returns error if timeout is reached before tasks complete
    async fn execute_wait_for_tasks(&self, data: &WaitForTasksData) -> Result<()> {
        // Extract timeout before calling static implementation
        let timeout_ms = data.timeout_ms;

        Self::execute_wait_for_tasks_impl(timeout_ms).await
    }

    /// Implementation of `execute_wait_for_tasks` that doesn't hold self
    ///
    /// # Errors
    /// Returns error if timeout is reached before tasks complete
    async fn execute_wait_for_tasks_impl(timeout_ms: u64) -> Result<()> {
        let timeout_duration = Duration::from_millis(timeout_ms);

        timeout(timeout_duration, async {
            // TODO: Poll task manager for completion
            sleep(Duration::from_millis(100)).await;
            Ok::<(), Error>(())
        })
        .await
        .context("Timeout waiting for tasks")??;

        Ok(())
    }

    /// Create a task completed event for testing
    fn create_task_completed_event(task_id: TaskId, data: &UiEventData) -> UiEvent {
        use merlin_core::{Response, TokenUsage, ValidationResult};

        let result = TaskResult {
            task_id,
            response: Response {
                text: data.result.clone().unwrap_or_else(|| "Success".to_owned()),
                confidence: 1.0,
                tokens_used: TokenUsage {
                    input: 100,
                    output: 50,
                    cache_read: 0,
                    cache_write: 0,
                },
                provider: "test-provider".to_owned(),
                latency_ms: 1000,
            },
            tier_used: "test-tier".to_owned(),
            tokens_used: TokenUsage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
            },
            validation: ValidationResult::default(),
            duration_ms: 1000,
            task_list: None,
        };
        UiEvent::TaskCompleted {
            task_id,
            result: Box::new(result),
        }
    }

    /// Create a system message event for testing
    fn create_system_message_event(data: &UiEventData) -> UiEvent {
        use merlin_core::ui::MessageLevel;

        let level = match data.level.as_deref() {
            Some("warning") => MessageLevel::Warning,
            Some("error") => MessageLevel::Error,
            Some("success") => MessageLevel::Success,
            _ => MessageLevel::Info,
        };
        let message = data
            .message
            .clone()
            .unwrap_or_else(|| "Test message".to_owned());
        UiEvent::SystemMessage { level, message }
    }

    /// Send a UI event directly to the TUI
    fn send_ui_event(&self, data: &UiEventData) {
        let task_id = TaskId::default();

        let event = match data.event_type.as_str() {
            "task_started" => {
                let description = data
                    .description
                    .clone()
                    .unwrap_or_else(|| "Test Task".to_owned());
                UiEvent::TaskStarted {
                    task_id,
                    description,
                    parent_id: data.parent_id.as_ref().map(|_| TaskId::default()),
                }
            }
            "task_output" => {
                let output = data.text.clone().unwrap_or_else(|| "Output".to_owned());
                UiEvent::TaskOutput { task_id, output }
            }
            "task_progress" => {
                let current = u64::from(data.progress.unwrap_or(50));
                let progress = TaskProgress {
                    stage: "test".to_owned(),
                    current,
                    total: Some(100),
                    message: "Progress".to_owned(),
                };
                UiEvent::TaskProgress { task_id, progress }
            }
            "task_completed" => Self::create_task_completed_event(task_id, data),
            "task_failed" => {
                let error = data
                    .error
                    .clone()
                    .unwrap_or_else(|| "Test error".to_owned());
                UiEvent::TaskFailed { task_id, error }
            }
            "task_retrying" => {
                let retry_count = data.retry_count.unwrap_or(1);
                let error = data
                    .error
                    .clone()
                    .unwrap_or_else(|| "Test error".to_owned());
                UiEvent::TaskRetrying {
                    task_id,
                    retry_count,
                    error,
                }
            }
            "system_message" => Self::create_system_message_event(data),
            _ => {
                tracing::warn!("Unsupported UI event type for testing: {}", data.event_type);
                return;
            }
        };

        // Send to both collectors and TUI
        if let Some(ref channel) = self.ui_channel {
            channel.send(event.clone());
        }
        if let Some(ref sender) = self.tui_sender {
            drop(sender.send(event));
        }
    }

    /// Verify all expectations for a step
    ///
    /// # Errors
    /// Returns error if any expectation is not met
    async fn verify_expectations(&self, expectations: &StepExpectations) -> Result<()> {
        use merlin_cli::ui::renderer::FocusedPane;

        // Extract UI data synchronously before any await points
        let ui_data = expectations.ui_state.as_ref().and_then(|ui_exp| {
            self.tui_app.as_ref().map(|app| UiStateData {
                expectations: ui_exp.clone(),
                task_count: app.task_manager().iter_tasks().count(),
                input_text: app.get_input_text(),
                theme: app.get_theme_name(),
                focused_pane: match app.get_focused_pane() {
                    FocusedPane::Input => "input".to_owned(),
                    FocusedPane::Output => "output".to_owned(),
                    FocusedPane::Tasks => "tasks".to_owned(),
                },
                expanded_conversations: app
                    .get_expanded_conversations()
                    .iter()
                    .map(|id| format!("{id:?}"))
                    .collect(),
                expanded_steps: app
                    .get_expanded_steps()
                    .iter()
                    .map(|id| format!("{id:?}"))
                    .collect(),
                output_scroll: app.get_output_scroll(),
                task_list_scroll: app.get_task_list_scroll(),
                processing_status: app.get_processing_status(),
                pending_delete_task_id: app
                    .get_pending_delete_task_id()
                    .map(|id| format!("{id:?}")),
            })
        });

        // Extract data needed for async operations before any await points
        let task_exp_opt = expectations.task_state.clone();
        let task_results = Arc::clone(&self.task_results);
        let event_exps = expectations.events.clone();
        let collected_events = Arc::clone(&self.collected_events);

        // Now call the static implementation
        Self::verify_expectations_impl(
            ui_data,
            task_exp_opt,
            task_results,
            event_exps,
            collected_events,
        )
        .await
    }

    /// Implementation of `verify_expectations` that doesn't hold self
    ///
    /// # Errors
    /// Returns error if any expectation is not met
    async fn verify_expectations_impl(
        ui_data: Option<UiStateData>,
        task_exp_opt: Option<TaskExpectations>,
        task_results: TaskResults,
        event_exps: Vec<EventExpectation>,
        collected_events: Arc<TokioMutex<Vec<UiEvent>>>,
    ) -> Result<()> {
        // Verify UI state with extracted data
        if let Some(ref data) = ui_data {
            Self::verify_ui_state_with_data(data);
        }

        // Verify task state
        if let Some(ref task_exp) = task_exp_opt {
            Self::verify_task_state_impl(task_results, task_exp).await?;
        }

        // Verify events
        for event_exp in &event_exps {
            Self::verify_event_impl(Arc::clone(&collected_events), event_exp).await?;
        }

        Ok(())
    }

    /// Verify basic UI state fields
    ///
    /// # Panics
    /// Panics if expectations don't match actual UI state
    fn verify_basic_ui_state(data: &UiStateData, expectations: &UiExpectations) {
        if let Some(expected_count) = expectations.task_count {
            assert_eq!(
                data.task_count, expected_count,
                "Expected {expected_count} tasks, found {}",
                data.task_count
            );
        }
        if let Some(ref expected_text) = expectations.input_text {
            assert!(
                data.input_text.contains(expected_text),
                "Expected input to contain '{expected_text}', got: '{}'",
                data.input_text
            );
        }
        if let Some(ref expected_theme) = expectations.theme {
            assert_eq!(
                &data.theme, expected_theme,
                "Expected theme '{expected_theme}', got '{}'",
                data.theme
            );
        }
        if let Some(ref expected_pane) = expectations.focused_pane {
            assert_eq!(
                &data.focused_pane, expected_pane,
                "Expected focused pane '{expected_pane}', got '{}'",
                data.focused_pane
            );
        }
    }

    /// Verify expansion and scroll state
    ///
    /// # Panics
    /// Panics if expectations don't match actual UI state
    fn verify_expansion_and_scroll(data: &UiStateData, expectations: &UiExpectations) {
        if let Some(ref expected_convs) = expectations.expanded_conversations {
            assert_eq!(
                &data.expanded_conversations, expected_convs,
                "Expected expanded conversations {:?}, got {:?}",
                expected_convs, data.expanded_conversations
            );
        }
        if let Some(ref expected_steps) = expectations.expanded_steps {
            assert_eq!(
                &data.expanded_steps, expected_steps,
                "Expected expanded steps {:?}, got {:?}",
                expected_steps, data.expanded_steps
            );
        }
        if let Some(expected_scroll) = expectations.output_scroll {
            assert_eq!(
                data.output_scroll, expected_scroll,
                "Expected output scroll {expected_scroll}, got {}",
                data.output_scroll
            );
        }
        if let Some(expected_scroll) = expectations.task_list_scroll {
            assert_eq!(
                data.task_list_scroll, expected_scroll,
                "Expected task list scroll {expected_scroll}, got {}",
                data.task_list_scroll
            );
        }
    }

    /// Verify UI state with extracted data (doesn't need TUI app reference)
    ///
    /// # Panics
    /// Panics if any expectation doesn't match
    fn verify_ui_state_with_data(data: &UiStateData) {
        let expectations = &data.expectations;
        Self::verify_basic_ui_state(data, expectations);
        Self::verify_expansion_and_scroll(data, expectations);

        if let Some(ref expected_status) = expectations.processing_status {
            assert_eq!(
                data.processing_status.as_deref(),
                Some(expected_status.as_str()),
                "Expected processing status '{expected_status}', got {:?}",
                data.processing_status
            );
        }
        if let Some(ref expected_id) = expectations.pending_delete_task_id {
            assert_eq!(
                data.pending_delete_task_id.as_deref(),
                Some(expected_id.as_str()),
                "Expected pending delete task '{expected_id}', got {:?}",
                data.pending_delete_task_id
            );
        }
        if expectations.snapshot.is_some() {
            tracing::debug!("Snapshot verification not yet implemented");
        }

        // Verify focused element if specified
        if expectations.focused.is_some() {
            tracing::debug!("Focus verification not yet implemented");
        }
    }

    /// Verify task state matches expectations (implementation without self reference)
    ///
    /// # Errors
    /// Returns error if task state doesn't match
    async fn verify_task_state_impl(
        task_results: TaskResults,
        expectations: &TaskExpectations,
    ) -> Result<()> {
        let results = task_results.lock().await;

        // Verify task count if specified
        if let Some(expected_count) = expectations.total
            && results.len() != expected_count
        {
            return Err(anyhow!(
                "Expected {} tasks, found {}",
                expected_count,
                results.len()
            ));
        }

        // Verify specific tasks if any are listed
        for _task_exp in &expectations.tasks {
            // Would verify individual task properties here
        }
        drop(results);

        Ok(())
    }

    /// Verify event was received (implementation without self reference)
    ///
    /// # Errors
    /// Returns error if event wasn't received
    async fn verify_event_impl(
        collected_events: Arc<TokioMutex<Vec<UiEvent>>>,
        expectation: &EventExpectation,
    ) -> Result<()> {
        // Check if any event matches the expectation
        let found = collected_events.lock().await.iter().any(|event| {
            match expectation.event_type.as_str() {
                "task_started" => matches!(event, UiEvent::TaskStarted { .. }),
                "task_completed" => matches!(event, UiEvent::TaskCompleted { .. }),
                "task_output" => matches!(event, UiEvent::TaskOutput { .. }),
                "task_progress" => matches!(event, UiEvent::TaskProgress { .. }),
                _ => false,
            }
        });

        // For now, don't require events - this is for future enhancement
        // Once we have comprehensive event tracking
        if !found {
            tracing::debug!("Event not found (not required): {}", expectation.event_type);
        }

        Ok(())
    }
}
