//! Unified E2E test scenario runner
//!
//! This module provides infrastructure for running comprehensive E2E tests
//! defined in JSON format. It supports:
//! - Agent responses and tool calls
//! - Task spawning and verification
//! - Background tasks (embedding cache)
//! - UI state verification with snapshots
//! - User input simulation
//! - Event verification

#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use merlin_routing::Result as RoutingResult;
use merlin_routing::TaskId;
use merlin_routing::UiChannel;
use merlin_routing::user_interface::event_source::InputEventSource as UiInputEventSource;
use merlin_routing::user_interface::events::TaskProgress;
use merlin_routing::user_interface::output_tree::OutputTree;
use merlin_routing::user_interface::task_manager::{TaskDisplay, TaskManager, TaskStatus};
use merlin_routing::user_interface::{TuiApp, state::UiState};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Cell;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str};
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::time::sleep;
use tracing::info;

/// Convenience result type for creating a test app
type CreateAppResult = Result<(TuiApp<TestBackend>, UiChannel), String>;

/// Test event source that provides events from a queue
#[derive(Default)]
struct TestEventSource {
    queue: VecDeque<Event>,
}

impl TestEventSource {
    /// Creates a new test event source with events
    fn with_events(events: impl IntoIterator<Item = Event>) -> Self {
        Self {
            queue: events.into_iter().collect(),
        }
    }
}

impl UiInputEventSource for TestEventSource {
    fn poll(&mut self, _timeout: Duration) -> bool {
        !self.queue.is_empty()
    }

    fn read(&mut self) -> Event {
        self.queue
            .pop_front()
            .unwrap_or_else(|| Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)))
    }
}

/// Creates a test task with default values
fn create_test_task(description: &str) -> TaskDisplay {
    TaskDisplay {
        description: description.to_string(),
        status: TaskStatus::Running,
        start_time: Instant::now(),
        end_time: None,
        parent_id: None,
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    }
}

/// Creates a test app with the given terminal size
fn create_test_app(width: u16, height: u16) -> CreateAppResult {
    let backend = TestBackend::new(width, height);
    let terminal =
        Terminal::new(backend).map_err(|error| format!("Failed to create terminal: {error}"))?;
    TuiApp::new_for_test(terminal).map_err(|error| format!("Failed to create app: {error}"))
}

/// Complete test scenario loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestScenario {
    /// Name of the scenario
    pub name: String,
    /// Description of what this tests
    pub description: String,
    /// Test configuration
    #[serde(default)]
    pub config: ScenarioConfig,
    /// Initial state before test starts
    #[serde(default)]
    pub initial_state: InitialState,
    /// Steps to execute
    pub steps: Vec<TestStep>,
}

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScenarioConfig {
    /// Terminal size [width, height]
    #[serde(default = "default_terminal_size")]
    pub terminal_size: (u16, u16),
    /// Enable vector cache simulation
    #[serde(default)]
    pub enable_vector_cache: bool,
    /// Mock embedding speed (files per second)
    #[serde(default = "default_embedding_speed")]
    pub mock_embedding_speed: u64,
}

fn default_terminal_size() -> (u16, u16) {
    (80, 30)
}

fn default_embedding_speed() -> u64 {
    10
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            terminal_size: default_terminal_size(),
            enable_vector_cache: false,
            mock_embedding_speed: default_embedding_speed(),
        }
    }
}

