//! State verification logic.

use crate::fixture::StateVerify;
use crate::verification_result::VerificationResult;
use merlin_cli::TuiApp;
use merlin_deps::ratatui::backend::TestBackend;

/// Verify state
pub fn verify_state(
    result: &mut VerificationResult,
    tui_app: Option<&TuiApp<TestBackend>>,
    verify: &StateVerify,
) {
    let Some(app) = tui_app else {
        return;
    };

    let state = &app.state;

    // Verify conversation count (excluding system messages)
    if let Some(expected_count) = verify.conversation_count {
        use merlin_cli::ConversationRole;
        // Only count user and assistant messages, not system messages
        let actual_count = state
            .conversation_history
            .iter()
            .filter(|entry| {
                matches!(
                    entry.role,
                    ConversationRole::User | ConversationRole::Assistant
                )
            })
            .count();
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
