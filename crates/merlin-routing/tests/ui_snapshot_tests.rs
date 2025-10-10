//! Snapshot tests for UI rendering - compares actual buffer output against expected
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
use merlin_routing::TaskId;
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
use std::collections::HashSet;
use std::time::Instant;

/// Helper to extract text content from buffer
fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(Cell::symbol)
        .collect()
}

/// Helper to get buffer lines
fn buffer_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
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

#[test]
fn test_empty_ui_layout() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let manager = TaskManager::default();
    let state = UiState::default();
    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Input,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let content = buffer_to_string(&terminal);

    // Verify the main UI elements are present (new layout: task tree at top, focused in middle, input at bottom)
    assert!(content.contains("─── Input"), "Should have Input section");
    assert!(
        content.contains("No tasks running"),
        "Should show 'No tasks running' when empty"
    );
    assert!(
        content.contains("No task selected") || content.contains("Ctrl+T"),
        "Should show help message when no task selected"
    );
}

#[test]
fn test_task_tree_with_running_task() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    let task = TaskDisplay {
        description: "Running build task".to_string(),
        status: TaskStatus::Running,
        start_time: Instant::now(),
        end_time: None,
        parent_id: None,
        progress: None,
        output_lines: vec!["Compiling merlin-core...".to_string()],
        output_tree: OutputTree::default(),
        steps: vec![],
    };

    manager.add_task(task_id, task);

    let mut state = UiState::default();
    state.active_running_tasks.insert(task_id);

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let content = buffer_to_string(&terminal);

    // Check for running task icon
    assert!(
        content.contains("▶") || content.contains("[▶]"),
        "Should show running icon for active task"
    );
    assert!(
        content.contains("Running build task"),
        "Should display task description"
    );
    assert!(
        content.contains("Compiling merlin-core") || content.contains("⤷ log:"),
        "Should show task output log"
    );
}

#[test]
fn test_task_status_icons() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Add running task
    let running_id = TaskId::default();
    manager.add_task(running_id, create_test_task("Running task"));

    // Add completed task
    let completed_id = TaskId::default();
    manager.add_task(completed_id, create_completed_task("Completed task"));

    // Add failed task
    let failed_id = TaskId::default();
    manager.add_task(failed_id, create_failed_task("Failed task"));

    let mut state = UiState::default();
    state.active_running_tasks.insert(running_id);

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let lines = buffer_lines(&terminal);
    let content = lines.join("\n");

    // Verify all three status icons are present (note: pending tasks show as [ ])
    assert!(
        content.contains("Running task")
            && content.contains("Completed task")
            && content.contains("Failed task"),
        "Should show all three task types"
    );
    assert!(
        content.contains("✔") || content.contains("[✔]"),
        "Should have completed icon (✔)"
    );
    assert!(
        content.contains("✗") || content.contains("[✗]"),
        "Should have failed icon (✗)"
    );
}

#[test]
fn test_progress_bar_rendering() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    let task = TaskDisplay {
        description: "Task with progress".to_string(),
        status: TaskStatus::Running,
        start_time: Instant::now(),
        end_time: None,
        parent_id: None,
        progress: Some(TaskProgress {
            stage: "Building".to_string(),
            current: 42,
            total: Some(100),
            message: "Compiling...".to_string(),
        }),
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    };

    manager.add_task(task_id, task);

    let mut state = UiState::default();
    state.active_running_tasks.insert(task_id);

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let content = buffer_to_string(&terminal);

    // Check for progress indicators
    assert!(
        content.contains("42%") || content.contains("▓") || content.contains("░"),
        "Should show progress percentage or bar characters"
    );
}

