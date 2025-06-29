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
            self.draw_language_selection_modal(f, editor, config);
        }
        
        // Draw theme selection modal if active
        if editor.theme_selection_mode {
            self.draw_theme_selection_modal(f, editor, config);
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
                self.apply_syntax_highlighting_wrapped(
                    wl.content.clone(), 
                    buffer, 
                    wl.logical_line, 
                    wl.line_start_col,
                    config
                )
            }).collect();

            // Simple title based on language
            let display_name = Buffer::get_language_display_name(&buffer.language);
            let title = format!("Thyme Editor [{}]", display_name);

            let border_color = config.theme.parse_color(&config.theme.colors.border);
            let bg_color = config.theme.parse_color(&config.theme.colors.background);
            let fg_color = config.theme.parse_color(&config.theme.colors.foreground);

            let paragraph = Paragraph::new(lines)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(title))
                .style(Style::default().bg(bg_color).fg(fg_color));

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
            let border_color = config.theme.parse_color(&config.theme.colors.border);
            let bg_color = config.theme.parse_color(&config.theme.colors.background);
            let fg_color = config.theme.parse_color(&config.theme.colors.foreground);

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
                Line::from("• Customizable color themes"),
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
                Line::from("• Ctrl+T: Change color theme"),
                Line::from("• Ctrl+S: Save"),
                Line::from("• Ctrl+Q: Quit"),
            ])
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title("Thyme Editor"))
            .style(Style::default().bg(bg_color).fg(fg_color));
            
            f.render_widget(welcome, area);
        }
    }

    // Draw language selection modal
    fn draw_language_selection_modal(&self, f: &mut ratatui::Frame, editor: &Editor, config: &Config) {
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

            let modal_bg = config.theme.parse_color(&config.theme.colors.modal_bg);
            let modal_fg = config.theme.parse_color(&config.theme.colors.modal_fg);
            let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
            let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
            let border_color = config.theme.parse_color(&config.theme.colors.border_active);

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
                                .bg(selection_bg)
                                .fg(selection_fg)
                                .add_modifier(Modifier::BOLD)
                        )
                    } else {
                        ListItem::new(text).style(Style::default().fg(modal_fg))
                    }
                })
                .collect();

            // Create the list widget
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .title("Select Language (↑↓ to navigate, Enter to select, Esc to cancel)")
                        .style(Style::default().bg(modal_bg))
                )
                .style(Style::default().fg(modal_fg));

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

    // Draw theme selection modal
    fn draw_theme_selection_modal(&self, f: &mut ratatui::Frame, editor: &Editor, config: &Config) {
        if let Some((themes, selected_index)) = editor.get_theme_selection_info() {
            // Calculate modal size and position
            let modal_width = 60;
            let modal_height = (themes.len() as u16).min(15) + 4; // +4 for borders and title, max 15 items visible
            
            let area = f.area();
            let modal_area = Rect {
                x: (area.width.saturating_sub(modal_width)) / 2,
                y: (area.height.saturating_sub(modal_height)) / 2,
                width: modal_width,
                height: modal_height,
            };

            // Clear the background
            f.render_widget(Clear, modal_area);

            let modal_bg = config.theme.parse_color(&config.theme.colors.modal_bg);
            let modal_fg = config.theme.parse_color(&config.theme.colors.modal_fg);
            let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
            let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
            let border_color = config.theme.parse_color(&config.theme.colors.border_active);

            // Create theme list items with numbering
            let items: Vec<ListItem> = themes
                .iter()
                .enumerate()
                .map(|(i, (_, display_name))| {
                    let number = i + 1;
                    let text = format!("{}. {}", number, display_name);
                    
                    if i == selected_index {
                        ListItem::new(text).style(
                            Style::default()
                                .bg(selection_bg)
                                .fg(selection_fg)
                                .add_modifier(Modifier::BOLD)
                        )
                    } else {
                        ListItem::new(text).style(Style::default().fg(modal_fg))
                    }
                })
                .collect();

            // Create the list widget
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .title("Select Theme (↑↓ to navigate, Enter to select, Esc to cancel)")
                        .style(Style::default().bg(modal_bg))
                )
                .style(Style::default().fg(modal_fg));

            f.render_widget(list, modal_area);

            // Add instruction text at the bottom of the modal
            let instruction_area = Rect {
                x: modal_area.x + 1,
                y: modal_area.y + modal_area.height - 2,
                width: modal_area.width - 2,
                height: 1,
            };

            let current_theme = &config.theme.name;
            let instruction = Paragraph::new(
                format!("Current: {} | Press 1-{} for quick select", current_theme, themes.len().min(9))
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

    fn apply_syntax_highlighting_wrapped(
        &self, 
        text: String, 
        buffer: &Buffer, 
        line_idx: usize, 
        segment_start: usize,
        config: &Config
    ) -> Line<'static> {
        if let Some(tokens) = buffer.syntax_highlighter.get_line_tokens(line_idx) {
            let mut spans = Vec::new();
            let mut last_end = 0;
            let text_chars: Vec<char> = text.chars().collect();
            let text_len = text_chars.len();
            let segment_end = segment_start + text_len;

            for token in tokens {
                // Skip tokens that are entirely outside this segment
                if token.end <= segment_start || token.start >= segment_end {
                    continue;
                }

                // Adjust token positions relative to this segment
                let token_start_in_segment = if token.start >= segment_start {
                    token.start - segment_start
                } else {
                    0
                };
                
                let token_end_in_segment = if token.end <= segment_end {
                    token.end - segment_start
                } else {
                    text_len
                };

                // Skip if adjusted positions are invalid
                if token_start_in_segment >= token_end_in_segment || token_start_in_segment >= text_len {
                    continue;
                }

                // Add unstyled text before token
                if token_start_in_segment > last_end && last_end < text_len {
                    let slice_start = last_end;
                    let slice_end = token_start_in_segment.min(text_len);
                    
                    if slice_start < text_chars.len() && slice_end <= text_chars.len() {
                        let text_slice: String = text_chars[slice_start..slice_end].iter().collect();
                        let normal_color = config.theme.parse_color(&config.theme.colors.normal);
                        spans.push(Span::styled(text_slice, Style::default().fg(normal_color)));
                    }
                }

                // Add styled token (with additional bounds checking)
                if token_start_in_segment < text_chars.len() && 
                   token_end_in_segment <= text_chars.len() && 
                   token_start_in_segment < token_end_in_segment {
                    let token_text: String = text_chars[token_start_in_segment..token_end_in_segment].iter().collect();
                    let style = self.get_token_style(&token.token_type, config);
                    spans.push(Span::styled(token_text, style));
                    last_end = token_end_in_segment;
                }
            }

            // Add remaining unstyled text
            if last_end < text_len {
                if last_end < text_chars.len() {
                    let remaining_text: String = text_chars[last_end..].iter().collect();
                    let normal_color = config.theme.parse_color(&config.theme.colors.normal);
                    spans.push(Span::styled(remaining_text, Style::default().fg(normal_color)));
                }
            }

            // If no spans were added, return the entire text as normal
            if spans.is_empty() {
                let normal_color = config.theme.parse_color(&config.theme.colors.normal);
                Line::styled(text, Style::default().fg(normal_color))
            } else {
                Line::from(spans)
            }
        } else {
            let normal_color = config.theme.parse_color(&config.theme.colors.normal);
            Line::styled(text, Style::default().fg(normal_color))
        }
    }

    fn get_token_style(&self, token_type: &TokenType, config: &Config) -> Style {
        let color = match token_type {
            TokenType::Keyword => config.theme.parse_color(&config.theme.colors.keyword),
            TokenType::String => config.theme.parse_color(&config.theme.colors.string),
            TokenType::Comment => config.theme.parse_color(&config.theme.colors.comment),
            TokenType::Number => config.theme.parse_color(&config.theme.colors.number),
            TokenType::Operator => config.theme.parse_color(&config.theme.colors.operator),
            TokenType::Identifier => config.theme.parse_color(&config.theme.colors.identifier),
            TokenType::Type => config.theme.parse_color(&config.theme.colors.type_),
            TokenType::Function => config.theme.parse_color(&config.theme.colors.function),
            TokenType::Variable => config.theme.parse_color(&config.theme.colors.variable),
            TokenType::Property => config.theme.parse_color(&config.theme.colors.property),
            TokenType::Parameter => config.theme.parse_color(&config.theme.colors.parameter),
            TokenType::Constant => config.theme.parse_color(&config.theme.colors.constant),
            TokenType::Namespace => config.theme.parse_color(&config.theme.colors.namespace),
            TokenType::Punctuation => config.theme.parse_color(&config.theme.colors.punctuation),
            TokenType::Tag => config.theme.parse_color(&config.theme.colors.tag),
            TokenType::Attribute => config.theme.parse_color(&config.theme.colors.attribute),
            TokenType::Normal => config.theme.parse_color(&config.theme.colors.normal),
        };

        // Add modifiers based on token type
        let style = Style::default().fg(color);
        
        match token_type {
            TokenType::Keyword | TokenType::Constant => style.add_modifier(Modifier::BOLD),
            TokenType::Comment => style.add_modifier(Modifier::ITALIC),
            TokenType::Tag => style.add_modifier(Modifier::BOLD),
            _ => style,
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

        status_text.push_str(&format!("M:{}x{} ", config.margins.horizontal, config.margins.vertical));

        // Theme name
        status_text.push_str(&format!("Theme: {} ", config.theme.name));

        // Language selection hint
        if editor.language_selection_mode {
            status_text.push_str("| LANGUAGE SELECTION MODE");
        } else if editor.theme_selection_mode {
            status_text.push_str("| THEME SELECTION MODE");
        } else {
            status_text.push_str("| Ctrl+L: Language | Ctrl+T: Theme");
        }

        let status_bg = config.theme.parse_color(&config.theme.colors.status_bar_bg);
        let status_fg = config.theme.parse_color(&config.theme.colors.status_bar_fg);

        let status = Paragraph::new(status_text)
            .style(Style::default().bg(status_bg).fg(status_fg));

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
