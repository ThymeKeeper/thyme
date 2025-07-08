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

#[derive(Clone, Debug, PartialEq)]
pub enum LineEnding {
    LF,   // Unix: \n
    CRLF, // Windows: \r\n
}

impl LineEnding {
    fn as_str(&self) -> &'static str {
        match self {
            LineEnding::LF => "\n",
            LineEnding::CRLF => "\r\n",
        }
    }
    
    fn detect(text: &str) -> Self {
        // If we find any \r\n, it's CRLF
        if text.contains("\r\n") {
            LineEnding::CRLF
        } else {
            LineEnding::LF
        }
    }
}

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
    pub group_id: Option<usize>, // Group ID for grouping related actions
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
    pub line_ending: LineEnding,
    undo_stack: Vec<UndoState>,
    redo_stack: Vec<UndoState>,
    max_undo_stack_size: usize,
    last_action_time: Option<Instant>,
    undo_grouping_active: bool,
    current_undo_group_id: usize,
}

impl Buffer {
    pub fn new() -> Self {
        let mut syntax_highlighter = SyntaxHighlighter::new();
        syntax_highlighter.set_language("text");
        
        // Default to platform-specific line ending
        let line_ending = if cfg!(windows) {
            LineEnding::CRLF
        } else {
            LineEnding::LF
        };
        
        Self {
            rope: Rope::new(),
            cursor: Cursor::new(),
            file_path: None,
            dirty: false,
            language: "text".to_string(),
            last_change: None,
            syntax_highlighter,
            needs_syntax_update: true,
            line_ending,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_stack_size: 1000,
            last_action_time: None,
            undo_grouping_active: false,
            current_undo_group_id: 0,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        
        // Detect line ending style
        let line_ending = LineEnding::detect(&content);
        
        // Normalize to LF for internal use
        let normalized_content = content.replace("\r\n", "\n");
        let rope = Rope::from_str(&normalized_content);
        
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
            line_ending,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_stack_size: 1000,
            last_action_time: None,
            undo_grouping_active: false,
            current_undo_group_id: 0,
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
            // Force immediate update if buffer has content
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
        // Handle selection - delete selected text first
        if let Some((start, end)) = self.cursor.get_selection_range() {
            self.delete_selection(start, end);
        }
        
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        
        // Check if we should start a new undo group
        // If undo_grouping_active is true, we're in an explicit group (e.g., replace all)
        // Otherwise, check if 2+ seconds have passed since last action
        let should_start_new_group = if self.undo_grouping_active {
            false // We're in an explicit undo group, don't start a new one
        } else if let Some(last_time) = self.last_action_time {
            last_time.elapsed() > Duration::from_secs(2)
        } else {
            true // First action
        };
        
        if should_start_new_group {
            // Create a new undo group
            let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
            let undo_action = UndoAction::InsertText {
                position: current_pos,
                text: c.to_string(),
                cursor_after: Position { line: self.cursor.line, column: self.cursor.column + 1 },
            };
            self.add_undo_state_new_group(undo_action);
        } else {
            // Add to existing undo group
            if let Some(last_undo) = self.undo_stack.last_mut() {
                match &mut last_undo.action {
                    UndoAction::InsertText { ref mut text, ref mut cursor_after, .. } => {
                        text.push(c);
                        cursor_after.column += 1;
                    }
                    _ => {
                        // Different action type, create new group
                        let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                        let undo_action = UndoAction::InsertText {
                            position: current_pos,
                            text: c.to_string(),
                            cursor_after: Position { line: self.cursor.line, column: self.cursor.column + 1 },
                        };
                        self.add_undo_state_new_group(undo_action);
                    }
                }
            } else {
                // No previous action, create new group
                let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                let undo_action = UndoAction::InsertText {
                    position: current_pos,
                    text: c.to_string(),
                    cursor_after: Position { line: self.cursor.line, column: self.cursor.column + 1 },
                };
                self.add_undo_state_new_group(undo_action);
            }
        }
        
        // Update last action time
        self.last_action_time = Some(Instant::now());
        
        self.rope.insert_char(char_idx, c);
        self.cursor.column += 1;
        self.mark_dirty();
        
        // Update syntax highlighting for the current line only
        self.update_syntax_for_line(self.cursor.line);
    }

    pub fn insert_newline(&mut self) {
        // Handle selection - delete selected text first
        if let Some((start, end)) = self.cursor.get_selection_range() {
            self.delete_selection(start, end);
        }
        
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        
        // Check if we should start a new undo group
        // If undo_grouping_active is true, we're in an explicit group (e.g., replace all)
        // Otherwise, check if 2+ seconds have passed since last action
        let should_start_new_group = if self.undo_grouping_active {
            false // We're in an explicit undo group, don't start a new one
        } else if let Some(last_time) = self.last_action_time {
            last_time.elapsed() > Duration::from_secs(2)
        } else {
            true // First action
        };
        
        if should_start_new_group {
            // Create a new undo group
            let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
            let undo_action = UndoAction::InsertText {
                position: current_pos,
                text: "\n".to_string(),
                cursor_after: Position { line: self.cursor.line + 1, column: 0 },
            };
            self.add_undo_state_new_group(undo_action);
        } else {
            // Add to existing undo group
            if let Some(last_undo) = self.undo_stack.last_mut() {
                match &mut last_undo.action {
                    UndoAction::InsertText { ref mut text, ref mut cursor_after, .. } => {
                        text.push('\n');
                        cursor_after.line += 1;
                        cursor_after.column = 0;
                    }
                    _ => {
                        // Different action type, create new group
                        let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                        let undo_action = UndoAction::InsertText {
                            position: current_pos,
                            text: "\n".to_string(),
                            cursor_after: Position { line: self.cursor.line + 1, column: 0 },
                        };
                        self.add_undo_state_new_group(undo_action);
                    }
                }
            } else {
                // No previous action, create new group
                let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                let undo_action = UndoAction::InsertText {
                    position: current_pos,
                    text: "\n".to_string(),
                    cursor_after: Position { line: self.cursor.line + 1, column: 0 },
                };
                self.add_undo_state_new_group(undo_action);
            }
        }
        
        // Update last action time
        self.last_action_time = Some(Instant::now());
        
        self.rope.insert_char(char_idx, '\n');
        self.cursor.line += 1;
        self.cursor.column = 0;
        self.cursor.preferred_visual_column = 0;
        self.mark_dirty();
        
        // Handle line insertion in syntax highlighter
        self.syntax_highlighter.insert_line(&self.rope, self.cursor.line);
    }

    pub fn delete_char_backwards(&mut self) {
        // Handle selection - delete selected text first
        if let Some((start, end)) = self.cursor.get_selection_range() {
            self.delete_selection(start, end);
            return;
        }
        
        if self.cursor.column > 0 {
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column - 1;
            let deleted_char = self.rope.char(char_idx);
            
            // Check if we should start a new undo group
            // If undo_grouping_active is true, we're in an explicit group (e.g., replace all)
            // Otherwise, check if 2+ seconds have passed since last action
            let should_start_new_group = if self.undo_grouping_active {
                false // We're in an explicit undo group, don't start a new one
            } else if let Some(last_time) = self.last_action_time {
                last_time.elapsed() > Duration::from_secs(2)
            } else {
                true // First action
            };
            
            if should_start_new_group {
                // Create a new undo group
                let undo_action = UndoAction::DeleteText {
                    position: Position { line: self.cursor.line, column: self.cursor.column - 1 },
                    text: deleted_char.to_string(),
                    cursor_after: Position { line: self.cursor.line, column: self.cursor.column - 1 },
                };
                self.add_undo_state_new_group(undo_action);
            } else {
                // Add to existing undo group
                if let Some(last_undo) = self.undo_stack.last_mut() {
                    match &mut last_undo.action {
                        UndoAction::DeleteText { ref mut text, ref mut position, .. } => {
                            text.insert(0, deleted_char);
                            position.column -= 1;
                        }
                        _ => {
                            // Different action type, create new group
                            let undo_action = UndoAction::DeleteText {
                                position: Position { line: self.cursor.line, column: self.cursor.column - 1 },
                                text: deleted_char.to_string(),
                                cursor_after: Position { line: self.cursor.line, column: self.cursor.column - 1 },
                            };
                            self.add_undo_state_new_group(undo_action);
                        }
                    }
                } else {
                    // No previous action, create new group
                    let undo_action = UndoAction::DeleteText {
                        position: Position { line: self.cursor.line, column: self.cursor.column - 1 },
                        text: deleted_char.to_string(),
                        cursor_after: Position { line: self.cursor.line, column: self.cursor.column - 1 },
                    };
                    self.add_undo_state_new_group(undo_action);
                }
            }
            
            // Update last action time
            self.last_action_time = Some(Instant::now());
            
            self.cursor.column -= 1;
            self.rope.remove(char_idx..char_idx + 1);
            self.mark_dirty();
            
            // Update syntax for current line
            self.update_syntax_for_line(self.cursor.line);
        } else if self.cursor.line > 0 {
            let prev_line_len = self.rope.line(self.cursor.line - 1).len_chars() - 1; // -1 for newline
            let char_idx = self.rope.line_to_char(self.cursor.line) - 1; // Remove newline
            
            // Check if we should start a new undo group
            // If undo_grouping_active is true, we're in an explicit group (e.g., replace all)
            // Otherwise, check if 2+ seconds have passed since last action
            let should_start_new_group = if self.undo_grouping_active {
                false // We're in an explicit undo group, don't start a new one
            } else if let Some(last_time) = self.last_action_time {
                last_time.elapsed() > Duration::from_secs(2)
            } else {
                true // First action
            };
            
            if should_start_new_group {
                // Create a new undo group
                let undo_action = UndoAction::DeleteText {
                    position: Position { line: self.cursor.line - 1, column: prev_line_len },
                    text: "\n".to_string(),
                    cursor_after: Position { line: self.cursor.line - 1, column: prev_line_len },
                };
                self.add_undo_state_new_group(undo_action);
            } else {
                // Add to existing undo group
                if let Some(last_undo) = self.undo_stack.last_mut() {
                    match &mut last_undo.action {
                        UndoAction::DeleteText { ref mut text, ref mut position, .. } => {
                            text.insert(0, '\n');
                            position.line -= 1;
                            position.column = prev_line_len;
                        }
                        _ => {
                            // Different action type, create new group
                            let undo_action = UndoAction::DeleteText {
                                position: Position { line: self.cursor.line - 1, column: prev_line_len },
                                text: "\n".to_string(),
                                cursor_after: Position { line: self.cursor.line - 1, column: prev_line_len },
                            };
                            self.add_undo_state_new_group(undo_action);
                        }
                    }
                } else {
                    // No previous action, create new group
                    let undo_action = UndoAction::DeleteText {
                        position: Position { line: self.cursor.line - 1, column: prev_line_len },
                        text: "\n".to_string(),
                        cursor_after: Position { line: self.cursor.line - 1, column: prev_line_len },
                    };
                    self.add_undo_state_new_group(undo_action);
                }
            }
            
            // Update last action time
            self.last_action_time = Some(Instant::now());
            
            let deleted_line = self.cursor.line;
            self.rope.remove(char_idx..char_idx + 1);
            self.cursor.line -= 1;
            self.cursor.column = prev_line_len;
            self.mark_dirty();
            
            // Handle line deletion in syntax highlighter
            self.syntax_highlighter.delete_line(&self.rope, deleted_line);
        }
    }

    pub fn delete_char_forwards(&mut self) {
        // Handle selection first
        if let Some((start, end)) = self.cursor.get_selection_range() {
            self.delete_selection(start, end);
            return;
        }

        let line_text = self.get_line_text(self.cursor.line);
        let line_content_len = if line_text.ends_with('\n') {
            line_text.chars().count() - 1
        } else {
            line_text.chars().count()
        };
        
        if self.cursor.column < line_content_len {
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;

            // Proceed with forward deletion
            let deleted_char = self.rope.char(char_idx);

            // Check if we should start a new undo group
            // If undo_grouping_active is true, we're in an explicit group (e.g., replace all)
            // Otherwise, check if 2+ seconds have passed since last action
            let should_start_new_group = if self.undo_grouping_active {
                false // We're in an explicit undo group, don't start a new one
            } else if let Some(last_time) = self.last_action_time {
                last_time.elapsed() > Duration::from_secs(2)
            } else {
                true // First action
            };

            if should_start_new_group {
                // Create a new undo group
                let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                let undo_action = UndoAction::DeleteText {
                    position: current_pos.clone(),
                    text: deleted_char.to_string(),
                    cursor_after: current_pos,
                };
                self.add_undo_state_new_group(undo_action);
            } else {
                // Add to existing undo group
                if let Some(last_undo) = self.undo_stack.last_mut() {
                    match &mut last_undo.action {
                        UndoAction::DeleteText { ref mut text, .. } => {
                            text.push(deleted_char);
                        }
                        _ => {
                            // Different action type, create new group
                            let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                            let undo_action = UndoAction::DeleteText {
                                position: current_pos.clone(),
                                text: deleted_char.to_string(),
                                cursor_after: current_pos,
                            };
                            self.add_undo_state_new_group(undo_action);
                        }
                    }
                } else {
                    // No previous action, create new group
                    let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                    let undo_action = UndoAction::DeleteText {
                        position: current_pos.clone(),
                        text: deleted_char.to_string(),
                        cursor_after: current_pos,
                    };
                    self.add_undo_state_new_group(undo_action);
                }
            }

            // Update last action time
            self.last_action_time = Some(Instant::now());

            self.rope.remove(char_idx..char_idx + 1);
            self.mark_dirty();
            
            // Update syntax for current line
            self.update_syntax_for_line(self.cursor.line);
        } else if self.cursor.line < self.rope.len_lines() - 1 {
            let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;

            // Proceed with newline deletion
            // Check if we should start a new undo group
            // If undo_grouping_active is true, we're in an explicit group (e.g., replace all)
            // Otherwise, check if 2+ seconds have passed since last action
            let should_start_new_group = if self.undo_grouping_active {
                false // We're in an explicit undo group, don't start a new one
            } else if let Some(last_time) = self.last_action_time {
                last_time.elapsed() > Duration::from_secs(2)
            } else {
                true // First action
            };

            if should_start_new_group {
                // Create a new undo group
                let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                let undo_action = UndoAction::DeleteText {
                    position: current_pos.clone(),
                    text: "\n".to_string(),
                    cursor_after: current_pos,
                };
                self.add_undo_state_new_group(undo_action);
            } else {
                // Add to existing undo group
                if let Some(last_undo) = self.undo_stack.last_mut() {
                    match &mut last_undo.action {
                        UndoAction::DeleteText { ref mut text, .. } => {
                            text.push('\n');
                        }
                        _ => {
                            // Different action type, create new group
                            let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                            let undo_action = UndoAction::DeleteText {
                                position: current_pos.clone(),
                                text: "\n".to_string(),
                                cursor_after: current_pos,
                            };
                            self.add_undo_state_new_group(undo_action);
                        }
                    }
                } else {
                    // No previous action, create new group
                    let current_pos = Position { line: self.cursor.line, column: self.cursor.column };
                    let undo_action = UndoAction::DeleteText {
                        position: current_pos.clone(),
                        text: "\n".to_string(),
                        cursor_after: current_pos,
                    };
                    self.add_undo_state_new_group(undo_action);
                }
            }

            // Update last action time
            self.last_action_time = Some(Instant::now());

            let next_line = self.cursor.line + 1;
            self.rope.remove(char_idx..char_idx + 1);
            self.mark_dirty();
            
            // Handle line deletion in syntax highlighter
            self.syntax_highlighter.delete_line(&self.rope, next_line);
        }
    }

    fn delete_selection(&mut self, start: Position, end: Position) {
        let start_char_idx = self.rope.line_to_char(start.line) + start.column;
        let end_char_idx = self.rope.line_to_char(end.line) + end.column;

        if start_char_idx < end_char_idx {
            let deleted_text = self.rope.slice(start_char_idx..end_char_idx).to_string();
            
            // Count how many lines are being deleted
            let lines_deleted = end.line - start.line;
            
            self.rope.remove(start_char_idx..end_char_idx);

            let undo_action = UndoAction::DeleteText {
                position: start.clone(),
                text: deleted_text,
                cursor_after: start.clone(),
            };
            self.add_undo_state_new_group(undo_action);

            // Update cursor position and clear selection
            self.cursor.line = start.line;
            self.cursor.column = start.column;
            self.cursor.preferred_visual_column = start.column;
            self.cursor.clear_selection();

            self.mark_dirty();
            
            // Handle the deletion in the syntax highlighter
            if lines_deleted > 0 {
                // Multiple lines were deleted
                for _ in 0..lines_deleted {
                    self.syntax_highlighter.delete_line(&self.rope, start.line + 1);
                }
            }
            // Update the line where deletion occurred
            self.update_syntax_for_line(start.line);
        }
    }

    /// Delete selected text without copying to clipboard (for internal use like find/replace)
    pub fn delete_selection_no_clipboard(&mut self, start: Position, end: Position) {
        let start_char_idx = self.rope.line_to_char(start.line) + start.column;
        let end_char_idx = self.rope.line_to_char(end.line) + end.column;

        if start_char_idx < end_char_idx {
            let deleted_text = self.rope.slice(start_char_idx..end_char_idx).to_string();
            
            // Count how many lines are being deleted
            let lines_deleted = end.line - start.line;
            
            self.rope.remove(start_char_idx..end_char_idx);

            // Always add to undo stack - whether we're in a group or not
            // The grouping logic will handle whether to create a new group or add to existing
            let undo_action = UndoAction::DeleteText {
                position: start.clone(),
                text: deleted_text,
                cursor_after: start.clone(),
            };
            
            // Check if we should start a new undo group
            let should_start_new_group = if self.undo_grouping_active {
                false // We're in an explicit undo group, don't start a new one
            } else if let Some(last_time) = self.last_action_time {
                last_time.elapsed() > Duration::from_secs(2)
            } else {
                true // First action
            };
            
            if should_start_new_group || self.undo_stack.is_empty() {
                self.add_undo_state_new_group(undo_action);
            } else {
                // Add to existing group or create new one if different action type
                if let Some(last_undo) = self.undo_stack.last_mut() {
                    match &mut last_undo.action {
                        UndoAction::DeleteText { ref mut text, .. } => {
                            // Append to existing delete action
                            if let UndoAction::DeleteText { text: ref new_text, .. } = undo_action {
                                text.push_str(new_text);
                            }
                        }
                        _ => {
                            // Different action type, create new group
                            self.add_undo_state_new_group(undo_action);
                        }
                    }
                } else {
                    self.add_undo_state_new_group(undo_action);
                }
            }
            
            // Update last action time
            self.last_action_time = Some(Instant::now());

            // Update cursor position and clear selection
            self.cursor.line = start.line;
            self.cursor.column = start.column;
            self.cursor.preferred_visual_column = start.column;
            self.cursor.clear_selection();

            self.mark_dirty();
            
            // Handle the deletion in the syntax highlighter
            if lines_deleted > 0 {
                // Multiple lines were deleted
                for _ in 0..lines_deleted {
                    self.syntax_highlighter.delete_line(&self.rope, start.line + 1);
                }
            }
            // Update the line where deletion occurred
            self.update_syntax_for_line(start.line);
        }
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
        self.needs_syntax_update = true;
        self.update_last_change();
    }
    
    /// Update the last change timestamp (used for auto-save)
    fn update_last_change(&mut self) {
        self.last_change = Some(Instant::now());
    }

    pub fn save(&mut self, path: Option<PathBuf>) -> Result<()> {
        let save_path = path.or_else(|| self.file_path.clone())
            .ok_or_else(|| anyhow::anyhow!("No file path specified"))?;
        
        // Convert back to original line ending style
        let content = self.rope.to_string();
        let final_content = if self.line_ending == LineEnding::CRLF {
            content.replace("\n", "\r\n")
        } else {
            content
        };
        
        std::fs::write(&save_path, final_content)?;
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
    pub fn force_syntax_update(&mut self) {
        if self.language != "text" {
            self.syntax_highlighter.update(&self.rope);
            self.needs_syntax_update = false;
        }
    }
    
    // Update syntax highlighting for a specific line
    fn update_syntax_for_line(&mut self, line: usize) {
        if self.language != "text" {
            self.syntax_highlighter.update_line(&self.rope, line);
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
                
                // On Windows, convert to CRLF for clipboard
                #[cfg(windows)]
                let clipboard_text = selected_text.replace("\n", "\r\n");
                #[cfg(not(windows))]
                let clipboard_text = selected_text;
                
                clipboard.set_text(clipboard_text)?;
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
                
                // On Windows, convert to CRLF for clipboard
                #[cfg(windows)]
                let clipboard_text = selected_text.replace("\n", "\r\n");
                #[cfg(not(windows))]
                let clipboard_text = selected_text.clone();
                
                clipboard.set_text(clipboard_text)?;
                
                // Push cut to undo stack as delete
                // If we're in an undo group, use add_undo_state to keep it in the group
                // Otherwise, use add_undo_state_new_group to start a new group
                let undo_action = UndoAction::DeleteText {
                    position: start.clone(),
                    text: selected_text.clone(),
                    cursor_after: start.clone(),
                };
                
                if self.undo_grouping_active {
                    self.add_undo_state(undo_action);
                } else {
                    self.add_undo_state_new_group(undo_action);
                }

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
            // Normalize line endings to LF
            let normalized_text = text.replace("\r\n", "\n");
            
            // For very large pastes, we'll use optimized insertion
            let is_large_paste = normalized_text.len() > 10000 || normalized_text.matches('\n').count() > 100;
            
            // Handle selection - delete selected text first
            if let Some((start, end)) = self.cursor.get_selection_range() {
                self.delete_selection(start, end);
            }
            
            // Determine paste start position
            let start_position = Position { line: self.cursor.line, column: self.cursor.column };
            
            // Insert the text at cursor position
            if is_large_paste {
                // Use optimized insertion for large pastes
                self.insert_text_at_cursor_optimized(&normalized_text);
            } else {
                // Regular insertion with immediate syntax highlighting
                self.insert_text_at_cursor(&normalized_text);
            }

            // Determine cursor position after paste
            let cursor_after = Position { line: self.cursor.line, column: self.cursor.column };

            // Push paste to undo stack as insert
            self.add_undo_state(UndoAction::InsertText {
                position: start_position,
                text: normalized_text.clone(),
                cursor_after,
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
    
    /// Delete text within a range (without adding to undo stack)
    fn delete_text_range(&mut self, start: Position, end: Position) {
        let start_char_idx = self.rope.line_to_char(start.line) + start.column;
        let end_char_idx = self.rope.line_to_char(end.line) + end.column;

        if start_char_idx < end_char_idx && end_char_idx <= self.rope.len_chars() {
            // Count how many lines are being deleted
            let lines_deleted = end.line - start.line;
            
            self.rope.remove(start_char_idx..end_char_idx);
            self.mark_dirty();
            
            // Handle the deletion in the syntax highlighter
            if lines_deleted > 0 {
                // Multiple lines were deleted
                for _ in 0..lines_deleted {
                    self.syntax_highlighter.delete_line(&self.rope, start.line + 1);
                }
            }
            // Update the line where deletion occurred
            self.update_syntax_for_line(start.line);
        }
    }

    /// Insert text at current cursor position (without adding to undo stack)
    fn insert_text_at_cursor(&mut self, text: &str) {
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        
        // Count newlines to determine how many lines are being inserted
        let newline_count = text.matches('\n').count();
        let start_line = self.cursor.line;
        
        self.rope.insert(char_idx, text);
        
        // Update cursor position based on inserted text
        if newline_count > 0 {
            // Handle multi-line insertion
            let lines_before_cursor = self.cursor.line;
            self.cursor.line += newline_count;
            
            // Find the column position after the last newline
            if let Some(last_newline_pos) = text.rfind('\n') {
                self.cursor.column = text[last_newline_pos + 1..].chars().count();
            }
            
            // Update syntax highlighting for inserted lines
            for i in 1..=newline_count {
                self.syntax_highlighter.insert_line(&self.rope, lines_before_cursor + i);
            }
        } else {
            self.cursor.column += text.chars().count();
        }
        
        self.cursor.preferred_visual_column = self.cursor.column;
        self.mark_dirty();
        
        // Update syntax for all affected lines (from start line to end)
        // This is important for multi-line pastes to ensure proper highlighting
        if newline_count > 0 {
            // For multi-line insertion, update from start to end
            for line_idx in start_line..=self.cursor.line {
                self.update_syntax_for_line(line_idx);
            }
        } else {
            // Single line insertion, just update the current line
            self.update_syntax_for_line(self.cursor.line);
        }
    }
    
    /// Optimized version for large text insertions that defers syntax highlighting
    fn insert_text_at_cursor_optimized(&mut self, text: &str) {
        let char_idx = self.rope.line_to_char(self.cursor.line) + self.cursor.column;
        
        let newline_count = text.matches('\n').count();
        let start_line = self.cursor.line;
        
        // Insert the text
        self.rope.insert(char_idx, text);
        
        // Update cursor position
        if newline_count > 0 {
            self.cursor.line += newline_count;
            if let Some(last_newline_pos) = text.rfind('\n') {
                self.cursor.column = text[last_newline_pos + 1..].chars().count();
            }
            
            // Just update line tracking for syntax highlighter
            for i in 1..=newline_count {
                self.syntax_highlighter.insert_line(&self.rope, start_line + i);
            }
        } else {
            self.cursor.column += text.chars().count();
        }
        
        self.cursor.preferred_visual_column = self.cursor.column;
        self.mark_dirty();
        
        // Mark that we need syntax update but don't do it now
        self.needs_syntax_update = true;
    }
    
    /// Add an action to the undo stack (for operations like paste/cut that don't group)
    fn add_undo_state(&mut self, action: UndoAction) {
        let group_id = if self.undo_grouping_active {
            Some(self.current_undo_group_id)
        } else {
            None
        };
        
        let undo_state = UndoState {
            action,
            selection_before: self.cursor.get_selection_range(),
            selection_after: None, // Will be set after the action is performed
            group_id,
        };
        
        self.undo_stack.push(undo_state);
        
        // Clear redo stack when new action is added
        self.redo_stack.clear();
        
        // Limit undo stack size
        if self.undo_stack.len() > self.max_undo_stack_size {
            self.undo_stack.remove(0);
        }
    }
    
    /// Add an action to the undo stack for a new group (updates timestamp)
    fn add_undo_state_new_group(&mut self, action: UndoAction) {
        let group_id = if self.undo_grouping_active {
            Some(self.current_undo_group_id)
        } else {
            None
        };
        
        let undo_state = UndoState {
            action,
            selection_before: self.cursor.get_selection_range(),
            selection_after: None, // Will be set after the action is performed
            group_id,
        };
        
        self.undo_stack.push(undo_state);
        
        // Update action timestamp for grouping (starting a new group)
        // If undo_grouping_active is true, we keep the existing timestamp to continue the group
        if !self.undo_grouping_active {
            self.last_action_time = Some(Instant::now());
        }
        
        // Clear redo stack when new action is added
        self.redo_stack.clear();
        
        // Limit undo stack size
        if self.undo_stack.len() > self.max_undo_stack_size {
            self.undo_stack.remove(0);
        }
    }
    
    /// Undo the last action or group of actions
    pub fn undo(&mut self) -> bool {
        if let Some(undo_state) = self.undo_stack.pop() {
            let group_id = undo_state.group_id;
            
            // Process the first action
            self.apply_undo_action(&undo_state);
            self.redo_stack.push(undo_state);
            
            // If this action belongs to a group, undo all actions in the same group
            if let Some(gid) = group_id {
                while let Some(next_state) = self.undo_stack.last() {
                    if next_state.group_id == Some(gid) {
                        let state = self.undo_stack.pop().unwrap();
                        self.apply_undo_action(&state);
                        self.redo_stack.push(state);
                    } else {
                        break;
                    }
                }
            }
            
            true
        } else {
            false
        }
    }
    
    /// Apply a single undo action
    fn apply_undo_action(&mut self, undo_state: &UndoState) {
        match &undo_state.action {
            UndoAction::InsertText { position, text, cursor_after: _ } => {
                // Undo insert by deleting the inserted text
                let end_pos = self.calculate_end_position(position.clone(), text);
                self.delete_text_range(position.clone(), end_pos);
                self.cursor.line = position.line;
                self.cursor.column = position.column;
                self.cursor.preferred_visual_column = position.column;
                self.cursor.clear_selection();
            }
            UndoAction::DeleteText { position, text, cursor_after: _ } => {
                // Undo delete by inserting the deleted text
                self.cursor.line = position.line;
                self.cursor.column = position.column;
                self.insert_text_at_cursor(text);
            }
            UndoAction::ReplaceText { position, old_text, new_text: _, cursor_after: _ } => {
                // Undo replace by replacing new text with old text
                let end_pos = self.calculate_end_position(position.clone(), old_text);
                self.delete_text_range(position.clone(), end_pos);
                self.cursor.line = position.line;
                self.cursor.column = position.column;
                self.insert_text_at_cursor(old_text);
            }
        }
    }
    
    /// Redo the last undone action or group of actions
    pub fn redo(&mut self) -> bool {
        if let Some(redo_state) = self.redo_stack.pop() {
            let group_id = redo_state.group_id;
            
            // If this is part of a group, we need to redo all actions in the group
            // But they're in reverse order in the redo stack, so collect them first
            let mut group_actions = vec![redo_state];
            
            if let Some(gid) = group_id {
                while let Some(next_state) = self.redo_stack.last() {
                    if next_state.group_id == Some(gid) {
                        group_actions.push(self.redo_stack.pop().unwrap());
                    } else {
                        break;
                    }
                }
            }
            
            // Apply actions in reverse order (since we popped them from the stack)
            for state in group_actions.into_iter().rev() {
                self.apply_redo_action(&state);
                self.undo_stack.push(state);
            }
            
            true
        } else {
            false
        }
    }
    
    /// Apply a single redo action
    fn apply_redo_action(&mut self, redo_state: &UndoState) {
        match &redo_state.action {
            UndoAction::InsertText { position, text, cursor_after } => {
                // Redo insert
                self.cursor.line = position.line;
                self.cursor.column = position.column;
                self.insert_text_at_cursor(text);
                self.cursor.line = cursor_after.line;
                self.cursor.column = cursor_after.column;
                self.cursor.preferred_visual_column = cursor_after.column;
            }
            UndoAction::DeleteText { position, text, cursor_after } => {
                // Redo delete
                let end_pos = self.calculate_end_position(position.clone(), text);
                self.delete_text_range(position.clone(), end_pos);
                self.cursor.line = cursor_after.line;
                self.cursor.column = cursor_after.column;
                self.cursor.preferred_visual_column = cursor_after.column;
            }
            UndoAction::ReplaceText { position, old_text: _, new_text, cursor_after } => {
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
    
    /// Indent the current line or all selected lines
    pub fn indent_lines(&mut self) {
        const INDENT: &str = "    "; // 4 spaces
        
        if let Some((start, end)) = self.cursor.get_selection_range() {
            // Multiple lines selected - indent all of them
            let start_line = start.line;
            let end_line = end.line;
            
            // Store the original text for undo
            let mut original_text = String::new();
            let mut new_text = String::new();
            
            for line_num in start_line..=end_line {
                let line_text = self.get_line_text(line_num);
                original_text.push_str(&line_text);
                new_text.push_str(INDENT);
                new_text.push_str(&line_text);
            }
            
            // Calculate the character positions
            let start_char_idx = self.rope.line_to_char(start_line);
            let end_char_idx = if end_line + 1 < self.rope.len_lines() {
                self.rope.line_to_char(end_line + 1)
            } else {
                self.rope.len_chars()
            };
            
            // Remove the original lines and insert the indented ones
            self.rope.remove(start_char_idx..end_char_idx);
            self.rope.insert(start_char_idx, &new_text);
            
            // Update cursor position - move it forward by the indent amount
            if self.cursor.line == end.line {
                self.cursor.column += INDENT.len();
            }
            
            // Update selection to maintain it after indentation
            if let Some(sel_start) = self.cursor.selection_start.as_mut() {
                if sel_start.line >= start_line && sel_start.line <= end_line {
                    sel_start.column += INDENT.len();
                }
            }
            
            // Add to undo stack
            let undo_action = UndoAction::ReplaceText {
                position: Position { line: start_line, column: 0 },
                old_text: original_text,
                new_text,
                cursor_after: Position { line: self.cursor.line, column: self.cursor.column },
            };
            self.add_undo_state_new_group(undo_action);
            
            self.mark_dirty();
            
            // Update syntax highlighting for affected lines
            for line_num in start_line..=end_line {
                self.update_syntax_for_line(line_num);
            }
        } else {
            // No selection - just indent the current line
            let line_num = self.cursor.line;
            let line_start_char_idx = self.rope.line_to_char(line_num);
            
            // Insert the indent at the beginning of the line
            self.rope.insert(line_start_char_idx, INDENT);
            
            // Update cursor position
            self.cursor.column += INDENT.len();
            self.cursor.preferred_visual_column = self.cursor.column;
            
            // Add to undo stack
            let undo_action = UndoAction::InsertText {
                position: Position { line: line_num, column: 0 },
                text: INDENT.to_string(),
                cursor_after: Position { line: self.cursor.line, column: self.cursor.column },
            };
            self.add_undo_state_new_group(undo_action);
            
            self.mark_dirty();
            self.update_syntax_for_line(line_num);
        }
    }
    
    /// Dedent (unindent) the current line or all selected lines
    pub fn dedent_lines(&mut self) {
    const INDENT_SIZE: usize = 4;
        
        if let Some((start, end)) = self.cursor.get_selection_range() {
            // Multiple lines selected - dedent all of them
            let start_line = start.line;
            let end_line = end.line;
            
            // Process each line
            for line_num in (start_line..=end_line).rev() {
                self.dedent_single_line(line_num);
            }
            
            // Update syntax highlighting for affected lines
            for line_num in start_line..=end_line {
                self.update_syntax_for_line(line_num);
            }
        } else {
            // No selection - just dedent the current line
            let line_num = self.cursor.line;
            self.dedent_single_line(line_num);
            self.update_syntax_for_line(line_num);
        }
    }
    
    /// Start an undo group - all subsequent operations will be grouped together
    pub fn start_undo_group(&mut self) {
        self.undo_grouping_active = true;
        self.current_undo_group_id += 1; // Increment group ID for this new group
        // Force the next action to NOT start a new group
        self.last_action_time = Some(Instant::now());
    }
    
    /// End an undo group - next operation will start a new group
    pub fn end_undo_group(&mut self) {
        self.undo_grouping_active = false;
        // Clear the last action time to force the next action to start a new group
        self.last_action_time = None;
    }
    
    /// Helper to dedent a single line
    fn dedent_single_line(&mut self, line_num: usize) {
        let line_text = self.get_line_text(line_num);
        let line_start_char_idx = self.rope.line_to_char(line_num);
        
        // Count spaces at the beginning of the line
        let mut spaces_to_remove = 0;
        for (i, ch) in line_text.chars().enumerate() {
            if ch == ' ' && i < 4 {
                spaces_to_remove += 1;
            } else if ch == '\t' && i == 0 {
                // If line starts with tab, remove it
                spaces_to_remove = 1;
                break;
            } else {
                break;
            }
        }
        
        if spaces_to_remove > 0 {
            // Remove the spaces
            self.rope.remove(line_start_char_idx..line_start_char_idx + spaces_to_remove);
            
            // Update cursor position if on this line
            if self.cursor.line == line_num {
                self.cursor.column = self.cursor.column.saturating_sub(spaces_to_remove);
                self.cursor.preferred_visual_column = self.cursor.column;
            }
            
            // Update selection if it's on this line
            if let Some(sel_start) = self.cursor.selection_start.as_mut() {
                if sel_start.line == line_num {
                    sel_start.column = sel_start.column.saturating_sub(spaces_to_remove);
                }
            }
            
            // Add to undo stack
            let deleted_text = " ".repeat(spaces_to_remove);
            let undo_action = UndoAction::DeleteText {
                position: Position { line: line_num, column: 0 },
                text: deleted_text,
                cursor_after: Position { line: self.cursor.line, column: self.cursor.column },
            };
            self.add_undo_state_new_group(undo_action);
            
            self.mark_dirty();
        }
    }
    
    /// Move selected lines up
    pub fn move_lines_up(&mut self) {
        if let Some((start, end)) = self.cursor.get_selection_range() {
            // Multiple lines selected - move all of them
            let start_line = start.line;
            let end_line = end.line;
            
            // Can't move if already at the top
            if start_line == 0 {
                return;
            }
            
            // Calculate the exact character positions
            let move_start = self.rope.line_to_char(start_line - 1);
            let move_end = if end_line + 1 < self.rope.len_lines() {
                self.rope.line_to_char(end_line + 1)
            } else {
                self.rope.len_chars()
            };
            
            // Extract the entire text block to move (line above + selected lines)
            let text_to_move = self.rope.slice(move_start..move_end).to_string();
            
            // Split the text into the line above and the selected lines
            let line_above_end = self.rope.line_to_char(start_line) - move_start;
            let line_above = &text_to_move[..line_above_end];
            let selected_lines = &text_to_move[line_above_end..];
            
            // Start undo group for the entire operation
            self.start_undo_group();
            
            // Remove the entire block
            self.rope.remove(move_start..move_end);
            
            // Insert in the new order: selected lines first, then line above
            self.rope.insert(move_start, selected_lines);
            self.rope.insert(move_start + selected_lines.len(), line_above);
            
            // Update cursor and selection positions
            self.cursor.line -= 1;
            if let Some(sel_start) = self.cursor.selection_start.as_mut() {
                sel_start.line -= 1;
            }
            
            // Add to undo stack
            let undo_action = UndoAction::ReplaceText {
                position: Position { line: start_line - 1, column: 0 },
                old_text: text_to_move.clone(),
                new_text: format!("{}{}", selected_lines, line_above),
                cursor_after: Position { line: self.cursor.line, column: self.cursor.column },
            };
            self.add_undo_state_new_group(undo_action);
            
            self.end_undo_group();
            self.mark_dirty();
            
            // Update syntax highlighting for affected lines
            for line_num in (start_line - 1)..=(end_line) {
                if line_num < self.rope.len_lines() {
                    self.update_syntax_for_line(line_num);
                }
            }
        } else {
            // No selection - just move the current line
            let line_num = self.cursor.line;
            
            if line_num == 0 {
                return;
            }
            
            // Calculate exact positions for both lines
            let line_above_start = self.rope.line_to_char(line_num - 1);
            let current_line_end = if line_num + 1 < self.rope.len_lines() {
                self.rope.line_to_char(line_num + 1)
            } else {
                self.rope.len_chars()
            };
            
            // Extract both lines together
            let both_lines = self.rope.slice(line_above_start..current_line_end).to_string();
            
            // Find where the current line starts within the extracted text
            let current_line_start_offset = self.rope.line_to_char(line_num) - line_above_start;
            let line_above = &both_lines[..current_line_start_offset];
            let current_line = &both_lines[current_line_start_offset..];
            
            // Start undo group
            self.start_undo_group();
            
            // Replace both lines with swapped order
            self.rope.remove(line_above_start..current_line_end);
            self.rope.insert(line_above_start, current_line);
            self.rope.insert(line_above_start + current_line.len(), line_above);
            
            // Update cursor position
            self.cursor.line -= 1;
            
            // Add to undo stack
            let undo_action = UndoAction::ReplaceText {
                position: Position { line: line_num - 1, column: 0 },
                old_text: both_lines.clone(),
                new_text: format!("{}{}", current_line, line_above),
                cursor_after: Position { line: self.cursor.line, column: self.cursor.column },
            };
            self.add_undo_state_new_group(undo_action);
            
            self.end_undo_group();
            self.mark_dirty();
            
            // Update syntax highlighting
            self.update_syntax_for_line(line_num - 1);
            self.update_syntax_for_line(line_num);
        }
    }
    
    /// Move selected lines down
    pub fn move_lines_down(&mut self) {
        if let Some((start, end)) = self.cursor.get_selection_range() {
            // Multiple lines selected - move all of them
            let start_line = start.line;
            let end_line = end.line;
            
            // Can't move if already at the bottom
            if end_line >= self.rope.len_lines() - 1 {
                return;
            }
            
            // Calculate the exact character positions
            let move_start = self.rope.line_to_char(start_line);
            let move_end = if end_line + 2 < self.rope.len_lines() {
                self.rope.line_to_char(end_line + 2)
            } else {
                self.rope.len_chars()
            };
            
            // Extract the entire text block to move (selected lines + line below)
            let text_to_move = self.rope.slice(move_start..move_end).to_string();
            
            // Split the text into selected lines and the line below
            let line_below_start_offset = self.rope.line_to_char(end_line + 1) - move_start;
            let selected_lines = &text_to_move[..line_below_start_offset];
            let line_below = &text_to_move[line_below_start_offset..];
            
            // Start undo group for the entire operation
            self.start_undo_group();
            
            // Remove the entire block
            self.rope.remove(move_start..move_end);
            
            // Insert in the new order: line below first, then selected lines
            self.rope.insert(move_start, line_below);
            self.rope.insert(move_start + line_below.len(), selected_lines);
            
            // Update cursor and selection positions
            self.cursor.line += 1;
            if let Some(sel_start) = self.cursor.selection_start.as_mut() {
                sel_start.line += 1;
            }
            
            // Add to undo stack
            let undo_action = UndoAction::ReplaceText {
                position: Position { line: start_line, column: 0 },
                old_text: text_to_move.clone(),
                new_text: format!("{}{}", line_below, selected_lines),
                cursor_after: Position { line: self.cursor.line, column: self.cursor.column },
            };
            self.add_undo_state_new_group(undo_action);
            
            self.end_undo_group();
            self.mark_dirty();
            
            // Update syntax highlighting for affected lines
            for line_num in start_line..=(end_line + 1) {
                if line_num < self.rope.len_lines() {
                    self.update_syntax_for_line(line_num);
                }
            }
        } else {
            // No selection - just move the current line
            let line_num = self.cursor.line;
            
            if line_num >= self.rope.len_lines() - 1 {
                return;
            }
            
            // Calculate exact positions for both lines
            let current_line_start = self.rope.line_to_char(line_num);
            let line_below_end = if line_num + 2 < self.rope.len_lines() {
                self.rope.line_to_char(line_num + 2)
            } else {
                self.rope.len_chars()
            };
            
            // Extract both lines together
            let both_lines = self.rope.slice(current_line_start..line_below_end).to_string();
            
            // Find where the line below starts within the extracted text
            let line_below_start_offset = self.rope.line_to_char(line_num + 1) - current_line_start;
            let current_line = &both_lines[..line_below_start_offset];
            let line_below = &both_lines[line_below_start_offset..];
            
            // Start undo group
            self.start_undo_group();
            
            // Replace both lines with swapped order
            self.rope.remove(current_line_start..line_below_end);
            self.rope.insert(current_line_start, line_below);
            self.rope.insert(current_line_start + line_below.len(), current_line);
            
            // Update cursor position
            self.cursor.line += 1;
            
            // Add to undo stack
            let undo_action = UndoAction::ReplaceText {
                position: Position { line: line_num, column: 0 },
                old_text: both_lines.clone(),
                new_text: format!("{}{}", line_below, current_line),
                cursor_after: Position { line: self.cursor.line, column: self.cursor.column },
            };
            self.add_undo_state_new_group(undo_action);
            
            self.end_undo_group();
            self.mark_dirty();
            
            // Update syntax highlighting
            self.update_syntax_for_line(line_num);
            self.update_syntax_for_line(line_num + 1);
        }
    }
}