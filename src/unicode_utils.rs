// src/unicode_utils.rs

use unicode_width::UnicodeWidthChar;

/// Calculate the display width of a character in terminal columns
pub fn char_display_width(ch: char) -> usize {
    ch.width().unwrap_or(1)
}

/// Calculate the display width of a string in terminal columns
pub fn str_display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

/// Convert a byte position to a column position considering Unicode width
pub fn byte_pos_to_column(text: &str, byte_pos: usize) -> usize {
    let mut col = 0;
    let mut current_byte_pos = 0;
    
    for ch in text.chars() {
        if current_byte_pos >= byte_pos {
            break;
        }
        col += char_display_width(ch);
        current_byte_pos += ch.len_utf8();
    }
    
    col
}

/// Convert a column position to a byte position considering Unicode width
pub fn column_to_byte_pos(text: &str, target_col: usize) -> Option<usize> {
    let mut current_col = 0;
    let mut byte_pos = 0;
    
    for ch in text.chars() {
        let ch_width = char_display_width(ch);
        if current_col + ch_width > target_col {
            return Some(byte_pos);
        }
        current_col += ch_width;
        byte_pos += ch.len_utf8();
        
        if current_col == target_col {
            return Some(byte_pos);
        }
    }
    
    if current_col < target_col {
        Some(text.len())
    } else {
        None
    }
}

/// Get the visual column position for a character position in a line
pub fn char_pos_to_visual_column(text: &str, char_pos: usize) -> usize {
    text.chars()
        .take(char_pos)
        .map(char_display_width)
        .sum()
}

/// Get the character position for a visual column in a line
pub fn visual_column_to_char_pos(text: &str, visual_col: usize) -> usize {
    let mut current_col = 0;
    let mut char_pos = 0;
    
    for ch in text.chars() {
        let ch_width = char_display_width(ch);
        
        // If we've reached or passed the target column, stop
        if current_col >= visual_col {
            break;
        }
        
        // If the target column falls in the middle of a wide character,
        // we should position at the start of that character
        if current_col + ch_width > visual_col {
            // If we're closer to the end of the character, move to next character
            if visual_col > current_col && (visual_col - current_col) >= (ch_width / 2) {
                char_pos += 1;
            }
            break;
        }
        
        current_col += ch_width;
        char_pos += 1;
    }
    
    char_pos
}

/// Calculate the visual width of a slice of characters
pub fn chars_display_width(chars: &[char]) -> usize {
    chars.iter().map(|&ch| char_display_width(ch)).sum()
}

/// Find the character index that would result in the given visual column
/// Returns (char_index, exact_match) where exact_match is true if the column exactly matches
pub fn find_char_at_visual_column(chars: &[char], target_col: usize) -> (usize, bool) {
    let mut current_col = 0;
    
    for (i, &ch) in chars.iter().enumerate() {
        let ch_width = char_display_width(ch);
        
        if current_col == target_col {
            return (i, true);
        }
        
        if current_col + ch_width > target_col {
            // We're in the middle of a wide character
            return (i, false);
        }
        
        current_col += ch_width;
    }
    
    // Past the end
    (chars.len(), current_col == target_col)
}

/// Calculate visual columns for substring of text
pub fn substring_visual_width(text: &str, start_char: usize, end_char: usize) -> usize {
    text.chars()
        .skip(start_char)
        .take(end_char - start_char)
        .map(char_display_width)
        .sum()
}
