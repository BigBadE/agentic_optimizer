use merlin_deps::crossterm::event::{Event, KeyCode};
use merlin_deps::ratatui::{
    style::{Modifier, Style},
    widgets::{Block, Borders},
};
use merlin_deps::tui_textarea::{CursorMove, TextArea};
use std::collections::HashSet;

/// Manages input area state and text wrapping
pub struct InputManager {
    input_area: TextArea<'static>,
    manual_newlines: HashSet<usize>,
}

impl InputManager {
    /// Gets a reference to the input area
    pub fn input_area(&self) -> &TextArea<'static> {
        &self.input_area
    }

    /// Handles a key event and optionally auto-wraps if needed
    ///
    /// **For testing:** Do not call this directly. Use a custom `InputEventSource` instead.
    /// See TESTS.md for proper testing patterns.
    ///
    /// # Parameters
    /// - `event`: The key event to handle
    /// - `max_line_width`: Maximum line width for auto-wrapping (None to skip wrapping)
    pub fn handle_input(&mut self, event: &Event, max_line_width: Option<usize>) {
        if let Event::Key(key) = event {
            self.input_area.input(Event::Key(*key));

            // Auto-wrap if this is a text-modifying event and width is specified
            if let Some(width) = max_line_width
                && matches!(
                    key.code,
                    KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
                )
            {
                self.auto_wrap(width);
            }
        }
    }

    /// Inserts a newline at the current position
    pub fn insert_newline_at_cursor(&mut self) {
        self.input_area.insert_newline();
    }

    /// Records a manual newline at the current cursor position
    pub fn record_manual_newline(&mut self) {
        let (row, _) = self.input_area.cursor();
        self.manual_newlines.insert(row);
    }

    /// Auto-wraps the input text to fit within the specified width
    fn auto_wrap(&mut self, max_line_width: usize) {
        let lines = self.input_area.lines().to_vec();
        let (cursor_row, _cursor_col) = self.input_area.cursor();

        // Don't wrap if only one line and it fits
        if lines.len() == 1 && lines[0].len() <= max_line_width {
            return;
        }

        // OPTIMIZATION: Only wrap the current line if it exceeds the limit
        // This prevents re-wrapping already-wrapped text which causes corruption
        if cursor_row < lines.len() && lines[cursor_row].len() > max_line_width {
            // Only the current line needs wrapping - insert a newline at the width limit
            let current_line = &lines[cursor_row];

            // Find a good break point (prefer spaces)
            let break_point = current_line[..max_line_width]
                .rfind(' ')
                .map_or(max_line_width, |space_idx| space_idx + 1);

            // Split the line by deleting from break point to end, inserting newline, then re-inserting text
            let remaining_text = current_line[break_point..].to_string();
            let chars_to_delete = current_line.len() - break_point;

            // Move cursor to break point
            self.input_area
                .move_cursor(CursorMove::Jump(cursor_row as u16, break_point as u16));

            // Delete everything after break point
            for _ in 0..chars_to_delete {
                self.input_area.delete_next_char();
            }

            // Insert newline
            self.input_area.insert_newline();

            // Insert remaining text
            for char_to_insert in remaining_text.chars() {
                self.input_area.insert_char(char_to_insert);
            }

            // Record this as a manual newline (actually auto, but treat as manual to preserve it)
            self.manual_newlines.insert(cursor_row);
        }
    }

    /// Clears the input area
    pub fn clear(&mut self) {
        self.input_area = TextArea::default();
        self.input_area
            .set_block(Block::default().borders(Borders::ALL).title("Input"));
        self.input_area.set_cursor_line_style(Style::default());
        self.input_area
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        self.manual_newlines.clear();
    }
}

impl Default for InputManager {
    fn default() -> Self {
        let mut input_area = TextArea::default();
        input_area.set_block(Block::default().borders(Borders::ALL).title("Input"));
        input_area.set_cursor_line_style(Style::default());

        // Ensure at least one empty line for proper rendering
        if input_area.lines().is_empty() {
            input_area.insert_char(' ');
            input_area.delete_char();
        }

        Self {
            input_area,
            manual_newlines: HashSet::default(),
        }
    }
}
