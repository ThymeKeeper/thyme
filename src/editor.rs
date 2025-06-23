// src/editor.rs

use crate::buffer::Buffer;
use anyhow::Result;
use std::path::PathBuf;

pub struct Editor {
    pub buffers: Vec<Buffer>,
    pub active_buffer: usize,
    pub viewport_line: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            active_buffer: 0,
            viewport_line: 0,
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

    /// Update the preferred visual column based on the current cursor position (with content width)
    /// This calculates the position within the current visual line segment
    /// ONLY call this when the user explicitly moves horizontally or edits text
    fn update_preferred_visual_column_with_width(&mut self, content_width: usize) {
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
        
        let wrapped_segments = wrap_line_simple(line_text_for_display, content_width);
        
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

    // Word-wrap aware cursor movement methods
    pub fn move_cursor_left(&mut self, content_width: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_left();
            // Update preferred visual column based on new position
            // This is intentional - horizontal movement should update the preferred column
            self.update_preferred_visual_column_with_width(content_width);
            self.adjust_viewport();
        }
    }

    pub fn move_cursor_right(&mut self, content_width: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.move_cursor_right();
            // Update preferred visual column based on new position
            // This is intentional - horizontal movement should update the preferred column
            self.update_preferred_visual_column_with_width(content_width);
            self.adjust_viewport();
        }
    }

    pub fn move_cursor_up(&mut self, word_wrap: bool, content_width: usize) {
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
                    let wrapped_segments = wrap_line_simple(line_text_for_display, content_width);
                    
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
                                self.adjust_viewport();
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
                    self.adjust_viewport();
                    return;
                }
            }
            if let Some(buffer) = self.current_buffer_mut() {
                buffer.move_cursor_up();
            }
        }
        self.adjust_viewport();
        // DO NOT call update_preferred_visual_column() here - vertical movement should preserve it
    }

    pub fn move_cursor_down(&mut self, word_wrap: bool, content_width: usize) {
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
                    let wrapped_segments = wrap_line_simple(line_text_for_display, content_width);
                    
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
                                self.adjust_viewport();
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
                    self.adjust_viewport();
                    return;
                }
            }
            if let Some(buffer) = self.current_buffer_mut() {
                buffer.move_cursor_down();
            }
        }
        self.adjust_viewport();
        // DO NOT call update_preferred_visual_column() here - vertical movement should preserve it
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
        let wrapped_segments = wrap_line_simple(line_text_for_display, content_width);
        
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
                        let new_wrapped = wrap_line_simple(new_line_for_display, content_width);
                        
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
        let wrapped_segments = wrap_line_simple(line_text_for_display, content_width);
        
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
                        let new_wrapped = wrap_line_simple(new_line_for_display, content_width);
                        
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

    pub fn move_cursor_page_up(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            for _ in 0..10 {
                buffer.move_cursor_up();
            }
            self.adjust_viewport();
        }
    }

    pub fn move_cursor_page_down(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            for _ in 0..10 {
                buffer.move_cursor_down();
            }
            self.adjust_viewport();
        }
    }

    // Text modification methods
    pub fn insert_char(&mut self, c: char, content_width: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.insert_char(c);
            // Update preferred visual column after typing
            // This is intentional - text insertion should update the preferred column
            self.update_preferred_visual_column_with_width(content_width);
        }
    }

    pub fn insert_newline(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.insert_newline();
            self.adjust_viewport();
            // Don't update preferred visual column - newline resets to column 0
        }
    }

    pub fn insert_tab(&mut self, content_width: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            // Insert 4 spaces for tab
            for _ in 0..4 {
                buffer.insert_char(' ');
            }
            // Update preferred visual column after typing
            // This is intentional - text insertion should update the preferred column
            self.update_preferred_visual_column_with_width(content_width);
        }
    }

    pub fn delete_char_backwards(&mut self, content_width: usize) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.delete_char_backwards();
            // Update preferred visual column after deletion
            // This is intentional - deletion should update the preferred column
            self.update_preferred_visual_column_with_width(content_width);
            self.adjust_viewport();
        }
    }

    pub fn delete_char_forwards(&mut self) {
        if let Some(buffer) = self.current_buffer_mut() {
            buffer.delete_char_forwards();
            // Note: Forward delete doesn't change cursor position, so no need to update preferred column
        }
    }

    fn adjust_viewport(&mut self) {
        if let Some(buffer) = self.current_buffer() {
            let cursor_line = buffer.cursor.line;
            let visible_lines = 20; // This should ideally come from the UI size
            
            // Simple viewport adjustment - keep cursor within visible area
            if cursor_line < self.viewport_line {
                self.viewport_line = cursor_line;
            } else if cursor_line >= self.viewport_line + visible_lines {
                self.viewport_line = cursor_line.saturating_sub(visible_lines - 1);
            }
            
            // Ensure viewport doesn't go beyond the start
            self.viewport_line = self.viewport_line.min(cursor_line);
        }
    }
}

// Move wrap_line_simple to be a standalone function to avoid borrowing issues
fn wrap_line_simple(text: &str, width: usize) -> Vec<(String, usize)> {
    if width == 0 {
        return vec![(text.to_string(), 0)];
    }

    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    
    if chars.is_empty() {
        return vec![(String::new(), 0)];
    }

    let mut start_pos = 0;
    
    while start_pos < chars.len() {
        let mut end_pos = (start_pos + width).min(chars.len());
        
        // If we're not at the end of the text, try to break at a word boundary
        if end_pos < chars.len() {
            // Look backwards from end_pos to find a space
            let mut break_pos = end_pos;
            for i in (start_pos..end_pos).rev() {
                if chars[i] == ' ' {
                    break_pos = i;
                    break;
                }
            }
            
            // If we found a space and it's not too close to the start, use it
            if break_pos > start_pos && (break_pos - start_pos) > width / 4 {
                end_pos = break_pos;
            }
        }
        
        // Extract the segment - DON'T trim trailing spaces to preserve cursor positioning
        let segment: String = chars[start_pos..end_pos].iter().collect();
        
        result.push((segment, start_pos));
        
        // Move to the next segment, skipping any spaces at the break point ONLY if we broke at a space
        if end_pos < chars.len() && chars[end_pos] == ' ' {
            start_pos = end_pos + 1;
        } else {
            start_pos = end_pos;
        }
    }

    if result.is_empty() {
        result.push((String::new(), 0));
    }

    result
}
