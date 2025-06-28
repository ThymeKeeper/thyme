// src/config.rs

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub keybindings: KeyBindings,
    pub margins: Margins,
    pub word_wrap: bool,
    pub auto_save_delay: Duration,
    #[serde(skip)]
    pub theme: Theme,
    #[serde(default)]
    pub theme_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    pub increase_vertical_margin: SerializableKeyEvent,
    pub decrease_vertical_margin: SerializableKeyEvent,
    pub increase_horizontal_margin: SerializableKeyEvent,
    pub decrease_horizontal_margin: SerializableKeyEvent,
    pub toggle_word_wrap: SerializableKeyEvent,
    pub language_selection: SerializableKeyEvent,
    pub theme_selection: SerializableKeyEvent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableKeyEvent {
    pub code: String,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Margins {
    pub vertical: u16,
    pub horizontal: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    // Editor colors
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub selection: String,
    pub line_number: String,
    pub current_line: String,
    
    // Syntax colors
    pub keyword: String,
    pub string: String,
    pub comment: String,
    pub number: String,
    pub operator: String,
    pub identifier: String,
    pub type_: String,  // Note: 'type' is a keyword, so we use 'type_'
    pub function: String,
    pub variable: String,
    pub property: String,
    pub parameter: String,
    pub constant: String,
    pub namespace: String,
    pub punctuation: String,
    pub tag: String,
    pub attribute: String,
    pub normal: String,
    
    // UI colors
    pub status_bar_bg: String,
    pub status_bar_fg: String,
    pub border: String,
    pub border_active: String,
    pub modal_bg: String,
    pub modal_fg: String,
    pub selection_bg: String,
    pub selection_fg: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Default Dark".to_string(),
            colors: ThemeColors {
                // Editor colors
                background: "#1e1e1e".to_string(),
                foreground: "#d4d4d4".to_string(),
                cursor: "#ffffff".to_string(),
                selection: "#264f78".to_string(),
                line_number: "#858585".to_string(),
                current_line: "#2a2a2a".to_string(),
                
                // Syntax colors
                keyword: "#569cd6".to_string(),
                string: "#ce9178".to_string(),
                comment: "#6a9955".to_string(),
                number: "#b5cea8".to_string(),
                operator: "#d4d4d4".to_string(),
                identifier: "#9cdcfe".to_string(),
                type_: "#4ec9b0".to_string(),
                function: "#dcdcaa".to_string(),
                variable: "#9cdcfe".to_string(),
                property: "#9cdcfe".to_string(),
                parameter: "#9cdcfe".to_string(),
                constant: "#d16969".to_string(),
                namespace: "#c586c0".to_string(),
                punctuation: "#d4d4d4".to_string(),
                tag: "#569cd6".to_string(),
                attribute: "#9cdcfe".to_string(),
                normal: "#d4d4d4".to_string(),
                
                // UI colors
                status_bar_bg: "#007acc".to_string(),
                status_bar_fg: "#ffffff".to_string(),
                border: "#3e3e3e".to_string(),
                border_active: "#007acc".to_string(),
                modal_bg: "#252526".to_string(),
                modal_fg: "#cccccc".to_string(),
                selection_bg: "#007acc".to_string(),
                selection_fg: "#ffffff".to_string(),
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keybindings: KeyBindings {
                increase_vertical_margin: SerializableKeyEvent {
                    code: "F1".to_string(),
                    modifiers: vec![],
                },
                decrease_vertical_margin: SerializableKeyEvent {
                    code: "F2".to_string(),
                    modifiers: vec![],
                },
                increase_horizontal_margin: SerializableKeyEvent {
                    code: "F3".to_string(),
                    modifiers: vec![],
                },
                decrease_horizontal_margin: SerializableKeyEvent {
                    code: "F4".to_string(),
                    modifiers: vec![],
                },
                toggle_word_wrap: SerializableKeyEvent {
                    code: "F5".to_string(),
                    modifiers: vec![],
                },
                language_selection: SerializableKeyEvent {
                    code: "l".to_string(),
                    modifiers: vec!["ctrl".to_string()],
                },
                theme_selection: SerializableKeyEvent {
                    code: "t".to_string(),
                    modifiers: vec!["ctrl".to_string()],
                },
            },
            margins: Margins {
                vertical: 1,
                horizontal: 2,
            },
            word_wrap: false,
            auto_save_delay: Duration::from_secs(2),
            theme: Theme::default(),
            theme_name: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            // Try to deserialize, but if it fails (e.g., due to new fields), 
            // fall back to default and save it
            match toml::from_str::<Config>(&content) {
                Ok(mut config) => {
                    // Load the theme if specified
                    if let Some(theme_name) = config.theme_name.clone() {
                        if theme_name != "_default" {
                            if let Err(e) = config.load_theme(&theme_name) {
                                eprintln!("Failed to load theme '{}': {}", theme_name, e);
                                config.theme = Theme::default();
                            }
                        }
                    }
                    Ok(config)
                }
                Err(_) => {
                    let config = Self::default();
                    config.save()?;
                    Ok(config)
                }
            }
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        path.push("thyme");
        path.push("config.toml");
        Ok(path)
    }

    pub fn load_theme(&mut self, theme_name: &str) -> Result<()> {
        let theme_path = Self::theme_path(theme_name)?;
        
        if theme_path.exists() {
            let content = std::fs::read_to_string(theme_path)?;
            let theme: Theme = toml::from_str(&content)?;
            self.theme = theme;
            self.theme_name = Some(theme_name.to_string());
            Ok(())
        } else {
            anyhow::bail!("Theme '{}' not found", theme_name)
        }
    }

    pub fn theme_path(theme_name: &str) -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        path.push("thyme");
        path.push("themes");
        path.push(format!("{}.toml", theme_name));
        Ok(path)
    }

    pub fn themes_dir() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        path.push("thyme");
        path.push("themes");
        Ok(path)
    }
}

