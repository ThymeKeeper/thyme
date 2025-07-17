// src/cursor.rs
//
// Cursor movement logic with word-wrap support

use crate::text_utils::wrap_line;
use crate::unicode_utils::{char_display_width, str_display_width, char_pos_to_visual_column, visual_column_to_char_pos, substring_visual_width};
use ropey::Rope;

#[derive(Debug, Clone)]
pub struct Cursor {
    pub line: usize,
    pub column: usize,
    pub preferred_visual_column: usize, // Visual column position (accounting for character widths)
    pub selection_start: Option<Position>, // Start of text selection
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self { 
            line: 0, 
            column: 0,
            preferred_visual_column: 0,
            selection_start: None,
        }
    }
    
    // Update preferred column when moving horizontally or typing
    pub fn update_preferred_visual_column(&mut self, visual_column: usize) {
        self.preferred_visual_column = visual_column;
    }

    pub fn reset_preferred_column(&mut self) {
        self.preferred_visual_column = self.column;
    }
    
    /// Start text selection at current cursor position
    pub fn start_selection(&mut self) {
        self.selection_start = Some(Position {
            line: self.line,
            column: self.column,
        });
    }
    
    /// Clear text selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
    }
    
    /// Check if there is an active selection
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some()
    }
    
    /// Get the selection range (start and end positions)
    /// Returns (start, end) where start is always before end
    pub fn get_selection_range(&self) -> Option<(Position, Position)> {
        if let Some(start) = self.selection_start {
            let current = Position {
                line: self.line,
                column: self.column,
            };
            
            // Ensure start comes before end
            if start.line < current.line || (start.line == current.line && start.column < current.column) {
                Some((start, current))
            } else if start.line > current.line || (start.line == current.line && start.column > current.column) {
                Some((current, start))
            } else {
                // Start and end are the same position - no selection
                None
            }
        } else {
            None
        }
    }
    
    /// Move cursor to a specific position and optionally extend selection
    pub fn move_to_position(&mut self, new_line: usize, new_column: usize, extend_selection: bool) {
        if extend_selection && self.selection_start.is_none() {
            // Start new selection from current position
            self.start_selection();
        } else if !extend_selection {
            // Clear selection if not extending
            self.clear_selection();
        }
        
        self.line = new_line;
        self.column = new_column;
        self.preferred_visual_column = new_column;
    }
}

pub struct CursorMovement;

impl CursorMovement {
    /// Calculate the visual column within a wrapped segment
    /// This accounts for any indentation added to continuation lines
    fn get_visual_column_in_segment(
        segment: &str,
        char_pos_in_original: usize,
        segment_start_in_original: usize
    ) -> usize {
        // Get the position within this segment
        let pos_in_segment = char_pos_in_original.saturating_sub(segment_start_in_original);
        
        // Calculate the visual column from the start of the segment
        // This includes any indentation that was added
        segment.chars()
            .take(pos_in_segment)
            .map(char_display_width)
            .sum()
    }

    /// Find the character position in the original text for a visual column in a segment
    fn get_char_pos_from_visual_column(
        segment: &str,
        visual_col: usize,
        segment_start_in_original: usize,
        original_line: &str
    ) -> usize {
        let mut current_visual_col = 0;
        let mut segment_char_pos = 0;
        
        // First, we need to determine how much of the segment is indentation
        let indent_width = {
            let mut width = 0;
            let mut original_pos = 0;
            for ch in segment.chars() {
                // Check if this character exists at the current position in the original
                if original_pos < original_line.len() {
                    let original_chars: Vec<char> = original_line.chars().collect();
                    if segment_start_in_original + original_pos < original_chars.len() &&
                       original_chars[segment_start_in_original + original_pos] == ch {
                        // This character is from the original text, not added indentation
                        break;
                    }
                }
                width += char_display_width(ch);
                original_pos += 1;
            }
            width
        };
        
        // Now find the position for the visual column
        for ch in segment.chars() {
            let ch_width = char_display_width(ch);
            
            if current_visual_col >= visual_col {
                break;
            }
            
            if current_visual_col + ch_width > visual_col {
                // Target column is in the middle of a wide character
                // Decide whether to position before or after based on which is closer
                if visual_col - current_visual_col >= ch_width / 2 {
                    segment_char_pos += 1;
                }
                break;
            }
            
            current_visual_col += ch_width;
            segment_char_pos += 1;
        }
        
        // Now we need to convert segment position to original text position
        // We need to account for any indentation that was added
        let indent_chars = (current_visual_col.min(indent_width) + char_display_width(' ') - 1) / char_display_width(' ');
        
        if segment_char_pos < indent_chars {
            // We're still in the indentation area, position at segment start
            segment_start_in_original
        } else {
            // We're past the indentation, calculate actual position
            segment_start_in_original + (segment_char_pos - indent_chars)
        }
    }

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
        
