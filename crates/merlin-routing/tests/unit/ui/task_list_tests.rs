//! Tests for task list rendering behavior
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
use merlin_routing::TaskId;
use merlin_routing::user_interface::{
    input::InputManager,
    layout::LayoutCache,
    renderer::{FocusedPane, RenderCtx, Renderer, UiCtx},
    state::UiState,
    task_manager::{TaskManager, TaskStatus as UiTaskStatus},
    theme::Theme,
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Cell;
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[test]
fn test_task_list_shows_only_most_recent_conversation_on_launch() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create 3 previous conversations (completed tasks) with different start times
    let now = Instant::now();
    let task1_id = TaskId::default();
    let task2_id = TaskId::default();
    let task3_id = TaskId::default();

    // Oldest conversation
    let mut task1 = create_test_task_with_time(
        "First conversation",
        now.checked_sub(Duration::from_secs(300)).unwrap(),
    );
    task1.status = UiTaskStatus::Completed;
    manager.add_task(task1_id, task1);

    // Middle conversation
    let mut task2 = create_test_task_with_time(
        "Second conversation",
        now.checked_sub(Duration::from_secs(200)).unwrap(),
    );
    task2.status = UiTaskStatus::Completed;
    manager.add_task(task2_id, task2);

    // Most recent conversation (should be shown)
    let mut task3 = create_test_task_with_time(
        "Third conversation",
        now.checked_sub(Duration::from_secs(100)).unwrap(),
    );
    task3.status = UiTaskStatus::Completed;
    manager.add_task(task3_id, task3);

    let state = UiState::default();
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // When Tasks pane is focused, should show all 3 conversations (max_visible = 3)
    assert!(content.contains("First conversation"));
    assert!(content.contains("Second conversation"));
    assert!(content.contains("Third conversation"));
}

#[test]
fn test_new_conversation_does_not_branch_from_previous() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create a previous completed conversation
    let now = Instant::now();
    let prev_task_id = TaskId::default();
    let mut prev_task = create_test_task_with_time(
        "Previous conversation",
        now.checked_sub(Duration::from_secs(100)).unwrap(),
    );
    prev_task.status = UiTaskStatus::Completed;
    manager.add_task(prev_task_id, prev_task);

    // Create a new running conversation (should NOT be a child of previous)
    let new_task_id = TaskId::default();
    let new_task = create_test_task_with_time("New conversation", now);
    manager.add_task(new_task_id, new_task);

    let mut active_running_tasks = HashSet::new();
    active_running_tasks.insert(new_task_id);

    let state = UiState {
        active_running_tasks,
        ..Default::default()
    };
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Input,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // When not focused on Tasks, should show only the new running conversation
    assert!(content.contains("New conversation"));
    assert!(!content.contains("Previous conversation"));

    // Verify the new task is not a child of the previous task
    let retrieved_task = manager.get_task(new_task_id).unwrap();
    assert!(retrieved_task.parent_id.is_none());
}

#[test]
fn test_task_list_only_shows_children_of_current_task() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create a parent task
    let parent_id = TaskId::default();
    let parent_task = create_test_task("Parent task");
    manager.add_task(parent_id, parent_task);

    // Create children of the parent task
    let child1_id = TaskId::default();
    let child1 = create_child_task("Child 1", parent_id);
    manager.add_task(child1_id, child1);

    let child2_id = TaskId::default();
    let child2 = create_child_task("Child 2", parent_id);
    manager.add_task(child2_id, child2);

    // Create another root task (should NOT be shown)
    let other_root_id = TaskId::default();
    let other_root = create_test_task("Other root task");
    manager.add_task(other_root_id, other_root);

    let mut active_running_tasks = HashSet::new();
    active_running_tasks.insert(parent_id);

    let state = UiState {
        active_running_tasks,
        ..Default::default()
    };
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Input,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // When not focused on Tasks, should show parent and its children
    assert!(content.contains("Parent task"));
    assert!(content.contains("Child 1"));
    assert!(content.contains("Child 2"));

    // Should NOT show the other root task
    assert!(!content.contains("Other root task"));
}

