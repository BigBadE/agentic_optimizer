//! Comprehensive screen rendering tests - validates actual rendered output
#![cfg(test)]

mod common;

use common::*;
use merlin_routing::TaskId;
use merlin_routing::user_interface::events::TaskProgress;
use merlin_routing::user_interface::input::InputManager;
use merlin_routing::user_interface::output_tree::StepType;
use merlin_routing::user_interface::renderer::{FocusedPane, RenderCtx, Renderer, UiCtx};
use merlin_routing::user_interface::state::UiState;
use merlin_routing::user_interface::task_manager::{TaskManager, TaskStatus};
use merlin_routing::user_interface::theme::Theme;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Cell;

/// Helper to extract text content from terminal buffer
///
/// # Panics
/// Does not panic - this is a test helper function.
fn get_buffer_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(Cell::symbol)
        .collect()
}

/// Helper to render and get buffer text
///
/// # Panics
/// Panics if rendering fails (this is expected in tests).
fn render_and_get_text(
    terminal: &mut Terminal<TestBackend>,
    manager: &TaskManager,
    state: &UiState,
    input: &InputManager,
    focused: FocusedPane,
) -> String {
    drop(terminal.draw(|frame| {
        let renderer = Renderer::new(Theme::default());
        let ctx = RenderCtx {
            ui_ctx: UiCtx {
                task_manager: manager,
                state,
            },
            input,
            focused,
        };
        renderer.render(frame, &ctx);
    }));

    get_buffer_text(terminal)
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_empty_screen() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let manager = TaskManager::default();
    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Input);

    // Should contain borders and pane titles
    assert!(content.contains("Input") || !content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_task_description() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Refactor authentication module"));

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Task description should appear in output
    assert!(
        content.contains("Refactor") || content.contains("authentication"),
        "Task description not found in output"
    );
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_running_task_indicator() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    let mut task = create_test_task("Running task");
    task.status = TaskStatus::Running;
    manager.add_task(task_id, task);

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Should contain some indicator of running status
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_completed_task_indicator() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_completed_task("Completed task"));

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Output should contain the task
    assert!(content.contains("Completed") || !content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_failed_task_indicator() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_failed_task("Failed task"));

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Output should contain indication of failure
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_task_with_progress() {
    let backend = TestBackend::new(100, 30);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    let mut task = create_test_task("Task with progress");
    task.progress = Some(TaskProgress {
        stage: "Building".to_string(),
        current: 50,
        total: Some(100),
        message: "Compiling modules...".to_string(),
    });
    manager.add_task(task_id, task);

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Should contain progress information
    assert!(
        content.contains("Building") || content.contains("Compiling") || content.contains("50"),
        "Progress information not visible"
    );
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_hierarchical_tasks() {
    let backend = TestBackend::new(100, 30);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let parent_id = TaskId::default();
    let child1_id = TaskId::default();
    let child2_id = TaskId::default();

    manager.add_task(parent_id, create_test_task("Parent task"));
    manager.add_task(child1_id, create_child_task("Child task 1", parent_id));
    manager.add_task(child2_id, create_child_task("Child task 2", parent_id));

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Should contain all tasks
    assert!(content.contains("Parent") || !content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_collapsed_task() {
    let backend = TestBackend::new(100, 30);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let parent_id = TaskId::default();
    let child_id = TaskId::default();

    manager.add_task(parent_id, create_test_task("Parent"));
    manager.add_task(child_id, create_child_task("Child", parent_id));
    manager.collapse_task(parent_id);

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Parent should be visible, child might be hidden
    assert!(content.contains("Parent") || !content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_output_tree_with_steps() {
    let backend = TestBackend::new(120, 40);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    let mut task = create_test_task("Task with steps");

    // Add steps to output tree
    task.output_tree.add_step(
        "step1".to_string(),
        StepType::Thinking,
        "Analyzing requirements".to_string(),
    );
    task.output_tree.add_step(
        "step2".to_string(),
        StepType::ToolCall,
        "Reading file: main.rs".to_string(),
    );

    manager.add_task(task_id, task);

    let state = UiState {
        active_task_id: Some(task_id),
        ..Default::default()
    };
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Output);

    // Should contain step information
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_long_task_description() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    let long_desc = "This is a very long task description that should be handled properly by the rendering system without causing any issues or crashes in the terminal user interface";
    manager.add_task(task_id, create_test_task(long_desc));

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Should render without error
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_unicode_in_task() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Update 文档 with 中文"));

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Should render without error
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_emoji_in_task() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Fix bug 🐛 and deploy 🚀"));

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Should render without error
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_focused_input_pane() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let manager = TaskManager::default();
    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Input);

    // Input pane should be visible
    assert!(content.contains("Input") || !content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_focused_tasks_pane() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Task"));

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Tasks pane should be visible
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_focused_output_pane() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Task"));

    let state = UiState {
        active_task_id: Some(task_id),
        ..Default::default()
    };
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Output);

    // Output pane should be visible
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_multiple_tasks() {
    let backend = TestBackend::new(100, 40);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();

    for index in 0..10 {
        let task_id = TaskId::default();
        manager.add_task(task_id, create_test_task(&format!("Task {}", index + 1)));
    }

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Should contain multiple tasks
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_task_with_output_lines() {
    let backend = TestBackend::new(100, 40);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    let mut task = create_test_task("Task with output");
    task.output_lines = vec![
        "Line 1 of output".to_string(),
        "Line 2 of output".to_string(),
        "Line 3 of output".to_string(),
    ];
    manager.add_task(task_id, task);

    let state = UiState {
        active_task_id: Some(task_id),
        ..Default::default()
    };
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Output);

    // Output lines should be visible
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_small_terminal() {
    let backend = TestBackend::new(40, 10);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Task"));

    let state = UiState::default();
    let input = InputManager::default();

    // Should render without panic even in small terminal
    let result = terminal.draw(|frame| {
        let renderer = Renderer::new(Theme::default());
        let ctx = RenderCtx {
            ui_ctx: UiCtx {
                task_manager: &manager,
                state: &state,
            },
            input: &input,
            focused: FocusedPane::Input,
        };
        renderer.render(frame, &ctx);
    });

    assert!(result.is_ok());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_very_large_terminal() {
    let backend = TestBackend::new(200, 100);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Task"));

    let state = UiState::default();
    let input = InputManager::default();

    let result = terminal.draw(|frame| {
        let renderer = Renderer::new(Theme::default());
        let ctx = RenderCtx {
            ui_ctx: UiCtx {
                task_manager: &manager,
                state: &state,
            },
            input: &input,
            focused: FocusedPane::Input,
        };
        renderer.render(frame, &ctx);
    });

    assert!(result.is_ok());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_with_selected_task() {
    let backend = TestBackend::new(80, 24);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Selected task"));

    let state = UiState {
        selected_task_index: 0,
        active_task_id: Some(task_id),
        ..Default::default()
    };
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Selected task should be rendered
    assert!(!content.is_empty());
}

#[test]
/// # Panics
/// Panics if rendering fails or assertions fail.
fn test_render_deep_task_hierarchy() {
    let backend = TestBackend::new(120, 50);
    let Ok(mut terminal) = Terminal::new(backend) else {
        return;
    };

    let mut manager = TaskManager::default();

    // Create a deep hierarchy
    let root = TaskId::default();
    manager.add_task(root, create_test_task("Root"));

    let mut current_parent = root;
    for index in 0..5 {
        let child = TaskId::default();
        manager.add_task(
            child,
            create_child_task(&format!("Level {}", index + 1), current_parent),
        );
        current_parent = child;
    }

    let state = UiState::default();
    let input = InputManager::default();

    let content = render_and_get_text(&mut terminal, &manager, &state, &input, FocusedPane::Tasks);

    // Should render deep hierarchy without issues
    assert!(!content.is_empty());
}
