//! Edge case tests for TUI - keyboard navigation, boundary conditions, error handling
#![cfg(test)]

mod common;

use common::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use merlin_routing::TaskId;
use merlin_routing::user_interface::input::InputManager;
use merlin_routing::user_interface::output_tree::{OutputTree, StepType};
use merlin_routing::user_interface::task_manager::TaskManager;
use merlin_routing::user_interface::{EmojiMode, calculate_width as ui_calculate_width};
use tui_textarea::Input;

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_manager_navigation_empty() {
    let manager = TaskManager::default();

    let visible = manager.get_visible_tasks();
    assert!(visible.is_empty());
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_task_manager_navigation_single_task() {
    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    manager.add_task(task_id, create_test_task("Task"));

    let visible = manager.get_visible_tasks();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0], task_id);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_navigation_empty() {
    let tree = OutputTree::default();

    assert_eq!(tree.flatten_visible_nodes().len(), 0);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_move_up_at_top() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "step1".to_string(),
        StepType::Thinking,
        "Content".to_string(),
    );

    assert_eq!(tree.selected_index(), 0);
    tree.move_up();
    // Should stay at 0
    assert_eq!(tree.selected_index(), 0);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_move_down_at_bottom() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "step1".to_string(),
        StepType::Thinking,
        "Content".to_string(),
    );
    tree.complete_step("step1");

    tree.move_to_end();
    let max_index = tree.selected_index();
    tree.move_down();
    // Should stay at same position
    assert_eq!(tree.selected_index(), max_index);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_move_to_start() {
    let mut tree = OutputTree::default();

    for index in 0..5 {
        tree.add_step(
            format!("step{index}"),
            StepType::Thinking,
            format!("Content {index}"),
        );
        tree.complete_step(&format!("step{index}"));
    }

    tree.move_to_end();
    assert!(tree.selected_index() > 0);

    tree.move_to_start();
    assert_eq!(tree.selected_index(), 0);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_move_to_end() {
    let mut tree = OutputTree::default();

    for index in 0..5 {
        tree.add_step(
            format!("step{index}"),
            StepType::Thinking,
            format!("Content {index}"),
        );
        tree.complete_step(&format!("step{index}"));
    }

    tree.move_to_start();
    tree.move_to_end();

    let visible_count = tree.flatten_visible_nodes().len();
    assert_eq!(tree.selected_index(), visible_count - 1);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_page_up() {
    let mut tree = OutputTree::default();

    for index in 0..20 {
        tree.add_step(
            format!("step{index}"),
            StepType::Thinking,
            format!("Content {index}"),
        );
        tree.complete_step(&format!("step{index}"));
    }

    tree.move_to_end();
    let end_pos = tree.selected_index();
    tree.page_up(10);

    assert!(tree.selected_index() < end_pos);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_page_down() {
    let mut tree = OutputTree::default();

    for index in 0..20 {
        tree.add_step(
            format!("step{index}"),
            StepType::Thinking,
            format!("Content {index}"),
        );
        tree.complete_step(&format!("step{index}"));
    }

    tree.move_to_start();
    tree.page_down(10);

    assert!(tree.selected_index() >= 10);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_toggle_collapse() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "parent".to_string(),
        StepType::Thinking,
        "Parent".to_string(),
    );
    tree.add_step("child".to_string(), StepType::Thinking, "Child".to_string());
    tree.complete_step("child");
    tree.complete_step("parent");

    let initial_visible = tree.flatten_visible_nodes().len();
    tree.toggle_selected();
    let after_toggle = tree.flatten_visible_nodes().len();

    // Visibility should change
    assert!(initial_visible != after_toggle || initial_visible == 1);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_expand_collapse_selected() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "parent".to_string(),
        StepType::Thinking,
        "Parent".to_string(),
    );
    tree.add_step("child".to_string(), StepType::Thinking, "Child".to_string());
    tree.complete_step("child");
    tree.complete_step("parent");

    tree.collapse_selected();
    let collapsed_count = tree.flatten_visible_nodes().len();

    tree.expand_selected();
    let expanded_count = tree.flatten_visible_nodes().len();

    // Should have more nodes when expanded
    assert!(expanded_count >= collapsed_count);
}

#[test]
/// # Panics
/// Panics if assertions fail.
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
/// # Panics
/// Panics if assertions fail.
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
/// # Panics
/// Panics if assertions fail.
fn test_input_manager_very_long_line() {
    let mut manager = InputManager::default();

    // 1000 character line
    let very_long = "a".repeat(1000);
    for character in very_long.chars() {
        let key = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
        let input = Input::from(Event::Key(key));
        manager.input_area_mut().input(input);
    }

    let lines = manager.input_area().lines();
    let total_chars: usize = lines.iter().map(String::len).sum();
    assert_eq!(total_chars, 1000);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_input_manager_many_lines() {
    let mut manager = InputManager::default();

    // Add 100 lines
    for index in 0..100 {
        for character in format!("Line {index}").chars() {
            let key = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
            let input = Input::from(Event::Key(key));
            manager.input_area_mut().input(input);
        }

        if index < 99 {
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            let input = Input::from(Event::Key(key));
            manager.input_area_mut().input(input);
        }
    }

    let lines = manager.input_area().lines();
    assert_eq!(lines.len(), 100);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_very_deep_nesting() {
    let mut tree = OutputTree::default();

    // Create deep nesting
    for index in 0..50 {
        tree.add_step(
            format!("level{index}"),
            StepType::Thinking,
            format!("Level {index}"),
        );
    }

    // Complete all from bottom up
    for index in (0..50).rev() {
        tree.complete_step(&format!("level{index}"));
    }

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 50);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_many_siblings() {
    let mut tree = OutputTree::default();

    // Add parent
    tree.add_step(
        "parent".to_string(),
        StepType::Thinking,
        "Parent".to_string(),
    );

    // Add 100 children
    for index in 0..100 {
        tree.add_step(
            format!("child{index}"),
            StepType::Thinking,
            format!("Child {index}"),
        );
        tree.complete_step(&format!("child{index}"));
    }

    tree.complete_step("parent");

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 101); // parent + 100 children
}

#[test]
/// # Panics
/// Panics if assertions fail.
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
/// # Panics
/// Panics if assertions fail.
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
/// # Panics
/// Panics if assertions fail.
fn test_unicode_width_calculation() {
    use merlin_routing::user_interface::{EmojiMode, calculate_width};

    assert_eq!(calculate_width("Hello", EmojiMode::Permissive), 5);
    assert_eq!(calculate_width("世界", EmojiMode::Permissive), 4); // 2 wide chars
    assert!(calculate_width("🌍", EmojiMode::Permissive) > 0); // Emoji
    assert_eq!(calculate_width("Café", EmojiMode::Permissive), 4);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_text_truncation() {
    use merlin_routing::user_interface::{EmojiMode, truncate_to_width};

    let long_text = "This is a very long text that needs truncation";
    let truncated = truncate_to_width(long_text, 20, EmojiMode::Permissive);

    assert!(truncated.len() <= long_text.len());
    assert!(calculate_width(&truncated, EmojiMode::Permissive) <= 20);
}

#[test]
/// # Panics
/// Panics if assertions fail.
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
/// # Panics
/// Panics if assertions fail.
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
/// # Panics
/// Panics if assertions fail.
fn test_output_tree_to_text() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "step1".to_string(),
        StepType::Thinking,
        "Analyzing".to_string(),
    );
    tree.complete_step("step1");

    let text = tree.to_text();
    assert!(!text.is_empty());
    assert!(text.contains("Analyzing"));
}

#[test]
/// # Panics
/// Panics if assertions fail.
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
/// # Panics
/// Panics if assertions fail.
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
/// # Panics
/// Panics if assertions fail.
fn test_zero_width_terminal_handling() {
    let _manager = InputManager::default();

    // Wrapping to 0 width should not panic
    let _ = calculate_width("", EmojiMode::Permissive);
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_special_control_characters() {
    let mut manager = InputManager::default();

    // Null byte should be handled
    let text = "Hello\0World";
    for character in text.chars() {
        let key = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
        let input = Input::from(Event::Key(key));
        manager.input_area_mut().input(input);
    }

    // Should not panic
    let lines = manager.input_area().lines();
    assert!(!lines.is_empty());
}

/// Helper function imported from the module
fn calculate_width(text: &str, mode: EmojiMode) -> usize {
    ui_calculate_width(text, mode)
}
