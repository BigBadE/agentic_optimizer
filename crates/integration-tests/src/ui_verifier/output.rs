//! Output pattern verification logic.

use crate::verification_result::VerificationResult;
use crate::verify::UiVerify;
use merlin_cli::ui::task_manager::TaskManager;

/// Verify output patterns (contains and not contains)
pub fn verify_output_patterns(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    verify_output_contains_patterns(result, task_manager, verify);
    verify_output_not_contains_patterns(result, task_manager, verify);
}

/// Verify output contains expected patterns
fn verify_output_contains_patterns(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    if verify.output_contains.is_empty() {
        return;
    }

    for expected_pattern in &verify.output_contains {
        let found = task_manager.task_order().iter().any(|task_id| {
            task_manager
                .get_task(*task_id)
                .is_some_and(|task| task.output.contains(expected_pattern))
        });
        if found {
            result.add_success(format!("Output contains pattern: '{expected_pattern}'"));
        } else {
            result.add_failure(format!(
                "Output doesn't contain pattern: '{expected_pattern}'"
            ));
        }
    }
}

/// Verify output does not contain unexpected patterns
fn verify_output_not_contains_patterns(
    result: &mut VerificationResult,
    task_manager: &TaskManager,
    verify: &UiVerify,
) {
    if verify.output_not_contains.is_empty() {
        return;
    }

    for unexpected_pattern in &verify.output_not_contains {
        let found = task_manager.task_order().iter().any(|task_id| {
            task_manager
                .get_task(*task_id)
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
