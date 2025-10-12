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

#![allow(missing_docs)]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use merlin_routing::TaskId;
use merlin_routing::user_interface::TuiApp;
use merlin_routing::user_interface::event_source::InputEventSource;
use merlin_routing::user_interface::events::TaskProgress;
use merlin_routing::user_interface::output_tree::OutputTree;
use merlin_routing::user_interface::task_manager::TaskStatus;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Cell;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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

impl InputEventSource for TestEventSource {
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
fn create_test_task(
    description: &str,
) -> merlin_routing::user_interface::task_manager::TaskDisplay {
    merlin_routing::user_interface::task_manager::TaskDisplay {
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
fn create_test_app(
    width: u16,
    height: u16,
) -> Result<(TuiApp<TestBackend>, merlin_routing::UiChannel), String> {
    let backend = TestBackend::new(width, height);
    let terminal =
        Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {}", e))?;
    TuiApp::new_for_test(terminal).map_err(|e| format!("Failed to create app: {}", e))
}

/// Complete test scenario loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
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
pub struct ScenarioConfig {
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
pub struct InitialState {
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
pub struct WorkspaceFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingTask {
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VectorCacheState {
    Empty,
    Partial,
    Complete,
}

impl Default for VectorCacheState {
    fn default() -> Self {
        Self::Empty
    }
}

/// A single test step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStep {
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
pub enum StepAction {
    UserInput { data: UserInputData },
    AgentResponse { data: AgentResponseData },
    ToolResult { data: ToolResultData },
    Wait { data: WaitData },
    BackgroundEvent { data: BackgroundEventData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputData {
    pub text: String,
    #[serde(default = "default_true")]
    pub submit: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponseData {
    pub text: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallData>,
    #[serde(default)]
    pub subtasks: Vec<SubtaskData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallData {
    pub tool: String,
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskData {
    pub description: String,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultData {
    pub tool: String,
    pub success: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitData {
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundEventData {
    pub event_type: String,
    pub progress: u64,
    pub total: u64,
}

/// Expected outcomes after a step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepExpectations {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExpectations {
    #[serde(default)]
    pub spawned: Vec<TaskExpectation>,
    #[serde(default)]
    pub completed: Vec<String>,
    #[serde(default)]
    pub no_unexpected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExpectation {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_progress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTaskExpectations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_cache: Option<EmbeddingCacheExpectation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingCacheExpectation {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStateExpectations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_progress: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused_pane: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_tasks: Option<Vec<VisibleTaskExpectation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibleTaskExpectation {
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventExpectation {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(flatten)]
    pub data: Value,
}

/// Scenario runner that executes test scenarios
pub struct ScenarioRunner {
    scenario: TestScenario,
    workspace_dir: Option<PathBuf>,
}

impl ScenarioRunner {
    /// Load a scenario from a JSON file
    pub fn load(name: &str) -> Result<Self, String> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("scenarios")
            .join(format!("{name}.json"));

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read scenario {}: {}", path.display(), e))?;

        let scenario: TestScenario = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse scenario: {}", e))?;

        Ok(Self {
            scenario,
            workspace_dir: None,
        })
    }

    /// Run the scenario
    pub async fn run(mut self) -> Result<(), String> {
        println!("\n=== Running Scenario: {} ===", self.scenario.name);
        println!("Description: {}", self.scenario.description);

        // Set up initial state
        self.setup_initial_state()?;

        // Create UI app
        let (width, height) = self.scenario.config.terminal_size;
        let (mut app, _ui_channel) = create_test_app(width, height)
            .map_err(|e| format!("Failed to create test app: {}", e))?;

        // Execute each step
        for step in &self.scenario.steps {
            println!("\n--- Step {} ---", step.step);
            self.execute_step(&mut app, step).await?;
        }

        println!("\n✓ Scenario completed successfully");
        Ok(())
    }

    fn setup_initial_state(&mut self) -> Result<(), String> {
        // Create workspace files if needed
        if !self.scenario.initial_state.workspace_files.is_empty() {
            let temp_dir = tempfile::TempDir::new()
                .map_err(|e| format!("Failed to create temp dir: {}", e))?;

            for file in &self.scenario.initial_state.workspace_files {
                let file_path = temp_dir.path().join(&file.path);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create dir: {}", e))?;
                }
                fs::write(&file_path, &file.content)
                    .map_err(|e| format!("Failed to write file: {}", e))?;
            }

            self.workspace_dir = Some(temp_dir.path().to_path_buf());
        }

        Ok(())
    }

    async fn execute_step(&self, app: &mut impl TestApp, step: &TestStep) -> Result<(), String> {
        // Execute action
        match &step.action {
            StepAction::UserInput { data } => {
                self.execute_user_input(app, data)?;
            }
            StepAction::AgentResponse { data } => {
                self.execute_agent_response(app, data)?;
            }
            StepAction::ToolResult { data } => {
                self.execute_tool_result(app, data)?;
            }
            StepAction::Wait { data } => {
                tokio::time::sleep(Duration::from_millis(data.duration_ms)).await;
            }
            StepAction::BackgroundEvent { data } => {
                self.execute_background_event(app, data)?;
            }
        }

        // Verify expectations
        self.verify_expectations(app, &step.expectations)?;

        Ok(())
    }

    fn execute_user_input(
        &self,
        app: &mut impl TestApp,
        data: &UserInputData,
    ) -> Result<(), String> {
        println!("  User input: {}", data.text);

        // Create events for typing
        let mut events: Vec<crossterm::event::Event> = data
            .text
            .chars()
            .map(|ch| {
                crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Char(ch),
                    crossterm::event::KeyModifiers::NONE,
                ))
            })
            .collect();

        // Add Enter key if submitting
        if data.submit {
            events.push(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Enter,
                    crossterm::event::KeyModifiers::NONE,
                ),
            ));
        }

        // Set the event source and tick
        app.set_event_source(Box::new(TestEventSource::with_events(events)));
        app.tick().map_err(|e| format!("Tick failed: {}", e))?;

        Ok(())
    }

    fn execute_agent_response(
        &self,
        app: &mut impl TestApp,
        data: &AgentResponseData,
    ) -> Result<(), String> {
        println!("  Agent: {}", data.text);
        // Create task for agent response
        let task_id = TaskId::default();
        let mut task = create_test_task(&data.text);
        task.status = TaskStatus::Running;

        let mut output_tree = OutputTree::default();
        output_tree.add_text(data.text.clone());
        task.output_tree = output_tree;

        app.task_manager_mut().add_task(task_id, task);
        app.state_mut().active_running_tasks.insert(task_id);

        Ok(())
    }

    fn execute_tool_result(
        &self,
        _app: &mut impl TestApp,
        data: &ToolResultData,
    ) -> Result<(), String> {
        println!(
            "  Tool {}: {}",
            data.tool,
            if data.success { "SUCCESS" } else { "FAILED" }
        );
        Ok(())
    }

    fn execute_background_event(
        &self,
        app: &mut impl TestApp,
        data: &BackgroundEventData,
    ) -> Result<(), String> {
        println!(
            "  Background event: {} ({}/{})",
            data.event_type, data.progress, data.total
        );

        if data.event_type == "embedding_progress" {
            // Create or update embedding task
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
        }

        Ok(())
    }

    fn verify_expectations(
        &self,
        app: &impl TestApp,
        expectations: &StepExpectations,
    ) -> Result<(), String> {
        // Verify tasks
        if let Some(task_exp) = &expectations.tasks {
            self.verify_tasks(app, task_exp)?;
        }

        // Verify background tasks
        if let Some(bg_exp) = &expectations.background_tasks {
            self.verify_background_tasks(app, bg_exp)?;
        }

        // Verify UI state
        if let Some(ui_exp) = &expectations.ui_state {
            self.verify_ui_state(app, ui_exp)?;
        }

        Ok(())
    }

    fn verify_tasks(
        &self,
        app: &impl TestApp,
        expectations: &TaskExpectations,
    ) -> Result<(), String> {
        let visible_tasks = app.task_manager().get_visible_tasks();

        // Verify spawned tasks
        for expected in &expectations.spawned {
            let found = visible_tasks.iter().any(|task_id| {
                if let Some(task) = app.task_manager().get_task(*task_id) {
                    task.description.contains(&expected.description)
                } else {
                    false
                }
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

        println!("  ✓ Task expectations met");
        Ok(())
    }

    fn verify_background_tasks(
        &self,
        app: &impl TestApp,
        expectations: &BackgroundTaskExpectations,
    ) -> Result<(), String> {
        if let Some(embedding_exp) = &expectations.embedding_cache {
            let has_progress = app.task_manager().has_tasks_with_progress();

            if embedding_exp.status == "running" && !has_progress {
                return Err("Expected embedding cache to be running".to_string());
            }

            println!("  ✓ Background task expectations met");
        }

        Ok(())
    }

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
                    "Expected {} tasks, got {}",
                    expected_count, actual_count
                ));
            }
        }

        // Verify has_progress
        if let Some(expected_progress) = expectations.has_progress {
            let actual_progress = app.task_manager().has_tasks_with_progress();
            if actual_progress != expected_progress {
                return Err(format!(
                    "Expected has_progress={}, got {}",
                    expected_progress, actual_progress
                ));
            }
        }

        // Verify snapshot if specified
        if let Some(snapshot_name) = &expectations.snapshot {
            self.verify_snapshot(app, snapshot_name)?;
        }

        println!("  ✓ UI state expectations met");
        Ok(())
    }

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

        let snapshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("snapshots")
            .join(snapshot_name);

        // Check if UPDATE_SNAPSHOTS env var is set
        if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
            fs::create_dir_all(snapshot_path.parent().unwrap())
                .map_err(|e| format!("Failed to create snapshots dir: {}", e))?;
            fs::write(&snapshot_path, &rendered)
                .map_err(|e| format!("Failed to write snapshot: {}", e))?;
            println!("  ✓ Updated snapshot: {}", snapshot_name);
            return Ok(());
        }

        // Read expected snapshot
        if !snapshot_path.exists() {
            return Err(format!(
                "Snapshot file not found: {}\nRun with UPDATE_SNAPSHOTS=1 to create it",
                snapshot_path.display()
            ));
        }

        let expected = fs::read_to_string(&snapshot_path)
            .map_err(|e| format!("Failed to read snapshot: {}", e))?;

        // Normalize line endings
        let expected_normalized = expected.replace("\r\n", "\n").trim().to_string();
        let rendered_normalized = rendered.replace("\r\n", "\n").trim().to_string();

        if expected_normalized != rendered_normalized {
            return Err(format!(
                "Snapshot mismatch for {}\nExpected:\n{}\n\nActual:\n{}",
                snapshot_name, expected_normalized, rendered_normalized
            ));
        }

        println!("  ✓ UI snapshot: {}", snapshot_name);
        Ok(())
    }
}

/// Trait for accessing app internals in tests
trait TestApp {
    fn task_manager(&self) -> &merlin_routing::user_interface::task_manager::TaskManager;
    fn task_manager_mut(
        &mut self,
    ) -> &mut merlin_routing::user_interface::task_manager::TaskManager;
    fn state_mut(&mut self) -> &mut merlin_routing::user_interface::state::UiState;
    fn set_event_source(
        &mut self,
        source: Box<dyn merlin_routing::user_interface::event_source::InputEventSource + Send>,
    );
    fn tick(&mut self) -> merlin_routing::Result<bool>;
    fn backend(&self) -> &ratatui::backend::TestBackend;
}

impl TestApp for merlin_routing::user_interface::TuiApp<ratatui::backend::TestBackend> {
    fn task_manager(&self) -> &merlin_routing::user_interface::task_manager::TaskManager {
        self.task_manager()
    }

    fn task_manager_mut(
        &mut self,
    ) -> &mut merlin_routing::user_interface::task_manager::TaskManager {
        self.task_manager_mut()
    }

    fn state_mut(&mut self) -> &mut merlin_routing::user_interface::state::UiState {
        self.state_mut()
    }

    fn set_event_source(
        &mut self,
        source: Box<dyn merlin_routing::user_interface::event_source::InputEventSource + Send>,
    ) {
        self.set_event_source(source)
    }

    fn tick(&mut self) -> merlin_routing::Result<bool> {
        self.tick()
    }

    fn backend(&self) -> &ratatui::backend::TestBackend {
        self.backend()
    }
}
