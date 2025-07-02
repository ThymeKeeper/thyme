// src/buffer.rs

use crate::{
    config::Config, 
    cursor::Cursor, 
    syntax::SyntaxHighlighter,
    text_utils::{detect_language_from_path, get_language_display_name, get_supported_languages}
};
use anyhow::Result;
use ropey::Rope;
use std::{path::PathBuf, time::Instant};

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
        let mut syntax_highlighter = SyntaxHighlighter::new();
        syntax_highlighter.set_language("text");
        
        Self {
            rope: Rope::new(),
            cursor: Cursor::new(),
            file_path: None,
            dirty: false,
            language: "text".to_string(),
            last_change: None,
            syntax_highlighter,
            needs_syntax_update: true,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let rope = Rope::from_str(&content);
        let language = detect_language_from_path(&path);
        
        let mut syntax_highlighter = SyntaxHighlighter::new();
        syntax_highlighter.set_language(&language);
        
        let mut buffer = Self {
            rope,
            cursor: Cursor::new(),
            file_path: Some(path),
            dirty: false,
            language: language.clone(),
            last_change: None,
            syntax_highlighter,
            needs_syntax_update: true,
        };

        // Force initial syntax highlighting
        buffer.syntax_highlighter.update(&buffer.rope);
        buffer.needs_syntax_update = false;
        
        Ok(buffer)
    }

    // Method to change the syntax highlighting language
    pub fn set_language(&mut self, language: &str) {
        if self.language != language {
            self.language = language.to_string();
            self.syntax_highlighter.set_language(language);
            self.needs_syntax_update = true;
            // Only force immediate update if buffer has content
            if self.rope.len_chars() > 0 {
                self.syntax_highlighter.update(&self.rope);
                self.needs_syntax_update = false;
            }
        }
    }

    // Get list of supported languages
    pub fn get_supported_languages() -> Vec<&'static str> {
        get_supported_languages()
    }

    // Get a display name for a language
    pub fn get_language_display_name(language: &str) -> &'static str {
        get_language_display_name(language)
    }

    pub fn insert_char(&mut self, c: char) {
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        self.rope.insert_char(char_idx, c);
        self.cursor.column += 1;
        self.mark_dirty();
        // Force immediate syntax update for responsive highlighting
        self.force_syntax_update();
    }

    pub fn insert_newline(&mut self) {
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        self.rope.insert_char(char_idx, '\n');
        self.cursor.line += 1;
        self.cursor.column = 0;
        self.cursor.preferred_visual_column = 0;
        self.mark_dirty();
        // Force immediate syntax update for responsive highlighting
        self.force_syntax_update();
    }

    pub fn delete_char_backwards(&mut self) {
        if self.cursor.column > 0 {
            self.cursor.column -= 1;
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
            self.rope.remove(char_idx..char_idx + 1);
            self.mark_dirty();
        } else if self.cursor.line > 0 {
            let prev_line_len = self.rope.line(self.cursor.line - 1).len_chars() - 1; // -1 for newline
            let char_idx = self.rope.line_to_char(self.cursor.line) - 1; // Remove newline
            self.rope.remove(char_idx..char_idx + 1);
            self.cursor.line -= 1;
            self.cursor.column = prev_line_len;
            self.mark_dirty();
        }
        // Force immediate syntax update for responsive highlighting
        self.force_syntax_update();
    }

    pub fn delete_char_forwards(&mut self) {
        let line_len = self.rope.line(self.cursor.line).len_chars();
        if self.cursor.column < line_len - 1 {
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
            self.rope.remove(char_idx..char_idx + 1);
            self.mark_dirty();
        } else if self.cursor.line < self.rope.len_lines() - 1 {
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
            self.rope.remove(char_idx..char_idx + 1);
            self.mark_dirty();
        }
        // Force immediate syntax update for responsive highlighting
        self.force_syntax_update();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor.column > 0 {
            self.cursor.column -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let line_text = self.get_line_text(self.cursor.line);
            self.cursor.column = if line_text.ends_with('\n') {
                line_text.len() - 1
            } else {
                line_text.len()
            };
        }
    }

    pub fn move_cursor_right(&mut self) {
        let line_text = self.get_line_text(self.cursor.line);
        let line_content_len = if line_text.ends_with('\n') {
            line_text.len() - 1
        } else {
            line_text.len()
        };
        
        if self.cursor.column < line_content_len {
            self.cursor.column += 1;
        } else if self.cursor.line < self.rope.len_lines() - 1 {
            self.cursor.line += 1;
            self.cursor.column = 0;
        }
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
            self.cursor.column = self.cursor.preferred_visual_column.min(line_content_len);
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor.column = 0;
        self.cursor.preferred_visual_column = 0;
    }

    pub fn move_cursor_end(&mut self) {
        let line_text = self.get_line_text(self.cursor.line);
        self.cursor.column = if line_text.ends_with('\n') {
            line_text.len() - 1
        } else {
            line_text.len()
        };
        self.cursor.preferred_visual_column = self.cursor.column;
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_change = Some(Instant::now());
        self.needs_syntax_update = true;
        self.syntax_highlighter.mark_dirty();
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

    pub fn reset_preferred_column(&mut self) {
        self.cursor.preferred_visual_column = self.cursor.column;
    }
    
    // Force syntax highlighting update
    fn force_syntax_update(&mut self) {
        if self.language != "text" {
            self.syntax_highlighter.mark_dirty();
            self.syntax_highlighter.update(&self.rope);
            self.needs_syntax_update = false;
        }
    }
}

