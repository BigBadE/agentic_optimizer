//! Snapshot testing framework for UI rendering
//!
//! This module provides utilities for snapshot-based testing of the TUI.
//! Snapshots are stored as text files and compared against actual rendered output.

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

mod common;

use common::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use merlin_routing::TaskId;
use merlin_routing::user_interface::event_source::InputEventSource;
use merlin_routing::user_interface::events::TaskProgress;
use merlin_routing::user_interface::output_tree::OutputTree;
use merlin_routing::user_interface::task_manager::{TaskDisplay, TaskStatus};
use merlin_routing::user_interface::{
    input::InputManager,
    renderer::{FocusedPane, RenderCtx, Renderer, UiCtx},
    state::UiState,
    task_manager::TaskManager,
    theme::Theme,
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Cell;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::Instant;
/// Configuration for a snapshot test
pub struct SnapshotTest {
    /// Name of the test (used for snapshot filename)
    pub name: String,
    /// Task manager setup
    pub task_manager: TaskManager,
    /// UI state
    pub state: UiState,
    /// Focused pane
    pub focused: FocusedPane,
    /// Terminal size (width, height)
    pub terminal_size: (u16, u16),
    /// Input manager with optional text
    pub input: InputManager,
}

// ============================================================================
// Test Event Source and Input Application
// ============================================================================

#[derive(Default)]
struct TestEventSource {
    queue: VecDeque<Event>,
}

impl TestEventSource {
    fn enqueue(&mut self, event: Event) {
        self.queue.push_back(event);
    }
}

impl InputEventSource for TestEventSource {
    fn poll(&mut self, timeout: Duration) -> bool {
        if !self.queue.is_empty() {
            return true;
        }
        if timeout.is_zero() {
            return false;
        }
        // Simple sleep to emulate blocking wait
        thread::sleep(timeout);
        !self.queue.is_empty()
    }

    fn read(&mut self) -> Event {
        loop {
            if let Some(event) = self.queue.pop_front() {
                return event;
            }
            // Busy-wait with small sleep
            thread::sleep(Duration::from_millis(1));
        }
    }
}

/// Applies queued events to the `InputManager` with semantics matching the app:
/// - Perform a blocking read once (guaranteeing at least one event)
/// - Drain immediately available events with poll(0)
/// - For each key event, feed it into the input area and perform wrapping for text edits
fn apply_events_to_input(
    input: &mut InputManager,
    terminal_width: u16,
    source: &mut dyn InputEventSource,
) {
    // Read at least one event
    let mut events = Vec::new();
    events.push(source.read());

    // Drain any immediately available events
    while source.poll(Duration::from_millis(0)) {
        events.push(source.read());
    }

    for event in events {
        if let Event::Key(key) = event {
            let should_wrap = matches!(
                key.code,
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
            );

            input.input_area_mut().input(Event::Key(key));

            if should_wrap {
                let input_width = (f32::from(terminal_width) * 0.7) as usize;
                let max_line_width = input_width.saturating_sub(4);
                input.auto_wrap(max_line_width);
            }
        }
    }
}

impl SnapshotTest {
    /// Create a new snapshot test
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            task_manager: TaskManager::default(),
            state: UiState::default(),
            focused: FocusedPane::Input,
            terminal_size: (80, 24),
            input: InputManager::default(),
        }
    }

    /// Set the terminal size
    #[must_use]
    pub fn with_terminal_size(mut self, width: u16, height: u16) -> Self {
        self.terminal_size = (width, height);
        self
    }

    /// Set the focused pane
    #[must_use]
    pub fn with_focused(mut self, focused: FocusedPane) -> Self {
        self.focused = focused;
        self
    }

    /// Add a task to the task manager
    #[must_use]
    pub fn with_task(mut self, task_id: TaskId, task: TaskDisplay) -> Self {
        self.task_manager.add_task(task_id, task);
        self
    }

    /// Set an active task
    #[must_use]
    pub fn with_active_task(mut self, task_id: TaskId) -> Self {
        self.state.active_task_id = Some(task_id);
        self.state.selected_task_index = 0;
        self
    }

    /// Mark a task as running
    #[must_use]
    pub fn with_running_task(mut self, task_id: TaskId) -> Self {
        self.state.active_running_tasks.insert(task_id);
        self
    }

    /// Simulate typing text into the input field using the same buffering semantics as the app's
    /// input event stream (blocking first read, then draining immediate events). This avoids
    /// mutating the buffer directly and exercises the same wrapping path.
    #[must_use]
    pub fn with_input_text(mut self, text: &str) -> Self {
        let mut source = TestEventSource::default();
        for character in text.chars() {
            let key_event = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
            source.enqueue(Event::Key(key_event));
        }
        // consume events with crossterm-like semantics
        apply_events_to_input(&mut self.input, self.terminal_size.0, &mut source);
        self
    }

    /// Get the snapshot file path
    fn snapshot_path(&self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("snapshots")
            .join(format!("{}.txt", self.name))
    }

    /// Render the UI and get the buffer as a string
    fn render_to_string(&self) -> String {
        let backend = TestBackend::new(self.terminal_size.0, self.terminal_size.1);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

        let renderer = Renderer::new(Theme::default());

        terminal
            .draw(|frame| {
                let ctx = RenderCtx {
                    ui_ctx: UiCtx {
                        task_manager: &self.task_manager,
                        state: &self.state,
                    },
                    input: &self.input,
                    focused: self.focused,
                };
                renderer.render(frame, &ctx);
            })
            .expect("Failed to render");

        buffer_to_lines(&terminal).join("\n")
    }

    /// Save the current render as a snapshot (for creating/updating snapshots)
    pub fn save_snapshot(&self) {
        let output = self.render_to_string();
        let path = self.snapshot_path();

        fs::create_dir_all(path.parent().unwrap()).expect("Failed to create snapshots dir");
        fs::write(&path, output).expect("Failed to write snapshot");
    }

    /// Run the snapshot test - compare rendered output against saved snapshot
    /// If `UPDATE_SNAPSHOTS` env var is set, updates the snapshot instead of checking
    pub fn assert_snapshot_matches(&self) {
        use std::env::var;

        // If in update mode, regenerate snapshot and return
        if var("UPDATE_SNAPSHOTS").is_ok() {
            self.save_snapshot();
            eprintln!("✓ Updated snapshot: {}", self.snapshot_path().display());
            return;
        }

        // Normal test mode: compare against saved snapshot
        let rendered = self.render_to_string();
        let snapshot_path = self.snapshot_path();

        assert!(
            snapshot_path.exists(),
            "Snapshot file not found: {}\n\
                 Run with UPDATE_SNAPSHOTS=1 to create it.\n\
                 Rendered output:\n{}",
            snapshot_path.display(),
            rendered
        );

        let expected = fs::read_to_string(&snapshot_path).expect("Failed to read snapshot file");

        // Normalize line endings
        let expected_normalized = expected.replace("\r\n", "\n");
        let rendered_normalized = rendered.replace("\r\n", "\n");

        if expected_normalized != rendered_normalized {
            // Print diff for debugging
            eprintln!("\n=== SNAPSHOT MISMATCH ===");
            eprintln!("Snapshot file: {}", snapshot_path.display());
            eprintln!("\n=== EXPECTED ===");
            eprintln!("{expected_normalized}");
            eprintln!("\n=== ACTUAL ===");
            eprintln!("{rendered_normalized}");
            eprintln!("\n=== DIFF ===");
            print_diff(&expected_normalized, &rendered_normalized);
            eprintln!("\nTo update this snapshot, run:");
            eprintln!(
                "  UPDATE_SNAPSHOTS=1 cargo test --test snapshot_framework -- {}",
                self.name
            );
            panic!("Snapshot mismatch! See stderr output above for details.");
        }
    }
}

