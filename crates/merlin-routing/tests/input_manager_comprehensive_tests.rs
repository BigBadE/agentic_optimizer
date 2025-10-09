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

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use merlin_routing::user_interface::input::InputManager;
use tui_textarea::{CursorMove, Input};

// Helper to create Input from KeyCode
fn key_input(code: KeyCode) -> Input {
    Input::from(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn test_input_manager_creation() {
    let manager = InputManager::default();
    assert_eq!(
        manager.input_area().lines(),
        &[""],
        "Should start with empty line"
    );
}

#[test]
fn test_basic_text_input() {
    let mut manager = InputManager::default();

    // Type "hello"
    for ch in "hello".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    assert_eq!(manager.input_area().lines()[0], "hello");
}

#[test]
fn test_manual_newline_recording() {
    let mut manager = InputManager::default();

    // Type "line1" and press Enter
    for ch in "line1".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    manager.input_area_mut().input(key_input(KeyCode::Enter));
    manager.record_manual_newline();

    // Type "line2"
    for ch in "line2".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    assert_eq!(manager.input_area().lines().len(), 2);
    assert_eq!(manager.input_area().lines()[0], "line1");
    assert_eq!(manager.input_area().lines()[1], "line2");
}

#[test]
fn test_auto_wrap_short_text() {
    let mut manager = InputManager::default();

    // Type short text that doesn't need wrapping
    for ch in "short".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    let lines_before = manager.input_area().lines().to_vec();

    // Auto-wrap with generous width
    manager.auto_wrap(80);

    // Should not change anything
    assert_eq!(manager.input_area().lines(), lines_before.as_slice());
}

#[test]
fn test_auto_wrap_long_text() {
    let mut manager = InputManager::default();

    // Type a long line
    let long_text = "This is a very long line of text that definitely needs to be wrapped when the width is constrained";
    for ch in long_text.chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Wrap with narrow width
    manager.auto_wrap(30);

    // Should have multiple lines now
    assert!(
        manager.input_area().lines().len() > 1,
        "Text should be wrapped into multiple lines"
    );

    // Join lines and verify content is preserved (minus spaces from wrapping)
    let joined = manager.input_area().lines().join(" ");
    assert!(joined.contains("This is a very long line"));
}

#[test]
fn test_cursor_position_after_wrap() {
    let mut manager = InputManager::default();

    // Type text and position cursor in middle
    for ch in "word1 word2 word3 word4 word5".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Move cursor to middle
    manager
        .input_area_mut()
        .move_cursor(CursorMove::Jump(0, 15));

    // Wrap
    manager.auto_wrap(20);

    // Cursor should still be valid
    let (row, col) = manager.input_area().cursor();
    assert!(row < manager.input_area().lines().len());
    assert!(col <= manager.input_area().lines()[row].len());
}

#[test]
fn test_clear() {
    let mut manager = InputManager::default();

    // Add content
    for ch in "some text".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Clear
    manager.clear();

    assert_eq!(
        manager.input_area().lines(),
        &[""],
        "Should be cleared to empty line"
    );
    assert_eq!(
        manager.input_area().cursor(),
        (0, 0),
        "Cursor should be at origin"
    );
}

#[test]
fn test_empty_line_handling() {
    let mut manager = InputManager::default();

    // Type line1, Enter, Enter, line3
    for ch in "line1".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }
    manager.input_area_mut().input(key_input(KeyCode::Enter));
    manager.input_area_mut().input(key_input(KeyCode::Enter));

    for ch in "line3".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    assert_eq!(manager.input_area().lines().len(), 3);
    assert_eq!(manager.input_area().lines()[0], "line1");
    assert_eq!(manager.input_area().lines()[1], "");
    assert_eq!(manager.input_area().lines()[2], "line3");
}

#[test]
fn test_backspace_handling() {
    let mut manager = InputManager::default();

    // Type "hello"
    for ch in "hello".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Backspace twice
    manager
        .input_area_mut()
        .input(key_input(KeyCode::Backspace));
    manager
        .input_area_mut()
        .input(key_input(KeyCode::Backspace));

    assert_eq!(manager.input_area().lines()[0], "hel");
}

#[test]
fn test_delete_handling() {
    let mut manager = InputManager::default();

    // Type "hello"
    for ch in "hello".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Move cursor to start
    manager.input_area_mut().move_cursor(CursorMove::Head);

    // Delete twice
    manager.input_area_mut().input(key_input(KeyCode::Delete));
    manager.input_area_mut().input(key_input(KeyCode::Delete));

    assert_eq!(manager.input_area().lines()[0], "llo");
}

#[test]
fn test_cursor_movement_left() {
    let mut manager = InputManager::default();

    // Type "hello"
    for ch in "hello".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Move left 2 positions
    manager.input_area_mut().input(key_input(KeyCode::Left));
    manager.input_area_mut().input(key_input(KeyCode::Left));

    let (_, col) = manager.input_area().cursor();
    assert_eq!(col, 3);
}

#[test]
fn test_cursor_movement_right() {
    let mut manager = InputManager::default();

    // Type "hello"
    for ch in "hello".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Move to start
    manager.input_area_mut().move_cursor(CursorMove::Head);

    // Move right 2 positions
    manager.input_area_mut().input(key_input(KeyCode::Right));
    manager.input_area_mut().input(key_input(KeyCode::Right));

    let (_, col) = manager.input_area().cursor();
    assert_eq!(col, 2);
}

#[test]
fn test_home_key() {
    let mut manager = InputManager::default();

    // Type "hello"
    for ch in "hello".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Press Home
    manager.input_area_mut().input(key_input(KeyCode::Home));

    let (_, col) = manager.input_area().cursor();
    assert_eq!(col, 0);
}

#[test]
fn test_end_key() {
    let mut manager = InputManager::default();

    // Type "hello"
    for ch in "hello".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Move to start
    manager.input_area_mut().move_cursor(CursorMove::Head);

    // Press End
    manager.input_area_mut().input(key_input(KeyCode::End));

    let (_, col) = manager.input_area().cursor();
    assert_eq!(col, 5);
}

#[test]
fn test_multiline_cursor_up_down() {
    let mut manager = InputManager::default();

    // Type multiline text
    for ch in "line1".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }
    manager.input_area_mut().input(key_input(KeyCode::Enter));
    for ch in "line2".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    let (row_before, _) = manager.input_area().cursor();
    assert_eq!(row_before, 1, "Should be on second line");

    // Move up
    manager.input_area_mut().input(key_input(KeyCode::Up));

    let (row_after, _) = manager.input_area().cursor();
    assert_eq!(row_after, 0, "Should be on first line");
}