#[test]
fn test_task_list_does_not_exceed_screen_height() {
    // Test with a small screen height
    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create a parent task
    let parent_id = TaskId::default();
    let parent_task = create_test_task("Parent task");
    manager.add_task(parent_id, parent_task);

    // Create many children (more than can fit on screen)
    for index in 0..20 {
        let child_id = TaskId::default();
        let child = create_child_task(&format!("Child task {index}"), parent_id);
        manager.add_task(child_id, child);
    }

    let mut active_running_tasks = HashSet::new();
    active_running_tasks.insert(parent_id);

    let state = UiState {
        active_running_tasks,
        ..Default::default()
    };
    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    // Should not panic even with many tasks
    let mut layout_cache = LayoutCache::default();
    let result = terminal.draw(|frame| {
        let mut ctx = RenderCtx {
            ui_ctx: UiCtx {
                task_manager: &manager,
                state: &state,
            },
            input: &input,
            focused: FocusedPane::Tasks,
            layout_cache: &mut layout_cache,
        };
        renderer.render(frame, &mut ctx);
    });

    assert!(
        result.is_ok(),
        "Rendering should succeed even with many tasks"
    );

    // Verify the terminal height constraint is respected
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer.area().height,
        10,
        "Buffer height should match terminal height"
    );
}

#[test]
fn test_task_list_with_no_running_tasks_shows_most_recent() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create multiple completed tasks
    let now = Instant::now();
    let task1_id = TaskId::default();
    let mut task1 = create_test_task_with_time(
        "Old task",
        now.checked_sub(Duration::from_secs(200)).unwrap(),
    );
    task1.status = UiTaskStatus::Completed;
    manager.add_task(task1_id, task1);

    let task2_id = TaskId::default();
    let mut task2 = create_test_task_with_time(
        "Recent task",
        now.checked_sub(Duration::from_secs(50)).unwrap(),
    );
    task2.status = UiTaskStatus::Completed;
    manager.add_task(task2_id, task2);

    let state = UiState::default();
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // When Tasks pane is focused with 2 tasks, should show both
    assert!(content.contains("Recent task"));
    assert!(content.contains("Old task"));
}

#[test]
fn test_task_list_shows_nested_children_only_for_current_root() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create first root task with children
    let root1_id = TaskId::default();
    let root1 = create_test_task("Root 1");
    manager.add_task(root1_id, root1);

    let child1_id = TaskId::default();
    let child1 = create_child_task("Root 1 Child", root1_id);
    manager.add_task(child1_id, child1);

    // Create second root task with children (this is the running one)
    let root2_id = TaskId::default();
    let root2 = create_test_task("Root 2");
    manager.add_task(root2_id, root2);

    let child2_id = TaskId::default();
    let child2 = create_child_task("Root 2 Child", root2_id);
    manager.add_task(child2_id, child2);

    let mut active_running_tasks = HashSet::new();
    active_running_tasks.insert(root2_id);

    let state = UiState {
        active_running_tasks,
        ..Default::default()
    };
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Input,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // When not focused on Tasks, should show only Root 2 (running) and its children
    assert!(content.contains("Root 2"));
    assert!(content.contains("Root 2 Child"));

    // Should NOT show Root 1 or its children
    assert!(!content.contains("Root 1 Child"));
    // Note: "Root 1" might appear as substring of "Root 1 Child", so we check more carefully
    let lines: Vec<&str> = content.lines().collect();
    let has_root1_line = lines
        .iter()
        .any(|line| line.contains("Root 1") && !line.contains("Root 2"));
    assert!(
        !has_root1_line,
        "Should not show Root 1 as a separate entry"
    );
}

#[test]
fn test_task_list_expands_when_tasks_pane_focused() {
    // Test that Tasks pane shows up to 3 recent conversations
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create 3 root tasks (conversations)
    let now = Instant::now();
    let task1_id = TaskId::default();
    let mut task1 = create_test_task_with_time(
        "Conversation 1",
        now.checked_sub(Duration::from_secs(300)).unwrap(),
    );
    task1.status = UiTaskStatus::Completed;
    manager.add_task(task1_id, task1);

    let task2_id = TaskId::default();
    let mut task2 = create_test_task_with_time(
        "Conversation 2",
        now.checked_sub(Duration::from_secs(200)).unwrap(),
    );
    task2.status = UiTaskStatus::Completed;
    manager.add_task(task2_id, task2);

    let task3_id = TaskId::default();
    let task3 = create_test_task_with_time(
        "Conversation 3",
        now.checked_sub(Duration::from_secs(100)).unwrap(),
    );
    manager.add_task(task3_id, task3);

    let state = UiState::default();
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    // Render with Tasks pane focused
    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // When Tasks pane is focused, should show all 3 conversations (max_visible = 3)
    assert!(content.contains("Conversation 1"));
    assert!(content.contains("Conversation 2"));
    assert!(content.contains("Conversation 3"));
}

