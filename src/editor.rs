// src/editor.rs

use crate::{
    buffer::Buffer,
    config::{Config, Theme},
    text_utils::wrap_line
};
use crate::cursor::Position;
use anyhow::Result;
use std::path::PathBuf;

pub struct Editor {
    pub buffers: Vec<Buffer>,
    pub active_buffer: usize,
    pub viewport_line: isize,  // In word-wrap mode, this represents the first VISUAL line in viewport
    pub horizontal_offset: usize,
    // Language selection mode
    pub language_selection_mode: bool,
    pub language_selection_index: usize,
    pub language_selection_scroll_offset: usize,
    // Theme selection mode
    pub theme_selection_mode: bool,
    pub theme_selection_index: usize,
    pub theme_selection_scroll_offset: usize,
    pub available_themes: Vec<(String, String)>, // (filename, display_name)
    // Help mode
    pub help_mode: bool,
    pub help_scroll_offset: usize,
    // Filename prompt mode
    pub filename_prompt_mode: bool,
    pub filename_prompt_text: String,
    pub filename_cursor_pos: usize,
    pub paste_in_progress: bool,
    pub paste_progress: Option<String>,
    // Find/Replace mode
    pub find_replace_mode: bool,
    pub find_query: String,
    pub replace_text: String,
    pub find_matches: Vec<(usize, usize, usize)>, // (line, start_col, end_col)
    pub current_match_index: Option<usize>,
    pub find_replace_focus: FindReplaceFocus,
    pub find_cursor_pos: usize, // Cursor position within find field
    pub replace_cursor_pos: usize, // Cursor position within replace field
    pub find_selection_start: Option<usize>, // Selection start in find field
    pub replace_selection_start: Option<usize>, // Selection start in replace field
    // Save prompt mode
    pub save_prompt_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FindReplaceFocus {
    FindField,
    ReplaceField,
    Editor,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            active_buffer: 0,
            viewport_line: 0,
            horizontal_offset: 0,
            language_selection_mode: false,
            language_selection_index: 0,
            language_selection_scroll_offset: 0,
            theme_selection_mode: false,
            theme_selection_index: 0,
            theme_selection_scroll_offset: 0,
            available_themes: Vec::new(),
            help_mode: false,
            help_scroll_offset: 0,
            filename_prompt_mode: false,
            filename_prompt_text: String::new(),
            filename_cursor_pos: 0,
            paste_in_progress: false,
            paste_progress: None,
            find_replace_mode: false,
            find_query: String::new(),
            replace_text: String::new(),
            find_matches: Vec::new(),
            current_match_index: None,
            find_replace_focus: FindReplaceFocus::FindField,
            find_cursor_pos: 0,
            replace_cursor_pos: 0,
            find_selection_start: None,
            replace_selection_start: None,
            save_prompt_mode: false,
        }
    }

    pub fn new_buffer(&mut self) {
        self.buffers.push(Buffer::new());
        self.active_buffer = self.buffers.len() - 1;
    }

    pub async fn open_file(&mut self, path: &PathBuf) -> Result<()> {
        let buffer = Buffer::from_file(path.clone())?;
        self.buffers.push(buffer);
        self.active_buffer = self.buffers.len() - 1;
        Ok(())
    }

    pub fn current_buffer(&self) -> Option<&Buffer> {
        self.buffers.get(self.active_buffer)
    }

    pub fn current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        self.buffers.get_mut(self.active_buffer)
    }

    pub async fn save_current_buffer(&mut self) -> Result<()> {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.save(None)?;
        }
        Ok(())
    }

    // Language selection methods
    pub fn enter_language_selection_mode(&mut self) {
        if self.current_buffer().is_some() {
            self.language_selection_mode = true;
            self.language_selection_index = 0;
            self.language_selection_scroll_offset = 0;
            
            // Set the current index to match the current language
            if let Some(buffer) = self.current_buffer() {
                let supported_languages = Buffer::get_supported_languages();
                if let Some(pos) = supported_languages.iter().position(|&lang| lang == buffer.language) {
                    self.language_selection_index = pos;
                    self.update_language_scroll();
                }
            }
        }
    }

    pub fn exit_language_selection_mode(&mut self) {
        self.language_selection_mode = false;
    }

    pub fn language_selection_up(&mut self) {
        if self.language_selection_mode {
            let languages = Buffer::get_supported_languages();
            if !languages.is_empty() {
                if self.language_selection_index > 0 {
                    self.language_selection_index -= 1;
                } else {
                    self.language_selection_index = languages.len() - 1;
                }
                self.update_language_scroll();
            }
        }
    }

    pub fn language_selection_down(&mut self) {
        if self.language_selection_mode {
            let languages = Buffer::get_supported_languages();
            if !languages.is_empty() {
                if self.language_selection_index < languages.len() - 1 {
                    self.language_selection_index += 1;
                } else {
                    self.language_selection_index = 0;
                }
                self.update_language_scroll();
            }
        }
    }

    fn update_language_scroll(&mut self) {
        let max_visible_items = 15; // Same as in UI
        
        // If selected item is before the current scroll offset, scroll up
        if self.language_selection_index < self.language_selection_scroll_offset {
            self.language_selection_scroll_offset = self.language_selection_index;
        }
        // If selected item is beyond the visible area, scroll down
        else if self.language_selection_index >= self.language_selection_scroll_offset + max_visible_items {
            self.language_selection_scroll_offset = self.language_selection_index - max_visible_items + 1;
        }
    }

    pub fn apply_selected_language(&mut self) -> bool {
        if self.language_selection_mode {
            let languages = Buffer::get_supported_languages();
            if let Some(&selected_language) = languages.get(self.language_selection_index) {
                if let Some(buffer) = self.current_buffer_mut() {
                    buffer.set_language(selected_language);
                }
                self.language_selection_mode = false;
                return true;
            }
        }
        false
    }

    pub fn get_language_selection_info(&self) -> Option<(Vec<&'static str>, usize)> {
        if self.language_selection_mode {
            Some((Buffer::get_supported_languages(), self.language_selection_index))
        } else {
            None
        }
    }


    // Theme selection methods
    pub fn enter_theme_selection_mode(&mut self, current_theme_name: &str) -> Result<()> {
        self.theme_selection_mode = true;
        self.theme_selection_index = 0;
        self.theme_selection_scroll_offset = 0;
        
        // Load available themes
        let themes_dir = Config::themes_dir()?;
        self.available_themes = Vec::new();
        
        // Add the built-in default theme
        self.available_themes.push(("_default".to_string(), "Default Dark".to_string()));
        
        if themes_dir.exists() {
            let mut theme_files: Vec<(String, String)> = std::fs::read_dir(themes_dir)?
                .filter_map(|entry| {
                    entry.ok().and_then(|e| {
                        let path = e.path();
                        if path.extension()?.to_str()? == "toml" {
                            let filename = path.file_stem()?.to_str()?.to_string();
                            // Try to load theme to get display name
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(theme) = toml::from_str::<Theme>(&content) {
                                    return Some((filename, theme.name));
                                }
                            }
                            Some((filename.clone(), filename))
                        } else {
                            None
                        }
                    })
                })
                .collect();
            
            theme_files.sort_by(|a, b| a.1.cmp(&b.1));
            self.available_themes.extend(theme_files);
        }
        
        // Find current theme index
        if let Some(pos) = self.available_themes.iter().position(|(_, name)| name == current_theme_name) {
            self.theme_selection_index = pos;
            self.update_theme_scroll();
        }
        
        Ok(())
    }

    pub fn exit_theme_selection_mode(&mut self) {
        self.theme_selection_mode = false;
        self.available_themes.clear();
    }

    pub fn theme_selection_up(&mut self) {
        if self.theme_selection_mode && !self.available_themes.is_empty() {
            if self.theme_selection_index > 0 {
                self.theme_selection_index -= 1;
            } else {
                self.theme_selection_index = self.available_themes.len() - 1;
            }
            self.update_theme_scroll();
        }
    }

    pub fn theme_selection_down(&mut self) {
        if self.theme_selection_mode && !self.available_themes.is_empty() {
            if self.theme_selection_index < self.available_themes.len() - 1 {
                self.theme_selection_index += 1;
            } else {
                self.theme_selection_index = 0;
            }
            self.update_theme_scroll();
        }
    }

    fn update_theme_scroll(&mut self) {
        let max_visible_items = 15; // Same as in UI
        
        // If selected item is before the current scroll offset, scroll up
        if self.theme_selection_index < self.theme_selection_scroll_offset {
            self.theme_selection_scroll_offset = self.theme_selection_index;
        }
        // If selected item is beyond the visible area, scroll down
        else if self.theme_selection_index >= self.theme_selection_scroll_offset + max_visible_items {
            self.theme_selection_scroll_offset = self.theme_selection_index - max_visible_items + 1;
        }
    }

    pub fn get_selected_theme(&self) -> Option<&str> {
        if self.theme_selection_mode {
            self.available_themes.get(self.theme_selection_index)
                .map(|(filename, _)| filename.as_str())
        } else {
            None
        }
    }

    pub fn get_theme_selection_info(&self) -> Option<(&[(String, String)], usize)> {
        if self.theme_selection_mode {
            Some((&self.available_themes, self.theme_selection_index))
        } else {
            None
        }
    }

    // Find/Replace methods
    pub fn enter_find_replace_mode(&mut self) {
        self.find_replace_mode = true;
        self.find_replace_focus = FindReplaceFocus::FindField;
        self.find_selection_start = None;
        self.replace_selection_start = None;
        
        // If there's selected text, use it as the initial search query
        if let Some(buffer) = self.current_buffer() {
            if let Some((start, end)) = buffer.cursor.get_selection_range() {
                if start.line == end.line {
                    let line_text = buffer.get_line_text(start.line);
                    let start_idx = start.column;
                    let end_idx = end.column.min(line_text.len());
                    if start_idx < end_idx {
                        self.find_query = line_text[start_idx..end_idx].to_string();
                        self.find_cursor_pos = self.find_query.len();
                        self.update_find_matches();
                    }
                }
            }
        }
    }

    pub fn exit_find_replace_mode(&mut self) {
        self.find_replace_mode = false;
        self.find_matches.clear();
        self.current_match_index = None;
        self.find_cursor_pos = 0;
        self.replace_cursor_pos = 0;
        self.find_query.clear();
        self.replace_text.clear();
        self.find_selection_start = None;
        self.replace_selection_start = None;
    }

    pub fn add_char_to_find_query(&mut self, c: char) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            // Delete any selected text first
            self.delete_find_field_selection();
            self.find_query.insert(self.find_cursor_pos, c);
            self.find_cursor_pos += 1;
            self.update_find_matches();
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            // Delete any selected text first
            self.delete_replace_field_selection();
            self.replace_text.insert(self.replace_cursor_pos, c);
            self.replace_cursor_pos += 1;
        }
    }

    pub fn backspace_find_replace_field(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            // If there's a selection, delete it
            if self.find_selection_start.is_some() {
                self.delete_find_field_selection();
                self.update_find_matches();
            } else if self.find_cursor_pos > 0 {
                // Otherwise, delete one character
                self.find_cursor_pos -= 1;
                self.find_query.remove(self.find_cursor_pos);
                self.update_find_matches();
            }
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            // If there's a selection, delete it
            if self.replace_selection_start.is_some() {
                self.delete_replace_field_selection();
            } else if self.replace_cursor_pos > 0 {
                // Otherwise, delete one character
                self.replace_cursor_pos -= 1;
                self.replace_text.remove(self.replace_cursor_pos);
            }
        }
    }

    pub fn delete_find_replace_field(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            // If there's a selection, delete it
            if self.find_selection_start.is_some() {
                self.delete_find_field_selection();
                self.update_find_matches();
            } else if self.find_cursor_pos < self.find_query.len() {
                // Otherwise, delete one character forward
                self.find_query.remove(self.find_cursor_pos);
                self.update_find_matches();
            }
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            // If there's a selection, delete it
            if self.replace_selection_start.is_some() {
                self.delete_replace_field_selection();
            } else if self.replace_cursor_pos < self.replace_text.len() {
                // Otherwise, delete one character forward
                self.replace_text.remove(self.replace_cursor_pos);
            }
        }
    }
    
    pub fn move_find_replace_cursor_left(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            if self.find_cursor_pos > 0 {
                self.find_cursor_pos -= 1;
            }
            self.find_selection_start = None; // Clear selection
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            if self.replace_cursor_pos > 0 {
                self.replace_cursor_pos -= 1;
            }
            self.replace_selection_start = None; // Clear selection
        }
    }
    
    pub fn move_find_replace_cursor_right(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            if self.find_cursor_pos < self.find_query.len() {
                self.find_cursor_pos += 1;
            }
            self.find_selection_start = None; // Clear selection
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            if self.replace_cursor_pos < self.replace_text.len() {
                self.replace_cursor_pos += 1;
            }
            self.find_selection_start = None; // Clear selection
        }
    }

    pub fn move_find_replace_cursor_left_with_selection(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            if self.find_selection_start.is_none() {
                self.find_selection_start = Some(self.find_cursor_pos);
            }
            if self.find_cursor_pos > 0 {
                self.find_cursor_pos -= 1;
            }
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            if self.replace_selection_start.is_none() {
                self.replace_selection_start = Some(self.replace_cursor_pos);
            }
            if self.replace_cursor_pos > 0 {
                self.replace_cursor_pos -= 1;
            }
        }
    }
    
    pub fn move_find_replace_cursor_right_with_selection(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            if self.find_selection_start.is_none() {
                self.find_selection_start = Some(self.find_cursor_pos);
            }
            if self.find_cursor_pos < self.find_query.len() {
                self.find_cursor_pos += 1;
            }
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            if self.replace_selection_start.is_none() {
                self.replace_selection_start = Some(self.replace_cursor_pos);
            }
            if self.replace_cursor_pos < self.replace_text.len() {
                self.replace_cursor_pos += 1;
            }
        }
    }
    
    pub fn move_find_replace_cursor_home(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            self.find_cursor_pos = 0;
            self.find_selection_start = None; // Clear selection
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            self.replace_cursor_pos = 0;
            self.find_selection_start = None; // Clear selection
        }
    }
    
    pub fn move_find_replace_cursor_end(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            self.find_cursor_pos = self.find_query.len();
            self.find_selection_start = None; // Clear selection
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            self.replace_cursor_pos = self.replace_text.len();
            self.find_selection_start = None; // Clear selection
        }
    }

    pub fn toggle_find_replace_focus(&mut self) {
        // Clear any selections when switching focus
        self.find_selection_start = None;
        self.replace_selection_start = None;
        self.find_replace_focus = match self.find_replace_focus {
            FindReplaceFocus::FindField => FindReplaceFocus::ReplaceField,
            FindReplaceFocus::ReplaceField => FindReplaceFocus::Editor,
            FindReplaceFocus::Editor => FindReplaceFocus::FindField,
        };
    }

    pub fn update_find_matches(&mut self) {
        self.find_matches.clear();
        self.current_match_index = None;

        if self.find_query.is_empty() {
            return;
        }

        // Convert query to lowercase for case-insensitive search
        let query_lower = self.find_query.to_lowercase();

        // Collect matches first to avoid borrowing issues
        let mut matches = Vec::new();
        let (cursor_line, cursor_col) = if let Some(buffer) = self.current_buffer() {
            let total_lines = buffer.rope.len_lines();
            for line_idx in 0..total_lines {
                let line_text = buffer.get_line_text(line_idx);
                let line_lower = line_text.to_lowercase();
                let mut search_start = 0;
                
                while let Some(match_pos) = line_lower[search_start..].find(&query_lower) {
                    let absolute_pos = search_start + match_pos;
                    // Use the original query length for the match span
                    matches.push((
                        line_idx,
                        absolute_pos,
                        absolute_pos + self.find_query.len()
                    ));
                    search_start = absolute_pos + 1;
                }
            }
            (buffer.cursor.line, buffer.cursor.column)
        } else {
            return;
        };

        // Now update self.find_matches
        self.find_matches = matches;

        // If we have matches, set current to the first one after cursor position
        if !self.find_matches.is_empty() {
            // Find the first match after the cursor
            for (idx, &(line, start_col, _)) in self.find_matches.iter().enumerate() {
                if line > cursor_line || (line == cursor_line && start_col >= cursor_col) {
                    self.current_match_index = Some(idx);
                    break;
                }
            }
            
            // If no match after cursor, wrap to the beginning
            if self.current_match_index.is_none() {
                self.current_match_index = Some(0);
            }
        }
    }

    pub fn find_next(&mut self, config: &Config, visible_lines: usize) {
        if self.find_matches.is_empty() {
            return;
        }

        if let Some(current) = self.current_match_index {
            self.current_match_index = Some((current + 1) % self.find_matches.len());
        } else {
            self.current_match_index = Some(0);
        }

        self.jump_to_current_match(config, visible_lines);
    }

    pub fn find_previous(&mut self, config: &Config, visible_lines: usize) {
        if self.find_matches.is_empty() {
            return;
        }

        if let Some(current) = self.current_match_index {
            self.current_match_index = Some(if current == 0 {
                self.find_matches.len() - 1
            } else {
                current - 1
            });
        } else {
            self.current_match_index = Some(self.find_matches.len() - 1);
        }

        self.jump_to_current_match(config, visible_lines);
    }

    fn jump_to_current_match(&mut self, config: &Config, visible_lines: usize) {
        if let Some(idx) = self.current_match_index {
            if let Some(&(line, start_col, _)) = self.find_matches.get(idx) {
                if let Some(buffer) = self.current_buffer_mut() {
                    buffer.cursor.line = line;
                    buffer.cursor.column = start_col;
                    buffer.cursor.preferred_visual_column = start_col;
                    buffer.cursor.clear_selection();
                }
                self.center_viewport_on_cursor(config, visible_lines);
            }
        }
    }

    pub fn replace_current_match(&mut self) -> bool {
        if let Some(idx) = self.current_match_index {
            if let Some(&(line, start_col, end_col)) = self.find_matches.get(idx) {
                // Clone the replacement text to avoid borrowing issues
                let replacement_text = self.replace_text.clone();
                let replacement_len = replacement_text.chars().count();
                
                if let Some(buffer) = self.current_buffer_mut() {
                    // Start an undo group for the replace operation
                    buffer.start_undo_group();
                    
                    // Set up selection for the match
                    buffer.cursor.selection_start = Some(Position { line, column: start_col });
                    buffer.cursor.line = line;
                    buffer.cursor.column = end_col;
                    
                    // Delete the selected text WITHOUT copying to clipboard
                    buffer.delete_selection_no_clipboard(Position { line, column: start_col }, Position { line, column: end_col });
                    
                    // Insert the replacement text
                    for ch in replacement_text.chars() {
                        buffer.insert_char(ch);
                    }
                    
                    // Clear selection after replacement
                    buffer.cursor.clear_selection();
                    
                    // End the undo group
                    buffer.end_undo_group();
                    
                    // Position cursor at the end of the replacement
                    buffer.cursor.line = line;
                    buffer.cursor.column = start_col + replacement_len;
                }
                
                // Update find matches after replacement
                self.update_find_matches();
                
                // DON'T automatically jump to the next match here!
                // The caller (handle_find_replace_key) will call find_next() if needed
                
                return true;
            }
        }
        false
    }

    pub fn replace_all_matches(&mut self) -> usize {
        if self.find_matches.is_empty() || self.find_query.is_empty() {
            return 0;
        }

        let mut replaced_count = 0;
        
        // Clone the data we need before borrowing the buffer mutably
        let matches = self.find_matches.clone();
        let replacement_text = self.replace_text.clone();
        
        if let Some(buffer) = self.current_buffer_mut() {
            let original_cursor = Position {
                line: buffer.cursor.line,
                column: buffer.cursor.column,
            };
            
            // Start a new undo group for all replacements
            buffer.start_undo_group();
            
            // Process matches in reverse order to avoid position shifts
            for &(line, start_col, end_col) in matches.iter().rev() {
                buffer.cursor.selection_start = Some(Position { line, column: start_col });
                buffer.cursor.line = line;
                buffer.cursor.column = end_col;
                
                // Delete the selected text WITHOUT copying to clipboard
                // This will now properly use the active undo group
                buffer.delete_selection_no_clipboard(Position { line, column: start_col }, Position { line, column: end_col });
                
                // Insert the replacement text
                // These will also be part of the same undo group
                for ch in replacement_text.chars() {
                    buffer.insert_char(ch);
                }
                replaced_count += 1;
            }
            
            // End the undo group to make all replacements atomic
            buffer.end_undo_group();
            
            // Restore cursor position
            buffer.cursor.line = original_cursor.line;
            buffer.cursor.column = original_cursor.column;
            buffer.cursor.preferred_visual_column = original_cursor.column;
            buffer.cursor.clear_selection();
        }
        
        // Clear matches after replacements
        self.find_matches.clear();
        self.current_match_index = None;
        
        replaced_count
    }

    pub fn get_find_status(&self) -> Option<(usize, usize)> {
        if self.find_matches.is_empty() {
            None
        } else {
            Some((
                self.current_match_index.map(|i| i + 1).unwrap_or(0),
                self.find_matches.len()
            ))
        }
    }

    // Find/Replace field selection and clipboard operations
    pub fn select_all_find_field(&mut self) {
        if self.find_replace_focus == FindReplaceFocus::FindField {
            self.find_selection_start = Some(0);
            self.find_cursor_pos = self.find_query.len();
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            self.replace_selection_start = Some(0);
            self.replace_cursor_pos = self.replace_text.len();
        }
    }

    pub fn copy_find_field_selection(&self) -> Result<()> {
        let text_to_copy = if self.find_replace_focus == FindReplaceFocus::FindField {
            self.get_find_field_selected_text()
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            self.get_replace_field_selected_text()
        } else {
            None
        };

        if let Some(text) = text_to_copy {
            let mut clipboard = arboard::Clipboard::new()?;
            clipboard.set_text(text)?;
        }
        Ok(())
    }

    pub fn cut_find_field_selection(&mut self) -> Result<()> {
        // First copy the selected text
        self.copy_find_field_selection()?;
        
        // Then delete it
        if self.find_replace_focus == FindReplaceFocus::FindField {
            self.delete_find_field_selection();
            self.update_find_matches();
        } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
            self.delete_replace_field_selection();
        }
        
        Ok(())
    }

    pub fn paste_to_find_field(&mut self) -> Result<()> {
        let mut clipboard = arboard::Clipboard::new()?;
        if let Ok(text) = clipboard.get_text() {
            if self.find_replace_focus == FindReplaceFocus::FindField {
                // Delete any selected text first
                self.delete_find_field_selection();
                
                // Insert pasted text
                self.find_query.insert_str(self.find_cursor_pos, &text);
                self.find_cursor_pos += text.len();
                self.update_find_matches();
            } else if self.find_replace_focus == FindReplaceFocus::ReplaceField {
                // Delete any selected text first
                self.delete_replace_field_selection();
                
                // Insert pasted text
                self.replace_text.insert_str(self.replace_cursor_pos, &text);
                self.replace_cursor_pos += text.len();
            }
        }
        Ok(())
    }

    fn get_find_field_selected_text(&self) -> Option<String> {
        if let Some(start) = self.find_selection_start {
            let end = self.find_cursor_pos;
            let (start, end) = if start <= end { (start, end) } else { (end, start) };
            Some(self.find_query[start..end].to_string())
        } else {
            None
        }
    }

    fn get_replace_field_selected_text(&self) -> Option<String> {
        if let Some(start) = self.replace_selection_start {
            let end = self.replace_cursor_pos;
            let (start, end) = if start <= end { (start, end) } else { (end, start) };
            Some(self.replace_text[start..end].to_string())
        } else {
            None
        }
    }

    fn delete_find_field_selection(&mut self) {
        if let Some(start) = self.find_selection_start {
            let end = self.find_cursor_pos;
            let (start, end) = if start <= end { (start, end) } else { (end, start) };
            self.find_query.drain(start..end);
            self.find_cursor_pos = start;
            self.find_selection_start = None;
        }
    }

    fn delete_replace_field_selection(&mut self) {
        if let Some(start) = self.replace_selection_start {
            let end = self.replace_cursor_pos;
            let (start, end) = if start <= end { (start, end) } else { (end, start) };
            self.replace_text.drain(start..end);
            self.replace_cursor_pos = start;
            self.replace_selection_start = None;
        }
    }

    // Cursor movement methods with word-wrap support
	pub fn update_preferred_visual_column_with_width(&mut self, content_width: usize) {
	    let (line_text, cursor_column) = if let Some(buffer) = self.current_buffer() {
	        (buffer.get_line_text(buffer.cursor.line), buffer.cursor.column)
	    } else {
	        return;
	    };
	    
	    let line_text_for_display = if line_text.ends_with('\n') {
	        &line_text[..line_text.len()-1]
	    } else {
	        &line_text
	    };
	    
	    let wrapped_segments = wrap_line(line_text_for_display, content_width);
	    
	    // For word wrap, preferred column should be the visual screen position
	    // taking into account indentation on continuation lines
	    
	    // Find which segment we're in
	    for (i, (segment, start_pos)) in wrapped_segments.iter().enumerate() {
	        let segment_end = if i + 1 < wrapped_segments.len() {
	            wrapped_segments[i + 1].1
	        } else {
	            line_text_for_display.chars().count()
	        };
	        
	        if cursor_column >= *start_pos && cursor_column < segment_end || 
	           (i == wrapped_segments.len() - 1 && cursor_column >= *start_pos) {
	            // We're in this segment
	            // For the first segment, visual column = cursor column
	            // For continuation segments, we need to account for indentation
	            let visual_column = if i == 0 {
	                cursor_column
	            } else {
	                // Count the indentation that was added to this segment
	                let indent_chars = segment.chars().take_while(|&c| c == ' ' || c == '\t').count();
	                // Visual position is: indentation + position within segment content
	                indent_chars + (cursor_column - start_pos)
	            };
	            
	            if let Some(buffer) = self.current_buffer_mut() {
	                buffer.cursor.preferred_visual_column = visual_column;
	            }
	            return;
	        }
	    }
	    
	    // Fallback - just use cursor column
	    if let Some(buffer) = self.current_buffer_mut() {
	        buffer.cursor.preferred_visual_column = cursor_column;
	    }
	}

    pub fn move_cursor_left(&mut self, content_width: usize, config: &Config, visible_lines: usize) -> bool {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_left();
            self.update_preferred_visual_column_with_width(content_width);
            return self.adjust_viewport(config, visible_lines);
        }
        false
    }

    pub fn move_cursor_right(&mut self, content_width: usize, config: &Config, visible_lines: usize) -> bool {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_right();
            self.update_preferred_visual_column_with_width(content_width);
            return self.adjust_viewport(config, visible_lines);
        }
        false
    }

    pub fn move_cursor_up(&mut self, word_wrap: bool, content_width: usize, config: &Config, visible_lines: usize) {
        if word_wrap {
            // Check if we're on the first visual line of the buffer
            if let Some(buffer) = self.current_buffer() {
                if buffer.cursor.line == 0 {
                    // We're on the first logical line - check if we're on the first visual segment
                    let line_text = buffer.get_line_text(0);
                    let line_text_for_display = if line_text.ends_with('\n') {
                        &line_text[..line_text.len()-1]
                    } else {
                        &line_text
                    };
                    let wrapped_segments = wrap_line(line_text_for_display, content_width);
                    
                    // Find which segment we're in
                    for (i, (_segment, start_pos)) in wrapped_segments.iter().enumerate() {
                        let segment_end = if i + 1 < wrapped_segments.len() {
                            wrapped_segments[i + 1].1
                        } else {
                            line_text_for_display.chars().count()
                        };
                        
                        if buffer.cursor.column >= *start_pos && buffer.cursor.column <= segment_end {
                            if i == 0 {
                                // We're on the first visual line of the buffer - go to beginning
                                if let Some(buffer) = self.current_buffer_mut() {
                                    buffer.cursor.column = 0;
                                    buffer.cursor.preferred_visual_column = 0;
                                }
                                self.adjust_viewport(config, visible_lines);
                                return;
                            }
                            break;
                        }
                    }
                }
            }
            self.move_cursor_up_visual(content_width, config, visible_lines);
        } else {
            // Check if we're on the first line - if so, go to beginning of buffer
            if let Some(buffer) = self.current_buffer() {
                if buffer.cursor.line == 0 {
                    if let Some(buffer) = self.current_buffer_mut() {
                        buffer.cursor.column = 0;
                        buffer.cursor.preferred_visual_column = 0;
                    }
                    self.adjust_viewport(config, visible_lines);
                    return;
                }
            }
            if let Some(buffer) = self.current_buffer_mut() {
                buffer.move_cursor_up();
            }
        }
        self.adjust_viewport(config, visible_lines);
    }

    pub fn move_cursor_down(&mut self, word_wrap: bool, content_width: usize, config: &Config, visible_lines: usize) {
        if word_wrap {
            // Check if we're on the last visual line of the buffer
            if let Some(buffer) = self.current_buffer() {
                let last_line_idx = buffer.rope.len_lines() - 1;
                if buffer.cursor.line == last_line_idx {
                    // We're on the last logical line - check if we're on the last visual segment
                    let line_text = buffer.get_line_text(last_line_idx);
                    let line_text_for_display = if line_text.ends_with('\n') {
                        &line_text[..line_text.len()-1]
                    } else {
                        &line_text
                    };
                    let wrapped_segments = wrap_line(line_text_for_display, content_width);
                    
                    // Find which segment we're in
                    for (i, (_segment, start_pos)) in wrapped_segments.iter().enumerate() {
                        let segment_end = if i + 1 < wrapped_segments.len() {
                            wrapped_segments[i + 1].1
                        } else {
                            line_text_for_display.chars().count()
                        };
                        
                        if buffer.cursor.column >= *start_pos && buffer.cursor.column <= segment_end {
                            if i == wrapped_segments.len() - 1 {
                                // We're on the last visual line of the buffer
                                // Go to end of line and update preferred column to that new position
                                if let Some(buffer) = self.current_buffer_mut() {
                                    buffer.move_cursor_end();
                                    // Update preferred visual column to the new end position
                                    self.update_preferred_visual_column_with_width(content_width);
                                }
                                self.adjust_viewport(config, visible_lines);
                                return;
                            }
                            break;
                        }
                    }
                }
            }
            
            self.move_cursor_down_visual(content_width, config, visible_lines);
        } else {
            // Check if we're on the last line - if so, go to end of that line
            if let Some(buffer) = self.current_buffer() {
                if buffer.cursor.line >= buffer.rope.len_lines() - 1 {
                    if let Some(buffer) = self.current_buffer_mut() {
                        buffer.move_cursor_end();
                        // Update preferred visual column to the new end position
                        self.update_preferred_visual_column_with_width(content_width);
                    }
                    self.adjust_viewport(config, visible_lines);
                    return;
                }
            }
            if let Some(buffer) = self.current_buffer_mut() {
                buffer.move_cursor_down();
            }
        }
        self.adjust_viewport(config, visible_lines);
    }

