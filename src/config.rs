// src/config.rs

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub keybindings: KeyBindings,
    pub margins: Margins,
    pub word_wrap: bool,
    pub auto_save_delay_seconds: u64,
    pub scrolloff: u16,
    #[serde(skip)]
    pub theme: Theme,
    #[serde(default)]
    pub theme_name: Option<String>,
    #[serde(default)]
    pub gutter: GutterMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub help: SerializableKeyEvent,
    pub increase_vertical_margin: SerializableKeyEvent,
    pub decrease_vertical_margin: SerializableKeyEvent,
    pub increase_horizontal_margin: SerializableKeyEvent,
    pub decrease_horizontal_margin: SerializableKeyEvent,
    pub toggle_word_wrap: SerializableKeyEvent,
    pub toggle_gutter: SerializableKeyEvent,
    pub language_selection: SerializableKeyEvent,
    pub theme_selection: SerializableKeyEvent,
    // Bullet journal hotkeys
    pub bullet_todo: SerializableKeyEvent,
    pub bullet_in_progress: SerializableKeyEvent,
    pub bullet_done: SerializableKeyEvent,
    // Paragraph navigation
    pub paragraph_up: SerializableKeyEvent,
    pub paragraph_down: SerializableKeyEvent,
    // Line movement
    pub move_line_up: SerializableKeyEvent,
    pub move_line_down: SerializableKeyEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableKeyEvent {
    pub code: String,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Margins {
    pub vertical: u16,
    pub horizontal: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GutterMode {
    None,
    Absolute,
    Relative,
}

impl Default for GutterMode {
    fn default() -> Self {
        GutterMode::None
    }
}

impl GutterMode {
    /// Cycle to the next gutter mode
    pub fn cycle(&self) -> Self {
        match self {
            GutterMode::None => GutterMode::Absolute,
            GutterMode::Absolute => GutterMode::Relative,
            GutterMode::Relative => GutterMode::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
    #[serde(default)]
    pub styles: ThemeStyles,
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
    
    // Optional virtual line color (defaults to comment color if not specified)
    #[serde(default = "default_virtual_line_color")]
    pub virtual_line: String,
    
    // Find/Replace highlighting
    #[serde(default = "default_find_match_bg")]
    pub find_match_bg: String,
    #[serde(default = "default_find_match_fg")]
    pub find_match_fg: String,
    #[serde(default = "default_find_current_match_bg")]
    pub find_current_match_bg: String,
    #[serde(default = "default_find_current_match_fg")]
    pub find_current_match_fg: String,
}

fn default_virtual_line_color() -> String {
    // This will be overridden in Theme::default() and when loading themes
    String::new()
}

fn default_find_match_bg() -> String {
    "#4a4a00".to_string() // Dark yellow background
}

fn default_find_match_fg() -> String {
    "#ffffff".to_string() // White foreground
}

fn default_find_current_match_bg() -> String {
    "#ffff00".to_string() // Bright yellow background for current match
}

fn default_find_current_match_fg() -> String {
    "#000000".to_string() // Black foreground for contrast
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeStyles {
    // Syntax styles (bold, italic, underline, etc.)
    pub keyword: Vec<String>,
    pub string: Vec<String>,
    pub comment: Vec<String>,
    pub number: Vec<String>,
    pub operator: Vec<String>,
    pub identifier: Vec<String>,
    pub type_: Vec<String>,
    pub function: Vec<String>,
    pub variable: Vec<String>,
    pub property: Vec<String>,
    pub parameter: Vec<String>,
    pub constant: Vec<String>,
    pub namespace: Vec<String>,
    pub punctuation: Vec<String>,
    pub tag: Vec<String>,
    pub attribute: Vec<String>,
    pub normal: Vec<String>,
}

impl Default for ThemeStyles {
    fn default() -> Self {
        Self {
            // Default: no text styles (all regular text)
            keyword: vec![],
            string: vec![],
            comment: vec![],
            number: vec![],
            operator: vec![],
            identifier: vec![],
            type_: vec![],
            function: vec![],
            variable: vec![],
            property: vec![],
            parameter: vec![],
            constant: vec![],
            namespace: vec![],
            punctuation: vec![],
            tag: vec![],
            attribute: vec![],
            normal: vec![],
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Default Dark".to_string(),
            colors: ThemeColors {
                // Editor colors - muted dark background
                background: "#1a1a1a".to_string(),      // Slightly darker background
                foreground: "#b0b0b0".to_string(),      // More muted foreground
                cursor: "#5a7a8a".to_string(),          // Muted blue-gray cursor
                selection: "#2a3a4a".to_string(),       // Muted selection
                line_number: "#606060".to_string(),     // Muted line numbers
                current_line: "#242424".to_string(),    // Subtle current line
                
                // Syntax colors - all muted down
                keyword: "#7a8fa6".to_string(),         // Muted blue
                string: "#9a8074".to_string(),          // Muted orange-brown
                comment: "#5a6a5a".to_string(),         // Muted green-gray
                number: "#8a9a8a".to_string(),          // Muted light green
                operator: "#909090".to_string(),        // Muted gray
                identifier: "#8a9aaa".to_string(),      // Muted light blue
                type_: "#6a9a8a".to_string(),           // Muted teal
                function: "#a0a080".to_string(),        // Muted yellow-gray
                variable: "#8a9aaa".to_string(),        // Same as identifier
                property: "#8a9aaa".to_string(),        // Same as identifier
                parameter: "#8a9aaa".to_string(),       // Same as identifier
                constant: "#9a7a7a".to_string(),        // Muted red
                namespace: "#8a7a9a".to_string(),       // Muted purple
                punctuation: "#808080".to_string(),     // Muted gray
                tag: "#7a8fa6".to_string(),             // Same as keyword
                attribute: "#8a9aaa".to_string(),       // Same as identifier
                normal: "#b0b0b0".to_string(),          // Same as foreground
                
                // UI colors - muted versions
                status_bar_bg: "#3a4a5a".to_string(),   // Muted blue-gray
                status_bar_fg: "#d0d0d0".to_string(),   // Muted white
                border: "#353535".to_string(),          // Muted border
                border_active: "#5a7a8a".to_string(),   // Muted active border
                modal_bg: "#202020".to_string(),        // Muted modal background
                modal_fg: "#a0a0a0".to_string(),        // Muted modal text
                selection_bg: "#4a5a6a".to_string(),    // Muted selection in UI
                selection_fg: "#d0d0d0".to_string(),    // Muted white
                virtual_line: "#2a2a2a".to_string(),    // Very subtle virtual lines
                find_match_bg: "#4a4a00".to_string(),   // Dark yellow background for find matches
                find_match_fg: "#ffffff".to_string(),   // White text for find matches
                find_current_match_bg: "#aaaa00".to_string(), // Bright yellow for current match
                find_current_match_fg: "#000000".to_string(), // Black text for contrast
            },
            styles: ThemeStyles::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keybindings: KeyBindings {
                help: SerializableKeyEvent {
                    code: "F1".to_string(),
                    modifiers: vec![],
                },
                increase_vertical_margin: SerializableKeyEvent {
                    code: "F2".to_string(),
                    modifiers: vec![],
                },
                decrease_vertical_margin: SerializableKeyEvent {
                    code: "F3".to_string(),
                    modifiers: vec![],
                },
                increase_horizontal_margin: SerializableKeyEvent {
                    code: "F4".to_string(),
                    modifiers: vec![],
                },
                decrease_horizontal_margin: SerializableKeyEvent {
                    code: "F5".to_string(),
                    modifiers: vec![],
                },
                toggle_word_wrap: SerializableKeyEvent {
                    code: "F6".to_string(),
                    modifiers: vec![],
                },
                toggle_gutter: SerializableKeyEvent {
                    code: "F7".to_string(),
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
                // Bullet journal hotkeys
                bullet_todo: SerializableKeyEvent {
                    code: "Left".to_string(),
                    modifiers: vec!["ctrl".to_string()],
                },
                bullet_in_progress: SerializableKeyEvent {
                    code: "Down".to_string(),
                    modifiers: vec!["ctrl".to_string()],
                },
                bullet_done: SerializableKeyEvent {
                    code: "Right".to_string(),
                    modifiers: vec!["ctrl".to_string()],
                },
                // Paragraph navigation
                paragraph_up: SerializableKeyEvent {
                    code: "PageUp".to_string(),
                    modifiers: vec!["ctrl".to_string()],
                },
                paragraph_down: SerializableKeyEvent {
                    code: "PageDown".to_string(),
                    modifiers: vec!["ctrl".to_string()],
                },
                // Line movement
                move_line_up: SerializableKeyEvent {
                    code: "Up".to_string(),
                    modifiers: vec!["ctrl".to_string(), "shift".to_string()],
                },
                move_line_down: SerializableKeyEvent {
                    code: "Down".to_string(),
                    modifiers: vec!["ctrl".to_string(), "shift".to_string()],
                },
            },
            margins: Margins {
                vertical: 0,
                horizontal: 0,
            },
            word_wrap: false,
            auto_save_delay_seconds: 0, // 0 = disabled
            scrolloff: 3,
            theme: Theme::default(),
            theme_name: None,
            gutter: GutterMode::None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        // Ensure themes directory and default themes exist
        Self::ensure_themes_directory()?;
        
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            // Try to deserialize, but if it fails (e.g., due to new fields), 
            // fall back to default and save it
            match toml::from_str::<Config>(&content) {
                Ok(mut config) => {
                    // Load the theme if specified
                    if let Some(theme_name) = config.theme_name.clone() {
                        if theme_name == "Default Dark" {
                            config.theme = Theme::default();
                        } else {
                            // Try to find theme by display name
                            match config.load_theme_by_display_name(&theme_name) {
                                Ok(_) => {},
                                Err(e) => {
                                    eprintln!("Failed to load theme '{}': {}", theme_name, e);
                                    config.theme = Theme::default();
                                    config.theme_name = Some("Default Dark".to_string());
                                }
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
        
        let mut config_clone = self.clone();
        // Save the display name, not the internal filename
        config_clone.theme_name = Some(self.theme.name.clone());
        
        let content = toml::to_string_pretty(&config_clone)?;
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
            let mut theme: Theme = toml::from_str(&content)?;
            // If virtual_line is empty, default to comment color
            if theme.colors.virtual_line.is_empty() {
                theme.colors.virtual_line = theme.colors.comment.clone();
            }
            self.theme = theme;
            self.theme_name = Some(theme_name.to_string());
            Ok(())
        } else {
            anyhow::bail!("Theme '{}' not found", theme_name)
        }
    }
    
    pub fn load_theme_by_display_name(&mut self, display_name: &str) -> Result<()> {
        let themes_dir = Self::themes_dir()?;
        
        if themes_dir.exists() {
            for entry in std::fs::read_dir(themes_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(mut theme) = toml::from_str::<Theme>(&content) {
                            if theme.name == display_name {
                                // If virtual_line is empty, default to comment color
                                if theme.colors.virtual_line.is_empty() {
                                    theme.colors.virtual_line = theme.colors.comment.clone();
                                }
                                self.theme = theme;
                                self.theme_name = Some(display_name.to_string());
                                return Ok(())
                            }
                        }
                    }
                }
            }
        }
        
        anyhow::bail!("Theme with display name '{}' not found", display_name)
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
    
    /// Ensure themes directory exists and create default themes if it's empty
    pub fn ensure_themes_directory() -> Result<()> {
        let themes_dir = Self::themes_dir()?;
        
        // Create themes directory if it doesn't exist
        std::fs::create_dir_all(&themes_dir)?;
        
        // Check if directory is empty or has no .toml files
        let has_themes = std::fs::read_dir(&themes_dir)?
            .any(|entry| {
                if let Ok(entry) = entry {
                    entry.path().extension().and_then(|s| s.to_str()) == Some("toml")
                } else {
                    false
                }
            });
        
        // If no themes exist, create default ones
        if !has_themes {
            Self::create_default_themes(&themes_dir)?;
        }
        
        Ok(())
    }
    
    /// Create a set of default themes
    fn create_default_themes(themes_dir: &std::path::Path) -> Result<()> {
        // Monokai theme
        let monokai = Theme {
            name: "Monokai".to_string(),
            colors: ThemeColors {
                background: "#272822".to_string(),
                foreground: "#f8f8f2".to_string(),
                cursor: "#66d9ef".to_string(),
                selection: "#49483e".to_string(),
                line_number: "#75715e".to_string(),
                current_line: "#3e3d32".to_string(),
                
                keyword: "#f92672".to_string(),
                string: "#e6db74".to_string(),
                comment: "#75715e".to_string(),
                number: "#ae81ff".to_string(),
                operator: "#f92672".to_string(),
                identifier: "#a6e22e".to_string(),
                type_: "#66d9ef".to_string(),
                function: "#a6e22e".to_string(),
                variable: "#f8f8f2".to_string(),
                property: "#a6e22e".to_string(),
                parameter: "#fd971f".to_string(),
                constant: "#ae81ff".to_string(),
                namespace: "#f92672".to_string(),
                punctuation: "#f8f8f2".to_string(),
                tag: "#f92672".to_string(),
                attribute: "#a6e22e".to_string(),
                normal: "#f8f8f2".to_string(),
                
                status_bar_bg: "#75715e".to_string(),
                status_bar_fg: "#f8f8f2".to_string(),
                border: "#75715e".to_string(),
                border_active: "#f92672".to_string(),
                modal_bg: "#3e3d32".to_string(),
                modal_fg: "#f8f8f2".to_string(),
                selection_bg: "#49483e".to_string(),
                selection_fg: "#f8f8f2".to_string(),
                virtual_line: "#49483e".to_string(), // Darker than comment, matches selection
                find_match_bg: "#75715e".to_string(), // Comment color as background
                find_match_fg: "#f8f8f2".to_string(),
                find_current_match_bg: "#e6db74".to_string(), // String color (yellow)
                find_current_match_fg: "#272822".to_string(), // Background color
            },
            styles: ThemeStyles::default(),
        };
        
        // Dracula theme
        let dracula = Theme {
            name: "Dracula".to_string(),
            colors: ThemeColors {
                background: "#282a36".to_string(),
                foreground: "#f8f8f2".to_string(),
                cursor: "#50fa7b".to_string(),
                selection: "#44475a".to_string(),
                line_number: "#6272a4".to_string(),
                current_line: "#44475a".to_string(),
                
                keyword: "#ff79c6".to_string(),
                string: "#f1fa8c".to_string(),
                comment: "#6272a4".to_string(),
                number: "#bd93f9".to_string(),
                operator: "#ff79c6".to_string(),
                identifier: "#50fa7b".to_string(),
                type_: "#8be9fd".to_string(),
                function: "#50fa7b".to_string(),
                variable: "#f8f8f2".to_string(),
                property: "#50fa7b".to_string(),
                parameter: "#ffb86c".to_string(),
                constant: "#bd93f9".to_string(),
                namespace: "#ff79c6".to_string(),
                punctuation: "#f8f8f2".to_string(),
                tag: "#ff79c6".to_string(),
                attribute: "#50fa7b".to_string(),
                normal: "#f8f8f2".to_string(),
                
                status_bar_bg: "#6272a4".to_string(),
                status_bar_fg: "#f8f8f2".to_string(),
                border: "#6272a4".to_string(),
                border_active: "#ff79c6".to_string(),
                modal_bg: "#44475a".to_string(),
                modal_fg: "#f8f8f2".to_string(),
                selection_bg: "#44475a".to_string(),
                selection_fg: "#f8f8f2".to_string(),
                virtual_line: "#44475a".to_string(), // Darker than comment, matches current line
                find_match_bg: "#6272a4".to_string(), // Comment color as background
                find_match_fg: "#f8f8f2".to_string(),
                find_current_match_bg: "#f1fa8c".to_string(), // String color (yellow)
                find_current_match_fg: "#282a36".to_string(), // Background color
            },
            styles: ThemeStyles::default(),
        };
        
        // Solarized Dark theme
        let solarized_dark = Theme {
            name: "Solarized Dark".to_string(),
            colors: ThemeColors {
                background: "#002b36".to_string(),
                foreground: "#839496".to_string(),
                cursor: "#268bd2".to_string(),
                selection: "#073642".to_string(),
                line_number: "#586e75".to_string(),
                current_line: "#073642".to_string(),
                
                keyword: "#859900".to_string(),
                string: "#2aa198".to_string(),
                comment: "#586e75".to_string(),
                number: "#d33682".to_string(),
                operator: "#859900".to_string(),
                identifier: "#268bd2".to_string(),
                type_: "#b58900".to_string(),
                function: "#268bd2".to_string(),
                variable: "#839496".to_string(),
                property: "#268bd2".to_string(),
                parameter: "#cb4b16".to_string(),
                constant: "#d33682".to_string(),
                namespace: "#6c71c4".to_string(),
                punctuation: "#839496".to_string(),
                tag: "#dc322f".to_string(),
                attribute: "#268bd2".to_string(),
                normal: "#839496".to_string(),
                
                status_bar_bg: "#073642".to_string(),
                status_bar_fg: "#93a1a1".to_string(),
                border: "#586e75".to_string(),
                border_active: "#268bd2".to_string(),
                modal_bg: "#073642".to_string(),
                modal_fg: "#839496".to_string(),
                selection_bg: "#073642".to_string(),
                selection_fg: "#93a1a1".to_string(),
                virtual_line: "#073642".to_string(), // Darker than comment, matches current line
                find_match_bg: "#586e75".to_string(), // Comment color as background
                find_match_fg: "#eee8d5".to_string(), // Light text
                find_current_match_bg: "#b58900".to_string(), // Type color (yellow)
                find_current_match_fg: "#002b36".to_string(), // Background color
            },
            styles: ThemeStyles::default(),
        };
        
        // Nord theme with custom dark background
        let nord = Theme {
            name: "Nord".to_string(),
            colors: ThemeColors {
                background: "#151515".to_string(), // Custom darker background
                foreground: "#d8dee9".to_string(),
                cursor: "#88c0d0".to_string(),
                selection: "#434c5e".to_string(),
                line_number: "#4c566a".to_string(),
                current_line: "#2e3440".to_string(),
                
                keyword: "#81a1c1".to_string(),
                string: "#a3be8c".to_string(),
                comment: "#616e88".to_string(),
                number: "#b48ead".to_string(),
                operator: "#81a1c1".to_string(),
                identifier: "#8fbcbb".to_string(),
                type_: "#8fbcbb".to_string(),
                function: "#88c0d0".to_string(),
                variable: "#d8dee9".to_string(),
                property: "#8fbcbb".to_string(),
                parameter: "#d08770".to_string(),
                constant: "#5e81ac".to_string(),
                namespace: "#81a1c1".to_string(),
                punctuation: "#eceff4".to_string(),
                tag: "#81a1c1".to_string(),
                attribute: "#8fbcbb".to_string(),
                normal: "#d8dee9".to_string(),
                
                status_bar_bg: "#3b4252".to_string(),
                status_bar_fg: "#eceff4".to_string(),
                border: "#4c566a".to_string(),
                border_active: "#88c0d0".to_string(),
                modal_bg: "#2e3440".to_string(),
                modal_fg: "#d8dee9".to_string(),
                selection_bg: "#434c5e".to_string(),
                selection_fg: "#eceff4".to_string(),
                virtual_line: "#2e3440".to_string(), // Darker than comment, matches current line
                find_match_bg: "#616e88".to_string(), // Comment color as background
                find_match_fg: "#eceff4".to_string(),
                find_current_match_bg: "#ebcb8b".to_string(), // Yellow/orange
                find_current_match_fg: "#2e3440".to_string(), // Dark background
            },
            styles: ThemeStyles::default(),
        };
        
        // Save the themes
        let themes = vec![
            ("monokai", monokai),
            ("dracula", dracula),
            ("solarized-dark", solarized_dark),
            ("nord", nord),
        ];
        
        for (filename, theme) in themes {
            let theme_path = themes_dir.join(format!("{}.toml", filename));
            let content = toml::to_string_pretty(&theme)?;
            std::fs::write(theme_path, content)?;
        }
        
        Ok(())
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
    
    pub fn parse_text_styles(&self, styles: &[String]) -> ratatui::style::Modifier {
        use ratatui::style::Modifier;
        
        let mut modifiers = Modifier::empty();
        
        for style in styles {
            match style.to_lowercase().as_str() {
                "bold" => modifiers |= Modifier::BOLD,
                "italic" => modifiers |= Modifier::ITALIC,
                "underlined" | "underline" => modifiers |= Modifier::UNDERLINED,
                "crossed_out" | "strikethrough" => modifiers |= Modifier::CROSSED_OUT,
                "dim" => modifiers |= Modifier::DIM,
                "reversed" | "reverse" => modifiers |= Modifier::REVERSED,
                "rapid_blink" => modifiers |= Modifier::RAPID_BLINK,
                "slow_blink" => modifiers |= Modifier::SLOW_BLINK,
                _ => {} // Ignore unknown styles
            }
        }
        
        modifiers
    }
}

impl Config {
    /// Toggle gutter mode to the next option
    pub fn toggle_gutter(&mut self) {
        self.gutter = self.gutter.cycle();
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