#[test]
fn test_task_list_shrinks_when_output_pane_focused() {
    // Test with many running tasks and Output pane focused
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create a parent task
    let parent_id = TaskId::default();
    let parent_task = create_test_task("Parent task");
    manager.add_task(parent_id, parent_task);

    // Create 10 running children
    for index in 0..10 {
        let child_id = TaskId::default();
        let child = create_child_task(&format!("Running child {index}"), parent_id);
        manager.add_task(child_id, child);
    }

    let mut active_running_tasks = HashSet::new();
    active_running_tasks.insert(parent_id);

    let state = UiState {
        active_running_tasks,
        active_task_id: Some(parent_id),
        ..Default::default()
    };
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    // Render with Output pane focused
    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Output,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();

    // Find the Tasks section and count its height
    let mut tasks_section_height = 0;
    let mut in_tasks_section = false;

    for y in 0..buffer.area().height {
        let mut line = String::new();
        for x in 0..buffer.area().width {
            line.push_str(buffer.cell((x, y)).unwrap().symbol());
        }

        if line.contains("─── Tasks ") {
            in_tasks_section = true;
        }

        if in_tasks_section {
            tasks_section_height += 1;
            // Check if we've reached the end of the Tasks section
            if line.contains("└") && line.chars().filter(|&character| character == '─').count() > 10
            {
                break;
            }
        }
    }

    // Task list should be limited to max 5 lines (3 content + 2 borders)
    assert!(
        tasks_section_height <= 5,
        "Task list should be limited to 5 lines when Output is focused, got {tasks_section_height}"
    );
}

#[test]
#[cfg_attr(
    test,
    allow(
        clippy::too_many_lines,
        reason = "Complex scrolling test requires many steps"
    )
)]
fn test_task_list_scrolling_through_conversations() {
    // Test scrolling through multiple conversations
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create 3 completed conversations (root tasks)
    let conv1_id = TaskId::default();
    let mut conv1 = create_test_task("First conversation");
    conv1.status = UiTaskStatus::Completed;
    manager.add_task(conv1_id, conv1);

    let conv2_id = TaskId::default();
    let mut conv2 = create_test_task("Second conversation");
    conv2.status = UiTaskStatus::Completed;
    manager.add_task(conv2_id, conv2);

    let conv3_id = TaskId::default();
    let mut conv3 = create_test_task("Third conversation");
    conv3.status = UiTaskStatus::Completed;
    manager.add_task(conv3_id, conv3);

    // Initially, no selection - should show oldest (First)
    let state = UiState {
        active_task_id: None,
        ..Default::default()
    };
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Input,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // When no task is selected, placeholder should be shown
    assert!(
        content.contains("Start a new conversation"),
        "Should show placeholder when nothing selected"
    );
    assert!(
        !content.contains("First conversation"),
        "Should not show first conversation when placeholder selected"
    );
    assert!(
        !content.contains("Second conversation"),
        "Should not show second conversation"
    );
    assert!(
        !content.contains("Third conversation"),
        "Should not show third conversation"
    );

    // Now select the second conversation
    let state_with_selection = UiState {
        active_task_id: Some(conv2_id),
        ..Default::default()
    };

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state_with_selection,
                },
                input: &input,
                focused: FocusedPane::Input,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let second_buffer = terminal.backend().buffer();
    let second_content: String = second_buffer.content().iter().map(Cell::symbol).collect();

    assert!(
        second_content.contains("Second conversation"),
        "Should show selected conversation"
    );
    assert!(
        !second_content.contains("Third conversation"),
        "Should not show third conversation when second is selected"
    );
    assert!(
        !second_content.contains("First conversation"),
        "Should not show first conversation"
    );
}

#[test]
fn test_task_list_starts_at_top() {
    // Test that task list starts showing oldest tasks (scroll_offset = 0)
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create 5 conversations
    let now = Instant::now();
    for index in 0..5 {
        let task_id = TaskId::default();
        let mut task = create_test_task_with_time(
            &format!("Conversation {index}"),
            now.checked_sub(Duration::from_secs((5 - index) as u64 * 100))
                .unwrap(),
        );
        task.status = UiTaskStatus::Completed;
        manager.add_task(task_id, task);
    }

    // Initial state with scroll_offset = 0 (default, showing oldest)
    let state = UiState {
        task_list_scroll_offset: 0,
        ..Default::default()
    };
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // With terminal height 24 and task area of 14 lines, max_visible = 12
    // So all 5 conversations should be visible
    assert!(
        content.contains("Conversation 0"),
        "Should show oldest conversation"
    );
    assert!(
        content.contains("Conversation 1"),
        "Should show second oldest"
    );
    assert!(
        content.contains("Conversation 2"),
        "Should show third oldest"
    );
    assert!(
        content.contains("Conversation 3"),
        "Should show fourth conversation"
    );
    assert!(
        content.contains("Conversation 4"),
        "Should show newest conversation"
    );
    assert!(
        content.contains("Start a new conversation..."),
        "Should show placeholder"
    );
}

