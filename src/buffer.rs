// src/buffer.rs

use crate::{config::Config, syntax::SyntaxHighlighter};
use anyhow::Result;
use ropey::Rope;
use std::{path::PathBuf, time::Instant};

#[derive(Debug, Clone)]
pub struct Cursor {
    pub line: usize,
    pub column: usize,
    pub preferred_visual_column: usize, // Position within the visual line segment
}

impl Cursor {
    pub fn new() -> Self {
        Self { 
            line: 0, 
            column: 0,
            preferred_visual_column: 0,
        }
    }
    
    // Update preferred column when moving horizontally or typing
    pub fn update_preferred_visual_column(&mut self, visual_column: usize) {
        self.preferred_visual_column = visual_column;
    }
}

pub struct Buffer {
    pub rope: Rope,
    pub cursor: Cursor,
    pub file_path: Option<PathBuf>,
    pub dirty: bool,
    pub language: String,
    pub last_change: Option<Instant>,
    pub syntax_highlighter: SyntaxHighlighter,
    pub needs_syntax_update: bool,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            cursor: Cursor::new(),
            file_path: None,
            dirty: false,
            language: "text".to_string(),
            last_change: None,
            syntax_highlighter: SyntaxHighlighter::new(),
            needs_syntax_update: true,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let rope = Rope::from_str(&content);
        let language = detect_language(&path);
        
        let mut buffer = Self {
            rope,
            cursor: Cursor::new(), // Fresh cursor with preferred_column = 0
            file_path: Some(path),
            dirty: false,
            language: language.clone(),
            last_change: None,
            syntax_highlighter: SyntaxHighlighter::new(),
            needs_syntax_update: true,
        };