        // For empty lines, set to 0
        if line_text_for_display.is_empty() {
            cursor.preferred_visual_column = 0;
            return;
        }
        
        // Get wrapped segments
        let wrapped_segments = wrap_line(line_text_for_display, content_width);
        
        // If no wrapped segments, fall back to simple calculation
        if wrapped_segments.is_empty() {
            cursor.preferred_visual_column = char_pos_to_visual_column(line_text_for_display, cursor.column);
            return;
        }
        
        // Find which segment contains the cursor
        let mut current_segment_idx = 0;
        let mut segment_found = false;
        
        for (i, (segment, start_pos)) in wrapped_segments.iter().enumerate() {
            let segment_end = if i + 1 < wrapped_segments.len() {
                wrapped_segments[i + 1].1
            } else {
                line_text_for_display.chars().count()
            };
            
            if cursor.column >= *start_pos && cursor.column < segment_end {
                current_segment_idx = i;
                segment_found = true;
                break;
            } else if cursor.column == segment_end && i == wrapped_segments.len() - 1 {
                // Cursor at the very end of the last segment
                current_segment_idx = i;
                segment_found = true;
                break;
            }
        }
        
        // If segment not found, use the last segment
        if !segment_found && !wrapped_segments.is_empty() {
            current_segment_idx = wrapped_segments.len() - 1;
        }
        
        let (segment, start_pos) = &wrapped_segments[current_segment_idx];
        let pos_in_segment = cursor.column.saturating_sub(*start_pos);
        
        // Calculate visual column
        cursor.preferred_visual_column = if current_segment_idx == 0 {
            // First segment - calculate from start of line to cursor
            // This handles any real indentation in the original text
            char_pos_to_visual_column(line_text_for_display, cursor.column)
        } else {
            // Continuation line - account for virtual indentation
            let mut virtual_indent_width = 0;
            
            // Count virtual indentation at the beginning of the segment
            for ch in segment.chars() {
                if ch == ' ' || ch == '\t' {
                    virtual_indent_width += char_display_width(ch);
                } else {
                    break;
                }
            }
            
            // Calculate visual width of content from segment start to cursor
            let content_visual_width = if *start_pos + pos_in_segment <= line_text_for_display.chars().count() {
                substring_visual_width(
                    line_text_for_display,
                    *start_pos,
                    start_pos + pos_in_segment
                )
            } else {
                // Cursor beyond end of line
                substring_visual_width(
                    line_text_for_display,
                    *start_pos,
                    line_text_for_display.chars().count()
                )
            };
            
            virtual_indent_width + content_visual_width
        };
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
        
        // Check if cursor is beyond the end of the line
        if cursor.column >= line_text_for_display.chars().count() {
            // Cursor is at or beyond EOL - it belongs to the last segment
            if !wrapped_segments.is_empty() {
                current_segment_idx = Some(wrapped_segments.len() - 1);
            } else {
                current_segment_idx = Some(0);
            }
        } else {
            // Normal case - find which segment contains the cursor
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
        }
        
