use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// UI theme configuration
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    /// Nord color palette
    Nord,
    /// Dracula color palette
    Dracula,
    /// Gruvbox color palette
    Gruvbox,
    /// Tokyo Night color palette
    #[default]
    TokyoNight,
    /// Catppuccin color palette
    Catppuccin,
    /// Monochrome color palette
    Monochrome,
}

impl Theme {
    /// Gets the next theme in sequence
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Nord => Self::Dracula,
            Self::Dracula => Self::Gruvbox,
            Self::Gruvbox => Self::TokyoNight,
            Self::TokyoNight => Self::Catppuccin,
            Self::Catppuccin => Self::Monochrome,
            Self::Monochrome => Self::Nord,
        }
    }

    /// Gets the focused border color
    pub fn focused_border(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(136, 192, 208),
            Self::Dracula => Color::Rgb(189, 147, 249),
            Self::Gruvbox => Color::Rgb(251, 184, 108),
            Self::TokyoNight => Color::Rgb(122, 162, 247),
            Self::Catppuccin => Color::Rgb(137, 180, 250),
            Self::Monochrome => Color::Rgb(100, 200, 255),
        }
    }

    /// Gets the unfocused border color
    pub fn unfocused_border(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(216, 222, 233),
            Self::Dracula => Color::Rgb(98, 114, 164),
            Self::Gruvbox => Color::Rgb(168, 153, 132),
            Self::TokyoNight => Color::Rgb(86, 95, 137),
            Self::Catppuccin => Color::Rgb(108, 112, 134),
            Self::Monochrome => Color::Rgb(128, 128, 128),
        }
    }

    /// Gets the text color
    pub fn text(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(236, 239, 244),
            Self::Dracula => Color::Rgb(248, 248, 242),
            Self::Gruvbox => Color::Rgb(235, 219, 178),
            Self::TokyoNight => Color::Rgb(192, 202, 245),
            Self::Catppuccin => Color::Rgb(205, 214, 244),
            Self::Monochrome => Color::Rgb(255, 255, 255),
        }
    }

    /// Gets the success color (for completed work)
    pub fn success(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(163, 190, 140),
            Self::Dracula => Color::Rgb(80, 250, 123),
            Self::Gruvbox => Color::Rgb(184, 187, 38),
            Self::TokyoNight => Color::Rgb(158, 206, 106),
            Self::Catppuccin => Color::Rgb(166, 227, 161),
            Self::Monochrome => Color::Rgb(150, 150, 150),
        }
    }

    /// Gets the error color (for failed work)
    pub fn error(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(191, 97, 106),
            Self::Dracula => Color::Rgb(255, 85, 85),
            Self::Gruvbox => Color::Rgb(251, 73, 52),
            Self::TokyoNight => Color::Rgb(247, 118, 142),
            Self::Catppuccin => Color::Rgb(243, 139, 168),
            Self::Monochrome => Color::Rgb(200, 200, 200),
        }
    }

    /// Gets the warning color (for in-progress/retrying work)
    pub fn warning(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(235, 203, 139),
            Self::Dracula => Color::Rgb(241, 250, 140),
            Self::Gruvbox => Color::Rgb(250, 189, 47),
            Self::TokyoNight => Color::Rgb(224, 175, 104),
            Self::Catppuccin => Color::Rgb(249, 226, 175),
            Self::Monochrome => Color::Rgb(180, 180, 180),
        }
    }
}
