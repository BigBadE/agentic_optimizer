use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::fs::{read_to_string as read_file_to_string, write as write_file};
use std::io;
use std::path::Path;

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
        let json =
            to_string(&self).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        write_file(theme_file, json)
    }
}
