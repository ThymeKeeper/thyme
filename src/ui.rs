// src/ui.rs

use crate::{buffer::Buffer, config::Config, editor::Editor, syntax::TokenType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
        // Get terminal size and calculate content width consistently
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
            .split(f.size());

        // Main editor area
        self.draw_editor(f, chunks[0], editor, config);

        // Status line
        self.draw_status_line(f, chunks[1], editor, config);
    }

    fn draw_editor(&self, f: &mut ratatui::Frame, area: Rect, editor: &Editor, config: &Config) {
        if let Some(buffer) = editor.current_buffer() {
            let editor_area = area.inner(&Margin {
                horizontal: config.margins.horizontal,
                vertical: config.margins.vertical,
            });

            let content_width = self.get_content_width(config);
            let content_height = editor_area.height.saturating_sub(2) as usize; // Account for borders

            // Get wrapped lines and cursor position
            let (wrapped_lines, cursor_visual_pos) = self.prepare_wrapped_content(
                buffer, editor, config, content_width, content_height
            );

            // Convert to ratatui Lines
            let lines: Vec<Line> = wrapped_lines.iter().map(|wl| {
                self.apply_syntax_highlighting(wl.content.clone(), buffer, wl.logical_line)
            }).collect();

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Editor"));

            f.render_widget(paragraph, editor_area);

            // Draw cursor at calculated position
            if let Some((cursor_x, cursor_y)) = cursor_visual_pos {
                let screen_x = editor_area.x + 1 + cursor_x as u16;
                let screen_y = editor_area.y + 1 + cursor_y as u16;
                
                if screen_x < editor_area.x + editor_area.width.saturating_sub(1) && 
                   screen_y < editor_area.y + editor_area.height.saturating_sub(1) {
                    f.set_cursor(screen_x, screen_y);
                }
            }
        } else {
            let welcome = Paragraph::new("Welcome to TUI Editor\nPress Ctrl+O to open a file")
                .block(Block::default().borders(Borders::ALL).title("Editor"));
            f.render_widget(welcome, area);
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

            for (segment_idx, (wrapped_content, start_col)) in line_wrapped.iter().enumerate() {
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
        // Use the standalone function to avoid borrowing issues
        wrap_line_simple(text, width)
    }

    fn apply_syntax_highlighting(&self, text: String, buffer: &Buffer, line_idx: usize) -> Line<'static> {
        if let Some(tokens) = buffer.syntax_highlighter.get_line_tokens(line_idx) {
            let mut spans = Vec::new();
            let mut last_end = 0;

            for token in tokens {
                // Add unstyled text before token
                if token.start > last_end {
                    spans.push(Span::raw(text[last_end..token.start].to_string()));
                }

                // Add styled token
                let token_text = &text[token.start..token.end];
                let style = self.get_token_style(&token.token_type);
                spans.push(Span::styled(token_text.to_string(), style));

                last_end = token.end;
            }

            // Add remaining unstyled text
            if last_end < text.len() {
                spans.push(Span::raw(text[last_end..].to_string()));
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
            TokenType::Identifier => Style::default().fg(Color::White),
            TokenType::Type => Style::default().fg(Color::Magenta),
            TokenType::Function => Style::default().fg(Color::Blue),
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

            // Language
            status_text.push_str(&format!("[{}] ", buffer.language));
        }

        // Editor settings
        if config.word_wrap {
            status_text.push_str("WRAP ");
        }

        status_text.push_str(&format!("M:{}x{}", config.margins.horizontal, config.margins.vertical));

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