fn move_cursor_up_visual(&mut self, content_width: usize, config: &Config, visible_lines: usize) {
    let (current_line_text, cursor_line, cursor_column, preferred_visual_column) = if let Some(buffer) = self.current_buffer() {
        (buffer.get_line_text(buffer.cursor.line), buffer.cursor.line, buffer.cursor.column, buffer.cursor.preferred_visual_column)
    } else {
        return;
    };
    
    let line_text_for_display = if current_line_text.ends_with('\n') {
        &current_line_text[..current_line_text.len()-1]
    } else {
        &current_line_text
    };
    let wrapped_segments = wrap_line(line_text_for_display, content_width);
    
    // Find which visual line segment we're currently in
    let mut current_segment_idx = None;
    
    if wrapped_segments.is_empty() {
        current_segment_idx = Some(0);
    } else {
        // Find the last segment that starts at or before our cursor position
        for (i, (_segment, start_pos)) in wrapped_segments.iter().enumerate().rev() {
            if cursor_column >= *start_pos {
                current_segment_idx = Some(i);
                break;
            }
        }
        
        // If still not found, use first segment
        if current_segment_idx.is_none() {
            current_segment_idx = Some(0);
        }
    }
    
    // Perform the movement
    if let Some(segment_idx) = current_segment_idx {
        if segment_idx > 0 {
            // Move to previous visual line within same logical line
            if let Some(buffer) = self.current_buffer_mut() {
                let prev_segment = &wrapped_segments[segment_idx - 1];
                let target_visual_col = preferred_visual_column;
                let prev_segment_len = prev_segment.0.chars().count();
                let new_col = prev_segment.1 + target_visual_col.min(prev_segment_len);
                buffer.cursor.column = new_col;
                buffer.cursor.preferred_visual_column = target_visual_col;
            }
        } else {
            // We're on the first visual segment of the current line
            if cursor_line > 0 {
                if let Some(buffer) = self.current_buffer_mut() {
                    let target_visual_col = preferred_visual_column;
                    buffer.move_cursor_up();
                    
                    let new_line_text = buffer.get_line_text(buffer.cursor.line);
                    let new_line_for_display = if new_line_text.ends_with('\n') {
                        &new_line_text[..new_line_text.len()-1]
                    } else {
                        &new_line_text
                    };
                    let new_wrapped = wrap_line(new_line_for_display, content_width);
                    
                    if !new_wrapped.is_empty() {
                        // Always go to the LAST visual line of the previous logical line
                        let last_segment = &new_wrapped[new_wrapped.len() - 1];
                        let segment_len = last_segment.0.chars().count();
                        let new_col = last_segment.1 + target_visual_col.min(segment_len);
                        buffer.cursor.column = new_col;
                        buffer.cursor.preferred_visual_column = target_visual_col;
                    } else {
                        buffer.cursor.column = target_visual_col.min(new_line_for_display.chars().count());
                        buffer.cursor.preferred_visual_column = target_visual_col;
                    }
                }
            }
        }
    }
    
    self.adjust_viewport(config, visible_lines);
}
	
	fn move_cursor_down_visual(&mut self, content_width: usize, config: &Config, visible_lines: usize) {
	    let (current_line_text, cursor_line, cursor_column, preferred_visual_column, total_lines) = if let Some(buffer) = self.current_buffer() {
	        (buffer.get_line_text(buffer.cursor.line), buffer.cursor.line, buffer.cursor.column, buffer.cursor.preferred_visual_column, buffer.rope.len_lines())
	    } else {
	        return;
	    };
	    
	    let line_text_for_display = if current_line_text.ends_with('\n') {
	        &current_line_text[..current_line_text.len()-1]
	    } else {
	        &current_line_text
	    };
	    let wrapped_segments = wrap_line(line_text_for_display, content_width);
	    
	    // Find which visual line segment we're currently in
	    let mut current_segment_idx = None;
	    
	    if wrapped_segments.is_empty() {
	        current_segment_idx = Some(0);
	    } else {
	        // Find the last segment that starts at or before our cursor position
	        for (i, (_segment, start_pos)) in wrapped_segments.iter().enumerate().rev() {
	            if cursor_column >= *start_pos {
	                current_segment_idx = Some(i);
	                break;
	            }
	        }
	        
	        // If still not found, use first segment
	        if current_segment_idx.is_none() {
	            current_segment_idx = Some(0);
	        }
	    }
	    
	    // Perform the movement
	    if let Some(segment_idx) = current_segment_idx {
	        if segment_idx < wrapped_segments.len() - 1 {
	            // Move to next visual line within same logical line
	            if let Some(buffer) = self.current_buffer_mut() {
	                let next_segment = &wrapped_segments[segment_idx + 1];
	                let target_visual_col = preferred_visual_column;
	                let next_segment_len = next_segment.0.chars().count();
	                let new_col = next_segment.1 + target_visual_col.min(next_segment_len);
	                buffer.cursor.column = new_col;
	                buffer.cursor.preferred_visual_column = target_visual_col;
	            }
	        } else {
	            // We're on the last visual segment of the current line
	            if cursor_line < total_lines - 1 {
	                if let Some(buffer) = self.current_buffer_mut() {
	                    let target_visual_col = preferred_visual_column;
	                    buffer.move_cursor_down();
	                    
	                    let new_line_text = buffer.get_line_text(buffer.cursor.line);
	                    let new_line_for_display = if new_line_text.ends_with('\n') {
	                        &new_line_text[..new_line_text.len()-1]
	                    } else {
	                        &new_line_text
	                    };
	                    let new_wrapped = wrap_line(new_line_for_display, content_width);
	                    
	                    if !new_wrapped.is_empty() {
	                        // Always go to the FIRST visual line of the next logical line
	                        let first_segment = &new_wrapped[0];
	                        let segment_len = first_segment.0.chars().count();
	                        let new_col = first_segment.1 + target_visual_col.min(segment_len);
	                        buffer.cursor.column = new_col;
	                        buffer.cursor.preferred_visual_column = target_visual_col;
	                    } else {
	                        buffer.cursor.column = target_visual_col.min(new_line_for_display.chars().count());
	                        buffer.cursor.preferred_visual_column = target_visual_col;
	                    }
	                }
	            }
	        }
	    }
	    
	    self.adjust_viewport(config, visible_lines);
	}

    pub fn move_cursor_home(&mut self, config: &Config, visible_lines: usize) -> bool {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_home();
        }
        self.adjust_viewport(config, visible_lines)
    }

    pub fn move_cursor_end(&mut self, config: &Config, visible_lines: usize) -> bool {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_end();
        }
        self.adjust_viewport(config, visible_lines)
    }

    pub fn move_cursor_page_up(&mut self, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            for _ in 0..10 {
                buffer.move_cursor_up();
            }
            self.adjust_viewport(config, visible_lines);
        }
    }

    pub fn move_cursor_page_down(&mut self, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            for _ in 0..10 {
                buffer.move_cursor_down();
            }
            self.adjust_viewport(config, visible_lines);
        }
    }

    // Text modification methods
    pub fn insert_char(&mut self, c: char, content_width: usize, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.insert_char(c);
            self.update_preferred_visual_column_with_width(content_width);
            self.adjust_viewport(config, visible_lines);
        }
    }

    pub fn insert_newline(&mut self, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.insert_newline();
            self.adjust_viewport(config, visible_lines);
        }
    }

    pub fn insert_tab(&mut self, content_width: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            // Insert 4 spaces for tab
            for _ in 0..4 {
                buffer.insert_char(' ');
            }
            self.update_preferred_visual_column_with_width(content_width);
        }
    }

    pub fn indent_lines(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.indent_lines();
        }
    }

    pub fn dedent_lines(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.dedent_lines();
        }
    }
    
    pub fn move_lines_up(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_lines_up();
        }
    }
    
    pub fn move_lines_down(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_lines_down();
        }
    }

    pub fn delete_char_backwards(&mut self, content_width: usize, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.delete_char_backwards();
            self.update_preferred_visual_column_with_width(content_width);
            self.adjust_viewport(config, visible_lines);
        }
    }

    pub fn delete_char_forwards(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.delete_char_forwards();
        }
    }

    /// Initial viewport adjustment when loading a file or creating a new buffer
    /// This ensures the viewport respects scrolloff from the start
    pub fn adjust_viewport_initial(&mut self, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer() {
            let scrolloff = config.scrolloff as usize;
            
            // For empty or very small buffers, position cursor in the top scrolloff zone
            if buffer.rope.len_lines() <= 1 {
                // Set viewport so cursor appears at scrolloff lines from the top
                // This means viewport should be negative to show virtual lines above
                self.viewport_line = -(scrolloff as isize);
            } else {
                // For files with content, use regular viewport adjustment
                self.adjust_viewport(config, visible_lines);
            }
        }
    }
    
    pub fn adjust_viewport(&mut self, config: &Config, visible_lines: usize) -> bool {
        let mut viewport_changed = false;
        
        if config.word_wrap {
            self.adjust_viewport_word_wrap(config, visible_lines);
        } else {
            self.adjust_viewport_no_wrap(config, visible_lines);
            // Also adjust horizontal viewport when word wrap is disabled
            viewport_changed = self.adjust_horizontal_viewport(config);
        }
        
        viewport_changed
    }
    
    fn adjust_viewport_no_wrap(&mut self, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer() {
            let cursor_line = buffer.cursor.line;
            let scrolloff = config.scrolloff as usize;
            let total_file_lines = buffer.rope.len_lines();
            
            // Calculate the effective scrollable area
            let effective_visible = visible_lines.saturating_sub(scrolloff * 2);
            if effective_visible == 0 {
                // If scrolloff is too large for the viewport, just center the cursor
                self.viewport_line = cursor_line as isize - visible_lines as isize / 2;
                return;
            }
            
            // For small buffers that fit entirely in viewport, always align with top
            // This matches the behavior of GUI text editors
            if total_file_lines <= visible_lines {
                // Buffer fits entirely in viewport - keep it aligned to top with scrolloff
                self.viewport_line = -(scrolloff as isize);
                return;
            }
            
            // For larger buffers, use full scrolloff logic
            // Calculate the scrolloff boundaries within the current viewport
            let top_boundary = self.viewport_line + scrolloff as isize;
            let bottom_boundary = self.viewport_line + (visible_lines - scrolloff - 1) as isize;
            
            // Adjust viewport if cursor is outside the scrolloff zone
            if (cursor_line as isize) < top_boundary {
                // Cursor is above the top scrolloff zone - scroll up
                self.viewport_line = cursor_line as isize - scrolloff as isize;
            } else if (cursor_line as isize) > bottom_boundary {
                // Cursor is below the bottom scrolloff zone - scroll down
                self.viewport_line = cursor_line as isize - (visible_lines - scrolloff - 1) as isize;
            }
            
            // Allow viewport to go negative (showing virtual lines above file)
            // But limit how far negative we can go (scrolloff lines above first line)
            let min_viewport = -(scrolloff as isize);
            // Don't scroll past the end when accounting for virtual lines at the end
            let max_viewport = (total_file_lines as isize + scrolloff as isize) - visible_lines as isize;
            
            self.viewport_line = self.viewport_line
                .max(min_viewport)
                .min(max_viewport);
        }
    }
    
    fn adjust_viewport_word_wrap(&mut self, config: &Config, visible_lines: usize) {
        // In word-wrap mode, viewport_line represents the visual line directly
        if let Some(buffer) = self.current_buffer() {
            let content_width = self.get_content_width(config);
            let scrolloff = config.scrolloff as usize;
            let cursor_visual_line = self.calculate_cursor_visual_line(buffer, content_width);
            let total_visual_lines = self.calculate_total_visual_lines(buffer, content_width);
            
            // Calculate the effective scrollable area
            let effective_visible = visible_lines.saturating_sub(scrolloff * 2);
            if effective_visible == 0 {
                // If scrolloff is too large for the viewport, just center the cursor
                self.viewport_line = cursor_visual_line as isize - visible_lines as isize / 2;
                return;
            }
            
            // For small buffers that fit entirely in viewport, always align with top
            // This matches the behavior of GUI text editors
            if total_visual_lines <= visible_lines {
                // Buffer fits entirely in viewport - keep it aligned to top with scrolloff
                self.viewport_line = -(scrolloff as isize);
                return;
            }
            
            // Calculate scrolloff boundaries in visual lines
            let top_boundary = self.viewport_line + scrolloff as isize;
            let bottom_boundary = self.viewport_line + (visible_lines - scrolloff - 1) as isize;
            
            // Adjust viewport if cursor is outside the scrolloff zone
            if (cursor_visual_line as isize) < top_boundary {
                // Cursor is above the top scrolloff zone - scroll up by visual lines
                self.viewport_line = cursor_visual_line as isize - scrolloff as isize;
            } else if (cursor_visual_line as isize) > bottom_boundary {
                // Cursor is below the bottom scrolloff zone - scroll down by visual lines
                self.viewport_line = cursor_visual_line as isize - (visible_lines - scrolloff - 1) as isize;
            }
            
            // Limit viewport to valid range
            let min_viewport = -(scrolloff as isize);
            let max_viewport = (total_visual_lines as isize + scrolloff as isize) - visible_lines as isize;
            
            self.viewport_line = self.viewport_line
                .max(min_viewport)
                .min(max_viewport);
        }
    }
    
    // Helper function to calculate content width
    fn get_content_width(&self, config: &Config) -> usize {
        use crossterm::terminal::size;
        let (terminal_width, _) = size().unwrap_or((80, 24));
        let content_width = terminal_width as usize - config.margins.horizontal as usize * 2;
        
        // Account for gutter width
        if let Some(buffer) = self.current_buffer() {
            let gutter_width = match config.gutter {
                crate::config::GutterMode::None => 0,
                crate::config::GutterMode::Absolute | crate::config::GutterMode::Relative => {
                    let total_lines = buffer.rope.len_lines();
                    let digits = total_lines.to_string().len();
                    digits + 2 // Add 2 for padding
                }
            };
            content_width.saturating_sub(gutter_width)
        } else {
            content_width
        }
    }
    
    // Calculate which visual line the cursor is on
    fn calculate_cursor_visual_line(&self, buffer: &crate::buffer::Buffer, content_width: usize) -> usize {
        use crate::text_utils::wrap_line;
        let mut visual_line = 0;
        
        // Count visual lines for all logical lines before the cursor line
        for line_idx in 0..buffer.cursor.line {
            let line_text = buffer.get_line_text(line_idx);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let wrapped_segments = wrap_line(line_text_for_display, content_width);
            visual_line += wrapped_segments.len().max(1);
        }
        
        // Add the segment index within the current line
        if buffer.cursor.line < buffer.rope.len_lines() {
            let line_text = buffer.get_line_text(buffer.cursor.line);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let wrapped_segments = wrap_line(line_text_for_display, content_width);
            
            // Handle edge case: cursor at or beyond the end of line
            if buffer.cursor.column >= line_text_for_display.chars().count() {
                visual_line += wrapped_segments.len().saturating_sub(1);
            } else {
                // Find which visual line segment we're currently in
                for (segment_idx, (_segment, start_pos)) in wrapped_segments.iter().enumerate() {
                    let segment_end = if segment_idx + 1 < wrapped_segments.len() {
                        wrapped_segments[segment_idx + 1].1
                    } else {
                        line_text_for_display.chars().count()
                    };
                    
                    // Use consistent boundary detection logic
                    let is_in_segment = if segment_idx == wrapped_segments.len() - 1 {
                        // Last segment: include the end position
                        buffer.cursor.column >= *start_pos && buffer.cursor.column <= segment_end
                    } else {
                        // Other segments: exclude the end position (it belongs to next segment)
                        buffer.cursor.column >= *start_pos && buffer.cursor.column < segment_end
                    };
                    
                    if is_in_segment {
                        visual_line += segment_idx;
                        break;
                    }
                }
            }
        }

        visual_line
    }
    
    // Calculate total number of visual lines in the buffer
    fn calculate_total_visual_lines(&self, buffer: &crate::buffer::Buffer, content_width: usize) -> usize {
        use crate::text_utils::wrap_line;
        let mut total_visual_lines = 0;
        
        for line_idx in 0..buffer.rope.len_lines() {
            let line_text = buffer.get_line_text(line_idx);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let wrapped_segments = wrap_line(line_text_for_display, content_width);
            total_visual_lines += wrapped_segments.len().max(1);
        }
        
        total_visual_lines
    }
    
    // Convert a logical line index to visual line index
    fn logical_to_visual_line(&self, logical_line: usize, buffer: &crate::buffer::Buffer, content_width: usize) -> usize {
        use crate::text_utils::wrap_line;
        let mut visual_line = 0;
        
        for line_idx in 0..logical_line.min(buffer.rope.len_lines()) {
            let line_text = buffer.get_line_text(line_idx);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let wrapped_segments = wrap_line(line_text_for_display, content_width);
            visual_line += wrapped_segments.len().max(1);
        }
        
        visual_line
    }
    
    // Convert a visual line index to logical line index
    fn visual_to_logical_line(&self, target_visual_line: usize, buffer: &crate::buffer::Buffer, content_width: usize) -> usize {
        use crate::text_utils::wrap_line;
        let mut visual_line = 0;
        
        for line_idx in 0..buffer.rope.len_lines() {
            let line_text = buffer.get_line_text(line_idx);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let wrapped_segments = wrap_line(line_text_for_display, content_width);
            let segments_count = wrapped_segments.len().max(1);
            
            if visual_line + segments_count > target_visual_line {
                return line_idx;
            }
            
            visual_line += segments_count;
        }
        
        // If we've gone past the end, return the last line
        buffer.rope.len_lines().saturating_sub(1)
    }
    

    // Help mode methods
    pub fn enter_help_mode(&mut self) {
        self.help_mode = true;
        self.help_scroll_offset = 0;
    }

    pub fn exit_help_mode(&mut self) {
        self.help_mode = false;
        self.help_scroll_offset = 0;
    }

    // Filename prompt methods
    pub fn enter_filename_prompt_mode(&mut self) {
        self.filename_prompt_mode = true;
        self.filename_prompt_text.clear();
    }

    pub fn exit_filename_prompt_mode(&mut self) {
        self.filename_prompt_mode = false;
        self.filename_prompt_text.clear();
        self.filename_cursor_pos = 0;
    }

    pub fn add_char_to_filename_prompt(&mut self, c: char) {
        self.filename_prompt_text.insert(self.filename_cursor_pos, c);
        self.filename_cursor_pos += 1;
    }

    pub fn backspace_filename_prompt(&mut self) {
        if self.filename_cursor_pos > 0 {
            self.filename_cursor_pos -= 1;
            self.filename_prompt_text.remove(self.filename_cursor_pos);
        }
    }

    pub fn move_filename_cursor_left(&mut self) {
        if self.filename_cursor_pos > 0 {
            self.filename_cursor_pos -= 1;
        }
    }

    pub fn move_filename_cursor_right(&mut self) {
        if self.filename_cursor_pos < self.filename_prompt_text.len() {
            self.filename_cursor_pos += 1;
        }
    }

    pub fn move_filename_cursor_home(&mut self) {
        self.filename_cursor_pos = 0;
    }

    pub fn move_filename_cursor_end(&mut self) {
        self.filename_cursor_pos = self.filename_prompt_text.len();
    }

    pub fn delete_char_in_filename_prompt(&mut self) {
        if self.filename_cursor_pos < self.filename_prompt_text.len() {
            self.filename_prompt_text.remove(self.filename_cursor_pos);
        }
    }

    pub async fn save_with_filename(&mut self) -> Result<()> {
        if self.filename_prompt_text.is_empty() {
            return Err(anyhow::anyhow!("No filename provided"));
        }
        
        let path = PathBuf::from(&self.filename_prompt_text);
        
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.save(Some(path))?;
        }
        
        self.exit_filename_prompt_mode();
        Ok(())
    }

    // Viewport scrolling methods (for mouse wheel/trackpad)
    pub fn scroll_viewport_up(&mut self, lines: usize, config: &Config, _visible_lines: usize) {
        if config.word_wrap {
            // In word-wrap mode, viewport_line IS the visual line directly
            self.viewport_line = self.viewport_line.saturating_sub(lines as isize);
            
            // Ensure we don't scroll too far up
            let scrolloff = config.scrolloff as isize;
            let min_viewport = -scrolloff;
            self.viewport_line = self.viewport_line.max(min_viewport);
        } else {
            self.viewport_line = self.viewport_line.saturating_sub(lines as isize);
            
            // Ensure we don't scroll too far up
            let scrolloff = config.scrolloff as isize;
            let min_viewport = -scrolloff;
            self.viewport_line = self.viewport_line.max(min_viewport);
        }
    }
    
    pub fn scroll_viewport_down(&mut self, lines: usize, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer() {
            let scrolloff = config.scrolloff as usize;
            
            if config.word_wrap {
                // In word-wrap mode, viewport_line IS the visual line directly
                let content_width = self.get_content_width(config);
                let total_visual_lines = self.calculate_total_visual_lines(buffer, content_width);
                
                // Scroll down by visual lines
                self.viewport_line = self.viewport_line.saturating_add(lines as isize);
                
                // Don't scroll past the end when accounting for virtual lines at the end
                let max_viewport = (total_visual_lines as isize + scrolloff as isize) - visible_lines as isize;
                self.viewport_line = self.viewport_line.min(max_viewport);
            } else {
                let total_file_lines = buffer.rope.len_lines();
                self.viewport_line = self.viewport_line.saturating_add(lines as isize);
                
                // Don't scroll past the end when accounting for virtual lines at the end
                let max_viewport = (total_file_lines as isize + scrolloff as isize) - visible_lines as isize;
                self.viewport_line = self.viewport_line.min(max_viewport);
            }
        }
    }

    pub fn scroll_left(&mut self, columns: usize) {
        self.horizontal_offset = self.horizontal_offset.saturating_sub(columns);
    }

    pub fn scroll_right(&mut self, columns: usize, content_width: usize) {
        if let Some(buffer) = self.current_buffer() {
            let longest_line = buffer.rope.lines().map(|line| line.len_chars()).max().unwrap_or(0);
            let max_offset = longest_line.saturating_sub(content_width);
            self.horizontal_offset = (self.horizontal_offset + columns).min(max_offset);
        }
    }
    
    // Adjust horizontal viewport to follow cursor
    // Returns true if the viewport was adjusted
    fn adjust_horizontal_viewport(&mut self, config: &Config) -> bool {
        // Extract needed values from buffer first to avoid borrowing issues
        let (cursor_column, _cursor_line, line_len) = if let Some(buffer) = self.current_buffer() {
            let line_text = buffer.get_line_text(buffer.cursor.line);
            let line_len = line_text.chars().count();
            (buffer.cursor.column, buffer.cursor.line, line_len)
        } else {
            return false;
        };
        
        // Store the original offset to detect changes
        let original_offset = self.horizontal_offset;
        
        // Get the actual content width available for text
        let content_width = self.get_content_width(config);
        
        // Use configurable horizontal scrolloff
        let h_scrolloff = config.horizontal_scrolloff as usize;
        
        // Calculate the visible range with scrolloff zones
        let left_boundary = self.horizontal_offset + h_scrolloff;
        let right_boundary = self.horizontal_offset + content_width.saturating_sub(h_scrolloff + 1);
        
        // Adjust horizontal offset if cursor is outside the scrolloff zones
        if cursor_column < left_boundary && cursor_column >= h_scrolloff {
            // Cursor is too far left - scroll left to maintain scrolloff
            self.horizontal_offset = cursor_column.saturating_sub(h_scrolloff);
        } else if cursor_column < h_scrolloff {
            // Cursor is very close to start - reset to beginning
            self.horizontal_offset = 0;
        } else if cursor_column >= right_boundary {
            // Cursor is at or past the right boundary - scroll right to maintain scrolloff
            // We need to ensure there's h_scrolloff space to the right of the cursor
            self.horizontal_offset = cursor_column.saturating_sub(content_width.saturating_sub(h_scrolloff + 1));
            
            // Note: We intentionally don't limit by line length here because:
            // 1. The scrolloff zone should work regardless of current line length
            // 2. Other lines in the buffer might be longer
            // 3. We want consistent scrolling behavior while typing
        }
        
        // Return true if the offset changed
        self.horizontal_offset != original_offset
    }

    pub async fn save_current_buffer_with_prompt(&mut self) -> Result<()> {
        if let Some(buffer) = self.current_buffer() {
            if buffer.file_path.is_some() {
                // File has a path, save directly
                self.save_current_buffer().await
            } else {
                // No path, enter filename prompt mode
                self.enter_filename_prompt_mode();
                Ok(())
            }
        } else {
            Err(anyhow::anyhow!("No buffer to save"))
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.buffers.iter().any(|buffer| buffer.dirty)
    }
    
    /// Select all text in the current buffer
    pub fn select_all(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.select_all();
        }
    }
    
    /// Copy selected text to clipboard
    pub fn copy_selection(&self) -> anyhow::Result<()> {
        if let Some(buffer) = self.current_buffer() {
            buffer.copy_selection()?;
        }
        Ok(())
    }
    
    /// Cut selected text to clipboard
    pub fn cut_selection(&mut self) -> anyhow::Result<()> {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.cut_selection()?;
        }
        Ok(())
    }
    
    /// Paste text from clipboard
    pub fn paste_from_clipboard(&mut self) -> Result<()> {
        // First check if we have a buffer
        if self.current_buffer().is_none() {
            return Ok(());
        }
        
        // Get clipboard text before borrowing buffer
        let mut clipboard = arboard::Clipboard::new()?;
        let text = match clipboard.get_text() {
            Ok(text) => text,
            Err(_) => return Ok(()), // No text in clipboard
        };
        
        // Show progress for large pastes
        let show_progress = text.len() > 10000;
        if show_progress {
            self.paste_in_progress = true;
            self.paste_progress = Some(format!("Pasting {} characters...", text.len()));
        }
        
        // Now do the actual paste
        let result = if let Some(buffer) = self.current_buffer_mut() {
            buffer.paste_from_clipboard()
        } else {
            Ok(())
        };
        
        self.paste_in_progress = false;
        self.paste_progress = None;
        
        result
    }
    
    /// Undo the last action in the current buffer
    pub fn undo(&mut self) -> bool {
        let result = if let Some(buffer) = self.current_buffer_mut() {
            buffer.undo()
        } else {
            false
        };
        
        // If in find/replace mode, update matches after undo
        if result && self.find_replace_mode {
            self.update_find_matches();
        }
        
        result
    }
    
    /// Redo the last undone action in the current buffer
    pub fn redo(&mut self) -> bool {
        let result = if let Some(buffer) = self.current_buffer_mut() {
            buffer.redo()
        } else {
            false
        };
        
        // If in find/replace mode, update matches after redo
        if result && self.find_replace_mode {
            self.update_find_matches();
        }
        
        result
    }
    
    /// Move cursor to the previous paragraph boundary
    pub fn move_to_paragraph_up(&mut self, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer() {
            let current_line = buffer.cursor.line;
            let mut target_line = None;
            
            // Search backwards from current line - 1
            if current_line > 0 {
                for line_idx in (0..current_line).rev() {
                    if self.is_paragraph_start(buffer, line_idx) {
                        target_line = Some(line_idx);
                        break;
                    }
                }
            }
            
            // If we found a paragraph boundary, move to it
            if let Some(line) = target_line {
                if let Some(buffer) = self.current_buffer_mut() {
                    buffer.cursor.line = line;
                    buffer.cursor.column = 0;
                    buffer.cursor.preferred_visual_column = 0;
                    buffer.cursor.clear_selection();
                }
                
                // Center the viewport on the cursor
                self.center_viewport_on_cursor(config, visible_lines);
            }
        }
    }
    
    /// Move cursor to the next paragraph boundary
    pub fn move_to_paragraph_down(&mut self, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer() {
            let current_line = buffer.cursor.line;
            let total_lines = buffer.rope.len_lines();
            let mut target_line = None;
            
            // Search forwards from current line + 1
            if current_line + 1 < total_lines {
                for line_idx in (current_line + 1)..total_lines {
                    if self.is_paragraph_start(buffer, line_idx) {
                        target_line = Some(line_idx);
                        break;
                    }
                }
            }
            
            // If we found a paragraph boundary, move to it
            if let Some(line) = target_line {
                if let Some(buffer) = self.current_buffer_mut() {
                    buffer.cursor.line = line;
                    buffer.cursor.column = 0;
                    buffer.cursor.preferred_visual_column = 0;
                    buffer.cursor.clear_selection();
                }
                
                // Center the viewport on the cursor
                self.center_viewport_on_cursor(config, visible_lines);
            }
        }
    }
    
    /// Check if a line is the start of a paragraph (has an empty line above it)
    fn is_paragraph_start(&self, buffer: &Buffer, line_idx: usize) -> bool {
        // First line is always a paragraph start
        if line_idx == 0 {
            return true;
        }
        
        // Check if the previous line is empty
        if line_idx > 0 {
            let prev_line = buffer.get_line_text(line_idx - 1);
            let prev_line_trimmed = prev_line.trim();
            
            // Previous line is empty or just whitespace
            if prev_line_trimmed.is_empty() {
                // Current line should not be empty to be a paragraph start
                let current_line = buffer.get_line_text(line_idx);
                let current_line_trimmed = current_line.trim();
                return !current_line_trimmed.is_empty();
            }
        }
        
        false
    }
    
    /// Center the viewport on the cursor position
    fn center_viewport_on_cursor(&mut self, config: &Config, visible_lines: usize) {
        if config.word_wrap {
            if let Some(buffer) = self.current_buffer() {
                let content_width = self.get_content_width(config);
                let cursor_visual_line = self.calculate_cursor_visual_line(buffer, content_width);
                let total_visual_lines = self.calculate_total_visual_lines(buffer, content_width);
                
                // Calculate the desired viewport position to center the cursor (in visual lines)
                let half_height = visible_lines as isize / 2;
                let new_viewport_visual = cursor_visual_line as isize - half_height;
                
                // Respect scrolloff limits
                let scrolloff = config.scrolloff as isize;
                let min_viewport = -scrolloff;
                let max_viewport = (total_visual_lines as isize + scrolloff) - visible_lines as isize;
                
                // In word-wrap mode, viewport_line IS the visual line directly
                self.viewport_line = new_viewport_visual
                    .max(min_viewport)
                    .min(max_viewport);
            }
        } else {
            if let Some(buffer) = self.current_buffer() {
                let cursor_line = buffer.cursor.line;
                let rope_len_lines = buffer.rope.len_lines();
                
                // Calculate the desired viewport position to center the cursor
                let half_height = visible_lines as isize / 2;
                let new_viewport_line = cursor_line as isize - half_height;
                
                // Respect scrolloff limits
                let scrolloff = config.scrolloff as isize;
                let min_viewport = -scrolloff;
                let max_viewport = (rope_len_lines as isize + scrolloff) - visible_lines as isize;
                
                // Buffer borrow is dropped here before mutating self
                
                self.viewport_line = new_viewport_line
                    .max(min_viewport)
                    .min(max_viewport);
            }
        }
    }
    
    /// Enter save prompt mode
    pub fn enter_save_prompt_mode(&mut self) {
        self.save_prompt_mode = true;
    }
    
    /// Exit save prompt mode
    pub fn exit_save_prompt_mode(&mut self) {
        self.save_prompt_mode = false;
    }
    
    /// Convert mouse coordinates to buffer position
    pub fn mouse_to_buffer_position(&self, mouse_x: u16, mouse_y: u16, config: &crate::config::Config) -> Option<(usize, usize)> {
        // Calculate the editor area (accounting for margins)
        let editor_x = config.margins.horizontal;
        let editor_y = config.margins.vertical;
        
        // Check if mouse is within editor area
        if mouse_x < editor_x || mouse_y < editor_y {
            return None;
        }
        
        // Calculate gutter width
        let gutter_width = if let Some(buffer) = self.current_buffer() {
            match config.gutter {
                crate::config::GutterMode::None => 0,
                crate::config::GutterMode::Absolute | crate::config::GutterMode::Relative => {
                    let total_lines = buffer.rope.len_lines();
                    let digits = total_lines.to_string().len();
                    digits + 2 // Add 2 for padding
                }
            }
        } else {
            0
        };
        
        // Adjust for gutter
        let content_start_x = editor_x + gutter_width as u16;
        if mouse_x < content_start_x {
            return None; // Click is in the gutter
        }
        
        let relative_x = (mouse_x - content_start_x) as usize;
        let relative_y = (mouse_y - editor_y) as usize;
        
        if let Some(buffer) = self.current_buffer() {
            let content_width = self.calculate_content_width(config);
            
            if config.word_wrap {
                // Handle word-wrapped position conversion
                self.mouse_to_wrapped_position_complex(relative_x, relative_y, content_width)
            } else {
                // Simple case: no word wrap
                let line = if self.viewport_line < 0 {
                    if relative_y < (-self.viewport_line) as usize {
                        return None; // Click is in virtual space above file
                    }
                    relative_y - (-self.viewport_line) as usize
                } else {
                    (self.viewport_line as usize) + relative_y
                };
                
                // Ensure line is within buffer bounds
                if line >= buffer.rope.len_lines() {
                    return None;
                }
                
                let line_text = buffer.get_line_text(line);
                let line_content_len = if line_text.ends_with('\n') {
                    line_text.chars().count().saturating_sub(1)
                } else {
                    line_text.chars().count()
                };
                let column = relative_x.min(line_content_len);
                Some((line, column))
            }
        } else {
            None
        }
    }
    
    fn mouse_to_wrapped_position(&self, line: usize, visual_x: usize, content_width: usize) -> Option<(usize, usize)> {
        if let Some(buffer) = self.current_buffer() {
            let line_text = buffer.get_line_text(line);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let wrapped_segments = crate::text_utils::wrap_line(line_text_for_display, content_width);
            
            // Find which visual line the mouse X coordinate corresponds to
            let mut cursor_column = 0;

            for segment in &wrapped_segments {
                let segment_length = segment.0.chars().count();
                if visual_x < segment_length {
                    cursor_column += visual_x;
                    break;
                } else {
                    cursor_column += segment_length;
                }
            }

            Some((line, cursor_column))
        } else {
            None
        }
    }
    fn mouse_to_wrapped_position_complex(&self, relative_x: usize, relative_y: usize, content_width: usize) -> Option<(usize, usize)> {
        if let Some(buffer) = self.current_buffer() {
            // When word wrap is enabled, viewport_line still refers to logical lines,
            // but we need to translate mouse Y position (which is in visual lines) correctly
            
            // First, we need to find which logical line and wrapped segment corresponds to the visual Y position
            let mut visual_lines_counted = 0;
            
            // Handle negative viewport (virtual lines before file)
            if self.viewport_line < 0 {
                let virtual_lines_before = (-self.viewport_line) as usize;
                if relative_y < virtual_lines_before {
                    return None; // Click is in virtual space
                }
                visual_lines_counted = virtual_lines_before;
            }
            
            // Start from the first logical line in viewport
            let start_logical_line = if self.viewport_line < 0 { 0 } else { self.viewport_line as usize };
            
            // Count visual lines until we reach the target Y position
            for logical_line in start_logical_line..buffer.rope.len_lines() {
                let line_text = buffer.get_line_text(logical_line);
                let line_text_for_display = if line_text.ends_with('\n') {
                    &line_text[..line_text.len()-1]
                } else {
                    &line_text
                };
                
                let wrapped_segments = crate::text_utils::wrap_line(line_text_for_display, content_width);
                let segments_count = wrapped_segments.len().max(1);
                
                // Check if the target Y is within this logical line's visual lines
                if relative_y >= visual_lines_counted && relative_y < visual_lines_counted + segments_count {
                    // Found the logical line! Now find which segment
                    let segment_index = relative_y - visual_lines_counted;
                    
                    if segment_index < wrapped_segments.len() {
                        let (segment_text, start_col) = &wrapped_segments[segment_index];
                        let segment_length = segment_text.chars().count();
                        let column_in_segment = relative_x.min(segment_length);
                        let actual_column = start_col + column_in_segment;
                        return Some((logical_line, actual_column));
                    } else if wrapped_segments.is_empty() {
                        // Empty line
                        return Some((logical_line, 0));
                    }
                }
                
                visual_lines_counted += segments_count;
            }
            
            // Click was beyond the available content
            None
        } else {
            None
        }
    }
    
    fn calculate_content_width(&self, config: &crate::config::Config) -> usize {
        let terminal_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);
        
        // Calculate gutter width
        let gutter_width = if let Some(buffer) = self.current_buffer() {
            match config.gutter {
                crate::config::GutterMode::None => 0,
                crate::config::GutterMode::Absolute | crate::config::GutterMode::Relative => {
                    let total_lines = buffer.rope.len_lines();
                    let digits = total_lines.to_string().len();
                    digits + 2 // Add 2 for padding
                }
            }
        } else {
            0
        };
        
        terminal_width
            .saturating_sub((config.margins.horizontal * 2) as usize)
            .saturating_sub(gutter_width)
            .max(10)
    }
    
    /// Handle regular mouse click - move cursor and clear selection
    pub fn handle_regular_click(&mut self, mouse_x: u16, mouse_y: u16, config: &crate::config::Config, visible_lines: usize) {
        if let Some((line, column)) = self.mouse_to_buffer_position(mouse_x, mouse_y, config) {
            if let Some(buffer) = self.current_buffer_mut() {
                // Clear any existing selection
                buffer.cursor.clear_selection();
                // Move cursor to click position
                buffer.cursor.line = line;
                buffer.cursor.column = column;
                buffer.cursor.preferred_visual_column = column;
                self.adjust_viewport(config, visible_lines);
            }
        }
    }
    
    /// Handle Shift+click - extend selection to mouse position
    pub fn handle_shift_click(&mut self, mouse_x: u16, mouse_y: u16, config: &crate::config::Config, visible_lines: usize) {
        if let Some((line, column)) = self.mouse_to_buffer_position(mouse_x, mouse_y, config) {
            if let Some(buffer) = self.current_buffer_mut() {
                // If no selection exists, start one from current cursor position
                if !buffer.cursor.has_selection() {
                    buffer.cursor.start_selection();
                }
                // Move cursor to clicked position (extending selection)
                buffer.cursor.line = line;
                buffer.cursor.column = column;
                buffer.cursor.preferred_visual_column = column;
                self.adjust_viewport(config, visible_lines);
            }
        }
    }
    
    /// Handle mouse drag - create or extend selection
    pub fn handle_mouse_drag(&mut self, mouse_x: u16, mouse_y: u16, config: &crate::config::Config, visible_lines: usize) {
        if let Some((line, column)) = self.mouse_to_buffer_position(mouse_x, mouse_y, config) {
            if let Some(buffer) = self.current_buffer_mut() {
                // If no selection exists yet, start one from the initial click position
                if !buffer.cursor.has_selection() {
                    buffer.cursor.start_selection();
                }
                // Move cursor to current drag position (extending selection)
                buffer.cursor.line = line;
                buffer.cursor.column = column;
                buffer.cursor.preferred_visual_column = column;
                self.adjust_viewport(config, visible_lines);
            }
        }
    }
}