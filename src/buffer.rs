// src/buffer.rs

use crate::{
    config::Config, 
    cursor::{Cursor, Position}, 
    syntax::SyntaxHighlighter,
    text_utils::{detect_language_from_path, get_language_display_name, get_supported_languages}
};
use anyhow::Result;
use arboard::Clipboard;
use ropey::Rope;
use std::{path::PathBuf, time::{Duration, Instant}};

#[derive(Clone, Debug)]
pub enum UndoAction {
    InsertText {
        position: Position,
        text: String,
        cursor_after: Position,
    },
    DeleteText {
        position: Position,
        text: String,
        cursor_after: Position,
    },
    ReplaceText {
        position: Position,
        old_text: String,
        new_text: String,
        cursor_after: Position,
    },
}

#[derive(Clone, Debug)]
pub struct UndoState {
    pub action: UndoAction,
    pub selection_before: Option<(Position, Position)>,
    pub selection_after: Option<(Position, Position)>,
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
    undo_stack: Vec<UndoState>,
    redo_stack: Vec<UndoState>,
    max_undo_stack_size: usize,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_stack_size: 1000,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_stack_size: 1000,
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
                line_text.chars().count().saturating_sub(1)
            } else {
                line_text.chars().count()
            };
        }
    }

    pub fn move_cursor_right(&mut self) {
        let line_text = self.get_line_text(self.cursor.line);
        let line_content_len = if line_text.ends_with('\n') {
            line_text.chars().count().saturating_sub(1)
        } else {
            line_text.chars().count()
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
                line_text.chars().count().saturating_sub(1)
            } else {
                line_text.chars().count()
            };
            self.cursor.column = self.cursor.preferred_visual_column.min(line_content_len);
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor.line < self.rope.len_lines() - 1 {
            self.cursor.line += 1;
            let line_text = self.get_line_text(self.cursor.line);
            let line_content_len = if line_text.ends_with('\n') {
                line_text.chars().count().saturating_sub(1)
            } else {
                line_text.chars().count()
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
            line_text.chars().count().saturating_sub(1)
        } else {
            line_text.chars().count()
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
        // If auto_save_delay_seconds is 0, autosave is disabled
        if config.auto_save_delay_seconds == 0 {
            return false;
        }
        
        if self.file_path.is_none() {
            return false;
        }
        
        if !self.dirty {
            return false;
        }

        if let Some(last_change) = self.last_change {
            last_change.elapsed() >= Duration::from_secs(config.auto_save_delay_seconds)
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
    
    /// Select all text in the buffer
    pub fn select_all(&mut self) {
        // Start selection at the beginning
        self.cursor.selection_start = Some(Position { line: 0, column: 0 });
        
        // Move cursor to the end
        let last_line = self.rope.len_lines().saturating_sub(1);
        let last_line_text = self.get_line_text(last_line);
        let last_column = if last_line_text.ends_with('\n') {
            last_line_text.chars().count().saturating_sub(1)
        } else {
            last_line_text.chars().count()
        };
        
        self.cursor.line = last_line;
        self.cursor.column = last_column;
        self.cursor.preferred_visual_column = last_column;
    }
    
    /// Copy selected text to clipboard
    pub fn copy_selection(&self) -> Result<()> {
        if let Some((start, end)) = self.cursor.get_selection_range() {
            let selected_text = self.get_text_range(start, end);
            if !selected_text.is_empty() {
                let mut clipboard = Clipboard::new()?;
                clipboard.set_text(selected_text)?;
            }
        }
        // If no selection, do nothing (standard behavior)
        Ok(())
    }
    
    /// Cut selected text to clipboard and delete it
    pub fn cut_selection(&mut self) -> Result<()> {
        if let Some((start, end)) = self.cursor.get_selection_range() {
            let selected_text = self.get_text_range(start, end);
            if !selected_text.is_empty() {
                let mut clipboard = Clipboard::new()?;
                clipboard.set_text(selected_text.clone())?;
                
                // Push cut to undo stack as delete
                self.add_undo_state(UndoAction::DeleteText {
                    position: start.clone(),
                    text: selected_text.clone(),
                    cursor_after: start.clone(),
                });

                // Clear redo stack
                self.redo_stack.clear();

                // Delete the selected text
                self.delete_text_range(start, end);
                
                // Move cursor to start of deleted range and clear selection
                self.cursor.line = start.line;
                self.cursor.column = start.column;
                self.cursor.preferred_visual_column = start.column;
                self.cursor.clear_selection();
            }
        }
        // If no selection, do nothing
        Ok(())
    }
    
    /// Paste text from clipboard
    pub fn paste_from_clipboard(&mut self) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        if let Ok(text) = clipboard.get_text() {
            // Determine paste start position
            let start_position = self.cursor.clone();

            // If there's a selection, delete it first
            if let Some((start, end)) = self.cursor.get_selection_range() {
                self.delete_text_range(start, end);
                self.cursor.line = start.line;
                self.cursor.column = start.column;
                self.cursor.preferred_visual_column = start.column;
                self.cursor.clear_selection();
            }
            
            // Insert the text at cursor position
            self.insert_text_at_cursor(&text);

            // Determine cursor position after paste
            let cursor_after = self.cursor.clone();

            // Push paste to undo stack as insert
            self.add_undo_state(UndoAction::InsertText {
                position: Position { line: start_position.line, column: start_position.column },
                text,
                cursor_after: Position { line: cursor_after.line, column: cursor_after.column },
            });

            // Clear redo stack
            self.redo_stack.clear();
        }
        Ok(())
    }
    
    /// Get text within a range
    fn get_text_range(&self, start: Position, end: Position) -> String {
        let start_char_idx = self.rope.line_to_char(start.line) + start.column;
        let end_char_idx = self.rope.line_to_char(end.line) + end.column;
        
        if start_char_idx < end_char_idx && end_char_idx <= self.rope.len_chars() {
            self.rope.slice(start_char_idx..end_char_idx).to_string()
        } else {
            String::new()
        }
    }
    
    /// Delete text within a range
    fn delete_text_range(&mut self, start: Position, end: Position) {
        let start_char_idx = self.rope.line_to_char(start.line) + start.column;
        let end_char_idx = self.rope.line_to_char(end.line) + end.column;
        
        if start_char_idx < end_char_idx && end_char_idx <= self.rope.len_chars() {
            self.rope.remove(start_char_idx..end_char_idx);
            self.mark_dirty();
            self.force_syntax_update();
        }
    }
    
    /// Insert text at current cursor position
    fn insert_text_at_cursor(&mut self, text: &str) {
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        self.rope.insert(char_idx, text);
        
        // Update cursor position based on inserted text
        let newline_count = text.matches('\n').count();
        if newline_count > 0 {
            self.cursor.line += newline_count;
            // Find the column position after the last newline
            if let Some(last_newline_pos) = text.rfind('\n') {
                self.cursor.column = text[last_newline_pos + 1..].chars().count();
            }
        } else {
            self.cursor.column += text.chars().count();
        }
        
        self.cursor.preferred_visual_column = self.cursor.column;
        self.mark_dirty();
        self.force_syntax_update();
    }
    
    /// Add an action to the undo stack
    fn add_undo_state(&mut self, action: UndoAction) {
        let undo_state = UndoState {
            action,
            selection_before: self.cursor.get_selection_range(),
            selection_after: None, // Will be set after the action is performed
        };
        
        self.undo_stack.push(undo_state);
        
        // Limit undo stack size
        if self.undo_stack.len() > self.max_undo_stack_size {
            self.undo_stack.remove(0);
        }
    }
    
    /// Undo the last action
    pub fn undo(&mut self) -> bool {
        if let Some(undo_state) = self.undo_stack.pop() {
            match undo_state.action {
                UndoAction::InsertText { position, ref text, cursor_after: _ } => {
                    // Undo insert by deleting the inserted text
                    let end_pos = self.calculate_end_position(position.clone(), text);
                    self.delete_text_range(position.clone(), end_pos);
                    self.cursor.line = position.line;
                    self.cursor.column = position.column;
                    self.cursor.preferred_visual_column = position.column;
                    self.cursor.clear_selection();
                }
                UndoAction::DeleteText { position, ref text, cursor_after: _ } => {
                    // Undo delete by inserting the deleted text
                    self.cursor.line = position.line;
                    self.cursor.column = position.column;
                    self.insert_text_at_cursor(text);
                }
                UndoAction::ReplaceText { position, ref old_text, new_text: _, cursor_after: _ } => {
                    // Undo replace by replacing new text with old text
                    let end_pos = self.calculate_end_position(position.clone(), old_text);
                    self.delete_text_range(position.clone(), end_pos);
                    self.cursor.line = position.line;
                    self.cursor.column = position.column;
                    self.insert_text_at_cursor(old_text);
                }
            }
            
            // Add to redo stack
            self.redo_stack.push(undo_state);
            true
        } else {
            false
        }
    }
    
    /// Redo the last undone action
    pub fn redo(&mut self) -> bool {
        if let Some(redo_state) = self.redo_stack.pop() {
            match redo_state.action {
                UndoAction::InsertText { position, ref text, cursor_after } => {
                    // Redo insert
                    self.cursor.line = position.line;
                    self.cursor.column = position.column;
                    self.insert_text_at_cursor(text);
                    self.cursor.line = cursor_after.line;
                    self.cursor.column = cursor_after.column;
                    self.cursor.preferred_visual_column = cursor_after.column;
                }
                UndoAction::DeleteText { position, ref text, cursor_after } => {
                    // Redo delete
                    let end_pos = self.calculate_end_position(position.clone(), text);
                    self.delete_text_range(position, end_pos);
                    self.cursor.line = cursor_after.line;
                    self.cursor.column = cursor_after.column;
                    self.cursor.preferred_visual_column = cursor_after.column;
                }
                UndoAction::ReplaceText { position, old_text: _, ref new_text, cursor_after } => {
                    // Redo replace
                    let end_pos = self.calculate_end_position(position.clone(), new_text);
                    self.delete_text_range(position.clone(), end_pos);
                    self.cursor.line = position.line;
                    self.cursor.column = position.column;
                    self.insert_text_at_cursor(new_text);
                    self.cursor.line = cursor_after.line;
                    self.cursor.column = cursor_after.column;
                    self.cursor.preferred_visual_column = cursor_after.column;
                }
            }
            
            // Add back to undo stack
            self.undo_stack.push(redo_state);
            true
        } else {
            false
        }
    }
    
    /// Calculate end position after inserting text at start position
    fn calculate_end_position(&self, start: Position, text: &str) -> Position {
        let newline_count = text.matches('\n').count();
        if newline_count > 0 {
            let line = start.line + newline_count;
            let column = if let Some(last_newline_pos) = text.rfind('\n') {
                text[last_newline_pos + 1..].chars().count()
            } else {
                start.column + text.chars().count()
            };
            Position { line, column }
        } else {
            Position {
                line: start.line,
                column: start.column + text.chars().count(),
            }
        }
    }
}

