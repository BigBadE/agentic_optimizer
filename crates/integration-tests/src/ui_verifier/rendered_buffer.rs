//! Rendered buffer verification logic.

use crate::verification_result::VerificationResult;
use crate::verify::{RenderedRegionVerify, UiVerify};
use merlin_cli::TuiApp;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

/// Verify rendered buffer patterns (contains and not contains)
pub fn verify_rendered_buffer(
    result: &mut VerificationResult,
    tui_app: &TuiApp<TestBackend>,
    verify: &UiVerify,
) {
    verify_rendered_buffer_contains(result, tui_app, verify);
    verify_rendered_buffer_not_contains(result, tui_app, verify);
    verify_rendered_buffer_regions(result, tui_app, verify);
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
            // Show first 500 chars of buffer for debugging (char-aware truncation)
            let preview = buffer_content.chars().take(500).collect::<String>();
            let preview = if buffer_content.chars().count() > 500 {
                format!("{preview}...")
            } else {
                preview
            };
            result.add_failure(format!(
                "Rendered buffer doesn't contain pattern: '{expected_pattern}'. Buffer preview: {preview}"
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

/// Find a region in the rendered buffer by searching for boundary markers
fn find_region_by_boundary(buffer_content: &str, region_name: &str) -> Option<String> {
    // Common region boundary patterns (note: titles have trailing spaces)
    let boundaries = match region_name.to_lowercase().as_str() {
        "tasks" => vec!["─── Tasks ", "── Tasks ", "Tasks ───"],
        "output" => vec!["─── Output ", "── Output ", "Output ───"],
        "input" => vec!["─── Input ", "── Input ", "Input ───"],
        "threads" => vec!["─── Threads ", "── Threads ", "Threads ───"],
        _ => return None,
    };

    // Find the start of the region
    let mut start_idx = None;
    let mut end_marker = None;

    for boundary in &boundaries {
        if let Some(idx) = buffer_content.find(boundary) {
            start_idx = Some(idx);
            end_marker = Some(*boundary);
            break;
        }
    }

    let start_idx = start_idx?;
    let end_marker = end_marker?;

    // Extract content from after the boundary marker to the next boundary or end
    let content_start = start_idx + end_marker.len();
    let remaining = &buffer_content[content_start..];

    // Find the next boundary marker (indicating next region) or take rest
    let next_boundary_patterns = ["─── ", "── "];
    let mut end_idx = remaining.len();

    for pattern in &next_boundary_patterns {
        if let Some(idx) = remaining.find(pattern)
            && idx < end_idx
            && idx > 0
        {
            end_idx = idx;
        }
    }

    Some(remaining[..end_idx].to_owned())
}

/// Verify rendered buffer regions
fn verify_rendered_buffer_regions(
    result: &mut VerificationResult,
    tui_app: &TuiApp<TestBackend>,
    verify: &UiVerify,
) {
    if verify.rendered_buffer_regions.is_empty() {
        return;
    }

    let backend = tui_app.terminal.backend();
    let full_buffer = buffer_to_string(backend.buffer());

    for region_verify in &verify.rendered_buffer_regions {
        verify_single_region(result, &full_buffer, region_verify);
    }
}

/// Verify a single rendered region
fn verify_single_region(
    result: &mut VerificationResult,
    full_buffer: &str,
    region_verify: &RenderedRegionVerify,
) {
    let region_name = &region_verify.region;

    // Try to extract the region by boundary markers
    let Some(region_content) = find_region_by_boundary(full_buffer, region_name) else {
        // Check if any boundary marker exists at all
        let all_boundaries = ["─── Tasks ", "─── Output ", "─── Input ", "─── Threads "];
        let found_any = all_boundaries
            .iter()
            .filter(|boundary| full_buffer.contains(*boundary))
            .collect::<Vec<_>>();

        let debug_info = if found_any.is_empty() {
            "No UI boundaries found in buffer at all".to_owned()
        } else {
            format!("Found boundaries: {found_any:?}")
        };

        result.add_failure(format!(
            "Could not find region '{region_name}' in rendered buffer. {debug_info}"
        ));
        return;
    };

    // Verify patterns that should appear
    for pattern in &region_verify.contains {
        if region_content.contains(pattern) {
            result.add_success(format!(
                "Region '{region_name}' contains pattern: '{pattern}'"
            ));
        } else {
            result.add_failure(format!(
                "Region '{region_name}' doesn't contain expected pattern: '{pattern}'"
            ));
        }
    }

    // Verify patterns that should NOT appear
    for pattern in &region_verify.not_contains {
        if region_content.contains(pattern) {
            result.add_failure(format!(
                "Region '{region_name}' contains unexpected pattern: '{pattern}'"
            ));
        } else {
            result.add_success(format!(
                "Region '{region_name}' correctly doesn't contain: '{pattern}'"
            ));
        }
    }

    // Verify line ordering
    if !region_verify.lines_in_order.is_empty() {
        verify_lines_in_order(
            result,
            region_name,
            &region_content,
            &region_verify.lines_in_order,
        );
    }
}

/// Verify that lines appear in the specified order (not necessarily consecutive)
fn verify_lines_in_order(
    result: &mut VerificationResult,
    region_name: &str,
    region_content: &str,
    expected_lines: &[String],
) {
    let mut last_position = 0;
    let mut all_in_order = true;

    for expected_line in expected_lines {
        if let Some(pos) = region_content[last_position..].find(expected_line) {
            last_position += pos + expected_line.len();
        } else {
            result.add_failure(format!(
                "Region '{region_name}' missing expected line in sequence: '{expected_line}'"
            ));
            all_in_order = false;
            break;
        }
    }

    if all_in_order {
        result.add_success(format!(
            "Region '{region_name}' has all {} lines in correct order",
            expected_lines.len()
        ));
    }
}
