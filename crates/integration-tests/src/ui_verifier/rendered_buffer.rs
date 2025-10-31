//! Rendered buffer verification logic.

use crate::fixture::UiVerify;
use crate::verification_result::VerificationResult;
use merlin_cli::TuiApp;
use merlin_deps::ratatui::backend::TestBackend;
use merlin_deps::ratatui::buffer::Buffer;

/// Verify rendered buffer patterns (contains and not contains)
pub fn verify_rendered_buffer(
    result: &mut VerificationResult,
    tui_app: &TuiApp<TestBackend>,
    verify: &UiVerify,
) {
    verify_rendered_buffer_contains(result, tui_app, verify);
    verify_rendered_buffer_not_contains(result, tui_app, verify);
}

/// Convert buffer to string representation
fn buffer_to_string(buffer: &Buffer) -> String {
    let area = buffer.area();
    let mut result = String::new();

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buffer.cell((x, y)) {
                result.push_str(cell.symbol());
            }
        }
        if y < area.bottom() - 1 {
            result.push('\n');
        }
    }

    result
}

/// Verify rendered buffer contains expected patterns
fn verify_rendered_buffer_contains(
    result: &mut VerificationResult,
    tui_app: &TuiApp<TestBackend>,
    verify: &UiVerify,
) {
    if verify.rendered_buffer_contains.is_empty() {
        return;
    }

    let backend = tui_app.terminal.backend();
    let buffer_content = buffer_to_string(backend.buffer());

    for expected_pattern in &verify.rendered_buffer_contains {
        if buffer_content.contains(expected_pattern) {
            result.add_success(format!(
                "Rendered buffer contains pattern: '{expected_pattern}'"
            ));
        } else {
            result.add_failure(format!(
                "Rendered buffer doesn't contain pattern: '{expected_pattern}'"
            ));
        }
    }
}

/// Verify rendered buffer does not contain unexpected patterns
fn verify_rendered_buffer_not_contains(
    result: &mut VerificationResult,
    tui_app: &TuiApp<TestBackend>,
    verify: &UiVerify,
) {
    if verify.rendered_buffer_not_contains.is_empty() {
        return;
    }

    let backend = tui_app.terminal.backend();
    let buffer_content = buffer_to_string(backend.buffer());

    for unexpected_pattern in &verify.rendered_buffer_not_contains {
        if buffer_content.contains(unexpected_pattern) {
            result.add_failure(format!(
                "Rendered buffer contains unexpected pattern: '{unexpected_pattern}'"
            ));
        } else {
            result.add_success(format!(
                "Rendered buffer correctly doesn't contain: '{unexpected_pattern}'"
            ));
        }
    }
}
