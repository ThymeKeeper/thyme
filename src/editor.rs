use crate::buffer::Buffer;
use crate::commands::Command;
use crate::syntax::SyntaxHighlighter;
use arboard::Clipboard;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthChar;

/// Token type for word boundary detection
#[derive(Debug, PartialEq, Copy, Clone)]
enum TokenType {
    Word,      // Alphanumeric and underscore
    Operator,  // Programming operators
    Space,     // Whitespace
    Other,     // Everything else
}

/// Categorize a character into a token type for word navigation
fn get_token_type(ch: char) -> TokenType {
    if ch.is_alphanumeric() || ch == '_' {
        TokenType::Word
    } else if ch.is_whitespace() {
        TokenType::Space
    } else if ">=<!=+-*/%&|^~.".contains(ch) {
        TokenType::Operator
    } else {
        TokenType::Other
    }
}

pub struct Editor {
    buffer: Buffer,
    cursor: usize,           // Byte position in the buffer
    pub selection_start: Option<usize>,  // Start of selection (if any)
    file_path: Option<PathBuf>,
    modified: bool,
    viewport_offset: (usize, usize),  // (row, col) offset for scrolling
    last_saved_undo_len: usize,       // Track save point for modified flag
    clipboard: Clipboard,             // System clipboard
    mouse_selecting: bool,            // Track if we're actively selecting with mouse
    last_click_time: Option<Instant>, // Track time of last click for double/triple click
    last_click_position: Option<usize>, // Track position of last click
    click_count: usize,               // Track consecutive clicks (1=single, 2=double, 3=triple)
    preferred_column: Option<usize>,  // Preferred column for vertical movement
    syntax: SyntaxHighlighter,       // Syntax highlighting state
    read_only: bool,                  // Whether the file is read-only
    pub status_message: Option<(String, bool)>, // Status bar message (text, is_error)
}

