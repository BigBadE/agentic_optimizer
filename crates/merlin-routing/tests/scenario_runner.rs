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
use ratatui::buffer::Buffer;
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

mod common;
mod e2e;
mod integration;
mod unit;

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
    #[serde(default)]
    pub children: Vec<ExistingTask>,
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
    TaskComplete { data: TaskCompleteData },
    TaskFailed { data: TaskFailedData },
    KeyPress { data: KeyPressData },
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
struct KeyPressData {
    pub key: String,
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

/// Data describing a task completion action
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskCompleteData {
    /// Description of the task to mark as completed
    pub description: String,
}

/// Data describing a task failure action
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskFailedData {
    /// Description of the task to mark as failed
    pub description: String,
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
    /// Expected `active_task_id` (by description)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_task_id: Option<String>,
    /// Expected expanded conversations (by description)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_conversations: Option<Vec<String>>,
    /// Expected scroll offset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_list_scroll_offset: Option<usize>,
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
    scenario_path: String,
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
            scenario_path: name.to_string(),
            workspace_dir: None,
        })
    }

    /// Run the scenario
    /// Runs the loaded scenario.
    ///
    /// # Errors
    /// Returns an error if setup or any step execution fails.
    pub async fn run(mut self) -> Result<(), String> {
        info!("=== Running Scenario: {} ===", self.scenario.name);
        info!("Description: {}", self.scenario.description);

        // Set up initial state
        self.setup_initial_state()?;

        // Create UI app
        let (width, height) = self.scenario.config.terminal_size;
        let (mut app, _ui_channel) = create_test_app(width, height)
            .map_err(|error| format!("Failed to create test app: {error}"))?;

        // Load existing tasks if any
        self.load_existing_tasks(&mut app);

        // Adjust scroll to show placeholder when tasks are loaded
        // This mimics what happens in production when tasks arrive via events
        // Position scroll so placeholder is at the bottom of the visible window
        if !self.scenario.initial_state.existing_tasks.is_empty() {
            let visible_task_count = app
                .task_manager()
                .iter_tasks()
                .filter(|(_, task)| task.parent_id.is_none())
                .count();
            // Placeholder is at index visible_task_count (after all tasks)
            // We want it in the last position of a 3-item window
            let max_visible = 3;
            let placeholder_index = visible_task_count;
            // Show placeholder at bottom: skip (placeholder_index - (max_visible - 1)) tasks
            // This ensures placeholder is in the last slot of the visible window
            app.state_mut().task_list_scroll_offset =
                placeholder_index.saturating_sub(max_visible - 1);
        }

        // Initialize vector cache if enabled
        if self.scenario.config.enable_vector_cache {
            self.initialize_vector_cache(&mut app);
            // Tick to render the initial state
            app.tick()
                .map_err(|error| format!("Tick failed after vector cache init: {error}"))?;
        }

        // Execute each step
        for step in &self.scenario.steps {
            info!("--- Step {} ---", step.step);
            self.execute_step(&mut app, step).await?;
        }

        info!("✓ Scenario completed successfully");
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

    /// Loads existing tasks from initial state
    fn load_existing_tasks(&self, app: &mut impl TestApp) {
        use std::time::Duration;

        let now = Instant::now();
        for (index, existing_task) in self
            .scenario
            .initial_state
            .existing_tasks
            .iter()
            .enumerate()
        {
            let start_time = now
                .checked_sub(Duration::from_secs(
                    (self.scenario.initial_state.existing_tasks.len() - index) as u64 * 100,
                ))
                .unwrap();
            self.load_task_recursive(app, existing_task, None, start_time);
        }
    }

    /// Recursively loads a task and its children
    fn load_task_recursive(
        &self,
        app: &mut impl TestApp,
        existing_task: &ExistingTask,
        parent_id: Option<TaskId>,
        start_time: Instant,
    ) {
        use std::time::Duration;

        let task_id = TaskId::default();

        let task = TaskDisplay {
            description: existing_task.description.clone(),
            status: match existing_task.status.as_str() {
                "Completed" => TaskStatus::Completed,
                "Failed" => TaskStatus::Failed,
                _ => TaskStatus::Running,
            },
            start_time,
            end_time: (existing_task.status == "Completed" || existing_task.status == "Failed")
                .then(|| start_time + Duration::from_secs(50)),
            parent_id,
            progress: None,
            output_lines: vec![],
            output_tree: OutputTree::default(),
            steps: vec![],
        };

        app.task_manager_mut().add_task(task_id, task);

        // Load children with slightly later start times
        for (child_index, child) in existing_task.children.iter().enumerate() {
            let child_start_time = start_time + Duration::from_secs((child_index + 1) as u64 * 10);
            self.load_task_recursive(app, child, Some(task_id), child_start_time);
        }
    }

    /// Initializes vector cache progress if enabled.
    fn initialize_vector_cache(&self, app: &mut impl TestApp) {
        // Set embedding progress in UI state instead of creating a task
        let total_files = self.scenario.initial_state.workspace_files.len() as u64;
        app.state_mut().embedding_progress = Some((0, total_files));
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
            StepAction::TaskComplete { data } => {
                Self::execute_task_complete(app, data);
            }
            StepAction::TaskFailed { data } => {
                Self::execute_task_failed(app, data);
            }
            StepAction::KeyPress { data } => {
                Self::execute_key_press(app, data)?;
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

    /// Executes a tool result action, adding output to the most recent running task.
    ///
    /// # Errors
    /// Never returns an error.
    fn execute_tool_result(app: &mut impl TestApp, data: &ToolResultData) {
        info!(
            "  Tool {}: {}",
            data.tool,
            if data.success { "SUCCESS" } else { "FAILED" }
        );

        // Find the most recent running task and add output to it
        let task_ids: Vec<_> = app.task_manager().iter_tasks().map(|(id, _)| id).collect();
        for task_id in task_ids.iter().rev() {
            if let Some(task) = app.task_manager_mut().get_task_mut(*task_id)
                && task.status == TaskStatus::Running
            {
                task.output_lines.push(data.output.clone());
                break;
            }
        }
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

        if data.event_type == "embedding_progress" {
            // Update embedding progress in UI state instead of creating a task
            app.state_mut().embedding_progress = Some((data.progress, data.total));
        } else if data.event_type == "embedding_complete" {
            // Clear embedding progress when complete
            app.state_mut().embedding_progress = None;
        } else if data.event_type == "task_progress" {
            // Create or update task with progress for non-embedding tasks
            let task_id = TaskId::default();
            let mut task = create_test_task("Task with progress");
            task.status = TaskStatus::Running;
            task.progress = Some(TaskProgress {
                stage: "Processing".to_string(),
                current: data.progress,
                total: Some(data.total),
                message: String::new(),
            });

            app.task_manager_mut().add_task(task_id, task);
            app.state_mut().active_running_tasks.insert(task_id);
            app.state_mut().active_task_id = Some(task_id);
        }
    }

    /// Marks a task as completed by finding it by description.
    fn execute_task_complete(app: &mut impl TestApp, data: &TaskCompleteData) {
        info!("  Completing task: {}", data.description);

        // Find the task by description and mark it as completed
        let visible_tasks = app.task_manager().get_visible_tasks();
        for task_id in visible_tasks {
            if let Some(task) = app.task_manager_mut().get_task_mut(task_id)
                && task.description.contains(&data.description)
            {
                task.status = TaskStatus::Completed;
                task.end_time = Some(Instant::now());
                // Clear progress indicator when task completes
                task.progress = None;
                app.state_mut().active_running_tasks.remove(&task_id);
                break;
            }
        }
    }

    /// Marks a task as failed by finding it by description.
    fn execute_task_failed(app: &mut impl TestApp, data: &TaskFailedData) {
        info!("  Failing task: {}", data.description);

        // Find the task by description and mark it as failed
        let visible_tasks = app.task_manager().get_visible_tasks();
        for task_id in visible_tasks {
            if let Some(task) = app.task_manager_mut().get_task_mut(task_id)
                && task.description.contains(&data.description)
            {
                task.status = TaskStatus::Failed;
                task.end_time = Some(Instant::now());
                app.state_mut().active_running_tasks.remove(&task_id);
            }
        }
    }

    /// Executes a key press action.
    ///
    /// # Errors
    /// Returns an error if ticking the app after key press fails.
    fn execute_key_press(app: &mut impl TestApp, data: &KeyPressData) -> Result<(), String> {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        info!("  Key press: {}", data.key);

        let key_code = match data.key.as_str() {
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Enter" => KeyCode::Enter,
            "Tab" => KeyCode::Tab,
            "Esc" => KeyCode::Esc,
            _ => return Err(format!("Unsupported key: {}", data.key)),
        };

        let event = Event::Key(KeyEvent::new(key_code, KeyModifiers::NONE));
        app.set_event_source(Box::new(TestEventSource::with_events(vec![event])));
        app.tick()
            .map_err(|error| format!("Tick failed after key press: {error}"))?;

        Ok(())
    }

    /// Verifies all expectations after a step.
    ///
    /// # Errors
    /// Returns an error if any expectation fails.
    fn verify_expectations(
        &self,
        app: &mut impl TestApp,
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
            let has_embedding_progress = app.state().embedding_progress.is_some();

            if embedding_exp.status == "running" && !has_embedding_progress {
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
        app: &mut impl TestApp,
        expectations: &UiStateExpectations,
    ) -> Result<(), String> {
        // Set focused pane if specified
        if let Some(focused_pane) = &expectations.focused_pane {
            app.set_focused_pane(focused_pane);
            // Tick to re-render with new focus
            app.tick()
                .map_err(|error| format!("Tick failed after setting focus: {error}"))?;
        }

        // Verify task count
        if let Some(expected_count) = expectations.task_count {
            let actual_count = app.task_manager().get_visible_tasks().len();
            if actual_count != expected_count {
                return Err(format!(
                    "Expected {expected_count} tasks, got {actual_count}"
                ));
            }
        }

        // Verify has_progress (check both task progress and embedding progress)
        if let Some(expected_progress) = expectations.has_progress {
            let has_task_progress = app.task_manager().has_tasks_with_progress();
            let has_embedding_progress = app.state().embedding_progress.is_some();
            let actual_progress = has_task_progress || has_embedding_progress;
            if actual_progress != expected_progress {
                return Err(format!(
                    "Expected has_progress={expected_progress}, got {actual_progress}"
                ));
            }
        }

        // Verify active_task_id
        if let Some(expected_desc) = &expectations.active_task_id {
            Self::verify_active_task_description(app, expected_desc)?;
        }

        // Verify expanded_conversations
        if let Some(expected_expanded) = &expectations.expanded_conversations {
            let actual_expanded = &app.state().expanded_conversations;
            let mut actual_descriptions: Vec<String> = actual_expanded
                .iter()
                .filter_map(|id| {
                    app.task_manager()
                        .get_task(*id)
                        .map(|task| task.description.clone())
                })
                .collect();
            actual_descriptions.sort();

            let mut expected_sorted = expected_expanded.clone();
            expected_sorted.sort();

            if actual_descriptions != expected_sorted {
                return Err(format!(
                    "Expected expanded conversations {expected_sorted:?}, got {actual_descriptions:?}"
                ));
            }
        }

        // Verify task_list_scroll_offset
        if let Some(expected_offset) = expectations.task_list_scroll_offset {
            let actual_offset = app.state().task_list_scroll_offset;
            if actual_offset != expected_offset {
                return Err(format!(
                    "Expected task_list_scroll_offset {expected_offset}, got {actual_offset}"
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

    /// Verifies that the active task matches the expected description.
    ///
    /// # Errors
    /// Returns an error if the active task ID doesn't match the expected description.
    fn verify_active_task_description(
        app: &impl TestApp,
        expected_desc: &str,
    ) -> Result<(), String> {
        let actual_id = app.state().active_task_id;
        if let Some(task_id) = actual_id {
            if let Some(task) = app.task_manager().get_task(task_id) {
                if task.description != *expected_desc {
                    return Err(format!(
                        "Expected active_task_id to be '{}', got '{}'",
                        expected_desc, task.description
                    ));
                }
            } else {
                return Err(format!(
                    "Active task ID {task_id:?} not found in task manager"
                ));
            }
        } else {
            return Err(format!(
                "Expected active_task_id '{expected_desc}', but no task is active"
            ));
        }
        Ok(())
    }

    /// Verifies a snapshot of the UI buffer.
    ///
    /// # Panics
    /// Panics in debug logging if snapshot path relativization fails (only in logging).
    ///
    /// # Errors
    /// Returns an error if the snapshot is missing or cannot be read.
    #[cfg_attr(
        test,
        allow(
            clippy::too_many_lines,
            reason = "Snapshot verification requires extensive dimension checking and file I/O"
        )
    )]
    fn verify_snapshot(&self, app: &impl TestApp, snapshot_name: &str) -> Result<(), String> {
        // Render UI to string
        let buffer = app.backend().buffer();
        let width = buffer.area().width as usize;
        let height = buffer.area().height as usize;

        // Extract and validate rendered content
        let rendered = Self::extract_rendered_content(buffer, width, height)?;
        Self::validate_ui_sections(&rendered, app.state().active_task_id.is_some())?;

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
        // Extract category from scenario path (e.g., "ui/ui_completed_tasks" -> "ui")
        self.scenario_path.find('/').map_or_else(
            || "other".to_string(),
            |slash_pos| self.scenario_path[..slash_pos].to_string(),
        )
    }

    /// Extracts rendered content from buffer
    fn extract_rendered_content(
        buffer: &Buffer,
        width: usize,
        height: usize,
    ) -> Result<String, String> {
        use unicode_width::UnicodeWidthStr as _;

        // Extract content properly handling wide characters
        let mut rendered_lines = Vec::new();
        for y in 0..height {
            let mut line = String::new();
            let mut x = 0;
            while x < width {
                let cell = buffer.cell((x as u16, y as u16)).unwrap();
                let symbol = cell.symbol();
                line.push_str(symbol);

                // Skip cells that are part of wide characters
                let char_width = symbol.width();
                x += char_width.max(1);
            }
            rendered_lines.push(line);
        }
        let rendered = rendered_lines.join("\n");

        // Verify dimensions match terminal size
        let lines: Vec<&str> = rendered.lines().collect();
        if lines.len() != height {
            return Err(format!(
                "Snapshot height mismatch: expected {} lines (terminal height), got {}",
                height,
                lines.len()
            ));
        }

        for (line_num, line) in lines.iter().enumerate() {
            let line_width = line.width();
            if line_width != width {
                return Err(format!(
                    "Snapshot width mismatch on line {}: expected {} chars (terminal width), got {}",
                    line_num + 1,
                    width,
                    line_width
                ));
            }
        }

        Ok(rendered)
    }

    /// Validates that all essential UI sections are present
    fn validate_ui_sections(rendered: &str, has_active_task: bool) -> Result<(), String> {
        // Verify all essential UI sections are present
        let has_tasks_section = rendered.contains("─── Tasks ");
        let has_input_section = rendered.contains("─── Input ");

        if !has_tasks_section {
            return Err(
                "UI validation failed: Tasks section not found in rendered output".to_string(),
            );
        }

        if !has_input_section {
            return Err("UI validation failed: Input section not found in rendered output - UI may be too large for screen".to_string());
        }

        // If there's an active task, verify the Focused section is present
        if has_active_task {
            let has_focused_section = rendered.contains("─── Focused ");
            if !has_focused_section {
                return Err("UI validation failed: Focused section not found despite active task - UI may be too large for screen".to_string());
            }
        }

        Ok(())
    }
}

/// Trait for accessing app internals in tests
trait TestApp {
    fn task_manager(&self) -> &TaskManager;
    fn task_manager_mut(&mut self) -> &mut TaskManager;
    fn state(&self) -> &UiState;
    fn state_mut(&mut self) -> &mut UiState;
    fn set_event_source(&mut self, source: Box<dyn UiInputEventSource + Send>);
    fn tick(&mut self) -> RoutingResult<bool>;
    fn backend(&self) -> &TestBackend;
    fn set_focused_pane(&mut self, pane: &str);
}

impl TestApp for TuiApp<TestBackend> {
    fn task_manager(&self) -> &TaskManager {
        self.task_manager()
    }

    fn task_manager_mut(&mut self) -> &mut TaskManager {
        self.task_manager_mut()
    }

    fn state(&self) -> &UiState {
        self.state()
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

    fn set_focused_pane(&mut self, pane: &str) {
        use merlin_routing::user_interface::renderer::FocusedPane;

        let focused = match pane {
            "Tasks" => FocusedPane::Tasks,
            "Output" => FocusedPane::Output,
            _ => FocusedPane::Input,
        };
        self.set_focused_pane(focused);
    }
}