// Helper function to parse hex color strings to ratatui Color
impl Theme {
    pub fn parse_color(&self, color_str: &str) -> ratatui::style::Color {
        use ratatui::style::Color;
        
        // Handle hex colors
        if let Some(hex) = color_str.strip_prefix('#') {
            if hex.len() == 6 {
                if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                    let r = ((rgb >> 16) & 0xFF) as u8;
                    let g = ((rgb >> 8) & 0xFF) as u8;
                    let b = (rgb & 0xFF) as u8;
                    return Color::Rgb(r, g, b);
                }
            }
        }
        
        // Handle named colors
        match color_str.to_lowercase().as_str() {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "gray" | "grey" => Color::Gray,
            "darkgray" | "darkgrey" => Color::DarkGray,
            "lightred" => Color::LightRed,
            "lightgreen" => Color::LightGreen,
            "lightyellow" => Color::LightYellow,
            "lightblue" => Color::LightBlue,
            "lightmagenta" => Color::LightMagenta,
            "lightcyan" => Color::LightCyan,
            "white" => Color::White,
            _ => Color::White, // Default fallback
        }
    }
}

impl PartialEq<KeyEvent> for SerializableKeyEvent {
    fn eq(&self, other: &KeyEvent) -> bool {
        // Check if the key code matches
        let key_matches = match (&self.code[..], other.code) {
            ("F1", KeyCode::F(1)) => true,
            ("F2", KeyCode::F(2)) => true,
            ("F3", KeyCode::F(3)) => true,
            ("F4", KeyCode::F(4)) => true,
            ("F5", KeyCode::F(5)) => true,
            ("F6", KeyCode::F(6)) => true,
            ("F7", KeyCode::F(7)) => true,
            ("F8", KeyCode::F(8)) => true,
            ("F9", KeyCode::F(9)) => true,
            ("F10", KeyCode::F(10)) => true,
            ("F11", KeyCode::F(11)) => true,
            ("F12", KeyCode::F(12)) => true,
            // Add support for more keys if needed
            ("Escape", KeyCode::Esc) => true,
            ("Enter", KeyCode::Enter) => true,
            ("Space", KeyCode::Char(' ')) => true,
            ("Tab", KeyCode::Tab) => true,
            ("Backspace", KeyCode::Backspace) => true,
            ("Delete", KeyCode::Delete) => true,
            ("Home", KeyCode::Home) => true,
            ("End", KeyCode::End) => true,
            ("PageUp", KeyCode::PageUp) => true,
            ("PageDown", KeyCode::PageDown) => true,
            ("Up", KeyCode::Up) => true,
            ("Down", KeyCode::Down) => true,
            ("Left", KeyCode::Left) => true,
            ("Right", KeyCode::Right) => true,
            // Handle single character keys
            (key_str, KeyCode::Char(c)) if key_str.len() == 1 => {
                key_str.chars().next() == Some(c)
            }
            // No match
            _ => false,
        };

        if !key_matches {
            return false;
        }

        // Check modifiers
        let mut expected_modifiers = KeyModifiers::empty();
        for modifier in &self.modifiers {
            match modifier.as_str() {
                "ctrl" => expected_modifiers |= KeyModifiers::CONTROL,
                "alt" => expected_modifiers |= KeyModifiers::ALT,
                "shift" => expected_modifiers |= KeyModifiers::SHIFT,
                _ => {}
            }
        }

        other.modifiers == expected_modifiers
    }
}

impl PartialEq<SerializableKeyEvent> for KeyEvent {
    fn eq(&self, other: &SerializableKeyEvent) -> bool {
        other.eq(self)
    }
}
