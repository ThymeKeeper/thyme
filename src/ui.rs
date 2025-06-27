// src/ui.rs

use crate::{buffer::Buffer, config::Config, editor::Editor, syntax::TokenType};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub struct Ui;

#[derive(Debug)]
struct WrappedLine {
    content: String,
    logical_line: usize,
    line_start_col: usize,
    line_end_col: usize,
}

impl Ui {
    pub fn new() -> Self {
        Self
    }

    pub fn get_content_width(&self, config: &Config) -> usize {
        let terminal_width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
        
        terminal_width
            .saturating_sub(2) // outer layout margins
            .saturating_sub((config.margins.horizontal * 2) as usize) // editor margins
            .saturating_sub(2) // editor borders
    }

    pub fn draw(&self, f: &mut ratatui::Frame, editor: &Editor, config: &Config) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
            .split(f.area());

        // Main editor area
        self.draw_editor(f, chunks[0], editor, config);

        // Status line
        self.draw_status_line(f, chunks[1], editor, config);

        // Draw language selection modal if active
        if editor.language_selection_mode {
            self.draw_language_selection_modal(f, editor);
        }
    }

    fn draw_editor(&self, f: &mut ratatui::Frame, area: Rect, editor: &Editor, config: &Config) {
        if let Some(buffer) = editor.current_buffer() {
            let editor_area = area.inner(Margin {
                horizontal: config.margins.horizontal,
                vertical: config.margins.vertical,
            });

            let content_width = self.get_content_width(config);
            let content_height = editor_area.height.saturating_sub(2) as usize; // Account for borders

            // Get wrapped lines and cursor position
            let (wrapped_lines, cursor_visual_pos) = self.prepare_wrapped_content(
                buffer, editor, config, content_width, content_height
            );

            // Convert to ratatui Lines with tree-sitter syntax highlighting
            let lines: Vec<Line> = wrapped_lines.iter().map(|wl| {
                self.apply_syntax_highlighting(wl.content.clone(), buffer, wl.logical_line)
            }).collect();

            // Simple title based on language
            let display_name = Buffer::get_language_display_name(&buffer.language);
            let title = format!("Thyme Editor [{}]", display_name);

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(title));

            f.render_widget(paragraph, editor_area);

            // Draw cursor at calculated position
            if let Some((cursor_x, cursor_y)) = cursor_visual_pos {
                let screen_x = editor_area.x + 1 + cursor_x as u16;
                let screen_y = editor_area.y + 1 + cursor_y as u16;
                
                if screen_x < editor_area.x + editor_area.width.saturating_sub(1) && 
                   screen_y < editor_area.y + editor_area.height.saturating_sub(1) {
                    f.set_cursor_position((screen_x, screen_y));
                }
            }
        } else {
            let welcome = Paragraph::new(vec![
                Line::from("Welcome to Thyme Editor"),
                Line::from(""),
                Line::from("Press Ctrl+O to open a file"),
                Line::from(""),
                Line::from("Supported languages with Tree-sitter syntax highlighting:"),
                Line::from("• Rust (.rs)"),
                Line::from("• Python (.py)"),
                Line::from("• JavaScript/TypeScript (.js, .jsx, .ts, .tsx)"),
                Line::from("• Bash (.sh, .bash)"),
                Line::from("• JSON (.json)"),
                Line::from("• SQL (.sql, .mysql, .pgsql, .sqlite)"),
                Line::from("• TOML (.toml)"),
                Line::from(""),
                Line::from("Features:"),
                Line::from("• Tree-sitter syntax highlighting for 7 languages"),
                Line::from("• Word wrapping with proper cursor handling"),
                Line::from("• Auto-save functionality"),
                Line::from("• Configurable margins and keybindings"),
                Line::from("• Language switching without file reload"),
                Line::from(""),
                Line::from("Keybindings:"),
                Line::from("• F1/F2: Adjust vertical margins"),
                Line::from("• F3/F4: Adjust horizontal margins"),
                Line::from("• F5: Toggle word wrap"),
                Line::from("• Ctrl+L: Change syntax highlighting language"),
                Line::from("• Ctrl+S: Save"),
                Line::from("• Ctrl+Q: Quit"),
            ])
            .block(Block::default().borders(Borders::ALL).title("Thyme Editor"));
            
            f.render_widget(welcome, area);
        }
    }

    // Draw language selection modal
    fn draw_language_selection_modal(&self, f: &mut ratatui::Frame, editor: &Editor) {
        if let Some((languages, selected_index)) = editor.get_language_selection_info() {
            // Calculate modal size and position
            let modal_width = 50;
            let modal_height = languages.len() as u16 + 4; // +4 for borders and title
            
            let area = f.area();
            let modal_area = Rect {
                x: (area.width.saturating_sub(modal_width)) / 2,
                y: (area.height.saturating_sub(modal_height)) / 2,
                width: modal_width,
                height: modal_height,
            };

            // Clear the background
            f.render_widget(Clear, modal_area);

            // Create language list items with numbering and tree-sitter version info
            let items: Vec<ListItem> = languages
                .iter()
                .enumerate()
                .map(|(i, &lang)| {
                    let display_name = Buffer::get_language_display_name(lang);
                    let number = i + 1;
                    
                    // Add tree-sitter version indication
                    let ts_version = match lang {
                        "text" => " (no highlighting)",
                        _ => " (tree-sitter)",
                    };
                    
                    let text = format!("{}. {}{}", number, display_name, ts_version);
                    
                    if i == selected_index {
                        ListItem::new(text).style(
                            Style::default()
                                .bg(Color::Blue)
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD)
                        )
                    } else {
                        ListItem::new(text)
                    }
                })
                .collect();

            // Create the list widget
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Select Language (↑↓ to navigate, Enter to select, Esc to cancel)")
                        .style(Style::default().bg(Color::Black))
                )
                .style(Style::default().fg(Color::White));

            f.render_widget(list, modal_area);

            // Add instruction text at the bottom of the modal
            let instruction_area = Rect {
                x: modal_area.x + 1,
                y: modal_area.y + modal_area.height - 2,
                width: modal_area.width - 2,
                height: 1,
            };

            let current_lang = editor.current_buffer()
                .map(|b| b.language.as_str())
                .unwrap_or("text");
            
            let current_display = Buffer::get_language_display_name(current_lang);
            let instruction = Paragraph::new(
                format!("Current: {} | Press 1-{} for quick select", current_display, languages.len())
            )
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

            f.render_widget(instruction, instruction_area);
        }
    }

    fn prepare_wrapped_content(
        &self,
        buffer: &Buffer,
        editor: &Editor,
        config: &Config,
        content_width: usize,
        content_height: usize,
    ) -> (Vec<WrappedLine>, Option<(usize, usize)>) {
        let mut wrapped_lines = Vec::new();
        let mut cursor_visual_pos = None;

        let start_line = editor.viewport_line;
        let end_line = buffer.rope.len_lines();

        for logical_line in start_line..end_line {
            let line_text = buffer.get_line_text(logical_line);
            // For display purposes, remove the trailing newline
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let line_wrapped = if config.word_wrap {
                self.wrap_line(line_text_for_display, content_width)
            } else {
                // No wrapping - just use the line as-is
                vec![(line_text_for_display.to_string(), 0)]
            };

            for (_segment_idx, (wrapped_content, start_col)) in line_wrapped.iter().enumerate() {
                let end_col = start_col + wrapped_content.chars().count();
                
                wrapped_lines.push(WrappedLine {
                    content: wrapped_content.clone(),
                    logical_line,
                    line_start_col: *start_col,
                    line_end_col: end_col,
                });

                // Check if cursor is in this wrapped segment
                if logical_line == buffer.cursor.line {
                    let cursor_col = buffer.cursor.column;
                    let visual_line_idx = wrapped_lines.len() - 1;
                    
                    // Check if cursor falls within this segment's range (including end position)
                    if cursor_col >= *start_col && cursor_col <= end_col {
                        let visual_col = cursor_col - start_col;
                        cursor_visual_pos = Some((visual_col, visual_line_idx));
                    }
                }

                // Stop if we've filled the visible area
                if wrapped_lines.len() >= content_height {
                    return (wrapped_lines, cursor_visual_pos);
                }
            }
        }

        // Handle cursor at end of file or beyond the currently displayed content
        if cursor_visual_pos.is_none() && !wrapped_lines.is_empty() {
            let cursor_line = buffer.cursor.line;
            
            // If cursor is at or beyond the last displayed line, show it at the end of the last line
            if cursor_line >= end_line || cursor_line >= start_line + wrapped_lines.len() {
                let last_visual_line = wrapped_lines.len() - 1;
                let last_line_content = &wrapped_lines[last_visual_line].content;
                cursor_visual_pos = Some((last_line_content.chars().count(), last_visual_line));
            }
        }

        (wrapped_lines, cursor_visual_pos)
    }

    fn wrap_line(&self, text: &str, width: usize) -> Vec<(String, usize)> {
        wrap_line_simple(text, width)
    }

    fn apply_syntax_highlighting(&self, text: String, buffer: &Buffer, line_idx: usize) -> Line<'static> {
        if let Some(tokens) = buffer.syntax_highlighter.get_line_tokens(line_idx) {
            let mut spans = Vec::new();
            let mut last_end = 0;
            let text_len = text.len();

            for token in tokens {
                // Ensure token positions are within bounds
                let token_start = token.start.min(text_len);
                let token_end = token.end.min(text_len);
                
                // Skip invalid tokens
                if token_start >= token_end || token_start > text_len {
                    continue;
                }

                // Add unstyled text before token
                if token_start > last_end && last_end < text_len {
                    let slice_end = token_start.min(text_len);
                    if let Some(text_slice) = text.get(last_end..slice_end) {
                        spans.push(Span::raw(text_slice.to_string()));
                    }
                }

                // Add styled token with bounds checking
                if let Some(token_text) = text.get(token_start..token_end) {
                    let style = self.get_token_style(&token.token_type);
                    spans.push(Span::styled(token_text.to_string(), style));
                    last_end = token_end;
                } else {
                    // If token is out of bounds, skip it
                    continue;
                }
            }

            // Add remaining unstyled text
            if last_end < text_len {
                if let Some(remaining_text) = text.get(last_end..) {
                    spans.push(Span::raw(remaining_text.to_string()));
                }
            }

            Line::from(spans)
        } else {
            Line::from(text)
        }
    }

    fn get_token_style(&self, token_type: &TokenType) -> Style {
        match token_type {
            TokenType::Keyword => Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            TokenType::String => Style::default().fg(Color::Green),
            TokenType::Comment => Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
            TokenType::Number => Style::default().fg(Color::Cyan),
            TokenType::Operator => Style::default().fg(Color::Yellow),
            TokenType::Identifier | TokenType::Variable => Style::default().fg(Color::White),
            TokenType::Type => Style::default().fg(Color::Magenta),
            TokenType::Function => Style::default().fg(Color::Blue),
            TokenType::Property => Style::default().fg(Color::Cyan),
            TokenType::Parameter => Style::default().fg(Color::LightBlue),
            TokenType::Constant => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            TokenType::Namespace => Style::default().fg(Color::Magenta),
            TokenType::Punctuation => Style::default().fg(Color::Gray),
            TokenType::Tag => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            TokenType::Attribute => Style::default().fg(Color::Yellow),
            TokenType::Normal => Style::default(),
        }
    }

    fn draw_status_line(&self, f: &mut ratatui::Frame, area: Rect, editor: &Editor, config: &Config) {
        let mut status_text = String::new();

        if let Some(buffer) = editor.current_buffer() {
            // File info
            let file_name = buffer.file_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("[No Name]");
            
            status_text.push_str(&format!("{} ", file_name));

            if buffer.dirty {
                status_text.push_str("[+] ");
            }

            // Cursor position
            status_text.push_str(&format!("{}:{} ", buffer.cursor.line + 1, buffer.cursor.column + 1));

            // Language with tree-sitter indicator
            let display_name = Buffer::get_language_display_name(&buffer.language);
            let ts_indicator = match buffer.language.as_str() {
                "text" => "TXT",          // No highlighting
                _ => "TS",                // Tree-sitter supported
            };
            status_text.push_str(&format!("[{}|{}] ", display_name, ts_indicator));
        }

        // Editor settings
        if config.word_wrap {
            status_text.push_str("WRAP ");
        }

        status_text.push_str(&format!("M:{}x{}", config.margins.horizontal, config.margins.vertical));

        // Language selection hint
        if editor.language_selection_mode {
            status_text.push_str(" | LANGUAGE SELECTION MODE");
        } else {
            status_text.push_str(" | Ctrl+L: Change Language");
        }

        let status = Paragraph::new(status_text)
            .style(Style::default().bg(Color::Blue).fg(Color::White));

        f.render_widget(status, area);
    }
}

// Standalone function to avoid borrowing issues
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
