use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use std::fs::{read_to_string as read_file_to_string, write as write_file};
use serde_json::{from_str, to_string};

/// UI theme configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    /// Nord color palette
    Nord,
    /// Dracula color palette
    Dracula,
    /// Gruvbox color palette
    Gruvbox,
    /// Tokyo Night color palette
    TokyoNight,
    /// Catppuccin color palette
    Catppuccin,
    /// Monochrome color palette
    Monochrome,
}

impl Theme {
    /// Gets the next theme in sequence
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

    /// Gets the theme name
    pub fn name(self) -> &'static str {
        match self {
            Self::Nord => "Nord",
            Self::Dracula => "Dracula",
            Self::Gruvbox => "Gruvbox",
            Self::TokyoNight => "Tokyo Night",
            Self::Catppuccin => "Catppuccin",
            Self::Monochrome => "Monochrome",
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

    /// Gets the success color
    pub fn success(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(163, 190, 140),
            Self::Dracula => Color::Rgb(80, 250, 123),
            Self::Gruvbox => Color::Rgb(184, 187, 38),
            Self::TokyoNight => Color::Rgb(158, 206, 106),
            Self::Catppuccin => Color::Rgb(166, 227, 161),
            Self::Monochrome => Color::Rgb(100, 255, 100),
        }
    }

    /// Gets the error color
    pub fn error(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(191, 97, 106),
            Self::Dracula => Color::Rgb(255, 85, 85),
            Self::Gruvbox => Color::Rgb(251, 73, 52),
            Self::TokyoNight => Color::Rgb(247, 118, 142),
            Self::Catppuccin => Color::Rgb(243, 139, 168),
            Self::Monochrome => Color::Rgb(255, 100, 100),
        }
    }

    /// Gets the warning color
    pub fn warning(self) -> Color {
        match self {
            Self::Nord => Color::Rgb(235, 203, 139),
            Self::Dracula => Color::Rgb(241, 250, 140),
            Self::Gruvbox => Color::Rgb(250, 189, 47),
            Self::TokyoNight => Color::Rgb(224, 175, 104),
            Self::Catppuccin => Color::Rgb(249, 226, 175),
            Self::Monochrome => Color::Rgb(255, 200, 100),
        }
    }

    /// Gets the highlight color
    pub fn highlight(self) -> Color {
        self.focused_border()
    }

    /// Loads theme from file
    ///
    /// # Errors
    /// Returns an error if the theme file cannot be located, read, or parsed.
    pub fn load(tasks_dir: &Path) -> io::Result<Self> {
        let theme_file = tasks_dir
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No parent directory"))?
            .join("theme.json");

        if !theme_file.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Theme file not found",
            ));
        }

        let content = read_file_to_string(theme_file)?;
        from_str(&content).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    /// Saves theme to file
    ///
    /// # Errors
    /// Returns an error if the theme cannot be serialized or written.
    pub fn save(self, tasks_dir: &Path) -> io::Result<()> {
        let Some(parent) = tasks_dir.parent() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No parent directory",
            ));
        };

        let theme_file = parent.join("theme.json");
        let json = to_string(&self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        write_file(theme_file, json)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::TokyoNight
    }
}
