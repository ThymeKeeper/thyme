// src/ui.rs

use crate::{
    buffer::Buffer, 
    config::{Config, GutterMode}, 
    cursor::Position,
    editor::Editor, 
    syntax::TokenType,
    text_utils::wrap_line
};
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
            .saturating_sub((config.margins.horizontal * 2) as usize) // editor margins only
            // No outer layout margin or border subtraction
    }

    pub fn draw(&self, f: &mut ratatui::Frame, editor: &Editor, config: &Config) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)  // Remove outer margin to allow editor to reach edges
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
        
        // Draw help modal if active
        if editor.help_mode {
            self.draw_help_modal(f, config);
        }
    }

    fn draw_editor(&self, f: &mut ratatui::Frame, area: Rect, editor: &Editor, config: &Config) {
        if let Some(buffer) = editor.current_buffer() {
            let editor_area = area.inner(Margin {
                horizontal: config.margins.horizontal,
                vertical: config.margins.vertical,
            });

            // Calculate gutter width based on mode
            let gutter_width = self.calculate_gutter_width(buffer, config.gutter);
            
            // Split the editor area into gutter and content areas
            let (gutter_area, content_area) = if gutter_width > 0 {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(gutter_width as u16),
                        Constraint::Min(0),
                    ])
                    .split(editor_area);
                (Some(chunks[0]), chunks[1])
            } else {
                (None, editor_area)
            };

            let content_width = content_area.width as usize;
            let content_height = content_area.height as usize;

            // Get wrapped lines and cursor position
            let (wrapped_lines, cursor_visual_pos) = self.prepare_wrapped_content(
                buffer, editor, config, content_width, content_height
            );

            // Convert to ratatui Lines with syntax highlighting
