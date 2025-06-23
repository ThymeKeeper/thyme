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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    pub increase_vertical_margin: SerializableKeyEvent,
    pub decrease_vertical_margin: SerializableKeyEvent,
    pub increase_horizontal_margin: SerializableKeyEvent,
    pub decrease_horizontal_margin: SerializableKeyEvent,
    pub toggle_word_wrap: SerializableKeyEvent,
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
            },
            margins: Margins {
                vertical: 1,
                horizontal: 2,
            },
            word_wrap: false,
            auto_save_delay: Duration::from_secs(2),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            Ok(toml::from_str(&content)?)
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
        path.push("tui-editor");
        path.push("config.toml");
        Ok(path)
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