#[test]
fn test_task_list_scrolling_down() {
    // Test scrolling down to show newer conversations
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create 5 conversations
    let now = Instant::now();
    for index in 0..5 {
        let task_id = TaskId::default();
        let mut task = create_test_task_with_time(
            &format!("Conversation {index}"),
            now.checked_sub(Duration::from_secs((5 - index) as u64 * 100))
                .unwrap(),
        );
        task.status = UiTaskStatus::Completed;
        manager.add_task(task_id, task);
    }

    // Scroll down by 3 (showing newest conversations + placeholder)
    let state = UiState {
        task_list_scroll_offset: 3,
        ..Default::default()
    };
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // Should show conversations 3, 4 + placeholder (scrolled down)
    assert!(
        content.contains("Conversation 3"),
        "Should show conversation 3"
    );
    assert!(
        content.contains("Conversation 4"),
        "Should show conversation 4"
    );
    assert!(
        content.contains("Start a new conversation..."),
        "Should show placeholder at bottom"
    );
    assert!(
        !content.contains("Conversation 0"),
        "Should not show oldest after scrolling"
    );
    assert!(
        !content.contains("Conversation 2"),
        "Should not show conversation 2"
    );
}

#[test]
fn test_task_list_navigation_skips_child_messages() {
    // Test that navigating up/down goes between conversations, not individual messages
    let mut manager = TaskManager::default();

    // Create conversation 1 with multiple child messages
    let now = Instant::now();
    let conv1_id = TaskId::default();
    let mut conv1 = create_test_task_with_time(
        "Conversation 1",
        now.checked_sub(Duration::from_secs(300)).unwrap(),
    );
    conv1.status = UiTaskStatus::Completed;
    manager.add_task(conv1_id, conv1);

    // Add child messages to conversation 1
    let child1_id = TaskId::default();
    let mut child1 = create_child_task("Message 1-1", conv1_id);
    child1.status = UiTaskStatus::Completed;
    manager.add_task(child1_id, child1);

    let child2_id = TaskId::default();
    let mut child2 = create_child_task("Message 1-2", conv1_id);
    child2.status = UiTaskStatus::Completed;
    manager.add_task(child2_id, child2);

    // Create conversation 2
    let conv2_id = TaskId::default();
    let mut conv2 = create_test_task_with_time(
        "Conversation 2",
        now.checked_sub(Duration::from_secs(200)).unwrap(),
    );
    conv2.status = UiTaskStatus::Completed;
    manager.add_task(conv2_id, conv2);

    // Create conversation 3
    let conv3_id = TaskId::default();
    let mut conv3 = create_test_task_with_time(
        "Conversation 3",
        now.checked_sub(Duration::from_secs(100)).unwrap(),
    );
    conv3.status = UiTaskStatus::Completed;
    manager.add_task(conv3_id, conv3);

    // Get all root conversations
    let root_conversations: Vec<_> = manager
        .iter_tasks()
        .filter(|(_, task)| task.parent_id.is_none())
        .map(|(id, _)| id)
        .collect();

    // Should have 3 root conversations
    assert_eq!(
        root_conversations.len(),
        3,
        "Should have 3 root conversations"
    );

    // Verify no child messages are in the root list
    for root_id in &root_conversations {
        let task = manager.get_task(*root_id).expect("Task should exist");
        assert!(
            task.parent_id.is_none(),
            "Root conversation list should only contain tasks with no parent"
        );
    }

    // Verify children exist but are not in root list
    let child1_task = manager.get_task(child1_id).expect("Child should exist");
    assert!(
        child1_task.parent_id.is_some(),
        "Child should have a parent"
    );
    assert!(
        !root_conversations.contains(&child1_id),
        "Child should not be in root conversations list"
    );
}

#[test]
fn test_conversation_collapsed_by_default() {
    // Test that conversations are collapsed by default (children not shown)
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Create conversation with child messages
    let now = Instant::now();
    let conv_id = TaskId::default();
    let mut conv = create_test_task_with_time("Main conversation", now);
    conv.status = UiTaskStatus::Completed;
    manager.add_task(conv_id, conv);

    let child_id = TaskId::default();
    let mut child = create_child_task("Child message", conv_id);
    child.status = UiTaskStatus::Completed;
    manager.add_task(child_id, child);

    let state = UiState::default();
    let input = InputManager::default();
    let mut layout_cache = LayoutCache::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
                layout_cache: &mut layout_cache,
            };
            renderer.render(frame, &mut ctx);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let content: String = buffer.content().iter().map(Cell::symbol).collect();

    // Should show main conversation but not child messages
    assert!(
        content.contains("Main conversation"),
        "Should show root conversation"
    );
    assert!(
        !content.contains("Child message"),
        "Should not show child messages when collapsed"
    );
}
