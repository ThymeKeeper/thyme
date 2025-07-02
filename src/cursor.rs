// src/cursor.rs
//
// Cursor movement logic with word-wrap support

use crate::text_utils::wrap_line;
use ropey::Rope;

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

    pub fn reset_preferred_column(&mut self) {
        self.preferred_visual_column = self.column;
    }
}

pub struct CursorMovement;

impl CursorMovement {
    /// Update preferred visual column with word-wrap awareness
    pub fn update_preferred_visual_column_with_width(
        cursor: &mut Cursor,
        rope: &Rope,
        content_width: usize,
    ) {
        let line_text = get_line_text(rope, cursor.line);
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
            
            // Use consistent boundary detection logic
            let is_in_segment = if i == wrapped_segments.len() - 1 {
                // Last segment: include the end position
                cursor.column >= *start_pos && cursor.column <= segment_end
            } else {
                // Other segments: exclude the end position (it belongs to next segment)
                cursor.column >= *start_pos && cursor.column < segment_end
            };
            
            if is_in_segment {
                // Calculate position within this visual line segment
                let visual_column = cursor.column - start_pos;
                cursor.preferred_visual_column = visual_column;
                return;
            }
        }
        
        // Fallback: use the logical column as visual column
        cursor.preferred_visual_column = cursor.column;
    }

    /// Move cursor up with word-wrap awareness
    pub fn move_cursor_up_visual(
        cursor: &mut Cursor,
        rope: &Rope,
        content_width: usize,
    ) {
        let current_line_text = get_line_text(rope, cursor.line);
        
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
                cursor.column >= *start_pos && cursor.column <= segment_end
            } else {
                // Other segments: exclude the end position (it belongs to next segment)
                cursor.column >= *start_pos && cursor.column < segment_end
            };
            
            if is_in_segment {
                current_segment_idx = Some(i);
                break;
            }
        }
        
        if let Some(segment_idx) = current_segment_idx {
            if segment_idx > 0 {
                // Move to previous visual line within same logical line
                let prev_segment = &wrapped_segments[segment_idx - 1];
                
                // Always use preferred visual column for consistency
                let target_visual_col = cursor.preferred_visual_column;
                
                // Position in previous segment
                let prev_segment_len = prev_segment.0.chars().count();
                let new_col = prev_segment.1 + target_visual_col.min(prev_segment_len);
                cursor.column = new_col;
                // PRESERVE the original preferred visual column - don't clamp it!
                cursor.preferred_visual_column = target_visual_col;
            } else {
                // Move to previous logical line - find the appropriate visual line
                if cursor.line > 0 {
                    let target_visual_col = cursor.preferred_visual_column;
                    
                    Self::move_cursor_up_basic(cursor, rope);
                    
                    // Position at the appropriate wrapped segment of the new line
                    let new_line_text = get_line_text(rope, cursor.line);
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
                        cursor.column = new_col.min(new_line_for_display.chars().count());
                        // PRESERVE the original preferred visual column - don't clamp it!
                        cursor.preferred_visual_column = target_visual_col;
                    }
                }
            }
        } else {
            // Fallback to regular up movement
            if cursor.line > 0 {
                let target_visual_col = cursor.preferred_visual_column;
                Self::move_cursor_up_basic(cursor, rope);
                // PRESERVE the preferred visual column
                cursor.preferred_visual_column = target_visual_col;
            }
        }
    }

    /// Move cursor down with word-wrap awareness
    pub fn move_cursor_down_visual(
        cursor: &mut Cursor,
        rope: &Rope,
        content_width: usize,
    ) {
        let current_line_text = get_line_text(rope, cursor.line);
        
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
            
            // Use consistent boundary detection logic
            let is_in_segment = if i == wrapped_segments.len() - 1 {
                // Last segment: include the end position
                cursor.column >= *start_pos && cursor.column <= segment_end
            } else {
                // Other segments: exclude the end position (it belongs to next segment)
                cursor.column >= *start_pos && cursor.column < segment_end
            };
            
            if is_in_segment {
                current_segment_idx = Some(i);
                break;
            }
        }
        
        if let Some(segment_idx) = current_segment_idx {
            if segment_idx < wrapped_segments.len() - 1 {
                // Move to next visual line within same logical line
                let next_segment = &wrapped_segments[segment_idx + 1];
                
                // Always use preferred visual column for consistency
                let target_visual_col = cursor.preferred_visual_column;
                
                // Position in next segment
                let next_segment_len = next_segment.0.chars().count();
                let new_col = next_segment.1 + target_visual_col.min(next_segment_len);
                
                cursor.column = new_col;
                // PRESERVE the original preferred visual column - don't clamp it!
                cursor.preferred_visual_column = target_visual_col;
            } else {
                // We're on the last visual segment of the current line
                // Move to next logical line - position at the first visual line
                if cursor.line < rope.len_lines() - 1 {
                    let target_visual_col = cursor.preferred_visual_column;
                    
                    Self::move_cursor_down_basic(cursor, rope);
                    
                    // Position at the appropriate wrapped segment of the new line
                    let new_line_text = get_line_text(rope, cursor.line);
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
                        cursor.column = new_col;
                        // PRESERVE the original preferred visual column - don't clamp it!
                        cursor.preferred_visual_column = target_visual_col;
                    } else {
                        // Empty line or no wrapping
                        cursor.column = target_visual_col.min(new_line_for_display.chars().count());
                        // PRESERVE the original preferred visual column - don't clamp it!
                        cursor.preferred_visual_column = target_visual_col;
                    }
                }
            }
        } else {
            // Fallback: couldn't find current segment
            if cursor.line < rope.len_lines() - 1 {
                let target_visual_col = cursor.preferred_visual_column;
                Self::move_cursor_down_basic(cursor, rope);
                // Preserve preferred visual column
                let line_text = get_line_text(rope, cursor.line);
                let line_content_len = if line_text.ends_with('\n') {
                    line_text.len() - 1
                } else {
                    line_text.len()
                };
                cursor.column = target_visual_col.min(line_content_len);
                // PRESERVE the original preferred visual column - don't clamp it!
                cursor.preferred_visual_column = target_visual_col;
            }
        }
    }

    /// Basic cursor movement without word-wrap awareness
    pub fn move_cursor_left(cursor: &mut Cursor, rope: &Rope) {
        if cursor.column > 0 {
            cursor.column -= 1;
        } else if cursor.line > 0 {
            cursor.line -= 1;
            let line_text = get_line_text(rope, cursor.line);
            cursor.column = if line_text.ends_with('\n') {
                line_text.len() - 1
            } else {
                line_text.len()
            };
        }
    }

    pub fn move_cursor_right(cursor: &mut Cursor, rope: &Rope) {
        let line_text = get_line_text(rope, cursor.line);
        let line_content_len = if line_text.ends_with('\n') {
            line_text.len() - 1
        } else {
            line_text.len()
        };
        
        if cursor.column < line_content_len {
            cursor.column += 1;
        } else if cursor.line < rope.len_lines() - 1 {
            cursor.line += 1;
            cursor.column = 0;
        }
    }

    pub fn move_cursor_up_basic(cursor: &mut Cursor, rope: &Rope) {
        if cursor.line > 0 {
            cursor.line -= 1;
            let line_text = get_line_text(rope, cursor.line);
            let line_content_len = if line_text.ends_with('\n') {
                line_text.len() - 1
            } else {
                line_text.len()
            };
            cursor.column = cursor.preferred_visual_column.min(line_content_len);
        }
    }

    pub fn move_cursor_down_basic(cursor: &mut Cursor, rope: &Rope) {
        if cursor.line < rope.len_lines() - 1 {
            cursor.line += 1;
            let line_text = get_line_text(rope, cursor.line);
            let line_content_len = if line_text.ends_with('\n') {
                line_text.len() - 1
            } else {
                line_text.len()
            };
            cursor.column = cursor.preferred_visual_column.min(line_content_len);
        }
    }

    pub fn move_cursor_home(cursor: &mut Cursor) {
        cursor.column = 0;
        cursor.preferred_visual_column = 0;
    }

    pub fn move_cursor_end(cursor: &mut Cursor, rope: &Rope) {
        let line_text = get_line_text(rope, cursor.line);
        cursor.column = if line_text.ends_with('\n') {
            line_text.len() - 1
        } else {
            line_text.len()
        };
        cursor.preferred_visual_column = cursor.column;
    }
}

/// Helper function to get line text from rope
fn get_line_text(rope: &Rope, line: usize) -> String {
    if line < rope.len_lines() {
        rope.line(line).to_string()
    } else {
        String::new()
    }
}
