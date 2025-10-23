//! TUI testing helpers using `InputEventSource` pattern

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use merlin_cli::ui::event_source::InputEventSource;
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Test event source that provides pre-programmed events
#[derive(Clone)]
pub struct TestEventSource {
    events: Arc<Mutex<VecDeque<Event>>>,
}

impl TestEventSource {
    /// Create a new test event source
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Add a key event
    ///
    /// # Panics
    /// Panics if the event queue lock is poisoned
    pub fn push_key(&self, code: KeyCode, modifiers: KeyModifiers) {
        let mut events = self.events.lock().unwrap_or_else(PoisonError::into_inner);
        events.push_back(Event::Key(KeyEvent::new(code, modifiers)));
    }

    /// Add text input (converts to individual key presses)
    pub fn push_text(&self, text: &str) {
        for char in text.chars() {
            self.push_key(KeyCode::Char(char), KeyModifiers::NONE);
        }
    }

    /// Add Enter key
    pub fn push_enter(&self) {
        self.push_key(KeyCode::Enter, KeyModifiers::NONE);
    }

    /// Add Escape key
    pub fn push_escape(&self) {
        self.push_key(KeyCode::Esc, KeyModifiers::NONE);
    }

    /// Add Tab key
    pub fn push_tab(&self) {
        self.push_key(KeyCode::Tab, KeyModifiers::NONE);
    }

    /// Add arrow key
    pub fn push_arrow(&self, direction: ArrowDirection) {
        let code = match direction {
            ArrowDirection::UpArrow => KeyCode::Up,
            ArrowDirection::DownArrow => KeyCode::Down,
            ArrowDirection::LeftArrow => KeyCode::Left,
            ArrowDirection::RightArrow => KeyCode::Right,
        };
        self.push_key(code, KeyModifiers::NONE);
    }

    /// Check if there are pending events
    ///
    /// # Panics
    /// Panics if the event queue lock is poisoned
    #[must_use]
    pub fn has_events(&self) -> bool {
        let events = self.events.lock().unwrap_or_else(PoisonError::into_inner);
        !events.is_empty()
    }

    /// Get number of pending events
    ///
    /// # Panics
    /// Panics if the event queue lock is poisoned
    #[must_use]
    pub fn event_count(&self) -> usize {
        let events = self.events.lock().unwrap_or_else(PoisonError::into_inner);
        events.len()
    }
}

impl Default for TestEventSource {
    fn default() -> Self {
        Self::new()
    }
}

impl InputEventSource for TestEventSource {
    fn read(&mut self) -> io::Result<Event> {
        let mut events = self.events.lock().unwrap_or_else(PoisonError::into_inner);
        events
            .pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::WouldBlock, "No more events"))
    }

    fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
        let events = self.events.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(!events.is_empty())
    }
}

/// Arrow key directions
#[derive(Debug, Clone, Copy)]
pub enum ArrowDirection {
    /// Up arrow
    UpArrow,
    /// Down arrow
    DownArrow,
    /// Left arrow
    LeftArrow,
    /// Right arrow
    RightArrow,
}

/// Helper to convert string key names to `KeyCode`
///
/// # Errors
/// Returns error if key name is not recognized
///
/// # Panics
/// Panics if a single-character string doesn't have exactly one character
pub fn parse_key(key: &str) -> Result<(KeyCode, KeyModifiers), String> {
    let mut modifiers = KeyModifiers::NONE;
    let mut parts: Vec<&str> = key.split('+').collect();

    // Extract modifiers
    while parts.len() > 1 {
        match parts[0].to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            _ => return Err(format!("Unknown modifier: {}", parts[0])),
        }
        parts.remove(0);
    }

    let code = match parts[0].to_lowercase().as_str() {
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "tab" => KeyCode::Tab,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        single if single.len() == 1 => KeyCode::Char(
            single
                .chars()
                .next()
                .ok_or_else(|| "Empty key name".to_owned())?,
        ),
        unknown => return Err(format!("Unknown key: {unknown}")),
    };

    Ok((code, modifiers))
}