/// Helper to convert terminal buffer to lines
fn buffer_to_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    let width = buffer.area().width as usize;
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    content
        .chars()
        .collect::<Vec<_>>()
        .chunks(width)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

/// Print a simple diff between two strings
fn print_diff(expected: &str, actual: &str) {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    for line_idx in 0..expected_lines.len().max(actual_lines.len()) {
        let exp_line = expected_lines.get(line_idx).unwrap_or(&"");
        let act_line = actual_lines.get(line_idx).unwrap_or(&"");

        if exp_line != act_line {
            println!("Line {} differs:", line_idx + 1);
            println!("  Expected: {exp_line}");
            println!("  Actual  : {act_line}");
        }
    }
}

// ============================================================================
// Snapshot Tests
// ============================================================================

#[test]
fn test_empty_ui_snapshot() {
    let test = SnapshotTest::new("empty_ui").with_terminal_size(80, 24);
    test.assert_snapshot_matches();
}

#[test]
fn test_single_running_task_snapshot() {
    let task_id = TaskId::default();
    let task = TaskDisplay {
        description: "Build project".to_string(),
        status: TaskStatus::Running,
        start_time: Instant::now(),
        end_time: None,
        parent_id: None,
        progress: None,
        output_lines: vec!["Compiling merlin-core v0.1.0".to_string()],
        output_tree: OutputTree::default(),
        steps: vec![],
    };

    let test = SnapshotTest::new("single_running_task")
        .with_terminal_size(80, 24)
        .with_task(task_id, task)
        .with_active_task(task_id)
        .with_running_task(task_id);

    test.assert_snapshot_matches();
}

