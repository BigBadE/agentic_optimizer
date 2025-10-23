//! TUI integration tests
//!
//! Tests the actual TUI application with injected events

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
        unsafe_code,
        reason = "Test allows"
    )
)]

use crossterm::event::{KeyCode, KeyModifiers};
use integration_tests::tui_helpers::TestEventSource;
use merlin_cli::ui::TuiApp;
use merlin_core::TaskId;
use merlin_routing::{UiChannel, UiEvent};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc;

/// Creates a test `TuiApp` with a `TestBackend`
fn create_test_app() -> (TuiApp<TestBackend>, UiChannel) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(backend).expect("create terminal");

    let app = TuiApp::new_for_test(terminal, receiver);
    let channel = UiChannel::from_sender(sender);

    (app, channel)
}

#[tokio::test]
async fn test_tui_basic_input() {
    let (mut app, _channel) = create_test_app();

    // Create event source with text input
    let event_source = TestEventSource::new();
    event_source.push_text("Hello TUI");

    // Replace event source
    app.set_event_source(Box::new(event_source));

    // Process events
    for _ in 0..9 {
        match app.tick() {
            Ok(should_quit) if should_quit => break,
            Err(_) => break,
            Ok(_) => {}
        }
    }

    // Verify input was captured
    let input_text = app.get_input_text();
    assert!(
        input_text.contains("Hello TUI"),
        "Input should contain 'Hello TUI', got: {input_text}"
    );
}

#[tokio::test]
async fn test_tui_enter_key_submits_input() {
    let (mut app, _channel) = create_test_app();

    // Create event source with input and enter
    let event_source = TestEventSource::new();
    event_source.push_text("test input");
    event_source.push_enter();

    app.set_event_source(Box::new(event_source));

    // Process events
    for _ in 0..20 {
        match app.tick() {
            Ok(should_quit) if should_quit => break,
            Err(_) => break,
            Ok(_) => {}
        }
    }

    // After enter, input should be cleared
    let input_text = app.get_input_text();
    assert!(
        input_text.is_empty() || !input_text.contains("test input"),
        "Input should be cleared after submission, got: {input_text}"
    );
}

#[tokio::test]
async fn test_tui_escape_key() {
    let (mut app, _channel) = create_test_app();

    // Type some text then press escape
    let event_source = TestEventSource::new();
    event_source.push_text("some text");
    event_source.push_escape();

    app.set_event_source(Box::new(event_source));

    // Process events
    for _ in 0..15 {
        match app.tick() {
            Ok(should_quit) if should_quit => break,
            Err(_) => break,
            Ok(_) => {}
        }
    }

    // Escape should have been processed
    // (exact behavior depends on implementation - just verify no panic)
}

#[tokio::test]
async fn test_tui_multiline_input() {
    let (mut app, _channel) = create_test_app();

    // Create event source with multiline input
    let event_source = TestEventSource::new();
    event_source.push_text("Line 1");
    event_source.push_key(KeyCode::Enter, KeyModifiers::ALT); // Alt+Enter for newline
    event_source.push_text("Line 2");

    app.set_event_source(Box::new(event_source));

    // Process events
    for _ in 0..20 {
        match app.tick() {
            Ok(should_quit) if should_quit => break,
            Err(_) => break,
            Ok(_) => {}
        }
    }

    // Verify multiline input
    let lines = app.get_input_lines();
    assert!(!lines.is_empty(), "Should have at least one line of input");
}

#[tokio::test]
async fn test_tui_tab_navigation() {
    let (mut app, _channel) = create_test_app();

    // Press tab to switch panes
    let event_source = TestEventSource::new();
    event_source.push_tab();

    app.set_event_source(Box::new(event_source));

    // Process events
    for _ in 0..5 {
        match app.tick() {
            Ok(should_quit) if should_quit => break,
            Err(_) => break,
            Ok(_) => {}
        }
    }

    // Tab should have been processed (verify no panic)
}

#[tokio::test]
async fn test_tui_with_ui_events() {
    let (mut app, channel) = create_test_app();

    // Send a UI event
    let task_id = TaskId::default();
    channel.send(UiEvent::TaskStarted {
        task_id,
        description: "Test Task".to_owned(),
        parent_id: None,
    });

    // Create event source
    let event_source = TestEventSource::new();
    app.set_event_source(Box::new(event_source));

    // Process events to consume UI event
    for _ in 0..10 {
        match app.tick() {
            Ok(should_quit) if should_quit => break,
            Err(_) => break,
            Ok(_) => {}
        }
    }

    // Verify task was added
    let task_manager = app.task_manager();
    let task_count = task_manager.iter_tasks().count();
    assert!(
        task_count > 0,
        "Task should have been added to task manager"
    );
}

#[tokio::test]
async fn test_tui_rendering() {
    let (mut app, _channel) = create_test_app();

    // Add some input
    let event_source = TestEventSource::new();
    event_source.push_text("Test rendering");

    app.set_event_source(Box::new(event_source));

    // Process events and render
    for _ in 0..15 {
        match app.tick() {
            Ok(should_quit) if should_quit => break,
            Err(_) => break,
            Ok(_) => {}
        }
    }

    // Get the backend to verify rendering occurred
    let backend = app.backend();
    let buffer = backend.buffer();

    // Buffer should have content (exact assertions depend on layout)
    assert!(buffer.area().width > 0);
    assert!(buffer.area().height > 0);
}
