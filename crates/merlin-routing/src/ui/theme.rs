use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

/// UI theme configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Nord,
    Dracula,
    Gruvbox,
    TokyoNight,
    Catppuccin,
    Monochrome,
}

impl Theme {
    /// Gets the next theme in sequence
    pub fn next(self) -> Self {
        match self {
            Theme::Nord => Theme::Dracula,
            Theme::Dracula => Theme::Gruvbox,
            Theme::Gruvbox => Theme::TokyoNight,
            Theme::TokyoNight => Theme::Catppuccin,
            Theme::Catppuccin => Theme::Monochrome,
            Theme::Monochrome => Theme::Nord,
        }
    }

    /// Gets the theme name
    pub fn name(self) -> &'static str {
        match self {
            Theme::Nord => "Nord",
            Theme::Dracula => "Dracula",
            Theme::Gruvbox => "Gruvbox",
            Theme::TokyoNight => "Tokyo Night",
            Theme::Catppuccin => "Catppuccin",
            Theme::Monochrome => "Monochrome",
        }
    }

    /// Gets the focused border color
    pub fn focused_border(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(136, 192, 208),
            Theme::Dracula => Color::Rgb(189, 147, 249),
            Theme::Gruvbox => Color::Rgb(251, 184, 108),
            Theme::TokyoNight => Color::Rgb(122, 162, 247),
            Theme::Catppuccin => Color::Rgb(137, 180, 250),
            Theme::Monochrome => Color::Rgb(100, 200, 255),
        }
    }

    /// Gets the unfocused border color
    pub fn unfocused_border(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(216, 222, 233),
            Theme::Dracula => Color::Rgb(98, 114, 164),
            Theme::Gruvbox => Color::Rgb(168, 153, 132),
            Theme::TokyoNight => Color::Rgb(86, 95, 137),
            Theme::Catppuccin => Color::Rgb(108, 112, 134),
            Theme::Monochrome => Color::Rgb(128, 128, 128),
        }
    }

    /// Gets the text color
    pub fn text(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(236, 239, 244),
            Theme::Dracula => Color::Rgb(248, 248, 242),
            Theme::Gruvbox => Color::Rgb(235, 219, 178),
            Theme::TokyoNight => Color::Rgb(192, 202, 245),
            Theme::Catppuccin => Color::Rgb(205, 214, 244),
            Theme::Monochrome => Color::Rgb(255, 255, 255),
        }
    }

    /// Gets the success color
    pub fn success(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(163, 190, 140),
            Theme::Dracula => Color::Rgb(80, 250, 123),
            Theme::Gruvbox => Color::Rgb(184, 187, 38),
            Theme::TokyoNight => Color::Rgb(158, 206, 106),
            Theme::Catppuccin => Color::Rgb(166, 227, 161),
            Theme::Monochrome => Color::Rgb(100, 255, 100),
        }
    }

    /// Gets the error color
    pub fn error(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(191, 97, 106),
            Theme::Dracula => Color::Rgb(255, 85, 85),
            Theme::Gruvbox => Color::Rgb(251, 73, 52),
            Theme::TokyoNight => Color::Rgb(247, 118, 142),
            Theme::Catppuccin => Color::Rgb(243, 139, 168),
            Theme::Monochrome => Color::Rgb(255, 100, 100),
        }
    }

    /// Gets the warning color
    pub fn warning(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(235, 203, 139),
            Theme::Dracula => Color::Rgb(241, 250, 140),
            Theme::Gruvbox => Color::Rgb(250, 189, 47),
            Theme::TokyoNight => Color::Rgb(224, 175, 104),
            Theme::Catppuccin => Color::Rgb(249, 226, 175),
            Theme::Monochrome => Color::Rgb(255, 200, 100),
        }
    }

    /// Gets the highlight color
    pub fn highlight(self) -> Color {
        self.focused_border()
    }

    /// Loads theme from file
    pub fn load(tasks_dir: &PathBuf) -> io::Result<Self> {
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

        let content = std::fs::read_to_string(theme_file)?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Saves theme to file
    pub fn save(self, tasks_dir: &PathBuf) -> io::Result<()> {
        let Some(parent) = tasks_dir.parent() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No parent directory",
            ));
        };

        let theme_file = parent.join("theme.json");
        let json = serde_json::to_string(&self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        std::fs::write(theme_file, json)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::TokyoNight
    }
}