#[test]
fn test_task_with_progress_snapshot() {
    let task_id = TaskId::default();
    let task = TaskDisplay {
        description: "Running tests".to_string(),
        status: TaskStatus::Running,
        start_time: Instant::now(),
        end_time: None,
        parent_id: None,
        progress: Some(TaskProgress {
            stage: "Testing".to_string(),
            current: 42,
            total: Some(100),
            message: "Running unit tests...".to_string(),
        }),
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    };

    let test = SnapshotTest::new("task_with_progress")
        .with_terminal_size(80, 24)
        .with_task(task_id, task)
        .with_active_task(task_id)
        .with_running_task(task_id);

    test.assert_snapshot_matches();
}

#[test]
fn test_multiple_task_statuses_snapshot() {
    let mut test = SnapshotTest::new("multiple_task_statuses").with_terminal_size(80, 24);

    // Running task
    let running_id = TaskId::default();
    test = test
        .with_task(running_id, create_test_task("Running task"))
        .with_running_task(running_id);

    // Completed task
    let completed_id = TaskId::default();
    test = test.with_task(completed_id, create_completed_task("Completed task"));

    // Failed task
    let failed_id = TaskId::default();
    test = test.with_task(failed_id, create_failed_task("Failed task"));

    // Set active task
    test = test.with_active_task(running_id);

    test.assert_snapshot_matches();
}

#[test]
fn test_hierarchical_tasks_snapshot() {
    let parent_id = TaskId::default();
    let child1_id = TaskId::default();
    let child2_id = TaskId::default();

    let test = SnapshotTest::new("hierarchical_tasks")
        .with_terminal_size(80, 24)
        .with_task(parent_id, create_test_task("Parent task"))
        .with_task(child1_id, create_child_task("Child task 1", parent_id))
        .with_task(child2_id, create_child_task("Child task 2", parent_id))
        .with_running_task(parent_id)
        .with_running_task(child1_id)
        .with_running_task(child2_id)
        .with_active_task(parent_id);

    test.assert_snapshot_matches();
}

#[test]
fn test_pending_tasks_section_snapshot() {
    let active_id = TaskId::default();
    let pending1_id = TaskId::default();
    let pending2_id = TaskId::default();

    let test = SnapshotTest::new("pending_tasks_section")
        .with_terminal_size(80, 24)
        .with_task(active_id, create_test_task("Active task"))
        .with_task(pending1_id, create_test_task("Pending task 1"))
        .with_task(pending2_id, create_test_task("Pending task 2"))
        .with_running_task(active_id)
        .with_running_task(pending1_id)
        .with_running_task(pending2_id)
        .with_active_task(active_id);

    test.assert_snapshot_matches();
}

#[test]
fn test_input_with_text_snapshot() {
    let test = SnapshotTest::new("input_with_text")
        .with_terminal_size(80, 24)
        .with_focused(FocusedPane::Input)
        .with_input_text("Fix the bug in the authentication module");

    test.assert_snapshot_matches();
}