        buffer.syntax_highlighter.set_language(&language);
        Ok(buffer)
    }

    pub fn insert_char(&mut self, c: char) {
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        self.rope.insert_char(char_idx, c);
        self.cursor.column += 1;
        // Preferred visual column will be properly updated in editor.rs
        self.mark_dirty();
    }

    pub fn insert_newline(&mut self) {
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        self.rope.insert_char(char_idx, '\n');
        self.cursor.line += 1;
        self.cursor.column = 0;
        self.cursor.preferred_visual_column = 0; // Reset to start of new line
        self.mark_dirty();
    }

    pub fn delete_char_backwards(&mut self) {
        if self.cursor.column > 0 {
            self.cursor.column -= 1;
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
            self.rope.remove(char_idx..char_idx + 1);
            // Preferred visual column will be properly updated in editor.rs
            self.mark_dirty();
        } else if self.cursor.line > 0 {
            let prev_line_len = self.rope.line(self.cursor.line - 1).len_chars() - 1; // -1 for newline
            let char_idx = self.rope.line_to_char(self.cursor.line) - 1; // Remove newline
            self.rope.remove(char_idx..char_idx + 1);
            self.cursor.line -= 1;
            self.cursor.column = prev_line_len;
            // Preferred visual column will be properly updated in editor.rs
            self.mark_dirty();
        }
    }

    pub fn delete_char_forwards(&mut self) {
        let line_len = self.rope.line(self.cursor.line).len_chars();
        if self.cursor.column < line_len - 1 {
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
            self.rope.remove(char_idx..char_idx + 1);
            // Don't update preferred visual column for forward delete
            self.mark_dirty();
        } else if self.cursor.line < self.rope.len_lines() - 1 {
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
            self.rope.remove(char_idx..char_idx + 1);
            // Don't update preferred visual column for forward delete
            self.mark_dirty();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor.column > 0 {
            self.cursor.column -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            // Move to the actual end of the previous line (after last visible character)
            let line_text = self.get_line_text(self.cursor.line);
            self.cursor.column = if line_text.ends_with('\n') {
                line_text.len() - 1 // Position after last visible char, before newline
            } else {
                line_text.len() // Position after last char if no newline
            };
        }
        // Preferred visual column will be updated in editor.rs
    }

    pub fn move_cursor_right(&mut self) {
        let line_text = self.get_line_text(self.cursor.line);
        let line_content_len = if line_text.ends_with('\n') {
            line_text.len() - 1 // Length of visible content (excluding newline)
        } else {
            line_text.len() // Length of content (no newline to exclude)
        };
        
        if self.cursor.column < line_content_len {
            self.cursor.column += 1;
        } else if self.cursor.line < self.rope.len_lines() - 1 {
            self.cursor.line += 1;
            self.cursor.column = 0;
        }
        // Preferred visual column will be updated in editor.rs
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let line_text = self.get_line_text(self.cursor.line);
            let line_content_len = if line_text.ends_with('\n') {
                line_text.len() - 1
            } else {
                line_text.len()
            };
            // Use preferred visual column (will be properly calculated in editor.rs for word wrap)
            self.cursor.column = self.cursor.preferred_visual_column.min(line_content_len);
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor.line < self.rope.len_lines() - 1 {
            self.cursor.line += 1;
            let line_text = self.get_line_text(self.cursor.line);
            let line_content_len = if line_text.ends_with('\n') {
                line_text.len() - 1
            } else {
                line_text.len()
            };
            // Use preferred visual column (will be properly calculated in editor.rs for word wrap)
            self.cursor.column = self.cursor.preferred_visual_column.min(line_content_len);
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor.column = 0;
        self.cursor.preferred_visual_column = 0;
    }

    pub fn move_cursor_end(&mut self) {
        let line_text = self.get_line_text(self.cursor.line);
        // Move to the end of visible content (after last visible character)
        self.cursor.column = if line_text.ends_with('\n') {
            line_text.len() - 1 // Position after last visible char, before newline
        } else {
            line_text.len() // Position after last char if no newline
        };
        // Note: preferred_visual_column will be updated properly in editor.rs when needed
        // For simple cases, just use the logical column
        self.cursor.preferred_visual_column = self.cursor.column;
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_change = Some(Instant::now());
        self.needs_syntax_update = true;
    }

    pub fn save(&mut self, path: Option<PathBuf>) -> Result<()> {
        let save_path = path.or_else(|| self.file_path.clone())
            .ok_or_else(|| anyhow::anyhow!("No file path specified"))?;
        
        std::fs::write(&save_path, self.rope.to_string())?;
        self.file_path = Some(save_path);
        self.dirty = false;
        Ok(())
    }

    pub fn should_auto_save(&self, config: &Config) -> bool {
        // Don't auto-save if there's no file path
        if self.file_path.is_none() {
            return false;
        }
        
        if !self.dirty {
            return false;
        }

        if let Some(last_change) = self.last_change {
            last_change.elapsed() >= config.auto_save_delay
        } else {
            false
        }
    }

    pub fn mark_auto_saved(&mut self) {
        self.last_change = None;
    }

    pub fn update_syntax_if_needed(&mut self) {
        if self.needs_syntax_update {
            self.syntax_highlighter.update(&self.rope);
            self.needs_syntax_update = false;
        }
    }

    pub fn get_line_text(&self, line: usize) -> String {
        if line < self.rope.len_lines() {
            self.rope.line(line).to_string()
        } else {
            String::new()
        }
    }

    /// Reset the preferred visual column to the current visual position
    /// This should be called when layout changes (terminal resize, margin changes, word wrap toggle)
    pub fn reset_preferred_column(&mut self) {
        // This will be properly calculated in editor.rs based on visual position
        self.cursor.preferred_visual_column = self.cursor.column;
    }
}

fn detect_language(path: &PathBuf) -> String {
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        match extension {
            "rs" => "rust".to_string(),
            "py" => "python".to_string(),
            "js" | "jsx" => "javascript".to_string(),
            "sql" => "sql".to_string(),
            "sh" | "bash" => "bash".to_string(),
            "xml" | "html" | "xhtml" => "xml".to_string(),
            _ => "text".to_string(),
        }
    } else {
        "text".to_string()
    }
}
