use ratatui::{
    style::{Modifier, Style},
    widgets::{Block, Borders},
};
use std::borrow::Cow;
use std::collections::HashSet;
use textwrap::{Options, WordSeparator, wrap};
use tui_textarea::{CursorMove, TextArea};

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

    /// Gets a mutable reference to the input area
    pub fn input_area_mut(&mut self) -> &mut TextArea<'static> {
        &mut self.input_area
    }

    /// Records a manual newline at the current cursor position
    pub fn record_manual_newline(&mut self) {
        let (row, _) = self.input_area.cursor();
        self.manual_newlines.insert(row);
    }

    /// Auto-wraps the input text to fit within the specified width
    pub fn auto_wrap(&mut self, max_line_width: usize) {
        let lines = self.input_area.lines().to_vec();
        let (cursor_row, cursor_col) = self.input_area.cursor();

        // Don't wrap if only one line and it fits
        if lines.len() == 1 && lines[0].len() <= max_line_width {
            return;
        }

        let paragraphs = self.split_into_paragraphs(&lines);
        let cursor_info = Self::calculate_cursor_info(&paragraphs, cursor_row, cursor_col);
        let wrapped_result = Self::wrap_paragraphs(&paragraphs, max_line_width, &cursor_info);

        // Only update if content changed
        if wrapped_result.lines != lines {
            self.update_input_area(wrapped_result);
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

    // Private helper methods

    fn split_into_paragraphs(&self, lines: &[String]) -> Vec<Vec<String>> {
        let mut paragraphs: Vec<Vec<String>> = Vec::default();
        let mut current_para: Vec<String> = Vec::default();

        for (idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                if !current_para.is_empty() {
                    paragraphs.push(current_para);
                    current_para = Vec::default();
                }
                paragraphs.push(vec![String::default()]);
            } else {
                current_para.push(line.clone());

                if self.manual_newlines.contains(&idx) {
                    paragraphs.push(current_para);
                    current_para = Vec::default();
                }
            }
        }

        if !current_para.is_empty() {
            paragraphs.push(current_para);
        }

        paragraphs
    }

    fn calculate_cursor_info(
        paragraphs: &[Vec<String>],
        cursor_row: usize,
        cursor_col: usize,
    ) -> CursorInfo {
        let mut cursor_paragraph = 0;
        let mut pos_in_paragraph = 0;
        let mut line_count = 0;

        for (para_idx, para) in paragraphs.iter().enumerate() {
            let para_line_count = if para.len() == 1 && para[0].is_empty() {
                1
            } else {
                para.len()
            };

            if cursor_row >= line_count + para_line_count {
                line_count += para_line_count;
                continue;
            }

            cursor_paragraph = para_idx;
            let line_in_para = cursor_row - line_count;

            for (line_idx, line) in para.iter().enumerate().take(line_in_para) {
                pos_in_paragraph += line.len();
                if line_idx > 0 {
                    pos_in_paragraph += 1;
                }
            }
            pos_in_paragraph += cursor_col;
            break;
        }

        CursorInfo {
            paragraph: cursor_paragraph,
            position: pos_in_paragraph,
        }
    }

    fn wrap_paragraphs(
        paragraphs: &[Vec<String>],
        max_line_width: usize,
        cursor_info: &CursorInfo,
    ) -> WrappedResult {
        let mut new_lines: Vec<String> = Vec::default();
        let mut new_cursor_row = 0;
        let mut new_cursor_col = 0;
        let mut found_cursor = false;
        let mut new_manual_newlines = HashSet::default();

        for (para_idx, para) in paragraphs.iter().enumerate() {
            if para.len() == 1 && para[0].is_empty() {
                new_lines.push(String::default());
                if para_idx < cursor_info.paragraph {
                    new_cursor_row += 1;
                }
            } else {
                let wrapped = wrap_paragraph(para, max_line_width);

                if para_idx == cursor_info.paragraph && !found_cursor {
                    let (row_offset, col) = find_cursor_position(&wrapped, cursor_info.position);
                    new_cursor_row += row_offset;
                    new_cursor_col = col;
                    found_cursor = true;
                } else if para_idx < cursor_info.paragraph {
                    new_cursor_row += wrapped.len();
                }

                new_lines.extend(wrapped);

                if para_idx < paragraphs.len() - 1 {
                    new_manual_newlines.insert(new_lines.len() - 1);
                }
            }
        }

        WrappedResult {
            lines: new_lines,
            cursor_row: new_cursor_row,
            cursor_col: new_cursor_col,
            manual_newlines: new_manual_newlines,
        }
    }

    fn update_input_area(&mut self, wrapped: WrappedResult) {
        let mut new_input = TextArea::new(wrapped.lines);

        if let Some(block) = self.input_area.block() {
            new_input.set_block(block.clone());
        }
        new_input.set_style(self.input_area.style());
        new_input.set_cursor_style(self.input_area.cursor_style());
        new_input.set_cursor_line_style(self.input_area.cursor_line_style());

        new_input.move_cursor(CursorMove::Jump(
            wrapped.cursor_row as u16,
            wrapped.cursor_col as u16,
        ));

        self.input_area = new_input;
        self.manual_newlines = wrapped.manual_newlines;
    }
}

impl Default for InputManager {
    fn default() -> Self {
        let mut input_area = TextArea::default();
        input_area.set_block(Block::default().borders(Borders::ALL).title("Input"));
        input_area.set_cursor_line_style(Style::default());

        Self {
            input_area,
            manual_newlines: HashSet::default(),
        }
    }
}

// Helper structures and functions

struct CursorInfo {
    paragraph: usize,
    position: usize,
}

struct WrappedResult {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    manual_newlines: HashSet<usize>,
}

/// Wraps a paragraph to fit within max width
fn wrap_paragraph(para: &[String], max_line_width: usize) -> Vec<String> {
    if para.len() == 1 && para[0].len() < max_line_width {
        return vec![para[0].clone()];
    }

    let para_text = para.join(" ");
    let ends_with_space = para_text.ends_with(' ');

    let options = Options::new(max_line_width)
        .break_words(true)
        .word_separator(WordSeparator::AsciiSpace);

    let mut wrapped_lines: Vec<String> = wrap(&para_text, options)
        .into_iter()
        .map(Cow::into_owned)
        .collect();

    if ends_with_space
        && !wrapped_lines.is_empty()
        && let Some(last) = wrapped_lines.last_mut()
    {
        last.push(' ');
    }

    wrapped_lines
}

/// Finds cursor position within wrapped lines
fn find_cursor_position(lines: &[String], cursor_pos: usize) -> (usize, usize) {
    if lines.is_empty() {
        return (0, 0);
    }

    let mut chars_seen = 0;
    for (row, line) in lines.iter().enumerate() {
        let line_len = line.len();

        if chars_seen + line_len >= cursor_pos {
            let col = cursor_pos - chars_seen;
            return (row, col);
        }

        chars_seen += line_len + 1;
    }

    let last_row = lines.len() - 1;
    let last_col = lines[last_row].len();
    (last_row, last_col)
}