let lines: Vec<Line> = wrapped_lines.iter().map(|wl| {
                let mut line = self.apply_syntax_highlighting_wrapped(
                    wl.content.clone(), 
                    buffer, 
                    wl.logical_line, 
                    wl.line_start_col,
                    config
                );
                if wl.logical_line == usize::MAX {
                    let virtual_color = config.theme.parse_color(&config.theme.colors.virtual_line);
                    line = line.style(Style::default().fg(virtual_color));
                }
                line
            }).collect();

            let bg_color = config.theme.parse_color(&config.theme.colors.background);
            let fg_color = config.theme.parse_color(&config.theme.colors.foreground);

            let paragraph = Paragraph::new(lines)
                .style(Style::default().bg(bg_color).fg(fg_color));

            f.render_widget(paragraph, content_area);

            // Draw gutter if enabled
            if let Some(gutter_area) = gutter_area {
                self.draw_gutter(f, gutter_area, buffer, editor, config, &wrapped_lines);
            }

            // Draw cursor at calculated position (adjust for gutter)
            if let Some((cursor_x, cursor_y)) = cursor_visual_pos {
                let screen_x = content_area.x + cursor_x as u16;
                let screen_y = content_area.y + cursor_y as u16;
                
                if screen_x < content_area.x + content_area.width && 
                   screen_y < content_area.y + content_area.height {
                    f.set_cursor_position((screen_x, screen_y));
                    
                    // Apply cursor color if supported by terminal
                    self.set_cursor_style(config, buffer);
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
                Line::from("Supported languages with syntax highlighting:"),
                Line::from("• Rust (.rs)"),
                Line::from("• Python (.py)"),
                Line::from("• JavaScript/TypeScript (.js, .jsx, .ts, .tsx)"),
                Line::from("• Bash (.sh, .bash)"),
                Line::from("• JSON (.json)"),
                Line::from("• SQL (.sql, .mysql, .pgsql, .sqlite)"),
                Line::from("• TOML (.toml)"),
                Line::from("• HTML (.html, .htm)"),
                Line::from("• CSS (.css)"),
                Line::from("• Markdown (.md, .markdown)"),
                Line::from("• YAML (.yaml, .yml)"),
                Line::from("• XML (.xml)"),
                Line::from(""),
                Line::from("Features:"),
                Line::from("• Simple syntax highlighting for 35+ languages"),
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
            // Calculate modal size and position with bounds checking
            let modal_width = 50;
            let max_visible_items = 15; // Maximum items to show at once
            let max_modal_height = (languages.len().min(max_visible_items) as u16) + 4; // +4 for borders and title
            
            let area = f.area();
            // Ensure modal doesn't exceed screen bounds
            let modal_height = max_modal_height.min(area.height.saturating_sub(2));
            let content_height = modal_height.saturating_sub(4); // Available space for list items
            
            let modal_area = Rect {
                x: (area.width.saturating_sub(modal_width)) / 2,
                y: (area.height.saturating_sub(modal_height)) / 2,
                width: modal_width.min(area.width.saturating_sub(2)),
                height: modal_height,
            };

            // Clear the background
            f.render_widget(Clear, modal_area);

            let modal_bg = config.theme.parse_color(&config.theme.colors.modal_bg);
            let modal_fg = config.theme.parse_color(&config.theme.colors.modal_fg);
            let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
            let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
            let border_color = config.theme.parse_color(&config.theme.colors.border_active);

            // Calculate visible range based on scroll offset
            let scroll_offset = editor.language_selection_scroll_offset;
            let visible_end = (scroll_offset + content_height as usize).min(languages.len());
            
            // Create language list items with numbering for visible items only
            let items: Vec<ListItem> = languages[scroll_offset..visible_end]
                .iter()
                .enumerate()
                .map(|(visible_i, &lang)| {
                    let actual_i = scroll_offset + visible_i;
                    let display_name = Buffer::get_language_display_name(lang);
                    let number = actual_i + 1;
                    
                    let text = format!("{}. {}", number, display_name);
                    
                    if actual_i == selected_index {
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

            // Create scroll indicator text
            let has_more_above = scroll_offset > 0;
            let has_more_below = visible_end < languages.len();
            let scroll_info = if has_more_above || has_more_below {
                let mut info = String::new();
                if has_more_above { info.push_str("▲ "); }
                info.push_str(&format!("{}-{}/{}", scroll_offset + 1, visible_end, languages.len()));
                if has_more_below { info.push_str(" ▼"); }
                format!(" [{}]", info)
            } else {
                String::new()
            };
            
            // Create the list widget
            let title = format!("Select Language (↑↓ to navigate, Enter to select, Esc to cancel){}", scroll_info);
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .title(title)
                        .style(Style::default().bg(modal_bg))
                )
                .style(Style::default().fg(modal_fg));

            f.render_widget(list, modal_area);

            // Add instruction text at the bottom of the modal (with bounds checking)
            let instruction_y = if modal_area.height >= 3 {
                modal_area.y + modal_area.height - 2
            } else {
                modal_area.y + modal_area.height.saturating_sub(1)
            };
            
            let instruction_area = Rect {
                x: modal_area.x + 1,
                y: instruction_y,
                width: modal_area.width.saturating_sub(2),
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
            // Calculate modal size and position with bounds checking
            let modal_width = 60;
            let max_visible_items = 15;
            let max_modal_height = max_visible_items.min(themes.len()) as u16 + 4; // +4 for borders and title
            
            let area = f.area();
            // Ensure modal doesn't exceed screen bounds
            let modal_height = max_modal_height.min(area.height.saturating_sub(2));
            
            let modal_area = Rect {
                x: (area.width.saturating_sub(modal_width)) / 2,
                y: (area.height.saturating_sub(modal_height)) / 2,
                width: modal_width.min(area.width.saturating_sub(2)),
                height: modal_height,
            };

            // Clear the background
            f.render_widget(Clear, modal_area);

            let modal_bg = config.theme.parse_color(&config.theme.colors.modal_bg);
            let modal_fg = config.theme.parse_color(&config.theme.colors.modal_fg);
            let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
            let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
            let border_color = config.theme.parse_color(&config.theme.colors.border_active);

            // Calculate scrolling
            let scroll_offset = editor.theme_selection_scroll_offset;
            let visible_end = (scroll_offset + max_visible_items).min(themes.len());
            
            // Create theme list items with numbering (only visible items)
            let items: Vec<ListItem> = themes[scroll_offset..visible_end]
                .iter()
                .enumerate()
                .map(|(visible_i, (_, display_name))| {
                    let actual_i = scroll_offset + visible_i;
                    let number = actual_i + 1;
                    let text = format!("{}. {}", number, display_name);
                    
                    if actual_i == selected_index {
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

            // Create scroll indicator text
            let has_more_above = scroll_offset > 0;
            let has_more_below = visible_end < themes.len();
            let scroll_info = if has_more_above || has_more_below {
                let mut info = String::new();
                if has_more_above { info.push_str("▲ "); }
                info.push_str(&format!("{}-{}/{}", scroll_offset + 1, visible_end, themes.len()));
                if has_more_below { info.push_str(" ▼"); }
                format!(" [{}]", info)
            } else {
                String::new()
            };

            // Create the list widget
            let title = format!("Select Theme (↑↓ to navigate, Enter to select, Esc to cancel){}", scroll_info);
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .title(title)
                        .style(Style::default().bg(modal_bg))
                )
                .style(Style::default().fg(modal_fg));

            f.render_widget(list, modal_area);

            // Add instruction text at the bottom of the modal (with bounds checking)
            let instruction_y = if modal_area.height >= 3 {
                modal_area.y + modal_area.height - 2
            } else {
                modal_area.y + modal_area.height.saturating_sub(1)
            };
            
            let instruction_area = Rect {
                x: modal_area.x + 1,
                y: instruction_y,
                width: modal_area.width.saturating_sub(2),
                height: 1,
            };

            let current_theme = &config.theme.name;
            let visible_count = visible_end - scroll_offset;
            let instruction = Paragraph::new(
                format!("Current: {} | Press 1-{} for quick select", current_theme, visible_count.min(9))
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
        let scrolloff = config.scrolloff as usize;
        
        // Calculate virtual line boundaries
        let virtual_end = scrolloff; // Number of virtual lines at end
        let total_file_lines = buffer.rope.len_lines();
        
        // Adjust viewport to account for virtual lines (removed unused variables)
        
        let mut current_visual_line = 0;
        
        // Add virtual lines at the start if viewport is negative (above file start)
        if editor.viewport_line < 0 {
let virtual_lines_to_show = (-editor.viewport_line) as usize;
            for _ in 0..virtual_lines_to_show.min(content_height) {
                wrapped_lines.push(WrappedLine {
                    content: "~".to_string(), // Filled virtual line
                    logical_line: usize::MAX, // Mark as virtual
                    line_start_col: 0,
                    line_end_col: 1,
                });
                current_visual_line += 1;
                if current_visual_line >= content_height {
                    break;
                }
            }
        }
        
        // Add actual file content
        let start_line = if editor.viewport_line >= 0 {
            editor.viewport_line as usize
        } else {
            0
        };
        let end_line = total_file_lines;

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
                current_visual_line = wrapped_lines.len();
                if current_visual_line >= content_height {
                    return (wrapped_lines, cursor_visual_pos);
                }
            }
        }
        
        // Add virtual lines at the end if we've displayed all file content and have space
        current_visual_line = wrapped_lines.len();
        
        // Check if we've shown all the file content
        let lines_from_file = if editor.viewport_line < 0 {
            // If viewport is negative, we need to account for that
            end_line - start_line
        } else {
            // Normal case
            (end_line - start_line).min(current_visual_line)
        };
        
        // Only add virtual lines at end if we've reached the end of the file content
        if current_visual_line < content_height && start_line + lines_from_file >= total_file_lines {
            let remaining_space = content_height - current_visual_line;
            let virtual_lines_to_add = remaining_space.min(virtual_end);
            
            for _ in 0..virtual_lines_to_add {
                wrapped_lines.push(WrappedLine {
                    content: "~".to_string(), // Filled virtual line
                    logical_line: usize::MAX, // Mark as virtual
                    line_start_col: 0,
                    line_end_col: 1,
                });
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
        wrap_line(text, width)
    }

    fn apply_syntax_highlighting_wrapped(
        &self, 
        text: String, 
        buffer: &Buffer, 
        line_idx: usize, 
        segment_start: usize,
        config: &Config
    ) -> Line<'static> {
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();
        let segment_end = segment_start + text_len;
        
        // Get selection range if any
        let selection_range = buffer.cursor.get_selection_range();
        
        // For character-precise selection, we need to split text at selection boundaries
        if let Some((sel_start, sel_end)) = selection_range {
            return self.apply_highlighting_with_precise_selection(
                text, buffer, line_idx, segment_start, sel_start, sel_end, config
            );
        }
        
        // No selection - apply normal syntax highlighting
        if let Some(tokens) = buffer.syntax_highlighter.get_line_tokens(line_idx) {
            let mut spans = Vec::new();
            let mut last_end = 0;

            // Sort tokens by start position to ensure proper rendering order
            let mut sorted_tokens = tokens.clone();
            sorted_tokens.sort_by_key(|token| token.start);

            for token in &sorted_tokens {
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
            // No syntax highlighting, return as normal text
            let normal_color = config.theme.parse_color(&config.theme.colors.normal);
            Line::styled(text, Style::default().fg(normal_color))
        }
    }

    fn get_token_style(&self, token_type: &TokenType, config: &Config) -> Style {
        let (color, text_styles) = match token_type {
            TokenType::Keyword => (config.theme.parse_color(&config.theme.colors.keyword), &config.theme.styles.keyword),
            TokenType::String => (config.theme.parse_color(&config.theme.colors.string), &config.theme.styles.string),
            TokenType::Comment => (config.theme.parse_color(&config.theme.colors.comment), &config.theme.styles.comment),
            TokenType::Number => (config.theme.parse_color(&config.theme.colors.number), &config.theme.styles.number),
            TokenType::Operator => (config.theme.parse_color(&config.theme.colors.operator), &config.theme.styles.operator),
            TokenType::Identifier => (config.theme.parse_color(&config.theme.colors.identifier), &config.theme.styles.identifier),
            TokenType::Type => (config.theme.parse_color(&config.theme.colors.type_), &config.theme.styles.type_),
            TokenType::Function => (config.theme.parse_color(&config.theme.colors.function), &config.theme.styles.function),
            TokenType::Variable => (config.theme.parse_color(&config.theme.colors.variable), &config.theme.styles.variable),
            TokenType::Property => (config.theme.parse_color(&config.theme.colors.property), &config.theme.styles.property),
            TokenType::Parameter => (config.theme.parse_color(&config.theme.colors.parameter), &config.theme.styles.parameter),
            TokenType::Constant => (config.theme.parse_color(&config.theme.colors.constant), &config.theme.styles.constant),
            TokenType::Namespace => (config.theme.parse_color(&config.theme.colors.namespace), &config.theme.styles.namespace),
            TokenType::Punctuation => (config.theme.parse_color(&config.theme.colors.punctuation), &config.theme.styles.punctuation),
            TokenType::Tag => (config.theme.parse_color(&config.theme.colors.tag), &config.theme.styles.tag),
            TokenType::Attribute => (config.theme.parse_color(&config.theme.colors.attribute), &config.theme.styles.attribute),
            TokenType::Normal => (config.theme.parse_color(&config.theme.colors.normal), &config.theme.styles.normal),
        };

        // Apply color and text styles from theme
        let modifiers = config.theme.parse_text_styles(text_styles);
        Style::default().fg(color).add_modifier(modifiers)
    }

    fn draw_status_line(&self, f: &mut ratatui::Frame, area: Rect, editor: &Editor, config: &Config) {
        let status_bg = config.theme.parse_color(&config.theme.colors.status_bar_bg);
        let status_fg = config.theme.parse_color(&config.theme.colors.status_bar_fg);

        if let Some(buffer) = editor.current_buffer() {
            // Left side: filename, dirty indicator, language, and mode indicators
            let mut left_text = String::new();
            
            // File info
            let file_name = buffer.file_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("[No Name]");
            
            left_text.push_str(&format!("  {}", file_name));

            if buffer.dirty {
                left_text.push_str("*");
            }

            // Language indicator
            let display_name = Buffer::get_language_display_name(&buffer.language);
            left_text.push_str(&format!(" | {}", display_name));
            
            // Word wrap indicator (only if enabled)
            if config.word_wrap {
                left_text.push_str(" | WRAP");
            }
            
            // Mode indicators (only when in special modes)
            if editor.language_selection_mode {
                left_text.push_str(" | LANGUAGE SELECTION");
            } else if editor.theme_selection_mode {
                left_text.push_str(" | THEME SELECTION");
            }

            // Right side: cursor position (row/total:column)
            let total_lines = buffer.rope.len_lines();
            let right_text = format!("{}/{}:{}  ", 
                buffer.cursor.line + 1, 
                total_lines,
                buffer.cursor.column + 1
            );

            // Create layout for left and right alignment
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(right_text.len() as u16)])
                .split(area);

            // Left-aligned content
            let left_status = Paragraph::new(left_text)
                .style(Style::default().bg(status_bg).fg(status_fg))
                .alignment(Alignment::Left);
            f.render_widget(left_status, chunks[0]);

            // Right-aligned content
            let right_status = Paragraph::new(right_text)
                .style(Style::default().bg(status_bg).fg(status_fg))
                .alignment(Alignment::Right);
            f.render_widget(right_status, chunks[1]);
        } else {
            // No buffer - just show a simple status
            let status = Paragraph::new("[No Buffer]")
                .style(Style::default().bg(status_bg).fg(status_fg))
                .alignment(Alignment::Left);
            f.render_widget(status, area);
        }
    }

    // Draw help modal
    fn draw_help_modal(&self, f: &mut ratatui::Frame, config: &Config) {
        let area = f.area();
        let modal_width = 70;
        let modal_height = 25;
        
        let modal_area = Rect {
            x: (area.width.saturating_sub(modal_width)) / 2,
            y: (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width.min(area.width.saturating_sub(2)),
            height: modal_height.min(area.height.saturating_sub(2)),
        };

        // Clear the background
        f.render_widget(Clear, modal_area);

        let modal_bg = config.theme.parse_color(&config.theme.colors.modal_bg);
        let modal_fg = config.theme.parse_color(&config.theme.colors.modal_fg);
        let border_color = config.theme.parse_color(&config.theme.colors.border_active);

        let help_content = vec![
            Line::from(""),
            Line::from("📝 EDITOR COMMANDS"),
            Line::from("  Ctrl+S         Save file"),
            Line::from("  Ctrl+O         Open file (TODO)"),
            Line::from("  Ctrl+Q         Quit editor"),
            Line::from(""),
            Line::from("🔤 CURSOR MOVEMENT"),
            Line::from("  Arrow Keys     Move cursor"),
            Line::from("  Home           Move to beginning of line"),
            Line::from("  End            Move to end of line"),
            Line::from("  Page Up/Down   Move by page"),
            Line::from(""),
            Line::from("✏️  TEXT EDITING"),
            Line::from("  Enter          Insert new line"),
            Line::from("  Tab            Insert 4 spaces"),
            Line::from("  Backspace      Delete character backward"),
            Line::from("  Delete         Delete character forward"),
            Line::from("  Ctrl+A         Select all text"),
            Line::from("  Ctrl+C         Copy selected text"),
            Line::from("  Ctrl+X         Cut selected text"),
            Line::from("  Ctrl+V         Paste from clipboard"),
            Line::from(""),
            Line::from("🖱️  MOUSE & SELECTION"),
            Line::from("  Click          Move cursor"),
            Line::from("  Click+Drag     Select text"),
            Line::from("  Shift+Click    Extend selection"),
            Line::from("  Shift+Arrows   Extend selection with keyboard"),
            Line::from(""),
                Line::from("🎨 CUSTOMIZATION"),
                Line::from("  F1             Show this help"),
                Line::from("  F2/F3          Adjust vertical margins"),
                Line::from("  F4/F5          Adjust horizontal margins"),
                Line::from("  F6             Toggle word wrap"),
                Line::from("  F7             Toggle gutter (None/Absolute/Relative)"),
                Line::from("  Ctrl+L         Change language/syntax"),
                Line::from("  Ctrl+T         Change color theme"),
            Line::from(""),
            Line::from("💡 FEATURES"),
            Line::from("  • Syntax highlighting for 35+ languages"),
            Line::from("  • Word wrapping with smart cursor movement"),
            Line::from("  • Manual save only (Ctrl+S) - no auto-save"),
            Line::from("  • Configurable margins (0 to any size)"),
            Line::from("  • Multiple color themes"),
            Line::from(""),
            Line::from(Span::styled("Press ESC, F1, or Q to close this help", Style::default().add_modifier(Modifier::BOLD))),
        ];

        let help_paragraph = Paragraph::new(help_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title("Thyme Editor - Help")
                    .style(Style::default().bg(modal_bg))
            )
            .style(Style::default().fg(modal_fg))
            .alignment(Alignment::Left);

        f.render_widget(help_paragraph, modal_area);
    }
    
    /// Apply selection highlighting to text style
    fn get_text_style_with_selection(
        &self,
        line_idx: usize,
        start_col: usize,
        end_col: usize,
        base_color: &str,
        selection_range: Option<(Position, Position)>,
        config: &Config,
    ) -> Style {
        let base_style = Style::default().fg(config.theme.parse_color(base_color));
        
        if let Some((sel_start, sel_end)) = selection_range {
            // Check if this text range overlaps with selection
            if self.text_overlaps_with_selection(line_idx, start_col, end_col, sel_start, sel_end) {
                let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
                let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
                return Style::default().bg(selection_bg).fg(selection_fg);
            }
        }
        
        base_style
    }
    
    /// Apply selection highlighting to an existing style
    fn apply_selection_style(
        &self,
        base_style: Style,
        line_idx: usize,
        start_col: usize,
        end_col: usize,
        selection_range: Option<(Position, Position)>,
        config: &Config,
    ) -> Style {
        if let Some((sel_start, sel_end)) = selection_range {
            // Check if this text range overlaps with selection
            if self.text_overlaps_with_selection(line_idx, start_col, end_col, sel_start, sel_end) {
                let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
                let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
                return Style::default().bg(selection_bg).fg(selection_fg);
            }
        }
        
        base_style
    }
    
    /// Apply highlighting with character-precise selection boundaries
    fn apply_highlighting_with_precise_selection(
        &self,
        text: String,
        buffer: &Buffer,
        line_idx: usize,
        segment_start: usize,
        sel_start: Position,
        sel_end: Position,
        config: &Config,
    ) -> Line<'static> {
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();
        let segment_end = segment_start + text_len;
        
        let mut spans = Vec::new();
        
        // Calculate selection boundaries within this segment
        let selection_boundaries = self.calculate_selection_boundaries(
            line_idx, segment_start, segment_end, sel_start, sel_end
        );
        
        // Get syntax tokens for this line
        let tokens = buffer.syntax_highlighter.get_line_tokens(line_idx)
            .map(|tokens| {
                let mut sorted = tokens.clone();
                sorted.sort_by_key(|token| token.start);
                sorted
            })
            .unwrap_or_default();
        
        // Create a list of all boundaries (selection + token boundaries)
        let mut all_boundaries = Vec::new();
        
        // Add selection boundaries
        for boundary in selection_boundaries {
            if boundary >= segment_start && boundary <= segment_end {
                all_boundaries.push(boundary - segment_start);
            }
        }
        
        // Add token boundaries
        for token in &tokens {
            if token.start >= segment_start && token.start <= segment_end {
                all_boundaries.push(token.start - segment_start);
            }
            if token.end >= segment_start && token.end <= segment_end {
                all_boundaries.push(token.end - segment_start);
            }
        }
        
        // Add start and end
        all_boundaries.push(0);
        all_boundaries.push(text_len);
        
        // Remove duplicates and sort
        all_boundaries.sort();
        all_boundaries.dedup();
        
        // Process each segment between boundaries
        for i in 0..all_boundaries.len() - 1 {
            let start = all_boundaries[i];
            let end = all_boundaries[i + 1];
            
            if start >= end || start >= text_len {
                continue;
            }
            
            let actual_end = end.min(text_len);
            if start < actual_end && start < text_chars.len() && actual_end <= text_chars.len() {
                let segment_text: String = text_chars[start..actual_end].iter().collect();
                
                // Determine if this segment is selected
                let is_selected = self.is_segment_selected(
                    line_idx, segment_start + start, segment_start + actual_end,
                    sel_start, sel_end
                );
                
                // Find the appropriate token style for this segment
                let style = self.get_segment_style(
                    &tokens, segment_start + start, segment_start + actual_end,
                    is_selected, config
                );
                
                spans.push(Span::styled(segment_text, style));
            }
        }
        
        if spans.is_empty() {
            let is_selected = self.is_segment_selected(
                line_idx, segment_start, segment_end, sel_start, sel_end
            );
            let style = if is_selected {
                let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
                let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
                Style::default().bg(selection_bg).fg(selection_fg)
            } else {
                let normal_color = config.theme.parse_color(&config.theme.colors.normal);
                Style::default().fg(normal_color)
            };
            Line::styled(text, style)
        } else {
            Line::from(spans)
        }
    }
    
    /// Calculate selection boundaries within the given range
    fn calculate_selection_boundaries(
        &self,
        line_idx: usize,
        segment_start: usize,
        segment_end: usize,
        sel_start: Position,
        sel_end: Position,
    ) -> Vec<usize> {
        let mut boundaries = Vec::new();
        
        // Single line selection
        if sel_start.line == sel_end.line && sel_start.line == line_idx {
            if sel_start.column >= segment_start && sel_start.column <= segment_end {
                boundaries.push(sel_start.column);
            }
            if sel_end.column >= segment_start && sel_end.column <= segment_end {
                boundaries.push(sel_end.column);
            }
        }
        // Multi-line selection
        else {
            if line_idx == sel_start.line && sel_start.column >= segment_start && sel_start.column <= segment_end {
                boundaries.push(sel_start.column);
            }
            if line_idx == sel_end.line && sel_end.column >= segment_start && sel_end.column <= segment_end {
                boundaries.push(sel_end.column);
            }
        }
        
        boundaries
    }
    
    /// Check if a specific segment is within the selection
    fn is_segment_selected(
        &self,
        line_idx: usize,
        start_col: usize,
        end_col: usize,
        sel_start: Position,
        sel_end: Position,
    ) -> bool {
        // Check if line is within selection range
        if line_idx < sel_start.line || line_idx > sel_end.line {
            return false;
        }
        
        // Single line selection
        if sel_start.line == sel_end.line {
            if line_idx == sel_start.line {
                // Segment must be entirely within selection bounds
                return start_col >= sel_start.column && end_col <= sel_end.column;
            }
            return false;
        }
        
        // Multi-line selection
        if line_idx == sel_start.line {
            // First line: segment must start at or after selection start
            return start_col >= sel_start.column;
        } else if line_idx == sel_end.line {
            // Last line: segment must end at or before selection end
            return end_col <= sel_end.column;
        } else {
            // Middle lines: entire segment is selected
            return true;
        }
    }
    
    /// Get the appropriate style for a text segment
    fn get_segment_style(
        &self,
        tokens: &[crate::syntax::SyntaxToken],
        start_col: usize,
        end_col: usize,
        is_selected: bool,
        config: &Config,
    ) -> Style {
        // Find the token that contains this segment
        let token_style = tokens.iter()
            .find(|token| token.start <= start_col && token.end >= end_col)
            .map(|token| self.get_token_style(&token.token_type, config))
            .unwrap_or_else(|| {
                let normal_color = config.theme.parse_color(&config.theme.colors.normal);
                Style::default().fg(normal_color)
            });
        
        if is_selected {
            let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
            let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
            Style::default().bg(selection_bg).fg(selection_fg)
        } else {
            token_style
        }
    }
    
    /// Check if a text range overlaps with the selection
    fn text_overlaps_with_selection(
        &self,
        line_idx: usize,
        start_col: usize,
        end_col: usize,
        sel_start: Position,
        sel_end: Position,
    ) -> bool {
        // Check if the text line is within the selection range
        if line_idx < sel_start.line || line_idx > sel_end.line {
            return false;
        }
        
        // If the selection is on a single line
        if sel_start.line == sel_end.line {
            if line_idx == sel_start.line {
                // Check if the text range overlaps with the selection range on this line
                // Use max and min to find the actual overlap
                let text_start = start_col;
                let text_end = end_col;
                let sel_start_col = sel_start.column;
                let sel_end_col = sel_end.column;
                
                // There is overlap if: text_start < sel_end AND text_end > sel_start
                return text_start < sel_end_col && text_end > sel_start_col;
            }
            return false;
        }
        
        // Multi-line selection
        if line_idx == sel_start.line {
            // First line of selection - only the part after sel_start.column is selected
            return end_col > sel_start.column;
        } else if line_idx == sel_end.line {
            // Last line of selection - only the part before sel_end.column is selected
            return start_col < sel_end.column;
        } else {
            // Middle lines of selection - entire line is selected
            return true;
        }
    }
    
    /// Calculate total number of visual lines considering word wrapping
    fn calculate_total_visual_lines(&self, buffer: &Buffer, content_width: usize) -> usize {
        let mut total_visual_lines = 0;
        
        for line_idx in 0..buffer.rope.len_lines() {
            let line_text = buffer.get_line_text(line_idx);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let wrapped_segments = wrap_line(line_text_for_display, content_width);
            total_visual_lines += wrapped_segments.len().max(1); // At least 1 line per logical line
        }
        
        total_visual_lines
    }
    
    /// Calculate which visual line the cursor is currently on
    fn calculate_cursor_visual_line(&self, buffer: &Buffer, content_width: usize) -> usize {
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
            
            // Find which segment the cursor is in
            for (segment_idx, (_segment, start_pos)) in wrapped_segments.iter().enumerate() {
                let segment_end = if segment_idx + 1 < wrapped_segments.len() {
                    wrapped_segments[segment_idx + 1].1
                } else {
                    line_text_for_display.chars().count()
                };
                
                if buffer.cursor.column >= *start_pos && buffer.cursor.column <= segment_end {
                    visual_line += segment_idx;
                    break;
                }
            }
        }
        
        visual_line
    }
    
    /// Set cursor style based on theme configuration and selection state
    fn set_cursor_style(&self, config: &Config, buffer: &Buffer) {
        use crossterm::cursor::SetCursorStyle;
        use crossterm::queue;
        use std::io::{stdout, Write};
        
        // Parse cursor color from theme
        let cursor_color = &config.theme.colors.cursor;
        
        // Choose cursor style based on selection state
        let cursor_style = if buffer.cursor.get_selection_range().is_some() {
            SetCursorStyle::BlinkingUnderScore // Underscore when text is selected
        } else {
            SetCursorStyle::BlinkingBlock // Block when no selection
        };
        
        // Set cursor style first
        let _ = queue!(stdout(), cursor_style);
        
        // Try to set cursor color using multiple approaches for better compatibility
        if cursor_color.starts_with('#') && cursor_color.len() == 7 {
            // Method 1: OSC 12 sequence (most widely supported)
            // Format: ESC ] 12 ; color BEL
            let osc12_sequence = format!("\x1b]12;{}\x07", cursor_color);
            let _ = queue!(stdout(), crossterm::style::Print(osc12_sequence));
            
            // Method 2: Alternative OSC 12 with ST terminator (for some terminals)
            // Format: ESC ] 12 ; color ESC \
            let osc12_st_sequence = format!("\x1b]12;{}\x1b\\", cursor_color);
            let _ = queue!(stdout(), crossterm::style::Print(osc12_st_sequence));
            
            // Method 3: Try DECSCUSR with color (for terminals that support it)
            // This is less standard but some terminals support it
            if let Some(hex) = cursor_color.strip_prefix('#') {
                if hex.len() == 6 {
                    // Some terminals support color in DECSCUSR sequences
                    let decscusr_color = format!("\x1b[{} q\x1b]12;{}\x07", 
                        if buffer.cursor.get_selection_range().is_some() { "4" } else { "2" }, 
                        cursor_color
                    );
                    let _ = queue!(stdout(), crossterm::style::Print(decscusr_color));
                }
            }
        } else {
            // Fallback for non-hex colors or invalid format
            // Just set the cursor style without attempting color change
        }
        
        let _ = stdout().flush();
    }
    
    /// Convert hex color string to crossterm Color
    fn parse_crossterm_color(&self, color_str: &str) -> Option<crossterm::style::Color> {
        use crossterm::style::Color;
        
        // Handle hex colors
        if let Some(hex) = color_str.strip_prefix('#') {
            if hex.len() == 6 {
                if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                    let r = ((rgb >> 16) & 0xFF) as u8;
                    let g = ((rgb >> 8) & 0xFF) as u8;
                    let b = (rgb & 0xFF) as u8;
                    return Some(Color::Rgb { r, g, b });
                }
            }
        }
        
        // Handle named colors
        match color_str.to_lowercase().as_str() {
            "black" => Some(Color::Black),
            "red" => Some(Color::Red),
            "green" => Some(Color::Green),
            "yellow" => Some(Color::Yellow),
            "blue" => Some(Color::Blue),
            "magenta" => Some(Color::Magenta),
            "cyan" => Some(Color::Cyan),
            "gray" | "grey" => Some(Color::Grey),
            "darkgray" | "darkgrey" => Some(Color::DarkGrey),
            "white" => Some(Color::White),
            _ => None,
        }
    }
    
    /// Calculate the width needed for the gutter based on the mode and file size
    fn calculate_gutter_width(&self, buffer: &Buffer, mode: GutterMode) -> usize {
        match mode {
            GutterMode::None => 0,
            GutterMode::Absolute | GutterMode::Relative => {
                // Calculate the number of digits needed for the line count
                let total_lines = buffer.rope.len_lines();
                let digits = total_lines.to_string().len();
                // Add 2 spaces for padding (1 before, 1 after the number)
                digits + 2
            }
        }
    }
    
    /// Draw the gutter (line numbers) on the left side
    fn draw_gutter(
        &self,
        f: &mut ratatui::Frame,
        area: Rect,
        buffer: &Buffer,
        editor: &Editor,
        config: &Config,
        wrapped_lines: &[WrappedLine],
    ) {
        let line_number_color = config.theme.parse_color(&config.theme.colors.line_number);
        let current_line_color = config.theme.parse_color(&config.theme.colors.foreground);
        let bg_color = config.theme.parse_color(&config.theme.colors.background);
        
        let mut gutter_lines = Vec::new();
        let cursor_line = buffer.cursor.line;
        
        // Track the last logical line we rendered to avoid duplicate line numbers
        let mut last_logical_line = None;
        
        for wrapped in wrapped_lines {
            let line_content = if wrapped.logical_line == usize::MAX {
                // Virtual line - no line number
                " ".repeat(area.width as usize)
            } else {
                // Only show line number for the first wrapped segment of each logical line
                if last_logical_line == Some(wrapped.logical_line) {
                    // This is a continuation of the previous logical line
                    " ".repeat(area.width as usize)
                } else {
                    // This is a new logical line
                    last_logical_line = Some(wrapped.logical_line);
                    
                    let line_number = match config.gutter {
                        GutterMode::Absolute => wrapped.logical_line + 1,
                        GutterMode::Relative => {
                            if wrapped.logical_line == cursor_line {
                                wrapped.logical_line + 1
                            } else {
                                let diff = if wrapped.logical_line > cursor_line {
                                    wrapped.logical_line - cursor_line
                                } else {
                                    cursor_line - wrapped.logical_line
                                };
                                diff
                            }
                        }
                        GutterMode::None => unreachable!("Gutter should not be drawn when mode is None"),
                    };
                    
                    // Format the line number with right alignment
                    format!("{:>width$} ", line_number, width = area.width as usize - 1)
                }
            };
            
            // Use different color for current line
            let color = if wrapped.logical_line == cursor_line {
                current_line_color
            } else {
                line_number_color
            };
            
            let line = Line::styled(line_content, Style::default().fg(color).bg(bg_color));
            gutter_lines.push(line);
        }
        
        let gutter_paragraph = Paragraph::new(gutter_lines)
            .style(Style::default().bg(bg_color));
        
        f.render_widget(gutter_paragraph, area);
    }
}

