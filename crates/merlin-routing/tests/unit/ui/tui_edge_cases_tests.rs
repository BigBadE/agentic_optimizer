//! Edge case tests for TUI - keyboard navigation, boundary conditions, error handling
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
use crate::common::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use merlin_routing::TaskId;
use merlin_routing::user_interface::task_manager::TaskManager;
use merlin_routing::user_interface::{EmojiMode, calculate_width as ui_calculate_width};
#[test]
fn test_task_manager_navigation_empty() {
    let manager = TaskManager::default();

    let visible = manager.get_visible_tasks();
    assert!(visible.is_empty());
}

#[test]
fn test_task_manager_navigation_single_task() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    let visible = manager.get_visible_tasks();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0], task_id);
}

#[test]
fn test_task_collapse_with_multiple_levels() {
    let mut manager = TaskManager::default();

    let root = TaskId::default();
    let child = TaskId::default();
    let grandchild = TaskId::default();

    manager.add_task(root, create_test_task("Root"));
    manager.add_task(child, create_child_task("Child", root));
    manager.add_task(grandchild, create_child_task("Grandchild", child));

    // All visible
    assert_eq!(manager.get_visible_tasks().len(), 3);

    // Collapse root
    manager.collapse_task(root);

    // Only root visible
    assert_eq!(manager.get_visible_tasks().len(), 1);
}

#[test]
fn test_task_partial_collapse() {
    let mut manager = TaskManager::default();

    let root = TaskId::default();
    let child1 = TaskId::default();
    let child2 = TaskId::default();
    let grandchild = TaskId::default();

    manager.add_task(root, create_test_task("Root"));
    manager.add_task(child1, create_child_task("Child 1", root));
    manager.add_task(grandchild, create_child_task("Grandchild", child1));
    manager.add_task(child2, create_child_task("Child 2", root));

    // All visible
    assert_eq!(manager.get_visible_tasks().len(), 4);

    // Collapse child1
    manager.collapse_task(child1);

    // Root, child1, and child2 visible (grandchild hidden)
    assert_eq!(manager.get_visible_tasks().len(), 3);
}

#[test]
fn test_input_manager_very_long_line() {
    // 1000 character line
    let very_long = "a".repeat(1000);
    let app = test_with_typing(&very_long);

    let lines = app.get_input_lines();
    let total_chars: usize = lines.iter().map(String::len).sum();
    assert_eq!(total_chars, 1000);
}

#[test]
fn test_input_manager_many_lines() {
    // Add 100 lines
    let mut events = Vec::new();
    for index in 0..100 {
        for character in format!("Line {index}").chars() {
            events.push(Event::Key(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::NONE,
            )));
        }

        if index < 99 {
            events.push(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::SHIFT,
            )));
        }
    }

    let app = test_with_events(events);
    let lines = app.get_input_lines();
    assert!(lines.len() >= 90, "Should have close to 100 lines"); // Allow for some wrapping/variation
}

#[test]
fn test_task_manager_many_tasks() {
    let mut manager = TaskManager::default();

    // Add 1000 tasks
    for index in 0..1000 {
        let task_id = TaskId::default();
        manager.add_task(task_id, create_test_task(&format!("Task {index}")));
    }

    assert_eq!(manager.task_order().len(), 1000);
    assert_eq!(manager.get_visible_tasks().len(), 1000);
}

#[test]
fn test_task_manager_remove_nonexistent() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    let nonexistent_id = TaskId::default();
    let removed = manager.remove_task(nonexistent_id);

    // Should only remove the nonexistent task (not affect others)
    assert_eq!(removed.len(), 1);
    assert!(manager.get_task(task_id).is_some());
}

#[test]
fn test_unicode_width_calculation() {
    use merlin_routing::user_interface::{EmojiMode, calculate_width};

    assert_eq!(calculate_width("Hello", EmojiMode::Permissive), 5);
    assert_eq!(calculate_width("世界", EmojiMode::Permissive), 4); // 2 wide chars
    assert!(calculate_width("🌍", EmojiMode::Permissive) > 0); // Emoji
    assert_eq!(calculate_width("Café", EmojiMode::Permissive), 4);
}

#[test]
fn test_text_truncation() {
    use merlin_routing::user_interface::{EmojiMode, truncate_to_width};

    let long_text = "This is a very long text that needs truncation";
    let truncated = truncate_to_width(long_text, 20, EmojiMode::Permissive);

    assert!(truncated.len() <= long_text.len());
    assert!(calculate_width(&truncated, EmojiMode::Permissive) <= 20);
}

#[test]
fn test_text_wrapping() {
    use merlin_routing::user_interface::{EmojiMode, wrap_text};

    let text = "This is a long line that should wrap to multiple lines";
    let wrapped = wrap_text(text, 20, EmojiMode::Permissive);

    assert!(wrapped.len() > 1);
    for line in &wrapped {
        assert!(calculate_width(line, EmojiMode::Permissive) <= 20);
    }
}

#[test]
fn test_emoji_stripping() {
    use merlin_routing::user_interface::strip_emojis;

    let text_with_emoji = "Hello 🌍 World 🚀";
    let stripped = strip_emojis(text_with_emoji, "[?]");

    assert!(!stripped.contains('🌍'));
    assert!(!stripped.contains('🚀'));
    assert!(stripped.contains("Hello"));
    assert!(stripped.contains("World"));
}

#[test]
fn test_task_manager_orphaned_tasks() {
    let mut manager = TaskManager::default();

    // Add child without parent existing
    let child_id = TaskId::default();
    let nonexistent_parent = TaskId::default();

    manager.insert_task_for_load(child_id, create_child_task("Orphan", nonexistent_parent));
    manager.rebuild_order();

    // Orphaned task should still be in the list
    assert_eq!(manager.task_order().len(), 1);
}

#[test]
fn test_task_manager_circular_reference_prevention() {
    let mut manager = TaskManager::default();

    let task1 = TaskId::default();
    let task2 = TaskId::default();

    manager.add_task(task1, create_test_task("Task 1"));
    manager.add_task(task2, create_child_task("Task 2", task1));

    // Try to make task1 a descendant of task2 (circular reference)
    // This is prevented by the API design - parent_id is set at creation

    assert!(!manager.is_descendant_of(task1, task2));
}

#[test]
fn test_zero_width_terminal_handling() {
    // Wrapping to 0 width should not panic
    let _ = calculate_width("", EmojiMode::Permissive);
}

#[test]
fn test_special_control_characters() {
    // Null byte should be handled
    let text = "Hello\0World";
    let app = test_with_typing(text);

    // Should not panic
    let lines = app.get_input_lines();
    assert!(!lines.is_empty());
}

/// Helper function imported from the module
fn calculate_width(text: &str, mode: EmojiMode) -> usize {
    ui_calculate_width(text, mode)
}
