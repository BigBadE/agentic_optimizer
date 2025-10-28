//! UI and state verification logic.

use super::fixture::{StateVerify, UiVerify};
use super::verification_result::VerificationResult;
use merlin_cli::TuiApp;
use merlin_cli::ui::renderer::FocusedPane;
use merlin_cli::ui::task_manager::TaskStatus;
use merlin_deps::ratatui::backend::TestBackend;

/// UI verifier helper
pub struct UiVerifier;

impl UiVerifier {
    /// Verify UI
    pub fn verify_ui(
        result: &mut VerificationResult,
        tui_app: Option<&TuiApp<TestBackend>>,
        verify: &UiVerify,
    ) {
        let Some(app) = tui_app else {
            result.add_failure("TUI app not available for verification".to_owned());
            return;
        };

        let _state = app.test_state();
        let task_manager = app.test_task_manager();
        let input_manager = app.test_input_manager();

        // Verify input text
        if let Some(expected_input) = &verify.input_text {
            let actual_input = input_manager.input_area().lines().join("\n");
            if actual_input == *expected_input {
                result.add_success(format!("Input text matches: '{expected_input}'"));
            } else {
                result.add_failure(format!(
                    "Input text mismatch. Expected: '{expected_input}', Actual: '{actual_input}'"
                ));
            }
        }

        // Verify focused pane
        if let Some(expected_focus) = verify.focused_pane.as_deref() {
            let actual_focus = match app.test_focused_pane() {
                FocusedPane::Input => "input",
                FocusedPane::Tasks => "tasks",
                FocusedPane::Output => "output",
                FocusedPane::Threads => "threads",
            };
            if actual_focus == expected_focus {
                result.add_success(format!("Focused pane matches: '{expected_focus}'"));
            } else {
                result.add_failure(format!(
                    "Focused pane mismatch. Expected: '{expected_focus}', Actual: '{actual_focus}'"
                ));
            }
        }

        // Verify task count
        if let Some(expected_count) = verify.tasks_displayed {
            let actual_count = task_manager.task_order().len();
            if actual_count == expected_count {
                result.add_success(format!("Task count matches: {expected_count}"));
            } else {
                result.add_failure(format!(
                    "Task count mismatch. Expected: {expected_count}, Actual: {actual_count}"
                ));
            }
        }

        // Verify input cleared
        if let Some(expected_cleared) = verify.input_cleared {
            let actual_input = input_manager.input_area().lines().join("\n");
            let is_cleared = actual_input.is_empty();
            if is_cleared == expected_cleared {
                result.add_success(format!("Input cleared check matches: {expected_cleared}"));
            } else {
                result.add_failure(format!(
                    "Input cleared mismatch. Expected: {expected_cleared}, Actual: {is_cleared}"
                ));
            }
        }

        // Verify cursor position (it's a tuple (row, col))
        if let Some(expected_pos) = verify.cursor_position {
            let actual_pos = input_manager.input_area().cursor();
            // Assuming expected_pos is the column position (second element of tuple)
            if actual_pos.1 == expected_pos {
                result.add_success(format!("Cursor column position matches: {expected_pos}"));
            } else {
                result.add_failure(format!(
                    "Cursor column position mismatch. Expected: {expected_pos}, Actual: {}",
                    actual_pos.1
                ));
            }
        }

        // Verify pending tasks count
        if let Some(expected) = verify.pending_tasks_count {
            let actual = task_manager
                .task_order()
                .iter()
                .filter(|id| {
                    task_manager
                        .get_task(**id)
                        .is_some_and(|task| matches!(task.status, TaskStatus::Pending))
                })
                .count();
            if actual == expected {
                result.add_success(format!("Pending tasks count matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Pending tasks count mismatch. Expected: {expected}, Actual: {actual}"
                ));
            }
        }

        // Verify running tasks count
        if let Some(expected) = verify.running_tasks_count {
            let actual = task_manager
                .task_order()
                .iter()
                .filter(|id| {
                    task_manager
                        .get_task(**id)
                        .is_some_and(|t| matches!(t.status, TaskStatus::Running))
                })
                .count();
            if actual == expected {
                result.add_success(format!("Running tasks count matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Running tasks count mismatch. Expected: {expected}, Actual: {actual}"
                ));
            }
        }

        // Verify completed tasks count
        if let Some(expected) = verify.completed_tasks_count {
            let actual = task_manager
                .task_order()
                .iter()
                .filter(|id| {
                    task_manager
                        .get_task(**id)
                        .is_some_and(|t| matches!(t.status, TaskStatus::Completed))
                })
                .count();
            if actual == expected {
                result.add_success(format!("Completed tasks count matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Completed tasks count mismatch. Expected: {expected}, Actual: {actual}"
                ));
            }
        }

        // Verify failed tasks count
        if let Some(expected) = verify.failed_tasks_count {
            let actual = task_manager
                .task_order()
                .iter()
                .filter(|id| {
                    task_manager
                        .get_task(**id)
                        .is_some_and(|t| matches!(t.status, TaskStatus::Failed))
                })
                .count();
            if actual == expected {
                result.add_success(format!("Failed tasks count matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Failed tasks count mismatch. Expected: {expected}, Actual: {actual}"
                ));
            }
        }

        // Verify task descriptions visible
        if !verify.task_descriptions_visible.is_empty() {
            for expected_desc in &verify.task_descriptions_visible {
                let found = task_manager.task_order().iter().any(|id| {
                    task_manager
                        .get_task(*id)
                        .is_some_and(|t| t.description.contains(expected_desc))
                });
                if found {
                    result.add_success(format!("Task description visible: '{expected_desc}'"));
                } else {
                    result.add_failure(format!("Task description not found: '{expected_desc}'"));
                }
            }
        }

        // Verify all tasks completed
        if let Some(expected) = verify.all_tasks_completed {
            let total = task_manager.task_order().len();
            let completed = task_manager
                .task_order()
                .iter()
                .filter(|id| {
                    task_manager
                        .get_task(**id)
                        .is_some_and(|t| matches!(t.status, TaskStatus::Completed))
                })
                .count();
            let all_completed = total > 0 && completed == total;
            if all_completed == expected {
                result.add_success(format!("All tasks completed check matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "All tasks completed mismatch. Expected: {expected}, Actual: {all_completed} ({completed}/{total})"
                ));
            }
        }

        // Verify task created
        if let Some(expected) = verify.task_created {
            let has_tasks = !task_manager.task_order().is_empty();
            if has_tasks == expected {
                result.add_success(format!("Task created check matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Task created mismatch. Expected: {expected}, Actual: {has_tasks}"
                ));
            }
        }

        // Verify focus changed
        if let Some(expected) = verify.focus_changed {
            // This field indicates whether focus has changed - we verify by checking if focused pane is not Input
            let focus_changed = !matches!(app.test_focused_pane(), FocusedPane::Input);
            if focus_changed == expected {
                result.add_success(format!("Focus changed check matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Focus changed mismatch. Expected: {expected}, Actual: {focus_changed}"
                ));
            }
        }

        // Verify task status (for selected task)
        if let Some(expected_status) = &verify.task_status {
            if let Some(active_task_id) = _state.active_task_id {
                if let Some(task) = task_manager.get_task(active_task_id) {
                    let actual_status = match task.status {
                        TaskStatus::Pending => "pending",
                        TaskStatus::Running => "running",
                        TaskStatus::Completed => "completed",
                        TaskStatus::Failed => "failed",
                    };
                    if actual_status == expected_status {
                        result.add_success(format!("Task status matches: {expected_status}"));
                    } else {
                        result.add_failure(format!(
                            "Task status mismatch. Expected: {expected_status}, Actual: {actual_status}"
                        ));
                    }
                } else {
                    result.add_failure("Active task not found in task manager".to_owned());
                }
            } else {
                result.add_failure("Expected task status but no active task selected".to_owned());
            }
        }

        // Verify selected task description contains
        if let Some(expected_text) = &verify.selected_task_contains {
            if let Some(active_task_id) = _state.active_task_id {
                if let Some(task) = task_manager.get_task(active_task_id) {
                    if task.description.contains(expected_text) {
                        result.add_success(format!(
                            "Selected task description contains: '{expected_text}'"
                        ));
                    } else {
                        result.add_failure(format!(
                            "Selected task description doesn't contain '{expected_text}'. Actual: '{}'",
                            task.description
                        ));
                    }
                } else {
                    result.add_failure("Active task not found in task manager".to_owned());
                }
            } else {
                result.add_failure(
                    "Expected selected task description but no active task".to_owned(),
                );
            }
        }

        // Verify output contains patterns
        if !verify.output_contains.is_empty() {
            for expected_pattern in &verify.output_contains {
                // Check if any task's output contains the pattern
                let found = task_manager.task_order().iter().any(|id| {
                    task_manager
                        .get_task(*id)
                        .is_some_and(|task| task.output.contains(expected_pattern))
                });
                if found {
                    result.add_success(format!("Output contains pattern: '{expected_pattern}'"));
                } else {
                    result.add_failure(format!("Output doesn't contain pattern: '{expected_pattern}'"));
                }
            }
        }

        // Verify output does not contain patterns
        if !verify.output_not_contains.is_empty() {
            for unexpected_pattern in &verify.output_not_contains {
                // Check that no task's output contains the pattern
                let found = task_manager.task_order().iter().any(|id| {
                    task_manager
                        .get_task(*id)
                        .is_some_and(|task| task.output.contains(unexpected_pattern))
                });
                if found {
                    result.add_failure(format!(
                        "Output contains unexpected pattern: '{unexpected_pattern}'"
                    ));
                } else {
                    result.add_success(format!(
                        "Output correctly doesn't contain: '{unexpected_pattern}'"
                    ));
                }
            }
        }

        // Verify placeholder visible
        if let Some(expected) = verify.placeholder_visible {
            // Placeholder is visible when there are no tasks and input is empty
            let placeholder_visible =
                task_manager.task_order().is_empty() && input_manager.input_area().lines().is_empty();
            if placeholder_visible == expected {
                result.add_success(format!("Placeholder visible check matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Placeholder visible mismatch. Expected: {expected}, Actual: {placeholder_visible}"
                ));
            }
        }

        // Verify thread count using thread store
        if let Some(expected) = verify.thread_count {
            let thread_store = app.test_thread_store();
            let actual = thread_store.active_threads().len();
            if actual == expected {
                result.add_success(format!("Thread count matches: {expected}"));
            } else {
                result.add_failure(format!(
                    "Thread count mismatch. Expected: {expected}, Actual: {actual}"
                ));
            }
        }

        // Verify selected thread ID
        if let Some(expected_id) = &verify.selected_thread_id {
            if let Some(actual_id) = &_state.active_thread_id {
                if expected_id == "any" || actual_id.to_string().contains(expected_id) {
                    result.add_success(format!("Selected thread ID check passed: '{expected_id}'"));
                } else {
                    result.add_failure(format!(
                        "Selected thread ID mismatch. Expected contains: '{expected_id}', Actual: '{actual_id}'"
                    ));
                }
            } else {
                result.add_failure(format!(
                    "Expected thread ID '{expected_id}' but no thread selected"
                ));
            }
        }

        // Verify thread names visible
        if !verify.thread_names_visible.is_empty() {
            let thread_store = app.test_thread_store();
            let threads = thread_store.active_threads();
            for expected_name in &verify.thread_names_visible {
                let found = threads.iter().any(|t| t.name.contains(expected_name));
                if found {
                    result.add_success(format!("Thread name visible: '{expected_name}'"));
                } else {
                    result.add_failure(format!("Thread name not found: '{expected_name}'"));
                }
            }
        }
    }

    /// Verify state
    pub fn verify_state(
        result: &mut VerificationResult,
        tui_app: Option<&TuiApp<TestBackend>>,
        verify: &StateVerify,
    ) {
        let Some(app) = tui_app else {
            return;
        };

        let state = app.test_state();

        // Verify conversation count
        if let Some(expected_count) = verify.conversation_count {
            let actual_count = state.conversation_history.len();
            if actual_count == expected_count {
                result.add_success(format!("Conversation count matches: {expected_count}"));
            } else {
                result.add_failure(format!(
                    "Conversation count mismatch. Expected: {expected_count}, Actual: {actual_count}"
                ));
            }
        }

        // Verify active thread
        if let Some(expected_thread) = &verify.selected_task {
            let has_active_thread = state.active_thread_id.is_some();
            if expected_thread == "any" && has_active_thread {
                result.add_success("Has active thread".to_owned());
            } else if !has_active_thread {
                result.add_failure(format!(
                    "Expected thread '{expected_thread}' but none active"
                ));
            }
        }
    }
}
