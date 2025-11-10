//! Layout calculation utilities for UI components
//!
//! This module provides centralized layout calculations and caching of actual
//! rendered areas to ensure consistency between rendering and scroll calculations.

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

    /// Gets the output viewport height (excluding borders)
    ///
    /// Returns the actual content height available for scrollable output.
    /// Accounts for borders only (2) - horizontal padding doesn't affect height.
    pub fn output_viewport_height(&self) -> u16 {
        self.output_area
            .map_or(0, |(_, height)| height.saturating_sub(2))
    }
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

    /// Tests layout cache viewport height calculation.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_layout_cache_viewport_height() {
        let cache = LayoutCache::new();
        assert_eq!(cache.output_viewport_height(), 0); // No area set yet
    }

    /// Tests input area height calculation.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_input_area_height() {
        assert_eq!(calculate_input_area_height(1), 3); // Minimum
        assert_eq!(calculate_input_area_height(3), 5); // 3 + 2
        assert_eq!(calculate_input_area_height(0), 3); // Minimum even with 0
    }
}
