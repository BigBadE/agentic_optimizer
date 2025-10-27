//! Conversation threading system types.
//!
//! This module provides the core types for the unified thread-based conversation system,
//! including threads, messages, work units, and subtasks.

use serde::{Deserialize, Serialize};
use std::fmt;

// Submodules
mod ids;
mod types;
mod work;

// Re-export all public types
pub use ids::{MessageId, SubtaskId, ThreadId, WorkUnitId};
pub use types::{BranchPoint, Message, Thread};
pub use work::{Subtask, SubtaskStatus, VerificationStep, WorkStatus, WorkUnit};

/// Thread colors for visual identification in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreadColor {
    /// Blue thread (🔵)
    Blue,
    /// Green thread (🟢)
    Green,
    /// Purple thread (🟣)
    Purple,
    /// Yellow thread (🟡)
    Yellow,
    /// Red thread (🔴)
    Red,
    /// Orange thread (🟠)
    Orange,
}

impl ThreadColor {
    /// Returns the emoji representation of this color
    #[must_use]
    pub const fn emoji(self) -> &'static str {
        match self {
            Self::Blue => "🔵",
            Self::Green => "🟢",
            Self::Purple => "🟣",
            Self::Yellow => "🟡",
            Self::Red => "🔴",
            Self::Orange => "🟠",
        }
    }

    /// Assigns a color based on thread index (cycles through colors)
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        match index % 6 {
            0 => Self::Blue,
            1 => Self::Green,
            2 => Self::Purple,
            3 => Self::Yellow,
            4 => Self::Red,
            _ => Self::Orange,
        }
    }
}

impl fmt::Display for ThreadColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.emoji())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_color_cycling() {
        assert_eq!(ThreadColor::from_index(0), ThreadColor::Blue);
        assert_eq!(ThreadColor::from_index(1), ThreadColor::Green);
        assert_eq!(ThreadColor::from_index(2), ThreadColor::Purple);
        assert_eq!(ThreadColor::from_index(3), ThreadColor::Yellow);
        assert_eq!(ThreadColor::from_index(4), ThreadColor::Red);
        assert_eq!(ThreadColor::from_index(5), ThreadColor::Orange);
        assert_eq!(ThreadColor::from_index(6), ThreadColor::Blue); // Wraps around
    }
}
