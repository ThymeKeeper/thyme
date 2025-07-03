// src/editor.rs

use crate::{
    buffer::Buffer,
    config::{Config, Theme},
    text_utils::wrap_line
};
use anyhow::Result;
use std::path::PathBuf;

pub struct Editor {
    pub buffers: Vec<Buffer>,
    pub active_buffer: usize,
    pub viewport_line: isize,
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
    // Filename prompt mode
    pub filename_prompt_mode: bool,
    pub filename_prompt_text: String,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            active_buffer: 0,
            viewport_line: 0,
            language_selection_mode: false,
            language_selection_index: 0,
            language_selection_scroll_offset: 0,
            theme_selection_mode: false,
            theme_selection_index: 0,
            theme_selection_scroll_offset: 0,
            available_themes: Vec::new(),
            help_mode: false,
            filename_prompt_mode: false,
            filename_prompt_text: String::new(),
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

    // Cursor movement methods with word-wrap support
    pub fn update_preferred_visual_column_with_width(&mut self, content_width: usize) {
        // Extract needed data first to avoid borrowing conflicts
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
        
        // Find which visual line segment the cursor is in
        for (i, (_segment, start_pos)) in wrapped_segments.iter().enumerate() {
            let segment_end = if i + 1 < wrapped_segments.len() {
                wrapped_segments[i + 1].1
            } else {
                line_text_for_display.chars().count()
            };
            
            // FIXED: Use consistent boundary detection logic
            let is_in_segment = if i == wrapped_segments.len() - 1 {
                // Last segment: include the end position
                cursor_column >= *start_pos && cursor_column <= segment_end
            } else {
                // Other segments: exclude the end position (it belongs to next segment)
                cursor_column >= *start_pos && cursor_column < segment_end
            };
            
            if is_in_segment {
                // Calculate position within this visual line segment
                let visual_column = cursor_column - start_pos;
                // Now update the buffer
                if let Some(buffer) = self.current_buffer_mut() {
                    buffer.cursor.preferred_visual_column = visual_column;
                }
                return;
            }
        }
        
        // Fallback: use the logical column as visual column
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.cursor.preferred_visual_column = cursor_column;
        }
    }

    pub fn move_cursor_left(&mut self, content_width: usize, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_left();
            self.update_preferred_visual_column_with_width(content_width);
            self.adjust_viewport(config, visible_lines);
        }
    }

    pub fn move_cursor_right(&mut self, content_width: usize, config: &Config, visible_lines: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_right();
            self.update_preferred_visual_column_with_width(content_width);
            self.adjust_viewport(config, visible_lines);
        }
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
            self.move_cursor_up_visual(content_width);
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
            
            self.move_cursor_down_visual(content_width);
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

    fn move_cursor_up_visual(&mut self, content_width: usize) {
        let (current_line_text, _cursor_line, cursor_column, preferred_visual_column) = if let Some(buffer) = self.current_buffer() {
            (buffer.get_line_text(buffer.cursor.line), buffer.cursor.line, buffer.cursor.column, buffer.cursor.preferred_visual_column)
        } else {
            return;
        };
        
        // For cursor movement, work with display text (no newlines)
        let line_text_for_display = if current_line_text.ends_with('\n') {
            &current_line_text[..current_line_text.len()-1]
        } else {
            &current_line_text
        };
        let wrapped_segments = wrap_line(line_text_for_display, content_width);
        
        // Find which visual line segment we're currently in
        let mut current_segment_idx = None;
        for (i, (_segment, start_pos)) in wrapped_segments.iter().enumerate() {
            let segment_end = if i + 1 < wrapped_segments.len() {
                wrapped_segments[i + 1].1 // Next segment's start position
            } else {
                line_text_for_display.chars().count() // End of line
            };
            
            // Fix boundary detection: use < for segment boundaries, <= only for the last segment
            let is_in_segment = if i == wrapped_segments.len() - 1 {
                // Last segment: include the end position
                cursor_column >= *start_pos && cursor_column <= segment_end
            } else {
                // Other segments: exclude the end position (it belongs to next segment)
                cursor_column >= *start_pos && cursor_column < segment_end
            };
            
            if is_in_segment {
                current_segment_idx = Some(i);
                break;
            }
        }
        
        if let Some(buffer) = self.current_buffer_mut() {
            if let Some(segment_idx) = current_segment_idx {
                if segment_idx > 0 {
                    // Move to previous visual line within same logical line
                    let prev_segment = &wrapped_segments[segment_idx - 1];
                    
                    // Always use preferred visual column for consistency
                    let target_visual_col = preferred_visual_column;
                    
                    // Position in previous segment
                    let prev_segment_len = prev_segment.0.chars().count();
                    let new_col = prev_segment.1 + target_visual_col.min(prev_segment_len);
                    buffer.cursor.column = new_col;
                    // PRESERVE the original preferred visual column - don't clamp it!
                    buffer.cursor.preferred_visual_column = target_visual_col;
                } else {
                    // Move to previous logical line - find the appropriate visual line
                    if buffer.cursor.line > 0 {
                        let target_visual_col = preferred_visual_column;
                        
                        buffer.move_cursor_up();
                        
                        // Position at the appropriate wrapped segment of the new line
                        let new_line_text = buffer.get_line_text(buffer.cursor.line);
                        let new_line_for_display = if new_line_text.ends_with('\n') {
                            &new_line_text[..new_line_text.len()-1]
                        } else {
                            &new_line_text
                        };
                        let new_wrapped = wrap_line(new_line_for_display, content_width);
                        
                        if !new_wrapped.is_empty() {
                            // FIXED: Always go to the LAST visual line of the previous logical line
                            // This is the natural behavior users expect
                            let last_segment = &new_wrapped[new_wrapped.len() - 1];
                            let segment_len = last_segment.0.chars().count();
                            let new_col = last_segment.1 + target_visual_col.min(segment_len);
                            buffer.cursor.column = new_col.min(new_line_for_display.chars().count());
                            // PRESERVE the original preferred visual column - don't clamp it!
                            buffer.cursor.preferred_visual_column = target_visual_col;
                        }
                    }
                }
            } else {
                // Fallback to regular up movement
                if buffer.cursor.line > 0 {
                    let target_visual_col = preferred_visual_column;
                    buffer.move_cursor_up();
                    // PRESERVE the preferred visual column
                    buffer.cursor.preferred_visual_column = target_visual_col;
                }
            }
        }
    }

    fn move_cursor_down_visual(&mut self, content_width: usize) {
        let (current_line_text, cursor_column, preferred_visual_column, _cursor_line, total_lines) = if let Some(buffer) = self.current_buffer() {
            (buffer.get_line_text(buffer.cursor.line), buffer.cursor.column, buffer.cursor.preferred_visual_column, buffer.cursor.line, buffer.rope.len_lines())
        } else {
            return;
        };
        
        // For cursor movement, work with display text (no newlines)
        let line_text_for_display = if current_line_text.ends_with('\n') {
            &current_line_text[..current_line_text.len()-1]
        } else {
            &current_line_text
        };
        let wrapped_segments = wrap_line(line_text_for_display, content_width);
        
        // Find which visual line segment we're currently in
        let mut current_segment_idx = None;
        
        for (i, (_segment, start_pos)) in wrapped_segments.iter().enumerate() {
            let segment_end = if i + 1 < wrapped_segments.len() {
                wrapped_segments[i + 1].1
            } else {
                line_text_for_display.chars().count()
            };
            
            // FIXED: Use consistent boundary detection logic
            let is_in_segment = if i == wrapped_segments.len() - 1 {
                // Last segment: include the end position
                cursor_column >= *start_pos && cursor_column <= segment_end
            } else {
                // Other segments: exclude the end position (it belongs to next segment)
                cursor_column >= *start_pos && cursor_column < segment_end
            };
            
            if is_in_segment {
                current_segment_idx = Some(i);
                break;
            }
        }
        
        if let Some(buffer) = self.current_buffer_mut() {
            if let Some(segment_idx) = current_segment_idx {
                if segment_idx < wrapped_segments.len() - 1 {
                    // Move to next visual line within same logical line
                    let next_segment = &wrapped_segments[segment_idx + 1];
                    
                    // Always use preferred visual column for consistency
                    let target_visual_col = preferred_visual_column;
                    
                    // Position in next segment
                    let next_segment_len = next_segment.0.chars().count();
                    let new_col = next_segment.1 + target_visual_col.min(next_segment_len);
                    
                    buffer.cursor.column = new_col;
                    // PRESERVE the original preferred visual column - don't clamp it!
                    buffer.cursor.preferred_visual_column = target_visual_col;
                } else {
                    // We're on the last visual segment of the current line
                    // Move to next logical line - position at the first visual line
                    if buffer.cursor.line < buffer.rope.len_lines() - 1 {
                        let target_visual_col = preferred_visual_column;
                        
                        buffer.move_cursor_down();
                        
                        // Position at the appropriate wrapped segment of the new line
                        let new_line_text = buffer.get_line_text(buffer.cursor.line);
                        let new_line_for_display = if new_line_text.ends_with('\n') {
                            &new_line_text[..new_line_text.len()-1]
                        } else {
                            &new_line_text
                        };
                        let new_wrapped = wrap_line(new_line_for_display, content_width);
                        
                        if !new_wrapped.is_empty() {
                            // FIXED: Always go to the FIRST visual line of the next logical line
                            // This is the natural behavior users expect
                            let first_segment = &new_wrapped[0];
                            let segment_len = first_segment.0.chars().count();
                            let new_col = first_segment.1 + target_visual_col.min(segment_len);
                            buffer.cursor.column = new_col;
                            // PRESERVE the original preferred visual column - don't clamp it!
                            buffer.cursor.preferred_visual_column = target_visual_col;
                        } else {
                            // Empty line or no wrapping
                            buffer.cursor.column = target_visual_col.min(new_line_for_display.chars().count());
                            // PRESERVE the original preferred visual column - don't clamp it!
                            buffer.cursor.preferred_visual_column = target_visual_col;
                        }
                    }
                }
            } else {
                // Fallback: couldn't find current segment
                if let Some(buffer) = self.current_buffer() {
                    if buffer.cursor.line < total_lines - 1 {
                        let target_visual_col = preferred_visual_column;
                        if let Some(buffer) = self.current_buffer_mut() {
                            buffer.move_cursor_down();
                            // Preserve preferred visual column
                            let line_text = buffer.get_line_text(buffer.cursor.line);
                            let line_content_len = if line_text.ends_with('\n') {
                                line_text.len() - 1
                            } else {
                                line_text.len()
                            };
                            buffer.cursor.column = target_visual_col.min(line_content_len);
                            // PRESERVE the original preferred visual column - don't clamp it!
                            buffer.cursor.preferred_visual_column = target_visual_col;
                        }
                    }
                }
            }
        }
    }

    pub fn move_cursor_home(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_home();
        }
    }

    pub fn move_cursor_end(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_end();
        }
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
    pub fn insert_char(&mut self, c: char, content_width: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.insert_char(c);
            self.update_preferred_visual_column_with_width(content_width);
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
    
    fn adjust_viewport(&mut self, config: &Config, visible_lines: usize) {
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
            
            // For small buffers that fit entirely in viewport, prefer top positioning
            // but establish proper scrolloff zones as the buffer grows
            if total_file_lines <= visible_lines {
                // Buffer fits entirely in viewport
                if cursor_line < scrolloff {
                    // Cursor is in top area - keep top positioning
                    self.viewport_line = -(scrolloff as isize);
                } else {
                    // Cursor has moved down - establish proper bottom scrolloff
                    // Position viewport so cursor is at bottom scrolloff boundary
                    let desired_viewport = cursor_line as isize - (visible_lines - scrolloff - 1) as isize;
                    self.viewport_line = desired_viewport.max(-(scrolloff as isize));
                }
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

    // Help mode methods
    pub fn enter_help_mode(&mut self) {
        self.help_mode = true;
    }

    pub fn exit_help_mode(&mut self) {
        self.help_mode = false;
    }

    // Filename prompt methods
    pub fn enter_filename_prompt_mode(&mut self) {
        self.filename_prompt_mode = true;
        self.filename_prompt_text.clear();
    }

    pub fn exit_filename_prompt_mode(&mut self) {
        self.filename_prompt_mode = false;
        self.filename_prompt_text.clear();
    }

    pub fn add_char_to_filename_prompt(&mut self, c: char) {
        self.filename_prompt_text.push(c);
    }

    pub fn backspace_filename_prompt(&mut self) {
        self.filename_prompt_text.pop();
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
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.paste_from_clipboard()
        } else {
            Ok(())
        }
    }
    
    /// Undo the last action in the current buffer
    pub fn undo(&mut self) -> bool {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.undo()
        } else {
            false
        }
    }
    
    /// Redo the last undone action in the current buffer
    pub fn redo(&mut self) -> bool {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.redo()
        } else {
            false
        }
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
        
        let relative_x = (mouse_x - editor_x) as usize;
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
            // We need to traverse all lines from viewport_line to find which logical line
            // and column corresponds to the visual position (relative_x, relative_y)
            let mut current_visual_line = 0;
            
            let start_logical_line = if self.viewport_line < 0 { 0 } else { self.viewport_line as usize };
            let virtual_offset = if self.viewport_line < 0 { (-self.viewport_line) as usize } else { 0 };
            
            // Account for virtual lines at the start
            if self.viewport_line < 0 && relative_y < virtual_offset {
                return None; // Click is in virtual space
            }
            
            let adjusted_y = if self.viewport_line < 0 {
                relative_y.saturating_sub(virtual_offset)
            } else {
                relative_y
            };
            
            for logical_line in start_logical_line.. {
                if logical_line >= buffer.rope.len_lines() {
                    break;
                }
                
                let line_text = buffer.get_line_text(logical_line);
                let line_text_for_display = if line_text.ends_with('\n') {
                    &line_text[..line_text.len()-1]
                } else {
                    &line_text
                };
                
                let wrapped_segments = crate::text_utils::wrap_line(line_text_for_display, content_width);
                
                // Check each wrapped segment of this logical line
                for (_segment_idx, (segment_text, start_col)) in wrapped_segments.iter().enumerate() {
                    if current_visual_line == adjusted_y {
                        // This is the target visual line!
                        let segment_length = segment_text.chars().count();
                        let column_in_segment = relative_x.min(segment_length);
                        let actual_column = start_col + column_in_segment;
                        return Some((logical_line, actual_column));
                    }
                    current_visual_line += 1;
                }
                
                // If this line has no wrapped segments (empty line), still count it as one visual line
                if wrapped_segments.is_empty() {
                    if current_visual_line == adjusted_y {
                        return Some((logical_line, 0));
                    }
                    current_visual_line += 1;
                }
            }
            
            // If we get here, the click was beyond the available content
            None
        } else {
            None
        }
    }
    
    fn calculate_content_width(&self, config: &crate::config::Config) -> usize {
        let terminal_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);
        
        terminal_width
            .saturating_sub((config.margins.horizontal * 2) as usize)
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

