//! Input field verification logic.

use crate::fixture::UiVerify;
use crate::verification_result::VerificationResult;
use merlin_cli::ui::input::InputManager;

/// Verify input-related fields (text, cleared, cursor position)
pub fn verify_input_related_fields(
    result: &mut VerificationResult,
    input_manager: &InputManager,
    verify: &UiVerify,
) {
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

    if let Some(expected_pos) = verify.cursor_position {
        let actual_pos = input_manager.input_area().cursor();
        if actual_pos.1 == expected_pos {
            result.add_success(format!("Cursor column position matches: {expected_pos}"));
        } else {
            result.add_failure(format!(
                "Cursor column position mismatch. Expected: {expected_pos}, Actual: {}",
                actual_pos.1
            ));
        }
    }
}