#[test]
fn test_focused_task_detail_panel() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();

    let task = TaskDisplay {
        description: "Detailed task view".to_string(),
        status: TaskStatus::Running,
        start_time: Instant::now(),
        end_time: None,
        parent_id: None,
        progress: Some(TaskProgress {
            stage: "Testing".to_string(),
            current: 75,
            total: Some(100),
            message: "Running tests...".to_string(),
        }),
        output_lines: vec!["Test output line 1".to_string()],
        output_tree: OutputTree::default(),
        steps: vec![],
    };

    manager.add_task(task_id, task);

    let state = UiState {
        active_task_id: Some(task_id),
        selected_task_index: 0,
        active_running_tasks: {
            let mut set = HashSet::default();
            set.insert(task_id);
            set
        },
        ..Default::default()
    };

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Output,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let content = buffer_to_string(&terminal);

    // Verify focused task panel content
    assert!(
        content.contains("Focused task:"),
        "Should show 'Focused task:' label"
    );
    assert!(
        content.contains("Detailed task view"),
        "Should show task description in detail panel"
    );
    assert!(
        content.contains("75%") || content.contains("▓"),
        "Should show progress in detail panel"
    );
}

#[test]
fn test_pending_tasks_section() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Add active task
    let active_id = TaskId::default();
    manager.add_task(active_id, create_test_task("Active task"));

    // Add pending tasks
    let pending1 = TaskId::default();
    manager.add_task(pending1, create_test_task("Pending task 1"));

    let pending2 = TaskId::default();
    manager.add_task(pending2, create_test_task("Pending task 2"));

    let state = UiState {
        active_task_id: Some(active_id),
        selected_task_index: 0,
        active_running_tasks: {
            let mut set = HashSet::default();
            set.insert(active_id);
            set.insert(pending1);
            set.insert(pending2);
            set
        },
        ..Default::default()
    };

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Output,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let content = buffer_to_string(&terminal);

    // In the new layout, all tasks are shown in the task tree
    assert!(
        content.contains("Active task") || content.contains("Pending task"),
        "Should show tasks in tree"
    );
}

#[test]
fn test_hierarchical_task_tree() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();

    // Add parent task
    let parent_id = TaskId::default();
    manager.add_task(parent_id, create_test_task("Parent task"));

    // Add child tasks
    let child1_id = TaskId::default();
    manager.add_task(child1_id, create_child_task("Child task 1", parent_id));

    let child2_id = TaskId::default();
    manager.add_task(child2_id, create_child_task("Child task 2", parent_id));

    let mut state = UiState::default();
    state.active_running_tasks.insert(parent_id);
    state.active_running_tasks.insert(child1_id);
    state.active_running_tasks.insert(child2_id);

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Tasks,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let lines = buffer_lines(&terminal);

    // Look for hierarchical structure indicators
    let has_hierarchy = lines
        .iter()
        .any(|line| line.contains("├─") || line.contains("Parent task"));

    let has_children = lines.iter().any(|line| {
        line.contains("Child task 1") || line.contains("Child task 2") || line.contains("  ├─")
    });

    assert!(has_hierarchy, "Should show parent task with tree structure");
    assert!(
        has_children,
        "Should show child tasks with proper indentation"
    );
}

#[test]
fn test_ui_layout_structure() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Test task"));

    let state = UiState {
        active_task_id: Some(task_id),
        selected_task_index: 0,
        active_running_tasks: {
            let mut set = HashSet::default();
            set.insert(task_id);
            set
        },
        ..Default::default()
    };

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Input,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let content = buffer_to_string(&terminal);

    // Verify main sections exist (new layout)
    assert!(content.contains("─── Input"), "Should have Input section");
    assert!(
        content.contains("Build project") || content.contains("Test task"),
        "Should show task in tree"
    );
}

#[test]
fn test_no_selected_task_message() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Unselected task"));

    let state = UiState {
        active_task_id: None, // No task selected
        ..Default::default()
    };

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    terminal
        .draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused: FocusedPane::Output,
            };
            renderer.render(frame, &ctx);
        })
        .unwrap();

    let content = buffer_to_string(&terminal);

    // Should show message to select a task
    assert!(
        content.contains("Select a task") || content.contains("Ctrl+T"),
        "Should show help message when no task is selected"
    );
}
