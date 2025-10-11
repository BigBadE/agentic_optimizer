//! Test to verify the auto-wrap fix - repeated wrapping should not corrupt text
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

#[test]
fn test_repeated_autowrap_no_corruption() {
    // This test verifies that processing events in batches (which causes multiple auto-wraps)
    // doesn't corrupt the text by adding extra spaces

    let text = "a".repeat(1000);
    let app = test_with_typing(&text);

    // Count actual characters - should be exactly 1000
    let char_count: usize = app.get_input_lines().iter().map(String::len).sum();
    assert_eq!(
        char_count, 1000,
        "Auto-wrap should not add or remove characters"
    );
}

#[test]
fn test_text_with_spaces_preserved() {
    // Verify that text with actual spaces is preserved correctly through wrapping

    let text = "hello world this is a test of text wrapping with spaces between words";
    let app = test_with_typing(text);

    // Join all lines and compare with original
    let result = app.get_input_lines().join("");
    assert_eq!(result, text, "Spaces should be preserved through wrapping");
}

#[test]
fn test_mixed_content_wrapping() {
    // Test with a mix of text with and without spaces

    let text = format!("{}test with spaces{}", "a".repeat(100), "a".repeat(100));
    let app = test_with_typing(&text);

    let result = app.get_input_lines().join("");
    assert_eq!(result, text, "Mixed content should wrap correctly");
}
