//! Tests for TUI rendering components
#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::absolute_paths,
    reason = "Test code is allowed to use expect/unwrap and absolute paths"
)]

mod common;

use common::*;
use merlin_routing::TaskId;
use merlin_routing::user_interface::{
    input::InputManager,
    renderer::{FocusedPane, RenderCtx, Renderer, UiCtx},
    state::UiState,
    task_manager::TaskManager,
    theme::Theme,
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_renderer_creation() {
    let renderer = Renderer::new(Theme::default());
    // Default theme is Tokyo Night
    assert_eq!(renderer.theme().name(), "Tokyo Night");
}

#[test]
/// # Panics
/// Panics if assertions fail.
fn test_theme_cycling() {
    let mut renderer = Renderer::new(Theme::default());

    let initial = renderer.theme();
    let next = initial.next();

    renderer.set_theme(next);
    assert_ne!(renderer.theme().name(), initial.name());
}

#[test]
/// # Panics
/// Panics if terminal creation or rendering fails.
fn test_render_empty_state() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let manager = TaskManager::default();
    let state = UiState::default();
    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    let result = terminal.draw(|frame| {
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

    result.unwrap();
}

#[test]
/// # Panics
/// Panics if terminal creation or rendering fails.
fn test_render_with_tasks() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Test task"));

    let state = UiState {
        active_task_id: Some(task_id),
        selected_task_index: 0,
        ..Default::default()
    };

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    let result = terminal.draw(|frame| {
        let ctx = RenderCtx {
            ui_ctx: UiCtx {
                task_manager: &manager,
                state: &state,
            },
            input: &input,
            focused: FocusedPane::Tasks,
        };
        renderer.render(frame, &ctx);
    });

    result.unwrap();

    // Check that the buffer contains our task
    let buffer = terminal.backend().buffer();
    let content: String = buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(content.contains("Test task") || content.contains("Test"));
}

#[test]
/// # Panics
/// Panics if terminal creation or rendering fails for any pane.
fn test_render_all_panes() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_test_task("Test"));

    let state = UiState {
        active_task_id: Some(task_id),
        ..Default::default()
    };

    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    // Test rendering each focused pane
    for focused in [FocusedPane::Input, FocusedPane::Output, FocusedPane::Tasks] {
        let result = terminal.draw(|frame| {
            let ctx = RenderCtx {
                ui_ctx: UiCtx {
                    task_manager: &manager,
                    state: &state,
                },
                input: &input,
                focused,
            };
            renderer.render(frame, &ctx);
        });

        result.unwrap_or_else(|_| panic!("Rendering failed for pane: {focused:?}"));
    }
}

#[test]
/// # Panics
/// Panics if terminal creation or rendering fails.
fn test_render_with_completed_task() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_completed_task("Completed task"));

    let state = UiState::default();
    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    let result = terminal.draw(|frame| {
        let ctx = RenderCtx {
            ui_ctx: UiCtx {
                task_manager: &manager,
                state: &state,
            },
            input: &input,
            focused: FocusedPane::Tasks,
        };
        renderer.render(frame, &ctx);
    });

    result.unwrap();
}

#[test]
/// # Panics
/// Panics if terminal creation or rendering fails.
fn test_render_with_failed_task() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut manager = TaskManager::default();
    let task_id = TaskId::default();
    manager.add_task(task_id, create_failed_task("Failed task"));

    let state = UiState::default();
    let input = InputManager::default();
    let renderer = Renderer::new(Theme::default());

    let result = terminal.draw(|frame| {
        let ctx = RenderCtx {
            ui_ctx: UiCtx {
                task_manager: &manager,
                state: &state,
            },
            input: &input,
            focused: FocusedPane::Tasks,
        };
        renderer.render(frame, &ctx);
    });

    result.unwrap();
}

#[test]
/// # Panics
/// Panics if terminal creation or rendering fails for any theme.
fn test_all_themes_render() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let manager = TaskManager::default();
    let state = UiState::default();
    let input = InputManager::default();

    // Test all available themes
    let mut theme = Theme::default();
    let start_theme_name = theme.name().to_string();

    for _ in 0..10 {
        // Test up to 10 themes (cycling will repeat)
        let renderer = Renderer::new(theme);

        let result = terminal.draw(|frame| {
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

        result.unwrap_or_else(|_| panic!("Theme {} failed to render", theme.name()));

        let next = theme.next();
        if next.name() == start_theme_name {
            break; // We've cycled through all themes
        }
        theme = next;
    }
}
