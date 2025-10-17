//! Layout calculation utilities for UI components
//!
//! This module provides centralized layout calculations and caching of actual
//! rendered areas to ensure consistency between rendering and scroll calculations.

use super::renderer::FocusedPane;

/// Cache of actual rendered layout dimensions
///
/// Stores the real dimensions calculated by ratatui's Layout system during rendering.
/// This ensures scroll calculations use the exact same dimensions as the renderer.
#[derive(Debug, Clone, Default)]
pub struct LayoutCache {
    /// Actual output area rectangle (content + borders + padding)
    pub output_area: Option<(u16, u16)>, // (width, height)
}

impl LayoutCache {
    /// Creates a new empty layout cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores the output area dimensions from actual rendering
    pub fn set_output_area(&mut self, width: u16, height: u16) {
        self.output_area = Some((width, height));
    }

    /// Gets the output viewport height (excluding borders)
    ///
    /// Returns the actual content height available for scrollable output.
    /// Accounts for borders only (2) - horizontal padding doesn't affect height.
    pub fn output_viewport_height(&self) -> u16 {
        self.output_area
            .map_or(0, |(_, height)| height.saturating_sub(2))
    }
}

/// Calculates task area height constraints based on focus and content
///
/// # Arguments
/// * `terminal_height` - Total terminal height
/// * `task_content_lines` - Number of lines of task content
/// * `focused` - Which pane currently has focus
///
/// # Returns
/// The height to allocate for the task area (including borders)
pub fn calculate_task_area_height(
    terminal_height: u16,
    task_content_lines: u16,
    focused: FocusedPane,
) -> u16 {
    // Determine maximum task area height based on focus
    let max_task_area_height = if focused == FocusedPane::Tasks {
        // When Tasks pane is focused, allow up to 60% of screen height
        let max_height = (terminal_height * 60) / 100;
        // Ensure at least 10 lines remain for input
        max_height.min(terminal_height.saturating_sub(10))
    } else if focused == FocusedPane::Output {
        // When Output pane is focused, limit task list to 5 lines total
        5
    } else {
        // Default: use full height
        terminal_height
    };

    // Size to content but don't exceed max, add 2 for borders
    (task_content_lines + 2).min(max_task_area_height)
}

/// Calculates input area height based on content
///
/// # Arguments
/// * `input_content_lines` - Number of lines of input content
///
/// # Returns
/// The height to allocate for the input area (including borders)
pub fn calculate_input_area_height(input_content_lines: u16) -> u16 {
    // Content lines + 2 for borders, minimum 3 (2 borders + 1 content line)
    (input_content_lines + 2).max(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_cache_viewport_height() {
        let mut cache = LayoutCache::new();
        assert_eq!(cache.output_viewport_height(), 0); // No area set yet

        cache.set_output_area(80, 20);
        assert_eq!(cache.output_viewport_height(), 16); // 20 - 4 (borders/padding)

        cache.set_output_area(80, 10);
        assert_eq!(cache.output_viewport_height(), 6); // 10 - 4
    }

    #[test]
    fn test_task_area_height_focused() {
        // When tasks focused, allow up to 60%
        let height = calculate_task_area_height(30, 10, FocusedPane::Tasks);
        // 60% of 30 = 18, content is 10+2=12, so 12
        assert_eq!(height, 12);
    }

    #[test]
    fn test_task_area_height_output_focused() {
        // When output focused, limit to 5
        let height = calculate_task_area_height(30, 10, FocusedPane::Output);
        assert_eq!(height, 5);
    }

    #[test]
    fn test_input_area_height() {
        assert_eq!(calculate_input_area_height(1), 3); // Minimum
        assert_eq!(calculate_input_area_height(3), 5); // 3 + 2
        assert_eq!(calculate_input_area_height(0), 3); // Minimum even with 0
    }
}
