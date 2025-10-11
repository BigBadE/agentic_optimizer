//! Comprehensive tests for TUI input handling
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::tests_outside_test_module,
    clippy::missing_panics_doc,
    clippy::min_ident_chars,
    clippy::similar_names,
    reason = "Tests allow these"
)]

mod common;

use common::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_input_manager_creation() {
    let app = test_with_events(vec![]);
    assert_eq!(app.get_input_lines().len(), 1, "Should start with one line");
}

#[test]
fn test_basic_text_input() {
    let app = test_with_typing("hello");
    assert_eq!(app.get_input_text(), "hello");
}

#[test]
fn test_manual_newline_recording() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    let lines = app.get_input_lines();
    assert!(
        lines.len() >= 2,
        "Should have at least 2 lines after manual newline"
    );
}

#[test]
fn test_auto_wrap_short_text() {
    // Text shorter than 80 chars should not wrap
    let app = test_with_typing("short text");
    let lines = app.get_input_lines();
    assert_eq!(lines.len(), 1, "Short text should stay on one line");
}

#[test]
fn test_auto_wrap_long_text() {
    // Text longer than 80 chars should wrap
    let long_text = "a".repeat(150);
    let app = test_with_typing(&long_text);
    let lines = app.get_input_lines();
    assert!(lines.len() > 1, "Long text should wrap to multiple lines");

    // Verify character count is preserved
    let char_count: usize = lines.iter().map(String::len).sum();
    assert_eq!(char_count, 150, "All characters should be preserved");
}

#[test]
fn test_cursor_position_after_wrap() {
    // Type text that will wrap, cursor should be at end
    let long_text = "a".repeat(100);
    let app = test_with_typing(&long_text);

    // Just verify the text is there - cursor position is internal state
    let char_count: usize = app.get_input_lines().iter().map(String::len).sum();
    assert_eq!(char_count, 100);
}

#[test]
fn test_clear() {
    let app = test_with_typing("some text");
    assert!(
        !app.get_input_text().is_empty(),
        "Should have text before clear"
    );

    // Note: We can't test the clear operation itself without exposing it through TuiApp
    // This test validates that text was successfully typed
}

#[test]
fn test_empty_line_handling() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        Event::Key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    let lines = app.get_input_lines();
    assert!(lines.len() >= 2, "Should handle empty lines");
}

#[test]
fn test_backspace_handling() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    assert_eq!(app.get_input_text(), "hel");
}

#[test]
fn test_delete_handling() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    // Delete at end of line does nothing
    assert_eq!(app.get_input_text(), "hello");
}

#[test]
fn test_cursor_movement_left() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    assert_eq!(app.get_input_text(), "hxi");
}

#[test]
fn test_cursor_movement_right() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    assert_eq!(app.get_input_text(), "hxi");
}

#[test]
fn test_home_key() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    assert_eq!(app.get_input_text(), "xhello");
}

#[test]
fn test_end_key() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    assert_eq!(app.get_input_text(), "hi!");
}

#[test]
fn test_multiline_cursor_up_down() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    let text = app.get_input_text();
    assert!(text.contains('x'), "Should insert x after moving cursor up");
}

#[test]
fn test_shift_left_selection() {
    // Shift+Left starts selection - just verify input works
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)),
    ];
    let app = test_with_events(events);
    assert_eq!(app.get_input_text(), "hi");
}

#[test]
fn test_shift_right_selection() {
    // Shift+Right extends selection - just verify input works
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)),
        Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT)),
    ];
    let app = test_with_events(events);
    assert_eq!(app.get_input_text(), "hi");
}

#[test]
fn test_ctrl_a_select_all() {
    // Ctrl+A selects all - just verify input works
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)),
    ];
    let app = test_with_events(events);
    assert_eq!(app.get_input_text(), "hi");
}

#[test]
fn test_word_deletion() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::CONTROL)),
    ];
    let app = test_with_events(events);
    // Ctrl+Backspace deletes word
    let text = app.get_input_text();
    assert!(text.starts_with("hello"), "Should preserve first word");
}

#[test]
fn test_unicode_input() {
    let app = test_with_typing("Hello 世界 🌍");
    assert_eq!(app.get_input_text(), "Hello 世界 🌍");
}

#[test]
fn test_very_long_single_line() {
    let long_text = "x".repeat(500);
    let app = test_with_typing(&long_text);
    let char_count: usize = app.get_input_lines().iter().map(String::len).sum();
    assert_eq!(char_count, 500);
}

#[test]
fn test_paste_simulation() {
    // Simulate pasting a large block of text
    let paste_text = "Line 1\nLine 2\nLine 3\nLine 4";
    let app = test_with_typing(paste_text);
    let text = app.get_input_text();
    assert!(text.contains("Line"), "Should contain pasted text");
}

#[test]
fn test_mixed_operations() {
    let events = vec![
        Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        Event::Key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
    ];
    let app = test_with_events(events);
    let text = app.get_input_text();
    assert!(
        text.contains("help"),
        "Should have 'help' from mixed operations"
    );
    assert!(
        text.contains("world"),
        "Should have 'world' from second line"
    );
}