        if let Some(segment_idx) = current_segment_idx {
            if segment_idx > 0 {
                // Move to previous visual line within same logical line
                let prev_segment = &wrapped_segments[segment_idx - 1];
                
                // Use the preferred visual column to find position in previous segment
                let new_col = Self::get_char_pos_from_visual_column(
                    &prev_segment.0,
                    cursor.preferred_visual_column,
                    prev_segment.1,
                    line_text_for_display
                );
                
                cursor.column = new_col.min(line_text_for_display.chars().count());
            } else {
                // Move to previous logical line - find the appropriate visual line
                if cursor.line > 0 {
                    cursor.line -= 1;
                    let new_line_text = get_line_text(rope, cursor.line);
                    let new_line_for_display = if new_line_text.ends_with('\n') {
                        &new_line_text[..new_line_text.len()-1]
                    } else {
                        &new_line_text
                    };
                    let new_wrapped = wrap_line(new_line_for_display, content_width);
                    
                    if !new_wrapped.is_empty() {
                        // Go to the LAST visual line of the previous logical line
                        let last_segment = &new_wrapped[new_wrapped.len() - 1];
                        
                        // Use preferred visual column to find position
                        let new_col = Self::get_char_pos_from_visual_column(
                            &last_segment.0,
                            cursor.preferred_visual_column,
                            last_segment.1,
                            new_line_for_display
                        );
                        
                        cursor.column = new_col.min(new_line_for_display.chars().count());
                    } else {
                        cursor.column = 0;
                    }
                }
            }
        } else {
            // Fallback to regular up movement
            if cursor.line > 0 {
                Self::move_cursor_up_basic(cursor, rope);
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
        
        // Check if cursor is beyond the end of the line
        if cursor.column >= line_text_for_display.chars().count() {
            // Cursor is at or beyond EOL - it belongs to the last segment
            if !wrapped_segments.is_empty() {
                current_segment_idx = Some(wrapped_segments.len() - 1);
            } else {
                current_segment_idx = Some(0);
            }
        } else {
            // Normal case - find which segment contains the cursor
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
        }
        
        if let Some(segment_idx) = current_segment_idx {
            if segment_idx < wrapped_segments.len() - 1 {
                // Move to next visual line within same logical line
                let next_segment = &wrapped_segments[segment_idx + 1];
                
                // Use the preferred visual column to find position in next segment
                let new_col = Self::get_char_pos_from_visual_column(
                    &next_segment.0,
                    cursor.preferred_visual_column,
                    next_segment.1,
                    line_text_for_display
                );
                
                cursor.column = new_col.min(line_text_for_display.chars().count());
            } else {
                // We're on the last visual segment of the current line
                // Move to next logical line - position at the first visual line
                if cursor.line < rope.len_lines() - 1 {
                    cursor.line += 1;
                    let new_line_text = get_line_text(rope, cursor.line);
                    let new_line_for_display = if new_line_text.ends_with('\n') {
                        &new_line_text[..new_line_text.len()-1]
                    } else {
                        &new_line_text
                    };
                    let new_wrapped = wrap_line(new_line_for_display, content_width);
                    
                    if !new_wrapped.is_empty() {
                        // Go to the FIRST visual line of the next logical line
                        let first_segment = &new_wrapped[0];
                        
                        // Use preferred visual column to find position
                        let new_col = Self::get_char_pos_from_visual_column(
                            &first_segment.0,
                            cursor.preferred_visual_column,
                            first_segment.1,
                            new_line_for_display
                        );
                        
                        cursor.column = new_col.min(new_line_for_display.chars().count());
                    } else {
                        cursor.column = 0;
                    }
                }
            }
        } else {
            // Fallback: couldn't find current segment
            if cursor.line < rope.len_lines() - 1 {
                Self::move_cursor_down_basic(cursor, rope);
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
                line_text.chars().count() - 1
            } else {
                line_text.chars().count()
            };
        }
    }

    pub fn move_cursor_right(cursor: &mut Cursor, rope: &Rope) {
        let line_text = get_line_text(rope, cursor.line);
        let line_content_len = if line_text.ends_with('\n') {
            line_text.chars().count() - 1
        } else {
            line_text.chars().count()
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
            let line_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            // Convert preferred visual column to character position
            cursor.column = visual_column_to_char_pos(line_for_display, cursor.preferred_visual_column);
        }
    }

    pub fn move_cursor_down_basic(cursor: &mut Cursor, rope: &Rope) {
        if cursor.line < rope.len_lines() - 1 {
            cursor.line += 1;
            let line_text = get_line_text(rope, cursor.line);
            let line_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            // Convert preferred visual column to character position
            cursor.column = visual_column_to_char_pos(line_for_display, cursor.preferred_visual_column);
        }
    }

    pub fn move_cursor_home(cursor: &mut Cursor) {
        cursor.column = 0;
        cursor.preferred_visual_column = 0;
    }

    pub fn move_cursor_end(cursor: &mut Cursor, rope: &Rope) {
        let line_text = get_line_text(rope, cursor.line);
        cursor.column = if line_text.ends_with('\n') {
            line_text.chars().count() - 1
        } else {
            line_text.chars().count()
        };
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