#[test]
fn test_wrap_preserves_empty_paragraphs() {
    let mut manager = InputManager::default();

    // Create text with empty line
    for ch in "para1".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }
    manager.input_area_mut().input(key_input(KeyCode::Enter));
    manager.record_manual_newline();
    manager.input_area_mut().input(key_input(KeyCode::Enter));

    for ch in "para2".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Wrap
    manager.auto_wrap(80);

    // Should preserve structure
    assert!(manager.input_area().lines().len() >= 3);
}

#[test]
fn test_special_characters() {
    let mut manager = InputManager::default();

    // Type special characters
    let special = "Hello, World! @#$%^&*()";
    for ch in special.chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    assert_eq!(manager.input_area().lines()[0], special);
}

#[test]
fn test_unicode_characters() {
    let mut manager = InputManager::default();

    // Type unicode characters
    let unicode = "Hello 世界 🌍";
    for ch in unicode.chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    assert_eq!(manager.input_area().lines()[0], unicode);
}

#[test]
fn test_very_long_word_wrapping() {
    let mut manager = InputManager::default();

    // Type a very long word (should break mid-word)
    let long_word = "a".repeat(100);
    for ch in long_word.chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Wrap with narrow width
    manager.auto_wrap(20);

    // Should have multiple lines
    assert!(manager.input_area().lines().len() > 1);

    // Total character count should be preserved
    let total_chars: usize = manager.input_area().lines().iter().map(String::len).sum();
    assert_eq!(total_chars, 100);
}

#[test]
fn test_wrap_idempotence() {
    let mut manager = InputManager::default();

    // Type text
    for ch in "This is some text that will be wrapped".chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Wrap once
    manager.auto_wrap(20);
    let after_first = manager.input_area().lines().to_vec();

    // Wrap again with same width
    manager.auto_wrap(20);
    let after_second = manager.input_area().lines().to_vec();

    // Should be identical
    assert_eq!(
        after_first, after_second,
        "Multiple wraps should be idempotent"
    );
}

#[test]
fn test_cursor_at_boundary_after_wrap() {
    let mut manager = InputManager::default();

    // Type exactly the width
    let text = "a".repeat(20);
    for ch in text.chars() {
        manager.input_area_mut().input(key_input(KeyCode::Char(ch)));
    }

    // Cursor should be at end
    let (row, col) = manager.input_area().cursor();
    assert_eq!(row, 0);
    assert_eq!(col, 20);

    // Wrap at exact width
    manager.auto_wrap(20);

    // Cursor should still be valid
    let (new_row, new_col) = manager.input_area().cursor();
    assert!(new_row < manager.input_area().lines().len());
    assert!(new_col <= manager.input_area().lines()[new_row].len());
}