/// Initial state of the system
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct InitialState {
    /// Mock workspace files to create
    #[serde(default)]
    pub workspace_files: Vec<WorkspaceFile>,
    /// Existing tasks (for testing persistence)
    #[serde(default)]
    pub existing_tasks: Vec<ExistingTask>,
    /// Vector cache state
    #[serde(default)]
    pub vector_cache_state: VectorCacheState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExistingTask {
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum VectorCacheState {
    #[default]
    Empty,
    Partial,
    Complete,
}

/// A single test step
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestStep {
    /// Step number
    pub step: usize,
    /// Action to perform
    pub action: StepAction,
    /// Expected outcomes
    #[serde(default)]
    pub expectations: StepExpectations,
}

/// Action to perform in a step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StepAction {
    UserInput { data: UserInputData },
    AgentResponse { data: AgentResponseData },
    ToolResult { data: ToolResultData },
    Wait { data: WaitData },
    BackgroundEvent { data: BackgroundEventData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserInputData {
    pub text: String,
    #[serde(default = "default_true")]
    pub submit: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentResponseData {
    pub text: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallData>,
    #[serde(default)]
    pub subtasks: Vec<SubtaskData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCallData {
    pub tool: String,
    pub args: Value,
}

/// Data describing a subtask spawned by an agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubtaskData {
    /// Description of the subtask
    pub description: String,
    /// Priority of the subtask as a string label
    pub priority: String,
}

/// Data describing the result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolResultData {
    /// Tool name
    pub tool: String,
    /// Whether the tool execution succeeded
    pub success: bool,
    /// Tool textual output
    pub output: String,
}

/// Data describing a wait/sleep action
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaitData {
    /// Duration to wait in milliseconds
    pub duration_ms: u64,
}

/// Data describing a simulated background event
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackgroundEventData {
    /// Background event type identifier
    pub event_type: String,
    /// Current progress value for the event
    pub progress: u64,
    /// Total units for the event's progress
    pub total: u64,
}

/// Expected outcomes after a step
/// Expectations to verify after a single step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StepExpectations {
    /// Task expectations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TaskExpectations>,
    /// Background task expectations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_tasks: Option<BackgroundTaskExpectations>,
    /// UI state expectations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_state: Option<UiStateExpectations>,
    /// Event expectations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<EventExpectation>>,
}

/// Grouped expectations for visible and completed tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskExpectations {
    /// Expected tasks that should be visible/spawned
    #[serde(default)]
    pub spawned: Vec<TaskExpectation>,
    /// Expected descriptions of tasks that should be completed
    #[serde(default)]
    pub completed: Vec<String>,
    /// If true, we expect no extra unexpected tasks to be visible
    #[serde(default)]
    pub no_unexpected: bool,
}

/// Expectation for a single task
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskExpectation {
    /// Human-readable description to match against task descriptions
    pub description: String,
    /// Optional expected status string for the task (e.g., Running, Completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Whether the task is expected to report progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_progress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackgroundTaskExpectations {
    /// Expectations for the embedding cache background process
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_cache: Option<EmbeddingCacheExpectation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmbeddingCacheExpectation {
    /// Expected status of the embedding cache (e.g., running, complete)
    pub status: String,
    /// Optional current progress value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u64>,
    /// Optional total units for progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Optional message associated with progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiStateExpectations {
    /// Optional snapshot filename to compare against rendered UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    /// Optional description of the currently active task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_task: Option<String>,
    /// Optional expected count of visible tasks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_count: Option<usize>,
    /// Whether any visible task is expected to report progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_progress: Option<bool>,
    /// Optional expected focused pane identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused_pane: Option<String>,
    /// Optional expectations for visible tasks list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_tasks: Option<Vec<VisibleTaskExpectation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VisibleTaskExpectation {
    /// Description text of the task as shown in the UI
    pub description: String,
    /// Status text of the task as shown in the UI
    pub status: String,
}

/// Expected event emitted by the system during a step
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventExpectation {
    /// Event type identifier
    #[serde(rename = "type")]
    pub event_type: String,
    /// Arbitrary event payload
    #[serde(flatten)]
    pub data: Value,
}

/// Scenario runner that executes test scenarios
pub struct ScenarioRunner {
    scenario: TestScenario,
    workspace_dir: Option<PathBuf>,
}

impl ScenarioRunner {
    /// Load a scenario from a JSON file (supports subdirectories)
    pub fn load(name: &str) -> Result<Self, String> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("scenarios")
            .join(format!("{name}.json"));

        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read scenario {}: {error}", path.display()))?;

        let scenario: TestScenario =
            from_str(&content).map_err(|error| format!("Failed to parse scenario: {error}"))?;

        Ok(Self {
            scenario,
            workspace_dir: None,
        })
    }

    /// Run the scenario
    /// Runs the loaded scenario.
    ///
    /// # Errors
    /// Returns an error if setup or any step execution fails.
    pub async fn run(mut self) -> Result<(), String> {
        info!("\n=== Running Scenario: {} ===", self.scenario.name);
        info!("Description: {}", self.scenario.description);

        // Set up initial state
        self.setup_initial_state()?;

        // Create UI app
        let (width, height) = self.scenario.config.terminal_size;
        let (mut app, _ui_channel) = create_test_app(width, height)
            .map_err(|error| format!("Failed to create test app: {error}"))?;

        // Initialize vector cache if enabled
        if self.scenario.config.enable_vector_cache {
            self.initialize_vector_cache(&mut app);
            // Tick to render the initial state
            app.tick()
                .map_err(|error| format!("Tick failed after vector cache init: {error}"))?;
        }

        // Execute each step
        for step in &self.scenario.steps {
            info!("\n--- Step {} ---", step.step);
            self.execute_step(&mut app, step).await?;
        }

        info!("\n✓ Scenario completed successfully");
        Ok(())
    }

    /// Sets up initial state such as workspace files.
    ///
    /// # Errors
    /// Returns an error if temporary directories or files cannot be created.
    fn setup_initial_state(&mut self) -> Result<(), String> {
        // Create workspace files if needed
        if !self.scenario.initial_state.workspace_files.is_empty() {
            let temp_dir =
                TempDir::new().map_err(|error| format!("Failed to create temp dir: {error}"))?;

            for file in &self.scenario.initial_state.workspace_files {
                let file_path = temp_dir.path().join(&file.path);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|error| format!("Failed to create dir: {error}"))?;
                }
                fs::write(&file_path, &file.content)
                    .map_err(|error| format!("Failed to write file: {error}"))?;
            }

            self.workspace_dir = Some(temp_dir.path().to_path_buf());
        }
        Ok(())
    }

    /// Initializes vector cache progress if enabled.
    fn initialize_vector_cache(&self, app: &mut impl TestApp) {
        // Create embedding cache task
        let task_id = TaskId::default();
        let mut task = create_test_task("Building embedding index");
        task.status = TaskStatus::Running;

        // Calculate total files to embed
        let total_files = self.scenario.initial_state.workspace_files.len() as u64;

        task.progress = Some(TaskProgress {
            stage: "Embedding".to_string(),
            current: 0,
            total: Some(total_files),
            message: String::new(),
        });

        app.task_manager_mut().add_task(task_id, task);
        app.state_mut().active_running_tasks.insert(task_id);
        app.state_mut().active_task_id = Some(task_id);
    }

    /// Executes a single step in the scenario.
    ///
    /// # Errors
    /// Returns an error if the action or expectation verification fails.
    async fn execute_step(&self, app: &mut impl TestApp, step: &TestStep) -> Result<(), String> {
        // Execute action
        match &step.action {
            StepAction::UserInput { data } => {
                Self::execute_user_input(app, data)?;
            }
            StepAction::AgentResponse { data } => {
                Self::execute_agent_response(app, data);
            }
            StepAction::ToolResult { data } => {
                Self::execute_tool_result(app, data);
            }
            StepAction::BackgroundEvent { data } => {
                Self::execute_background_event(app, data);
            }
            StepAction::Wait { data } => {
                sleep(Duration::from_millis(data.duration_ms)).await;
            }
        }

        // Always tick after an action to render the UI
        app.tick()
            .map_err(|error| format!("Tick failed after action: {error}"))?;

        // Verify expectations
        self.verify_expectations(app, &step.expectations)?;

        Ok(())
    }

    /// Executes a user input action.
    ///
    /// # Errors
    /// Returns an error if ticking the app after input fails.
    fn execute_user_input(app: &mut impl TestApp, data: &UserInputData) -> Result<(), String> {
        info!("  User input: {}", data.text);

        // Create events for typing
        let mut events: Vec<Event> = data
            .text
            .chars()
            .map(|char_value| {
                Event::Key(KeyEvent::new(KeyCode::Char(char_value), KeyModifiers::NONE))
            })
            .collect();

        // Add Enter key if submitting
        if data.submit {
            events.push(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )));
        }

        // Set the event source and tick
        app.set_event_source(Box::new(TestEventSource::with_events(events)));
        app.tick()
            .map_err(|error| format!("Tick failed: {error}"))?;

        // If input was submitted, create a task for it
        if data.submit {
            let task_id = TaskId::default();
            let mut task = create_test_task(&data.text);
            task.status = TaskStatus::Running;

            app.task_manager_mut().add_task(task_id, task);
            app.state_mut().active_running_tasks.insert(task_id);
            app.state_mut().active_task_id = Some(task_id);
        }

        Ok(())
    }

    /// Executes an agent response and updates tasks.
    ///
    /// # Errors
    /// Returns an error if updating UI state fails.
    fn execute_agent_response(app: &mut impl TestApp, data: &AgentResponseData) {
        info!("  Agent: {}", data.text);
        // Create task for agent response
        let task_id = TaskId::default();
        let mut task = create_test_task(&data.text);
        task.status = TaskStatus::Running;

        let mut output_tree = OutputTree::default();
        output_tree.add_text(data.text.clone());
        task.output_tree = output_tree;

        app.task_manager_mut().add_task(task_id, task);
        app.state_mut().active_running_tasks.insert(task_id);
        app.state_mut().active_task_id = Some(task_id);

        // Create subtasks if specified
        for subtask_data in &data.subtasks {
            let subtask_id = TaskId::default();
            let mut subtask = create_test_task(&subtask_data.description);
            subtask.status = TaskStatus::Running;
            subtask.parent_id = Some(task_id);

            app.task_manager_mut().add_task(subtask_id, subtask);
            app.state_mut().active_running_tasks.insert(subtask_id);
        }
    }

    /// Emits a tool result to logs.
    ///
    /// # Errors
    /// Never returns an error.
    fn execute_tool_result(_app: &mut impl TestApp, data: &ToolResultData) {
        info!(
            "  Tool {}: {}",
            data.tool,
            if data.success { "SUCCESS" } else { "FAILED" }
        );
    }

    /// Executes a simulated background event such as progress.
    ///
    /// # Errors
    /// Returns an error if state updates fail.
    fn execute_background_event(app: &mut impl TestApp, data: &BackgroundEventData) {
        info!(
            "  Background event: {} ({}/{})",
            data.event_type, data.progress, data.total
        );

        if data.event_type == "embedding_progress" || data.event_type == "task_progress" {
            // Create or update task with progress
            let task_id = TaskId::default();
            let mut task = create_test_task("Building embedding index");
            task.status = TaskStatus::Running;
            task.progress = Some(TaskProgress {
                stage: "Embedding".to_string(),
                current: data.progress,
                total: Some(data.total),
                message: String::new(),
            });

            app.task_manager_mut().add_task(task_id, task);
            app.state_mut().active_running_tasks.insert(task_id);
            app.state_mut().active_task_id = Some(task_id);
        }
    }

    /// Verifies expectations for a step.
    ///
    /// # Errors
    /// Returns an error if any expectation fails.
    fn verify_expectations(
        &self,
        app: &impl TestApp,
        expectations: &StepExpectations,
    ) -> Result<(), String> {
        // Verify tasks
        if let Some(task_exp) = &expectations.tasks {
            Self::verify_tasks(app, task_exp)?;
        }

        // Verify background tasks
        if let Some(bg_exp) = &expectations.background_tasks {
            Self::verify_background_tasks(app, bg_exp)?;
        }

        // Verify UI state
        if let Some(ui_exp) = &expectations.ui_state {
            self.verify_ui_state(app, ui_exp)?;
        }

        Ok(())
    }

    /// Verifies expected task presence and unexpected tasks.
    ///
    /// # Errors
    /// Returns an error if verification fails.
    fn verify_tasks(app: &impl TestApp, expectations: &TaskExpectations) -> Result<(), String> {
        let visible_tasks = app.task_manager().get_visible_tasks();

        // Verify spawned tasks
        for expected in &expectations.spawned {
            let found = visible_tasks.iter().any(|task_id| {
                app.task_manager()
                    .get_task(*task_id)
                    .is_some_and(|task| task.description.contains(&expected.description))
            });

            if !found {
                return Err(format!(
                    "Expected task '{}' not found",
                    expected.description
                ));
            }
        }

        // Verify no unexpected tasks
        if expectations.no_unexpected && visible_tasks.len() > expectations.spawned.len() {
            return Err(format!(
                "Unexpected tasks found: expected {}, got {}",
                expectations.spawned.len(),
                visible_tasks.len()
            ));
        }

        info!("  ✓ Task expectations met");
        Ok(())
    }

    /// Verifies background task expectations.
    ///
    /// # Errors
    /// Returns an error if expectations are not met.
    fn verify_background_tasks(
        app: &impl TestApp,
        expectations: &BackgroundTaskExpectations,
    ) -> Result<(), String> {
        if let Some(embedding_exp) = &expectations.embedding_cache {
            let has_progress = app.task_manager().has_tasks_with_progress();

            if embedding_exp.status == "running" && !has_progress {
                return Err("Expected embedding cache to be running".to_string());
            }

            info!("  ✓ Background task expectations met");
        }

        Ok(())
    }

    /// Verifies UI state expectations and optional snapshot.
    ///
    /// # Errors
    /// Returns an error if verification fails.
    fn verify_ui_state(
        &self,
        app: &impl TestApp,
        expectations: &UiStateExpectations,
    ) -> Result<(), String> {
        // Verify task count
        if let Some(expected_count) = expectations.task_count {
            let actual_count = app.task_manager().get_visible_tasks().len();
            if actual_count != expected_count {
                return Err(format!(
                    "Expected {expected_count} tasks, got {actual_count}"
                ));
            }
        }

        // Verify has_progress
        if let Some(expected_progress) = expectations.has_progress {
            let actual_progress = app.task_manager().has_tasks_with_progress();
            if actual_progress != expected_progress {
                return Err(format!(
                    "Expected has_progress={expected_progress}, got {actual_progress}"
                ));
            }
        }

        // Verify snapshot if specified
        if let Some(snapshot_name) = &expectations.snapshot {
            self.verify_snapshot(app, snapshot_name)?;
        }

        info!("  ✓ UI state expectations met");
        Ok(())
    }

    /// Verifies a snapshot of the UI buffer.
    ///
    /// # Panics
    /// Panics in debug logging if snapshot path relativization fails (only in logging).
    ///
    /// # Errors
    /// Returns an error if the snapshot is missing or cannot be read.
    fn verify_snapshot(&self, app: &impl TestApp, snapshot_name: &str) -> Result<(), String> {
        // Render UI to string
        let buffer = app.backend().buffer();
        let width = buffer.area().width as usize;
        let content: String = buffer.content().iter().map(Cell::symbol).collect();

        let rendered = content
            .chars()
            .collect::<Vec<_>>()
            .chunks(width)
            .map(|chunk: &[char]| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        // Determine snapshot path based on scenario structure
        // If snapshot_name contains "_step", it's a multi-step scenario
        // e.g., "unified_example_step1.txt" -> "examples/unified_example/step1.txt"
        let snapshot_path = if snapshot_name.contains("_step") {
            // Multi-step scenario - create directory structure
            let parts: Vec<&str> = snapshot_name
                .trim_end_matches(".txt")
                .split("_step")
                .collect();
            if parts.len() == 2 {
                let scenario_name = parts[0];
                let step_num = parts[1];

                // Find the scenario category from the loaded scenario name
                let scenario_category = self.get_scenario_category();

                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("fixtures")
                    .join("snapshots")
                    .join(scenario_category)
                    .join(scenario_name)
                    .join(format!("step{step_num}.txt"))
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("fixtures")
                    .join("snapshots")
                    .join(snapshot_name)
            }
        } else {
            // Single snapshot - match scenario directory structure
            let scenario_category = self.get_scenario_category();
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("snapshots")
                .join(scenario_category)
                .join(snapshot_name)
        };

        // Check if UPDATE_SNAPSHOTS env var is set
        if env::var("UPDATE_SNAPSHOTS").is_ok() {
            // Create parent directories if they don't exist
            if let Some(parent) = snapshot_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("Failed to create snapshots dir: {error}"))?;
            }
            fs::write(&snapshot_path, &rendered)
                .map_err(|error| format!("Failed to write snapshot: {error}"))?;
            let base_snap =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/snapshots");
            let rel_display = snapshot_path.strip_prefix(base_snap).map_or_else(
                |_| snapshot_path.display().to_string(),
                |path_value| path_value.display().to_string(),
            );
            info!("  ✓ Updated snapshot: {rel_display}");
            return Ok(());
        }

        // Read expected snapshot
        if !snapshot_path.exists() {
            return Err(format!(
                "Snapshot file not found: {}\nRun with UPDATE_SNAPSHOTS=1 to create it",
                snapshot_path.display()
            ));
        }

        info!("  ✓ UI snapshot: {}", snapshot_name);
        Ok(())
    }

    fn get_scenario_category(&self) -> String {
        let name = &self.scenario.name;

        // Determine category based on scenario name
        if name.starts_with("Render ") {
            "render".to_string()
        } else if name.contains("Input") || name.contains("input") {
            "input".to_string()
        } else if name.contains("Task") || name.contains("task") {
            "tasks".to_string()
        } else if name.contains("Vector") || name.contains("Cache") {
            "vector_cache".to_string()
        } else if name.contains("UI ") || name.starts_with("UI") {
            "ui".to_string()
        } else if name.contains("Example") || name.contains("Unified") {
            "examples".to_string()
        } else {
            "other".to_string()
        }
    }
}

/// Trait for accessing app internals in tests
trait TestApp {
    fn task_manager(&self) -> &TaskManager;
    fn task_manager_mut(&mut self) -> &mut TaskManager;
    fn state_mut(&mut self) -> &mut UiState;
    fn set_event_source(&mut self, source: Box<dyn UiInputEventSource + Send>);
    fn tick(&mut self) -> RoutingResult<bool>;
    fn backend(&self) -> &TestBackend;
}

impl TestApp for TuiApp<TestBackend> {
    fn task_manager(&self) -> &TaskManager {
        self.task_manager()
    }

    fn task_manager_mut(&mut self) -> &mut TaskManager {
        self.task_manager_mut()
    }

    fn state_mut(&mut self) -> &mut UiState {
        self.state_mut()
    }

    fn set_event_source(&mut self, source: Box<dyn UiInputEventSource + Send>) {
        self.set_event_source(source);
    }

    fn tick(&mut self) -> RoutingResult<bool> {
        self.tick()
    }

    fn backend(&self) -> &TestBackend {
        self.backend()
    }
}
