//! Common test utilities and helpers for merlin-routing tests
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
use merlin_core::{Response, TokenUsage};
use merlin_routing::user_interface::event_source::InputEventSource;
use merlin_routing::user_interface::output_tree::OutputTree;
use merlin_routing::user_interface::task_manager::{TaskDisplay, TaskStatus};
use merlin_routing::user_interface::{TuiApp, UiChannel};
use merlin_routing::{Result, TaskId, TaskResult, ValidationResult};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::collections::VecDeque;
use std::env;
use std::sync::Once;
use std::time::{Duration, Instant};
use tracing_subscriber::{EnvFilter, fmt};

/// Create a basic test task with sensible defaults
pub fn create_test_task(desc: &str) -> TaskDisplay {
    create_test_task_with_time(desc, Instant::now())
}

// ----------------------------------------------------------------------------
// Tracing initialization for tests
// ----------------------------------------------------------------------------

static TRACING_INIT: Once = Once::new();

/// Initialize tracing for tests (idempotent).
/// Honors `RUST_LOG` if set, otherwise defaults to "trace" for rich diagnostics.
pub fn init_tracing() {
    TRACING_INIT.call_once(|| {
        let filter = env::var("RUST_LOG").unwrap_or_else(|_| "trace".to_string());
        // Consume the result to avoid must_use warnings; if already initialized, ignore error
        if fmt()
            .with_env_filter(EnvFilter::new(filter))
            .with_test_writer()
            .try_init()
            .is_err()
        {
            // tracing already initialized in this process
        }
    });
}

/// Create a test task with a specific start time
pub fn create_test_task_with_time(desc: &str, start: Instant) -> TaskDisplay {
    TaskDisplay {
        description: desc.to_string(),
        status: TaskStatus::Running,
        start_time: start,
        end_time: None,
        parent_id: None,
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
        current_step: None,
    }
}

/// Create a child task with a parent
pub fn create_child_task(desc: &str, parent_id: TaskId) -> TaskDisplay {
    TaskDisplay {
        description: desc.to_string(),
        status: TaskStatus::Running,
        start_time: Instant::now(),
        end_time: None,
        parent_id: Some(parent_id),
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
        current_step: None,
    }
}

/// Create a completed task
pub fn create_completed_task(desc: &str) -> TaskDisplay {
    let start = Instant::now();
    TaskDisplay {
        description: desc.to_string(),
        status: TaskStatus::Completed,
        start_time: start,
        end_time: Some(start),
        parent_id: None,
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
        current_step: None,
    }
}

/// Create a failed task
pub fn create_failed_task(desc: &str) -> TaskDisplay {
    let start = Instant::now();
    TaskDisplay {
        description: desc.to_string(),
        status: TaskStatus::Failed,
        start_time: start,
        end_time: Some(start),
        parent_id: None,
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
        current_step: None,
    }
}

/// Create a test task result
pub fn create_test_task_result(task_id: TaskId, text: &str) -> TaskResult {
    TaskResult {
        task_id,
        response: Response {
            text: text.to_string(),
            confidence: 0.95,
            tokens_used: TokenUsage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
            },
            provider: "test".to_string(),
            latency_ms: 1000,
        },
        tier_used: "local".to_string(),
        tokens_used: TokenUsage {
            input: 100,
            output: 50,
            cache_read: 0,
            cache_write: 0,
        },
        validation: ValidationResult {
            passed: true,
            score: 1.0,
            errors: vec![],
            warnings: vec![],
            stages: vec![],
        },
        duration_ms: 1000,
    }
}

// ============================================================================
// TUI Testing Utilities
// ============================================================================

/// Test event source that provides events from a queue
#[derive(Default)]
pub struct TestEventSource {
    queue: VecDeque<Event>,
}

impl TestEventSource {
    /// Creates a new test event source with events
    pub fn with_events(events: impl IntoIterator<Item = Event>) -> Self {
        Self {
            queue: events.into_iter().collect(),
        }
    }

    /// Enqueues a single event
    pub fn enqueue(&mut self, event: Event) {
        self.queue.push_back(event);
    }
}

impl InputEventSource for TestEventSource {
    fn poll(&mut self, _timeout: Duration) -> bool {
        // In tests, return immediately based on queue state
        // Don't sleep - we want tests to complete quickly
        !self.queue.is_empty()
    }

    fn read(&mut self) -> Event {
        // In tests, if there are no events, return a dummy event
        // This should never happen in practice since poll() is called first
        self.queue
            .pop_front()
            .unwrap_or_else(|| Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)))
    }
}

/// Type alias for test app creation result
type TestAppResult = Result<(TuiApp<TestBackend>, UiChannel)>;

/// Creates a `TuiApp` with `TestBackend` for snapshot testing
///
/// # Errors
/// Returns an error if `TuiApp` initialization fails
pub fn create_test_app(width: u16, height: u16) -> TestAppResult {
    use merlin_routing::RoutingError;

    let backend = TestBackend::new(width, height);
    let terminal = Terminal::new(backend)
        .map_err(|err| RoutingError::Other(format!("Failed to create terminal: {err}")))?;
    TuiApp::new_for_test(terminal)
}

/// Simulates typing text character by character
pub fn simulate_typing(text: &str) -> TestEventSource {
    let events = text
        .chars()
        .map(|char_value| Event::Key(KeyEvent::new(KeyCode::Char(char_value), KeyModifiers::NONE)));
    TestEventSource::with_events(events)
}

// ============================================================================
// Input Testing Helpers
// ============================================================================

/// Test helper that creates an app, applies events, and returns the app for further inspection
pub fn test_with_events(events: impl IntoIterator<Item = Event>) -> TuiApp<TestBackend> {
    let (mut app, _) = create_test_app(80, 24).expect("Failed to create app");
    let source = TestEventSource::with_events(events);
    app.set_event_source(Box::new(source));
    if let Err(error) = app.tick() {
        panic!("tick failed: {error}");
    }
    app
}

/// Batch size for processing events to avoid buffer overflow
const BATCH_SIZE: usize = 50;

/// Test helper that creates an app, types text, and returns the app
/// Processes events in batches to avoid buffer overflow on large inputs
pub fn test_with_typing(text: &str) -> TuiApp<TestBackend> {
    let (mut app, _) = create_test_app(80, 24).expect("Failed to create app");

    // Process in batches to avoid overwhelming the event buffer
    for chunk in text.chars().collect::<Vec<_>>().chunks(BATCH_SIZE) {
        let events: Vec<Event> = chunk
            .iter()
            .map(|&char_value| {
                Event::Key(KeyEvent::new(KeyCode::Char(char_value), KeyModifiers::NONE))
            })
            .collect();

        let source = TestEventSource::with_events(events);
        app.set_event_source(Box::new(source));
        if let Err(error) = app.tick() {
            panic!("tick failed: {error}");
        }
    }

    app
}