impl Editor {
    /// Normalize text by removing invisible characters and converting line endings/tabs
    fn normalize_text(text: String) -> String {
        text.chars()
            .filter_map(|c| match c {
                // Convert tabs to 4 spaces
                '\t' => Some("    ".to_string()),
                // Remove carriage returns (handled separately for CRLF)
                '\r' => None,
                // Remove zero-width and invisible characters
                '\u{200B}' | // Zero-width space
                '\u{200C}' | // Zero-width non-joiner
                '\u{200D}' | // Zero-width joiner
                '\u{200E}' | // Left-to-right mark
                '\u{200F}' | // Right-to-left mark
                '\u{202A}' | // Left-to-right embedding
                '\u{202B}' | // Right-to-left embedding
                '\u{202C}' | // Pop directional formatting
                '\u{202D}' | // Left-to-right override
                '\u{202E}' | // Right-to-left override
                '\u{2060}' | // Word joiner
                '\u{2061}' | // Function application
                '\u{2062}' | // Invisible times
                '\u{2063}' | // Invisible separator
                '\u{2064}' | // Invisible plus
                '\u{2066}' | // Left-to-right isolate
                '\u{2067}' | // Right-to-left isolate
                '\u{2068}' | // First strong isolate
                '\u{2069}' | // Pop directional isolate
                '\u{206A}' | // Inhibit symmetric swapping
                '\u{206B}' | // Activate symmetric swapping
                '\u{206C}' | // Inhibit Arabic form shaping
                '\u{206D}' | // Activate Arabic form shaping
                '\u{206E}' | // National digit shapes
                '\u{206F}' | // Nominal digit shapes
                '\u{FEFF}' | // Zero-width no-break space (BOM)
                '\u{FFF9}' | // Interlinear annotation anchor
                '\u{FFFA}' | // Interlinear annotation separator
                '\u{FFFB}' | // Interlinear annotation terminator
                '\u{00AD}' | // Soft hyphen
                '\u{034F}' | // Combining grapheme joiner
                '\u{061C}' | // Arabic letter mark
                '\u{115F}' | // Hangul choseong filler
                '\u{1160}' | // Hangul jungseong filler
                '\u{17B4}' | // Khmer vowel inherent AQ
                '\u{17B5}' | // Khmer vowel inherent AA
                '\u{180E}' | // Mongolian vowel separator
                '\u{3164}' | // Hangul filler
                '\u{FFA0}' | // Halfwidth hangul filler
                '\u{FE00}'..='\u{FE0F}' | // Variation selectors
                '\u{E0100}'..='\u{E01EF}' => None, // Variation selectors supplement
                // Keep normal characters
                _ => Some(c.to_string()),
            })
            .collect::<String>()
            // Handle CRLF -> LF conversion after filtering
            .replace("\r\n", "\n")
    }
    
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            cursor: 0,
            selection_start: None,
            file_path: None,
            modified: false,
            viewport_offset: (0, 0),
            last_saved_undo_len: 0,
            clipboard: Clipboard::new().expect("Failed to access clipboard"),
            mouse_selecting: false,
            last_click_time: None,
            last_click_position: None,
            click_count: 0,
            preferred_column: None,
            syntax: SyntaxHighlighter::new(),
            read_only: false,
            status_message: None,
        }
    }
    
    pub fn load_file(&mut self, path: &str) -> io::Result<()> {
        let content = fs::read_to_string(path)?;
        // Normalize: CRLF → LF, tabs → spaces, remove invisible characters
        let content = Self::normalize_text(content);
        self.buffer = Buffer::from_string(content);
        self.file_path = Some(PathBuf::from(path));
        self.cursor = 0;
        self.selection_start = None;
        self.modified = false;
        self.viewport_offset = (0, 0);
        self.last_saved_undo_len = 0;
        self.mouse_selecting = false;
        self.preferred_column = None;
        
        // Check if file is read-only
        self.read_only = self.is_file_read_only(path);
        
        // Initialize syntax highlighting
        self.syntax = SyntaxHighlighter::new();
        let line_count = self.buffer.len_lines();
        
        // For large files, use viewport mode; otherwise init all lines
        if line_count <= 50_000 {
            self.syntax.init_all_lines(line_count);
            self.syntax.process_dirty_lines(|line_index| {
                if line_index < self.buffer.len_lines() {
                    Some(self.buffer.line(line_index).to_string())
                } else {
                    None
                }
            });
        }
        // Large files will initialize viewport on first render
        
        Ok(())
    }
    
    fn is_file_read_only(&self, path: &str) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            
            if let Ok(metadata) = fs::metadata(path) {
                let permissions = metadata.permissions();
                // Check if file is read-only
                permissions.readonly() || (permissions.mode() & 0o200) == 0
            } else {
                false // If we can't get metadata, assume it's writable (will fail on save anyway)
            }
        }
        
        #[cfg(not(unix))]
        {
            if let Ok(metadata) = fs::metadata(path) {
                let permissions = metadata.permissions();
                // On Windows, just check the readonly flag
                permissions.readonly()
            } else {
                false // If we can't get metadata, assume it's writable (will fail on save anyway)
            }
        }
    }
    
    pub fn save(&mut self) -> io::Result<()> {
        if self.read_only {
            self.status_message = Some(("Cannot save: File is read-only".to_string(), true));
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "File is read-only"));
        }
        
        if let Some(ref path) = self.file_path {
            // Create parent directories if they don't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            match fs::write(path, self.buffer.to_string()) {
                Ok(_) => {
                    self.modified = false;
                    self.last_saved_undo_len = 0; // Reset save point
                    self.status_message = None; // Clear any error messages
                    Ok(())
                }
                Err(e) => {
                    self.status_message = Some((format!("Save failed: {}", e), true));
                    Err(e)
                }
            }
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "No file path set"))
        }
    }
    
    pub fn save_as(&mut self, path: PathBuf) -> io::Result<()> {
        // Check if the new path would be read-only
        let new_read_only = self.is_file_read_only(path.to_str().unwrap_or(""));
        if new_read_only {
            self.status_message = Some(("Cannot save: Target location is read-only".to_string(), true));
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Target location is read-only"));
        }
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        match fs::write(&path, self.buffer.to_string()) {
            Ok(_) => {
                self.file_path = Some(path.clone());
                self.modified = false;
                self.last_saved_undo_len = 0; // Reset save point
                self.read_only = new_read_only;
                self.status_message = None; // Clear any error messages
                Ok(())
            }
            Err(e) => {
                self.status_message = Some((format!("Save as failed: {}", e), true));
                Err(e)
            }
        }
    }
    
    /// Get the current selection as (start, end) byte positions
    fn get_selection(&self) -> Option<(usize, usize)> {
        self.selection_start.map(|start| {
            if start < self.cursor {
                (start, self.cursor)
            } else {
                (self.cursor, start)
            }
        })
    }
    
    /// Get selected text
    fn get_selected_text(&self) -> Option<String> {
        self.get_selection().map(|(start, end)| {
            self.buffer.rope().byte_slice(start..end).to_string()
        })
    }
    
    /// Delete the current selection
    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.get_selection() {
            let cursor_before = self.cursor;
            self.buffer.delete(start, end, cursor_before, start);
            self.cursor = start;
            self.selection_start = None;
            self.modified = true;
            true
        } else {
            false
        }
    }
    
    pub fn execute(&mut self, cmd: Command) -> io::Result<()> {
        // Clear mouse selection mode on any keyboard input
        self.mouse_selecting = false;
        
        // Clear error messages on any input (except for save commands)
        if !matches!(cmd, Command::Save | Command::SaveAs) && self.status_message.is_some() {
            if let Some((_, is_error)) = self.status_message {
                if is_error {
                    self.status_message = None;
                }
            }
        }
        
        // Track if cursor moved to update viewport
        let mut cursor_moved = false;
        
        // For non-selection movement commands, clear selection
        // Note: MoveLeft, MoveRight, MoveUp, and MoveDown handle their own selection clearing
        match cmd {
            Command::MoveHome | Command::MoveEnd | Command::PageUp | Command::PageDown |
            Command::MoveWordLeft | Command::MoveWordRight | 
            Command::MoveParagraphUp | Command::MoveParagraphDown => {
                self.selection_start = None;
                cursor_moved = true;
            }
            _ => {}
        }
        
        match cmd {
            Command::InsertChar(c) => {
                // Delete selection first if any
                self.delete_selection();
                
                let cursor_before = self.cursor;
                
                // Filter out invisible characters
                let text = match c {
                    '\t' => "    ".to_string(), // Convert tabs to 4 spaces
                    '\r' => return Ok(()), // Skip carriage returns
                    // Skip zero-width and invisible characters
                    '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{200E}' | '\u{200F}' |
                    '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' |
                    '\u{2066}'..='\u{206F}' | '\u{FEFF}' | '\u{FFF9}'..='\u{FFFB}' |
                    '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{115F}' | '\u{1160}' |
                    '\u{17B4}' | '\u{17B5}' | '\u{180E}' | '\u{3164}' | '\u{FFA0}' |
                    '\u{FE00}'..='\u{FE0F}' => return Ok(()), // Skip invisible chars
                    _ if c >= '\u{E0100}' && c <= '\u{E01EF}' => return Ok(()), // Variation selectors
                    _ => c.to_string(),
                };
                
                self.buffer.insert(self.cursor, &text, cursor_before, self.cursor + text.len());
                self.cursor += text.len();
                self.modified = true;
                self.preferred_column = None; // Clear preferred column
                
                // Update syntax highlighting for the modified line
                let line = self.buffer.byte_to_line(self.cursor);
                self.syntax.line_modified(line);
            }
            
            Command::InsertNewline => {
                // Delete selection first if any
                self.delete_selection();
                
                // Get the current line to check for indentation
                let current_line = self.buffer.byte_to_line(self.cursor);
                let line_start = self.buffer.line_to_byte(current_line);
                let is_at_line_start = self.cursor == line_start;
                
                let mut new_text = String::from("\n");
                
                // Only add indentation if cursor is NOT at the start of the line
                if !is_at_line_start {
                    let line_text = self.buffer.line(current_line);
                    
                    // Count leading spaces
                    let indent_count = line_text.chars()
                        .take_while(|&c| c == ' ')
                        .count();
                    
                    // Add indentation spaces
                    for _ in 0..indent_count {
                        new_text.push(' ');
                    }
                }
                
                let cursor_before = self.cursor;
                self.buffer.insert(self.cursor, &new_text, cursor_before, self.cursor + new_text.len());
                self.cursor += new_text.len();
                self.modified = true;
                self.preferred_column = None; // Clear preferred column
                
                // Update syntax highlighting - line was inserted
                self.syntax.line_modified(current_line); // Mark current line as dirty since its content changed
                self.syntax.lines_inserted(current_line + 1, 1);
            }
            
            Command::InsertTab => {
                // Delete selection first if any
                self.delete_selection();
                
                let cursor_before = self.cursor;
                self.buffer.insert(self.cursor, "    ", cursor_before, self.cursor + 4);
                self.cursor += 4;
                self.modified = true;
                self.preferred_column = None; // Clear preferred column
            }
            
            Command::Backspace => {
                // If there's a selection, delete it
                if !self.delete_selection() {
                    // Otherwise delete character before cursor
                    if self.cursor > 0 {
                        let cursor_before = self.cursor;
                        let line_before = self.buffer.byte_to_line(self.cursor);
                        
                        // Find the previous character boundary
                        let char_pos = self.buffer.byte_to_char(self.cursor);
                        if char_pos > 0 {
                            let prev_char_pos = char_pos - 1;
                            let prev_byte = self.buffer.char_to_byte(prev_char_pos);
                            
                            self.buffer.delete(prev_byte, self.cursor, cursor_before, prev_byte);
                            self.cursor = prev_byte;
                            self.modified = true;
                            
                            // Update syntax - check if we deleted a newline (merged lines)
                            let line_after = self.buffer.byte_to_line(self.cursor);
                            if line_before != line_after {
                                // Lines were merged
                                self.syntax.lines_deleted(line_after, 1);
                            }
                            self.syntax.line_modified(line_after);
                        }
                    }
                }
                self.preferred_column = None; // Clear preferred column
            }
            
            Command::Delete => {
                // If there's a selection, delete it
                if !self.delete_selection() {
                    // Otherwise delete character after cursor
                    if self.cursor < self.buffer.len_bytes() {
                        let cursor_before = self.cursor;
                        
                        // Find the next character boundary
                        let char_pos = self.buffer.byte_to_char(self.cursor);
                        let next_char_pos = char_pos + 1;
                        let next_byte = self.buffer.char_to_byte(next_char_pos);
                        
                        self.buffer.delete(self.cursor, next_byte, cursor_before, self.cursor);
                        self.modified = true;
                    }
                }
                self.preferred_column = None; // Clear preferred column
            }
            
            Command::MoveLeft => {
                // If there's a selection, just move to the start of it
                if let Some((start, _end)) = self.get_selection() {
                    self.cursor = start;
                    self.selection_start = None;
                } else {
                    // Otherwise perform normal left movement
                    if self.cursor > 0 {
                        let char_pos = self.buffer.byte_to_char(self.cursor);
                        if char_pos > 0 {
                            self.cursor = self.buffer.char_to_byte(char_pos - 1);
                        }
                    }
                }
                self.preferred_column = None; // Clear preferred column on horizontal movement
                cursor_moved = true;
            }
            
            Command::MoveRight => {
                // If there's a selection, just move to the end of it
                if let Some((_start, end)) = self.get_selection() {
                    self.cursor = end;
                    self.selection_start = None;
                } else {
                    // Otherwise perform normal right movement
                    if self.cursor < self.buffer.len_bytes() {
                        let char_pos = self.buffer.byte_to_char(self.cursor);
                        self.cursor = self.buffer.char_to_byte(char_pos + 1);
                    }
                }
                self.preferred_column = None; // Clear preferred column on horizontal movement
                cursor_moved = true;
            }
            
            Command::MoveUp => {
                // If there's a selection, move to the start of it
                if let Some((start, _end)) = self.get_selection() {
                    self.cursor = start;
                    self.selection_start = None;
                    self.preferred_column = None; // Clear preferred column when collapsing selection
                    cursor_moved = true;
                } else {
                    let current_line = self.buffer.byte_to_line(self.cursor);
                    if current_line > 0 {
                    // Set preferred column if not already set
                    if self.preferred_column.is_none() {
                        let (_, col) = self.cursor_position();
                        self.preferred_column = Some(col);
                    }
                    
                    // Use preferred column as target
                    let target_display_col = self.preferred_column.unwrap();
                    
                    let new_line = current_line - 1;
                    let new_line_start = self.buffer.line_to_byte(new_line);
                    let new_line_text = self.buffer.line(new_line);
                    
                    // Find the best position on the new line
                    let mut best_byte_pos = 0;
                    let mut current_byte_pos = 0;
                    let mut display_col = 0;
                    
                    for ch in new_line_text.chars() {
                        if ch == '\n' {
                            break; // Stop at newline
                        }
                        
                        let char_width = ch.width().unwrap_or(1);
                        
                        // If adding this character would overshoot, decide whether to include it
                        if display_col + char_width > target_display_col {
                            // Check if we're closer to target by including or excluding this char
                            let without_char_distance = target_display_col - display_col;
                            let with_char_distance = (display_col + char_width) - target_display_col;
                            
                            if with_char_distance < without_char_distance {
                                // Include this character
                                best_byte_pos = current_byte_pos + ch.len_utf8();
                            } else {
                                // Exclude this character
                                best_byte_pos = current_byte_pos;
                            }
                            break;
                        }
                        
                        // Move past this character
                        current_byte_pos += ch.len_utf8();
                        display_col += char_width;
                        best_byte_pos = current_byte_pos;
                        
                        // If we've reached exactly the target column, stop
                        if display_col == target_display_col {
                            break;
                        }
                    }
                    
                    self.cursor = new_line_start + best_byte_pos;
                } else {
                    // Already on first line, move to start of buffer
                    self.cursor = 0;
                    self.preferred_column = Some(0); // Reset preferred column at buffer start
                }
                cursor_moved = true;
                }
            }
            
            Command::MoveDown => {
                // If there's a selection, move to the end of it
                if let Some((_start, end)) = self.get_selection() {
                    self.cursor = end;
                    self.selection_start = None;
                    self.preferred_column = None; // Clear preferred column when collapsing selection
                    cursor_moved = true;
                } else {
                    let current_line = self.buffer.byte_to_line(self.cursor);
                    if current_line < self.buffer.len_lines() - 1 {
                    // Set preferred column if not already set
                    if self.preferred_column.is_none() {
                        let (_, col) = self.cursor_position();
                        self.preferred_column = Some(col);
                    }
                    
                    // Use preferred column as target
                    let target_display_col = self.preferred_column.unwrap();
                    
                    let new_line = current_line + 1;
                    let new_line_start = self.buffer.line_to_byte(new_line);
                    let new_line_text = self.buffer.line(new_line);
                    
                    // Find the best position on the new line
                    let mut best_byte_pos = 0;
                    let mut current_byte_pos = 0;
                    let mut display_col = 0;
                    
                    for ch in new_line_text.chars() {
                        if ch == '\n' {
                            break; // Stop at newline
                        }
                        
                        let char_width = ch.width().unwrap_or(1);
                        
                        // If adding this character would overshoot, decide whether to include it
                        if display_col + char_width > target_display_col {
                            // Check if we're closer to target by including or excluding this char
                            let without_char_distance = target_display_col - display_col;
                            let with_char_distance = (display_col + char_width) - target_display_col;
                            
                            if with_char_distance < without_char_distance {
                                // Include this character
                                best_byte_pos = current_byte_pos + ch.len_utf8();
                            } else {
                                // Exclude this character
                                best_byte_pos = current_byte_pos;
                            }
                            break;
                        }
                        
                        // Move past this character
                        current_byte_pos += ch.len_utf8();
                        display_col += char_width;
                        best_byte_pos = current_byte_pos;
                        
                        // If we've reached exactly the target column, stop
                        if display_col == target_display_col {
                            break;
                        }
                    }
                    
                    self.cursor = new_line_start + best_byte_pos;
                } else {
                    // Already on last line, move to end of buffer
                    self.cursor = self.buffer.len_bytes();
                    // Reset preferred column to end of last line for consistency
                    let (_, col) = self.cursor_position();
                    self.preferred_column = Some(col);
                }
                cursor_moved = true;
                }
            }
            
            Command::MoveHome => {
                let current_line = self.buffer.byte_to_line(self.cursor);
                self.cursor = self.buffer.line_to_byte(current_line);
                self.preferred_column = None; // Clear preferred column
            }
            
            Command::MoveEnd => {
                let current_line = self.buffer.byte_to_line(self.cursor);
                let line_start = self.buffer.line_to_byte(current_line);
                let line = self.buffer.line(current_line);
                let line_len = if line.ends_with('\n') {
                    line.len().saturating_sub(1)
                } else {
                    line.len()
                };
                self.cursor = line_start + line_len;
                self.preferred_column = None; // Clear preferred column
            }
            
            Command::PageUp => {
                // Move up ~20 lines
                for _ in 0..20 {
                    self.execute(Command::MoveUp)?;
                }
            }
            
            Command::PageDown => {
                // Move down ~20 lines
                for _ in 0..20 {
                    self.execute(Command::MoveDown)?;
                }
            }
            
            // Selection movement commands
            Command::SelectLeft => {
                if self.selection_start.is_none() {
                    self.set_selection_start(self.cursor);
                }
                if self.cursor > 0 {
                    let char_pos = self.buffer.byte_to_char(self.cursor);
                    if char_pos > 0 {
                        self.cursor = self.buffer.char_to_byte(char_pos - 1);
                        cursor_moved = true;
                    }
                }
                self.preferred_column = None; // Clear on horizontal movement
            }
            
            Command::SelectRight => {
                if self.selection_start.is_none() {
                    self.set_selection_start(self.cursor);
                }
                if self.cursor < self.buffer.len_bytes() {
                    let char_pos = self.buffer.byte_to_char(self.cursor);
                    self.cursor = self.buffer.char_to_byte(char_pos + 1);
                    cursor_moved = true;
                }
                self.preferred_column = None; // Clear on horizontal movement
            }
            
            Command::SelectUp => {
                if self.selection_start.is_none() {
                    self.set_selection_start(self.cursor);
                }
                let current_line = self.buffer.byte_to_line(self.cursor);
                if current_line > 0 {
                    // Set preferred column if not already set
                    if self.preferred_column.is_none() {
                        let (_, col) = self.cursor_position();
                        self.preferred_column = Some(col);
                    }
                    
                    // Use preferred column as target
                    let target_display_col = self.preferred_column.unwrap();
                    
                    let new_line = current_line - 1;
                    let new_line_start = self.buffer.line_to_byte(new_line);
                    let new_line_text = self.buffer.line(new_line);
                    
                    // Find the best position on the new line
                    let mut best_byte_pos = 0;
                    let mut current_byte_pos = 0;
                    let mut display_col = 0;
                    
                    for ch in new_line_text.chars() {
                        if ch == '\n' {
                            break; // Stop at newline
                        }
                        
                        let char_width = ch.width().unwrap_or(1);
                        
                        // If adding this character would overshoot, decide whether to include it
                        if display_col + char_width > target_display_col {
                            // Check if we're closer to target by including or excluding this char
                            let without_char_distance = target_display_col - display_col;
                            let with_char_distance = (display_col + char_width) - target_display_col;
                            
                            if with_char_distance < without_char_distance {
                                // Include this character
                                best_byte_pos = current_byte_pos + ch.len_utf8();
                            } else {
                                // Exclude this character
                                best_byte_pos = current_byte_pos;
                            }
                            break;
                        }
                        
                        // Move past this character
                        current_byte_pos += ch.len_utf8();
                        display_col += char_width;
                        best_byte_pos = current_byte_pos;
                        
                        // If we've reached exactly the target column, stop
                        if display_col == target_display_col {
                            break;
                        }
                    }
                    
                    self.cursor = new_line_start + best_byte_pos;
                    cursor_moved = true;
                } else {
                    // Already on first line, move to start of buffer
                    self.cursor = 0;
                    self.preferred_column = Some(0); // Reset preferred column at buffer start
                    cursor_moved = true;
                }
            }
            
            Command::SelectDown => {
                if self.selection_start.is_none() {
                    self.set_selection_start(self.cursor);
                }
                let current_line = self.buffer.byte_to_line(self.cursor);
                if current_line < self.buffer.len_lines() - 1 {
                    // Set preferred column if not already set
                    if self.preferred_column.is_none() {
                        let (_, col) = self.cursor_position();
                        self.preferred_column = Some(col);
                    }
                    
                    // Use preferred column as target
                    let target_display_col = self.preferred_column.unwrap();
                    
                    let new_line = current_line + 1;
                    let new_line_start = self.buffer.line_to_byte(new_line);
                    let new_line_text = self.buffer.line(new_line);
                    
                    // Find the best position on the new line
                    let mut best_byte_pos = 0;
                    let mut current_byte_pos = 0;
                    let mut display_col = 0;
                    
                    for ch in new_line_text.chars() {
                        if ch == '\n' {
                            break; // Stop at newline
                        }
                        
                        let char_width = ch.width().unwrap_or(1);
                        
                        // If adding this character would overshoot, decide whether to include it
                        if display_col + char_width > target_display_col {
                            // Check if we're closer to target by including or excluding this char
                            let without_char_distance = target_display_col - display_col;
                            let with_char_distance = (display_col + char_width) - target_display_col;
                            
                            if with_char_distance < without_char_distance {
                                // Include this character
                                best_byte_pos = current_byte_pos + ch.len_utf8();
                            } else {
                                // Exclude this character
                                best_byte_pos = current_byte_pos;
                            }
                            break;
                        }
                        
                        // Move past this character
                        current_byte_pos += ch.len_utf8();
                        display_col += char_width;
                        best_byte_pos = current_byte_pos;
                        
                        // If we've reached exactly the target column, stop
                        if display_col == target_display_col {
                            break;
                        }
                    }
                    
                    self.cursor = new_line_start + best_byte_pos;
                    cursor_moved = true;
                } else {
                    // Already on last line, move to end of buffer
                    self.cursor = self.buffer.len_bytes();
                    // Reset preferred column to end of last line for consistency
                    let (_, col) = self.cursor_position();
                    self.preferred_column = Some(col);
                    cursor_moved = true;
                }
            }
            
            Command::SelectHome => {
                if self.selection_start.is_none() {
                    self.set_selection_start(self.cursor);
                }
                let current_line = self.buffer.byte_to_line(self.cursor);
                self.cursor = self.buffer.line_to_byte(current_line);
                self.preferred_column = None; // Clear preferred column
                cursor_moved = true;
            }
            
            Command::SelectEnd => {
                if self.selection_start.is_none() {
                    self.set_selection_start(self.cursor);
                }
                let current_line = self.buffer.byte_to_line(self.cursor);
                let line_start = self.buffer.line_to_byte(current_line);
                let line = self.buffer.line(current_line);
                let line_len = if line.ends_with('\n') {
                    line.len().saturating_sub(1)
                } else {
                    line.len()
                };
                self.cursor = line_start + line_len;
                self.preferred_column = None; // Clear preferred column
                cursor_moved = true;
            }
            
            Command::SelectAll => {
                self.set_selection_start(0);
                self.cursor = self.buffer.len_bytes();
                self.preferred_column = None; // Clear preferred column
            }
            
            Command::SelectWordLeft => {
                if self.selection_start.is_none() {
                    // First, we need to adjust the anchor position based on current token
                    let current_line = self.buffer.byte_to_line(self.cursor);
                    let line_start = self.buffer.line_to_byte(current_line);
                    let line_text = self.buffer.line(current_line);
                    let cursor_in_line = self.cursor - line_start;
                    
                    // Find the end of the current token
                    let mut anchor_pos = self.cursor;
                    
                    // Build token list to find current token boundaries
                    let mut tokens = Vec::new();
                    let mut byte_pos = 0;
                    let mut current_token_start = 0;
                    let mut last_token_type = None;
                    
                    for ch in line_text.chars() {
                        let token_type = get_token_type(ch);
                        
                        if last_token_type.is_none() || last_token_type != Some(token_type) {
                            if last_token_type.is_some() {
                                tokens.push((current_token_start, byte_pos, last_token_type.unwrap()));
                            }
                            current_token_start = byte_pos;
                            last_token_type = Some(token_type);
                        }
                        
                        byte_pos += ch.len_utf8();
                    }
                    
                    // Don't forget the last token
                    if last_token_type.is_some() {
                        tokens.push((current_token_start, byte_pos, last_token_type.unwrap()));
                    }
                    
                    // Find which token contains the cursor and use its end
                    for &(start, end, _token_type) in &tokens {
                        if cursor_in_line >= start && cursor_in_line < end {
                            anchor_pos = line_start + end;
                            break;
                        }
                    }
                    
                    self.selection_start = Some(anchor_pos);
                }
                
                let current_line = self.buffer.byte_to_line(self.cursor);
                let line_start = self.buffer.line_to_byte(current_line);
                let line_text = self.buffer.line(current_line);
                let cursor_in_line = self.cursor - line_start;
                
                if cursor_in_line > 0 {
                    // Build a list of tokens with their positions
                    let mut tokens = Vec::new();
                    let mut byte_pos = 0;
                    let mut current_token_start = 0;
                    let mut last_token_type = None;
                    
                    for ch in line_text.chars() {
                        let token_type = get_token_type(ch);
                        
                        // Check if we're starting a new token
                        if last_token_type.is_none() || last_token_type != Some(token_type) {
                            if last_token_type.is_some() && last_token_type != Some(TokenType::Space) {
                                tokens.push((current_token_start, byte_pos));
                            }
                            current_token_start = byte_pos;
                            last_token_type = Some(token_type);
                        }
                        
                        byte_pos += ch.len_utf8();
                    }
                    
                    // Don't forget the last token if it's not a space
                    if last_token_type.is_some() && last_token_type != Some(TokenType::Space) {
                        tokens.push((current_token_start, byte_pos));
                    }
                    
                    // Find the token to move to
                    let mut target_pos = 0;
                    for &(start, _end) in &tokens {
                        if start >= cursor_in_line {
                            break;
                        }
                        target_pos = start;
                    }
                    
                    self.cursor = line_start + target_pos;
                } else {
                    // Already at start of line, stay there
                    self.cursor = line_start;
                }
                self.preferred_column = None;
                cursor_moved = true;
            }
            
            Command::SelectWordRight => {
                if self.selection_start.is_none() {
                    // First, we need to adjust the anchor position based on current token
                    let current_line = self.buffer.byte_to_line(self.cursor);
                    let line_start = self.buffer.line_to_byte(current_line);
                    let line_text = self.buffer.line(current_line);
                    let cursor_in_line = self.cursor - line_start;
                    
                    // Find the start of the current token
                    let mut anchor_pos = self.cursor;
                    if cursor_in_line > 0 {
                        let mut byte_pos = 0;
                        let mut current_token_start = 0;
                        let mut current_token_type = None;
                        let mut cursor_token_type = None;
                        let mut cursor_token_start = 0;
                        
                        // First, determine what token the cursor is in
                        for ch in line_text.chars() {
                            let token_type = get_token_type(ch);
                            
                            if byte_pos <= cursor_in_line && byte_pos + ch.len_utf8() > cursor_in_line {
                                cursor_token_type = Some(token_type);
                                cursor_token_start = current_token_start;
                            }
                            
                            if current_token_type != Some(token_type) {
                                current_token_start = byte_pos;
                                current_token_type = Some(token_type);
                            }
                            
                            byte_pos += ch.len_utf8();
                        }
                        
                        // Special handling for spaces
                        if cursor_token_type == Some(TokenType::Space) {
                            // Check if we're in indentation spaces
                            let mut is_indentation = true;
                            let mut check_pos = 0;
                            for ch in line_text.chars() {
                                if check_pos >= cursor_token_start {
                                    break;
                                }
                                if ch != ' ' && ch != '\t' {
                                    is_indentation = false;
                                    break;
                                }
                                check_pos += ch.len_utf8();
                            }
                            
                            if !is_indentation {
                                // For non-indentation spaces, move to end of spaces
                                let mut pos = cursor_token_start;
                                for ch in line_text[cursor_token_start..].chars() {
                                    if get_token_type(ch) != TokenType::Space {
                                        break;
                                    }
                                    pos += ch.len_utf8();
                                }
                                anchor_pos = line_start + pos;
                            } else {
                                // For indentation spaces, use start of token
                                anchor_pos = line_start + cursor_token_start;
                            }
                        } else {
                            // For other tokens, move to beginning of token
                            anchor_pos = line_start + cursor_token_start;
                        }
                    }
                    
                    self.selection_start = Some(anchor_pos);
                }
                
                let current_line = self.buffer.byte_to_line(self.cursor);
                let line_start = self.buffer.line_to_byte(current_line);
                let line_text = self.buffer.line(current_line);
                let cursor_in_line = self.cursor - line_start;
                
                // Remove trailing newline from line text for processing
                let line_without_newline = if line_text.ends_with('\n') {
                    &line_text[..line_text.len() - 1]
                } else {
                    &line_text
                };
                
                if cursor_in_line < line_without_newline.len() {
                    // Build a list of tokens with their positions
                    let mut tokens = Vec::new();
                    let mut byte_pos = 0;
                    let mut current_token_start = 0;
                    let mut last_token_type = None;
                    
                    for ch in line_without_newline.chars() {
                        let token_type = get_token_type(ch);
                        
                        // Check if we're starting a new token
                        if last_token_type.is_none() || last_token_type != Some(token_type) {
                            if last_token_type.is_some() && last_token_type != Some(TokenType::Space) {
                                tokens.push((current_token_start, byte_pos));
                            }
                            current_token_start = byte_pos;
                            last_token_type = Some(token_type);
                        }
                        
                        byte_pos += ch.len_utf8();
                    }
                    
                    // Don't forget the last token if it's not a space
                    if last_token_type.is_some() && last_token_type != Some(TokenType::Space) {
                        tokens.push((current_token_start, byte_pos));
                    }
                    
                    // Find the next token end after cursor position
                    let mut target_pos = line_without_newline.len();
                    for &(_start, end) in &tokens {
                        if end > cursor_in_line {
                            target_pos = end;
                            break;
                        }
                    }
                    
                    self.cursor = line_start + target_pos;
                } else {
                    // Already at end of line, stay there
                    self.cursor = line_start + line_without_newline.len();
                }
                self.preferred_column = None;
                cursor_moved = true;
            }
            
            Command::SelectParagraphUp => {
                if self.selection_start.is_none() {
                    self.set_selection_start(self.cursor);
                }
                let current_line = self.buffer.byte_to_line(self.cursor);
                
                // Search backwards for a non-empty line preceded by an empty line
                let mut target_line = None;
                for line_num in (0..current_line).rev() {
                    let line_text = self.buffer.line(line_num);
                    let is_empty = line_text.is_empty() || line_text == "\n";
                    
                    if !is_empty && line_num > 0 {
                        let prev_line = self.buffer.line(line_num - 1);
                        if prev_line.is_empty() || prev_line == "\n" {
                            target_line = Some(line_num);
                            break;
                        }
                    }
                }
                
                if let Some(line) = target_line {
                    self.cursor = self.buffer.line_to_byte(line);
                } else {
                    // No paragraph found, go to start of file
                    self.cursor = 0;
                }
                self.preferred_column = None;
                cursor_moved = true;
            }
            
            Command::SelectParagraphDown => {
                if self.selection_start.is_none() {
                    self.set_selection_start(self.cursor);
                }
                let current_line = self.buffer.byte_to_line(self.cursor);
                let total_lines = self.buffer.len_lines();
                
                // Search forward for a non-empty line preceded by an empty line
                let mut found_empty = false;
                let mut target_line = None;
                
                for line_num in (current_line + 1)..total_lines {
                    let line_text = self.buffer.line(line_num);
                    let is_empty = line_text.is_empty() || line_text == "\n";
                    
                    if is_empty {
                        found_empty = true;
                    } else if found_empty {
                        // Found a non-empty line after an empty line
                        target_line = Some(line_num);
                        break;
                    }
                }
                
                if let Some(line) = target_line {
                    self.cursor = self.buffer.line_to_byte(line);
                } else {
                    // No paragraph found, go to end of file
                    self.cursor = self.buffer.len_bytes();
                }
                self.preferred_column = None;
                cursor_moved = true;
            }
            
            // Clipboard operations
            Command::Copy => {
                if let Some(text) = self.get_selected_text() {
                    if let Err(e) = self.clipboard.set_text(text) {
                        eprintln!("Failed to copy to clipboard: {}", e);
                    }
                }
            }
            
            Command::Cut => {
                if let Some(text) = self.get_selected_text() {
                    if let Err(e) = self.clipboard.set_text(text) {
                        eprintln!("Failed to copy to clipboard: {}", e);
                    } else {
                        self.delete_selection();
                    }
                }
            }
            
            Command::Paste => {
                match self.clipboard.get_text() {
                    Ok(text) => {
                        // Delete selection first if any
                        self.delete_selection();
                        
                        // Normalize: CRLF → LF, tabs → spaces, remove invisible characters
                        let text = Self::normalize_text(text);
                        
                        let line_before = self.buffer.byte_to_line(self.cursor);
                        let cursor_before = self.cursor;
                        self.buffer.insert(self.cursor, &text, cursor_before, self.cursor + text.len());
                        self.cursor += text.len();
                        self.modified = true;
                        self.preferred_column = None; // Clear preferred column
                        
                        // Update syntax - check if we added newlines
                        let line_after = self.buffer.byte_to_line(self.cursor);
                        if line_after > line_before {
                            let lines_added = line_after - line_before;
                            self.syntax.lines_inserted(line_before + 1, lines_added);
                        }
                        self.syntax.line_modified(line_before);
                    }
                    Err(e) => {
                        eprintln!("Failed to paste from clipboard: {}", e);
                    }
                }
            }
            
            Command::Undo => {
                if let Some(cursor) = self.buffer.undo() {
                    self.cursor = cursor.min(self.buffer.len_bytes());
                    self.modified = self.buffer.can_undo();
                    cursor_moved = true;
                }
            }
            
            Command::Redo => {
                if let Some(cursor) = self.buffer.redo() {
                    self.cursor = cursor.min(self.buffer.len_bytes());
                    self.modified = true;
                    cursor_moved = true;
                }
            }
            
            Command::Save => {
                self.save()?;
            }
            
            Command::SaveAs => {
                // This is handled in main.rs as it needs UI interaction
                return Ok(());
            }
            
            Command::FindReplace | Command::FindNext | Command::FindPrev | 
            Command::Replace | Command::ReplaceAll => {
                // These are handled in main.rs with the find/replace window
                return Ok(());
            }
            
            Command::MoveWordLeft => {
                let current_line = self.buffer.byte_to_line(self.cursor);
                let line_start = self.buffer.line_to_byte(current_line);
                let line_text = self.buffer.line(current_line);
                let cursor_in_line = self.cursor - line_start;
                
                if cursor_in_line > 0 {
                    // Find the previous word boundary within the current line
                    let mut new_pos = 0;
                    let mut in_word = false;
                    let mut byte_pos = 0;
                    
                    for ch in line_text.chars() {
                        if byte_pos >= cursor_in_line {
                            break;
                        }
                        
                        if ch.is_alphanumeric() || ch == '_' {
                            if !in_word {
                                // Start of a new word
                                new_pos = byte_pos;
                                in_word = true;
                            }
                        } else {
                            in_word = false;
                        }
                        
                        byte_pos += ch.len_utf8();
                    }
                    
                    self.cursor = line_start + new_pos;
                } else {
                    // Already at start of line, stay there
                    self.cursor = line_start;
                }
                self.preferred_column = None;
                cursor_moved = true;
            }
            
            Command::MoveWordRight => {
                let current_line = self.buffer.byte_to_line(self.cursor);
                let line_start = self.buffer.line_to_byte(current_line);
                let line_text = self.buffer.line(current_line);
                let cursor_in_line = self.cursor - line_start;
                
                // Remove trailing newline from line text for processing
                let line_without_newline = if line_text.ends_with('\n') {
                    &line_text[..line_text.len() - 1]
                } else {
                    &line_text
                };
                
                if cursor_in_line < line_without_newline.len() {
                    // Find the next word boundary within the current line
                    let mut in_word = false;
                    let mut found_next_word = false;
                    let mut byte_pos = 0;
                    
                    for ch in line_without_newline.chars() {
                        if byte_pos > cursor_in_line && !in_word && (ch.is_alphanumeric() || ch == '_') {
                            // Found start of next word
                            self.cursor = line_start + byte_pos;
                            found_next_word = true;
                            break;
                        }
                        
                        in_word = ch.is_alphanumeric() || ch == '_';
                        byte_pos += ch.len_utf8();
                    }
                    
                    if !found_next_word {
                        // No more words on this line, go to end of line
                        self.cursor = line_start + line_without_newline.len();
                    }
                } else {
                    // Already at end of line, stay there
                    self.cursor = line_start + line_without_newline.len();
                }
                self.preferred_column = None;
                cursor_moved = true;
            }
            
            Command::MoveParagraphUp => {
                let current_line = self.buffer.byte_to_line(self.cursor);
                
                // Search backwards for a non-empty line preceded by an empty line
                let mut target_line = None;
                for line_num in (0..current_line).rev() {
                    let line_text = self.buffer.line(line_num);
                    let is_empty = line_text.is_empty() || line_text == "\n";
                    
                    if !is_empty && line_num > 0 {
                        let prev_line = self.buffer.line(line_num - 1);
                        if prev_line.is_empty() || prev_line == "\n" {
                            target_line = Some(line_num);
                            break;
                        }
                    }
                }
                
                if let Some(line) = target_line {
                    self.cursor = self.buffer.line_to_byte(line);
                } else {
                    // No paragraph found, go to start of file
                    self.cursor = 0;
                }
                self.preferred_column = None;
                cursor_moved = true;
            }
            
            Command::MoveParagraphDown => {
                let current_line = self.buffer.byte_to_line(self.cursor);
                let total_lines = self.buffer.len_lines();
                
                // Search forward for a non-empty line preceded by an empty line
                let mut found_empty = false;
                let mut target_line = None;
                
                for line_num in (current_line + 1)..total_lines {
                    let line_text = self.buffer.line(line_num);
                    let is_empty = line_text.is_empty() || line_text == "\n";
                    
                    if is_empty {
                        found_empty = true;
                    } else if found_empty {
                        // Found a non-empty line after an empty line
                        target_line = Some(line_num);
                        break;
                    }
                }
                
                if let Some(line) = target_line {
                    self.cursor = self.buffer.line_to_byte(line);
                } else {
                    // No paragraph found, go to end of file
                    self.cursor = self.buffer.len_bytes();
                }
                self.preferred_column = None;
                cursor_moved = true;
            }
            
            Command::Indent => {
                // Get the lines to indent
                let (start_line, end_line) = if let Some((sel_start, sel_end)) = self.get_selection() {
                    // Indent all lines in selection
                    let start = self.buffer.byte_to_line(sel_start);
                    let end = self.buffer.byte_to_line(sel_end);
                    (start, end)
                } else {
                    // Indent current line only
                    let line = self.buffer.byte_to_line(self.cursor);
                    (line, line)
                };
                
                // Track cursor adjustment
                let mut cursor_adjustment = 0;
                let mut selection_start_adjustment = 0;
                
                // Process each line from last to first to maintain positions
                for line_num in (start_line..=end_line).rev() {
                    let line_start = self.buffer.line_to_byte(line_num);
                    
                    // Insert 4 spaces at the start of the line
                    let cursor_before = self.cursor;
                    self.buffer.insert(line_start, "    ", cursor_before, cursor_before);
                    
                    // Track adjustments for cursor and selection
                    if self.cursor >= line_start {
                        cursor_adjustment += 4;
                    }
                    if let Some(sel_start) = self.selection_start {
                        if sel_start >= line_start {
                            selection_start_adjustment += 4;
                        }
                    }
                }
                
                // Apply cursor adjustment
                self.cursor += cursor_adjustment;
                if let Some(ref mut sel_start) = self.selection_start {
                    *sel_start += selection_start_adjustment;
                }
                
                self.modified = true;
            }
            
            Command::Dedent => {
                // Get the lines to dedent
                let (start_line, end_line) = if let Some((sel_start, sel_end)) = self.get_selection() {
                    // Dedent all lines in selection
                    let start = self.buffer.byte_to_line(sel_start);
                    let end = self.buffer.byte_to_line(sel_end);
                    (start, end)
                } else {
                    // Dedent current line only
                    let line = self.buffer.byte_to_line(self.cursor);
                    (line, line)
                };
                
                // Store original positions
                let original_cursor = self.cursor;
                let original_selection_start = self.selection_start;
                
                // Track total adjustment needed
                let mut cursor_adjustment = 0;
                let mut selection_adjustment = 0;
                
                // Process lines from last to first so deletions don't affect line positions
                for line_num in (start_line..=end_line).rev() {
                    let line_start = self.buffer.line_to_byte(line_num);
                    let line_text = self.buffer.line(line_num);
                    
                    // Count leading spaces (up to 4)
                    let mut spaces_count = 0;
                    for ch in line_text.chars().take(4) {
                        if ch == ' ' {
                            spaces_count += 1;
                        } else {
                            break;
                        }
                    }
                    
                    if spaces_count > 0 {
                        // Delete the spaces
                        self.buffer.delete(line_start, line_start + spaces_count, 
                                         original_cursor, line_start);
                        
                        // Update adjustments if this deletion affects cursor/selection
                        if line_start < original_cursor {
                            cursor_adjustment += spaces_count;
                        }
                        if let Some(sel) = original_selection_start {
                            if line_start < sel {
                                selection_adjustment += spaces_count;
                            }
                        }
                    }
                }
                
                // Apply adjustments
                self.cursor = original_cursor.saturating_sub(cursor_adjustment);
                if let Some(sel) = original_selection_start {
                    self.set_selection_start(sel.saturating_sub(selection_adjustment));
                }
                
                self.modified = true;
            }
            
            Command::None => {}
        }
        
        // Update viewport if cursor moved (but not for pure viewport scrolling)
        if cursor_moved || matches!(cmd, 
            Command::InsertChar(_) | Command::InsertNewline | Command::InsertTab |
            Command::Indent | Command::Dedent |
            Command::Backspace | Command::Delete | Command::Paste |
            Command::SelectUp | Command::SelectDown | Command::SelectLeft | Command::SelectRight |
            Command::SelectHome | Command::SelectEnd | Command::SelectAll |
            Command::MoveWordLeft | Command::MoveWordRight | 
            Command::MoveParagraphUp | Command::MoveParagraphDown |
            Command::SelectWordLeft | Command::SelectWordRight |
            Command::SelectParagraphUp | Command::SelectParagraphDown
        ) {
            self.update_viewport_for_cursor();
        }
        
        Ok(())
    }
    
    // Getters for the renderer
    
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.get_selection()
    }
    
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    
    pub fn viewport_offset(&self) -> (usize, usize) {
        self.viewport_offset
    }
    
    pub fn set_viewport_offset(&mut self, offset: (usize, usize)) {
        self.viewport_offset = offset;
    }
    
    pub fn is_modified(&self) -> bool {
        self.modified
    }
    
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
    
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }
    
    pub fn file_name(&self) -> &str {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[No Name]")
    }
    
    pub fn set_file_path(&mut self, path: &str) {
        self.file_path = Some(PathBuf::from(path));
    }
    
    /// Get the current file path or the current directory for Save As prompt
    pub fn get_save_as_initial_path(&self) -> String {
        if let Some(ref path) = self.file_path {
            // Use the current file path
            path.to_string_lossy().to_string()
        } else {
            // Use current directory + "untitled.txt"
            if let Ok(cwd) = std::env::current_dir() {
                cwd.join("untitled.txt").to_string_lossy().to_string()
            } else {
                "untitled.txt".to_string()
            }
        }
    }
    
    /// Find all occurrences of a string in the buffer
    pub fn find_all(&self, search_text: &str) -> Vec<(usize, usize)> {
        if search_text.is_empty() {
            return Vec::new();
        }
        
        let mut matches = Vec::new();
        let content = self.buffer.to_string();
        let search_lower = search_text.to_lowercase();
        let content_lower = content.to_lowercase();
        
        let mut start = 0;
        while let Some(pos) = content_lower[start..].find(&search_lower) {
            let match_start = start + pos;
            let match_end = match_start + search_text.len();
            matches.push((match_start, match_end));
            start = match_start + 1; // Move forward by 1 to find overlapping matches
        }
        
        matches
    }
    
    /// Move cursor to a specific position
    pub fn move_cursor_to(&mut self, position: usize) {
        self.cursor = position.min(self.buffer.len_bytes());
        self.selection_start = None;
        self.preferred_column = None; // Clear preferred column
        self.update_viewport_for_cursor();
    }
    
    /// Select a range of text
    pub fn select_range(&mut self, start: usize, end: usize) {
        self.set_selection_start(start);
        self.cursor = end;
        self.preferred_column = None; // Clear preferred column
        self.update_viewport_for_cursor();
    }
    
    /// Replace the currently selected text
    pub fn replace_selection(&mut self, replacement: &str) -> bool {
        if let Some((start, end)) = self.get_selection() {
            let cursor_before = self.cursor;
            self.buffer.delete(start, end, cursor_before, start);
            self.buffer.insert(start, replacement, start, start + replacement.len());
            self.cursor = start + replacement.len();
            self.selection_start = None;
            self.modified = true;
            self.update_viewport_for_cursor();
            true
        } else {
            false
        }
    }
    
    /// Replace text at a specific position
    pub fn replace_at(&mut self, start: usize, end: usize, replacement: &str) {
        let cursor_before = self.cursor;
        self.buffer.delete(start, end, cursor_before, start);
        self.buffer.insert(start, replacement, start, start + replacement.len());
        
        // Adjust cursor if it was after the replacement
        if self.cursor > end {
            let diff = (end - start) as isize - replacement.len() as isize;
            self.cursor = (self.cursor as isize - diff) as usize;
        } else if self.cursor >= start && self.cursor <= end {
            self.cursor = start + replacement.len();
        }
        
        self.modified = true;
        self.update_viewport_for_cursor();
    }
    
    /// Get cursor position as (line, display_column)
    /// The column value accounts for Unicode character widths
    /// Check if syntax highlighting has pending work
    pub fn has_syntax_work(&self) -> bool {
        self.syntax.has_dirty_lines()
    }
    
    /// Direct paste method for bracketed paste support
    pub fn paste_text(&mut self, text: String) {
        // Delete selection first if any
        self.delete_selection();
        
        // Normalize: CRLF → LF, tabs → spaces, remove invisible characters
        let text = Self::normalize_text(text);
        
        let line_before = self.buffer.byte_to_line(self.cursor);
        let cursor_before = self.cursor;
        self.buffer.insert(self.cursor, &text, cursor_before, self.cursor + text.len());
        self.cursor += text.len();
        self.modified = true;
        self.preferred_column = None; // Clear preferred column
        
        // Update syntax - check if we added newlines
        let line_after = self.buffer.byte_to_line(self.cursor);
        if line_after > line_before {
            let lines_added = line_after - line_before;
            self.syntax.lines_inserted(line_before + 1, lines_added);
        }
        self.syntax.line_modified(line_before);
    }
    
    /// Process syntax highlighting updates
    pub fn update_syntax_highlighting(&mut self) {
        // Update viewport for large files
        let line_count = self.buffer.len_lines();
        if line_count > 50_000 {
            // Calculate current viewport from cursor position
            let (cursor_line, _) = self.cursor_position();
            let viewport_height = 50; // Approximate visible lines
            let viewport_start = self.viewport_offset.0;
            let viewport_end = viewport_start + viewport_height;
            
            // Set viewport for syntax highlighter
            self.syntax.set_viewport(viewport_start, viewport_end, line_count);
        }
        
        // Process any pending dirty lines
        self.syntax.process_dirty_lines(|line_index| {
            if line_index < self.buffer.len_lines() {
                Some(self.buffer.line(line_index).to_string())
            } else {
                None
            }
        });
    }
    
    /// Update viewport for syntax highlighting in large files
    pub fn update_syntax_viewport(&mut self, viewport_height: usize) {
        let line_count = self.buffer.len_lines();
        if line_count > 50_000 || self.syntax.is_viewport_mode() {
            // Use actual viewport from renderer
            let viewport_start = self.viewport_offset.0;
            let viewport_end = (viewport_start + viewport_height).min(line_count);
            self.syntax.set_viewport(viewport_start, viewport_end, line_count);
        }
    }
    
    /// Get syntax spans for a line (immutable access)
    pub fn get_syntax_spans(&self, line_index: usize) -> Option<&[crate::syntax::HighlightSpan]> {
        self.syntax.get_line_spans(line_index)
    }
    
    pub fn cursor_position(&self) -> (usize, usize) {
        let line = self.buffer.byte_to_line(self.cursor);
        let line_start = self.buffer.line_to_byte(line);
        
        // Calculate display column by summing character widths
        let line_text = self.buffer.line(line);
        let mut byte_pos = 0;
        let mut display_col = 0;
        
        for ch in line_text.chars() {
            if line_start + byte_pos >= self.cursor {
                break;
            }
            display_col += ch.width().unwrap_or(1);
            byte_pos += ch.len_utf8();
        }
        
        (line, display_col)
    }
    
    /// Get position of a byte offset as (line, display_column)
    fn byte_position_to_display(&self, byte_pos: usize) -> (usize, usize) {
        let line = self.buffer.byte_to_line(byte_pos);
        let line_start = self.buffer.line_to_byte(line);
        
        // Calculate display column by summing character widths
        let line_text = self.buffer.line(line);
        let mut current_byte_pos = 0;
        let mut display_col = 0;
        
        for ch in line_text.chars() {
            if line_start + current_byte_pos >= byte_pos {
                break;
            }
            display_col += ch.width().unwrap_or(1);
            current_byte_pos += ch.len_utf8();
        }
        
        (line, display_col)
    }
    
    /// Update viewport to follow cursor - call this when cursor moves
    pub fn update_viewport_for_cursor(&mut self) {
        // Get terminal size
        if let Ok((width, height)) = crossterm::terminal::size() {
            // Account for status bar
            let viewport_height = (height - 1) as usize;
            let viewport_width = width as usize;
            self.update_viewport(viewport_height, viewport_width);
        }
    }
    
    
    /// Update viewport to follow cursor with scrolloff
    fn update_viewport(&mut self, viewport_height: usize, viewport_width: usize) {
        let scrolloff = 3;
        let (cursor_line, cursor_col) = self.cursor_position();
        
        // Logical line includes the 2 virtual lines before the buffer
        let logical_cursor_line = cursor_line + 3;
        
        // Vertical scrolling
        let cursor_screen_row = logical_cursor_line.saturating_sub(self.viewport_offset.0);
        
        // Upward scrolling: but respect that we can't go above 0
        if cursor_screen_row < scrolloff && self.viewport_offset.0 > 0 {
            let desired_offset = logical_cursor_line.saturating_sub(scrolloff);
            self.viewport_offset.0 = desired_offset.max(0);
        }
        // Downward scrolling
        else if cursor_screen_row >= viewport_height - scrolloff {
            self.viewport_offset.0 = logical_cursor_line + scrolloff - viewport_height;
        }
        
        // Horizontal scrolling - consider both cursor and selection start
        let mut left_col = cursor_col;
        let mut right_col = cursor_col;
        
        // If there's a selection, we need to consider both ends
        if let Some(sel_start) = self.selection_start {
            let (_, sel_col) = self.byte_position_to_display(sel_start);
            left_col = left_col.min(sel_col);
            right_col = right_col.max(sel_col);
        }
        
        // Ensure the leftmost position is visible with scrolloff
        if left_col < self.viewport_offset.1 + scrolloff {
            self.viewport_offset.1 = left_col.saturating_sub(scrolloff);
        }
        // Always check if we need to scroll right for the cursor
        if cursor_col >= self.viewport_offset.1 + viewport_width - scrolloff {
            self.viewport_offset.1 = cursor_col + scrolloff + 1 - viewport_width;
        }
        
    }
    
    /// Convert screen coordinates to buffer position
    pub fn screen_to_buffer_position(&self, screen_col: usize, screen_row: usize) -> Option<usize> {
        // Get terminal height to check if click is on status bar
        if let Ok((_, height)) = crossterm::terminal::size() {
            // Ignore clicks on the status bar (last row)
            if screen_row >= (height - 1) as usize {
                return None;
            }
        }
        
        // Calculate logical line from screen row
        let logical_line = self.viewport_offset.0 + screen_row;
        
        // Virtual lines before the buffer (lines 0 and 1)
        if logical_line < 2 {
            return Some(0); // Click on virtual lines maps to start of buffer
        }
        
        // Map logical line to buffer line
        let buffer_line = logical_line - 2;
        
        // Check if the line exists in the buffer
        if buffer_line >= self.buffer.len_lines() {
            // Click beyond the buffer content
            return Some(self.buffer.len_bytes());
        }
        
        // Get the line content
        let line = self.buffer.line(buffer_line);
        let line_start = self.buffer.line_to_byte(buffer_line);
        
        // Calculate the display column accounting for horizontal scroll
        let target_display_col = screen_col + self.viewport_offset.1;
        
        // Find the byte position for the target display column
        let mut byte_pos = 0;
        let mut display_col = 0;
        
        for ch in line.chars() {
            if display_col >= target_display_col {
                break;
            }
            
            let char_width = ch.width().unwrap_or(1);
            
            // Check if clicking in the middle of a wide character
            if display_col + char_width > target_display_col {
                // Click is within this character, decide based on position
                if target_display_col - display_col < char_width / 2 {
                    // Closer to start of character
                    break;
                } else {
                    // Closer to end of character
                    byte_pos += ch.len_utf8();
                    break;
                }
            }
            
            display_col += char_width;
            byte_pos += ch.len_utf8();
        }
        
        // Don't include the newline in selection
        if line.ends_with('\n') && byte_pos >= line.len() - 1 {
            byte_pos = line.len().saturating_sub(1);
        }
        
        Some(line_start + byte_pos)
    }
    
    /// Start a mouse selection
    pub fn start_mouse_selection(&mut self, position: usize) {
        let now = Instant::now();
        let double_click_time = Duration::from_millis(500);
        
        // Clear preferred column on mouse interaction
        self.preferred_column = None;
        
        // Check for double/triple click
        if let Some(last_time) = self.last_click_time {
            if now.duration_since(last_time) < double_click_time {
                if let Some(last_pos) = self.last_click_position {
                    // Check if clicking near the same position (within 3 characters)
                    let pos_diff = if position > last_pos {
                        position - last_pos
                    } else {
                        last_pos - position
                    };
                    
                    if pos_diff <= 3 {
                        self.click_count += 1;
                        if self.click_count > 3 {
                            self.click_count = 1;
                        }
                    } else {
                        self.click_count = 1;
                    }
                } else {
                    self.click_count = 1;
                }
            } else {
                self.click_count = 1;
            }
        } else {
            self.click_count = 1;
        }
        
        self.last_click_time = Some(now);
        self.last_click_position = Some(position);
        
        match self.click_count {
            2 => {
                // Double click - select word
                self.select_word_at(position);
                self.mouse_selecting = false; // Don't continue selecting on drag
                self.update_viewport_for_cursor();
            }
            3 => {
                // Triple click - select line
                self.select_line_at(position);
                self.mouse_selecting = false; // Don't continue selecting on drag
                self.update_viewport_for_cursor();
            }
            _ => {
                // Single click - start normal selection
                self.cursor = position;
                self.selection_start = None;
                self.mouse_selecting = true;
                self.update_viewport_for_cursor();
            }
        }
    }
    
    /// Skip forward over spaces from a given position to exclude them from selection
    /// Exception: preserve leading indentation spaces at the start of lines
    fn skip_forward_spaces(&self, position: usize) -> usize {
        // Check if we're at the start of a line (including indentation)
        let line = self.buffer.byte_to_line(position);
        let line_start = self.buffer.line_to_byte(line);
        let line_text = self.buffer.line(line);
        
        // Find where the actual content (non-space) begins on this line
        let mut first_non_space = 0;
        for ch in line_text.chars() {
            if ch != ' ' && ch != '\t' {
                break;
            }
            first_non_space += ch.len_utf8();
        }
        
        // If position is within the leading indentation, don't skip spaces
        if position >= line_start && position <= line_start + first_non_space {
            return position;
        }
        
        // Otherwise, skip forward over spaces
        let content = self.buffer.to_string();
        let bytes = content.as_bytes();
        let mut pos = position;
        
        while pos < bytes.len() && bytes[pos] == b' ' {
            pos += 1;
        }
        
        pos
    }

    /// Set selection start with space exclusion
    fn set_selection_start(&mut self, position: usize) {
        self.selection_start = Some(self.skip_forward_spaces(position));
    }

    /// Select the word at the given position
    fn select_word_at(&mut self, position: usize) {
        let content = self.buffer.to_string();
        let chars: Vec<char> = content.chars().collect();
        
        // Convert byte position to char position
        let mut byte_pos = 0;
        let mut char_pos = 0;
        for (i, ch) in chars.iter().enumerate() {
            if byte_pos >= position {
                char_pos = i;
                break;
            }
            byte_pos += ch.len_utf8();
        }
        
        // Find word boundaries
        let mut start_char = char_pos;
        let mut end_char = char_pos;
        
        // Move start backward to beginning of word
        while start_char > 0 && (chars[start_char - 1].is_alphanumeric() || chars[start_char - 1] == '_') {
            start_char -= 1;
        }
        
        // Move end forward to end of word
        while end_char < chars.len() && (chars[end_char].is_alphanumeric() || chars[end_char] == '_') {
            end_char += 1;
        }
        
        // Convert char positions back to byte positions
        let mut start_byte = 0;
        for i in 0..start_char {
            start_byte += chars[i].len_utf8();
        }
        
        let mut end_byte = start_byte;
        for i in start_char..end_char {
            end_byte += chars[i].len_utf8();
        }
        
        // Set selection
        if start_byte < end_byte {
            self.set_selection_start(start_byte);
            self.cursor = end_byte;
        }
    }
    
    /// Select the line at the given position
    fn select_line_at(&mut self, position: usize) {
        let line = self.buffer.byte_to_line(position);
        let line_start = self.buffer.line_to_byte(line);
        let line_text = self.buffer.line(line);
        
        // Don't include the newline in the selection
        let line_end = if line_text.ends_with('\n') {
            line_start + line_text.len() - 1
        } else {
            line_start + line_text.len()
        };
        
        self.set_selection_start(line_start);
        self.cursor = line_end;
    }
    
    /// Update mouse selection while dragging
    pub fn update_mouse_selection(&mut self, position: usize) {
        if self.mouse_selecting {
            if self.selection_start.is_none() {
                // Start selection from the initial cursor position
                self.set_selection_start(self.cursor);
            }
            // Update cursor to current mouse position
            self.cursor = position;
            self.update_viewport_for_cursor();
        }
    }
    
    /// Finish mouse selection
    pub fn finish_mouse_selection(&mut self) {
        self.mouse_selecting = false;
        // If selection start equals cursor, clear the selection
        if let Some(start) = self.selection_start {
            if start == self.cursor {
                self.selection_start = None;
            }
        }
    }
    
    /// Scroll viewport vertically without moving cursor
    pub fn scroll_viewport_vertical(&mut self, lines: i32) {
        if lines > 0 {
            // Scrolling down (viewport moves down, content moves up)
            self.viewport_offset.0 = self.viewport_offset.0.saturating_add(lines as usize);
            // Don't scroll past the end of the buffer
            // We have 2 virtual lines before the buffer and allow some after
            let max_offset = self.buffer.len_lines().saturating_add(2).saturating_add(10);
            if self.viewport_offset.0 > max_offset {
                self.viewport_offset.0 = max_offset;
            }
        } else {
            // Scrolling up (viewport moves up, content moves down)
            self.viewport_offset.0 = self.viewport_offset.0.saturating_sub((-lines) as usize);
        }
    }
    
    /// Scroll viewport horizontally without moving cursor
    pub fn scroll_viewport_horizontal(&mut self, cols: i32) {
        if cols > 0 {
            // Scrolling right (viewport moves right, content moves left)
            self.viewport_offset.1 = self.viewport_offset.1.saturating_add(cols as usize);
            
            // Find the longest line to limit scrolling
            let mut max_line_width = 0;
            for i in 0..self.buffer.len_lines() {
                let line = self.buffer.line(i);
                let mut line_width = 0;
                for ch in line.chars() {
                    if ch == '\n' {
                        break;
                    }
                    line_width += ch.width().unwrap_or(1);
                }
                max_line_width = max_line_width.max(line_width);
            }
            
            // Limit horizontal scrolling to the longest line + some padding
            let max_offset = max_line_width.saturating_add(20);
            if self.viewport_offset.1 > max_offset {
                self.viewport_offset.1 = max_offset;
            }
        } else {
            // Scrolling left (viewport moves left, content moves right)
            self.viewport_offset.1 = self.viewport_offset.1.saturating_sub((-cols) as usize);
        }
    }
    
    /// Clear the status message
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }
}