// src/ui.rs

use crate::{
    buffer::Buffer, 
    config::{Config, GutterMode}, 
    cursor::Position,
    editor::Editor, 
    syntax::TokenType,
    text_utils::wrap_line,
    unicode_utils::{char_display_width, str_display_width}
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
        // Calculate layout based on whether find/replace is active
        let constraints = if editor.find_replace_mode {
            vec![
                Constraint::Min(0),      // Main editor area
                Constraint::Length(3),   // Find/replace bar
                Constraint::Length(1),   // Status line
            ]
        } else {
            vec![
                Constraint::Min(0),      // Main editor area
                Constraint::Length(1),   // Status line
            ]
        };
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)  // Remove outer margin to allow editor to reach edges
            .constraints(constraints)
            .split(f.area());

        // Main editor area
        self.draw_editor(f, chunks[0], editor, config);

        if editor.find_replace_mode {
            // Draw find/replace bar
            self.draw_find_replace_bar(f, chunks[1], editor, config);
            // Status line
            self.draw_status_line(f, chunks[2], editor, config);
        } else {
            // Status line
            self.draw_status_line(f, chunks[1], editor, config);
        }

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
            self.draw_help_modal(f, editor, config);
        }
        
        // Draw save prompt overlay if active
        if editor.save_prompt_mode {
            self.draw_save_prompt_overlay(f, config);
        } else if editor.filename_prompt_mode {
            self.draw_filename_prompt_modal(f, editor, config);
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
            let (wrapped_lines, cursor_visual_pos) = if config.word_wrap {
                self.prepare_wrapped_content_visual(
                    buffer, editor, config, content_width, content_height
                )
            } else {
                self.prepare_wrapped_content(
                    buffer, editor, config, content_width, content_height
                )
            };

            // Convert to ratatui Lines with syntax highlighting
            let lines: Vec<Line> = wrapped_lines.iter().map(|wl| {
                // Apply horizontal offset only to non-virtual lines when word wrap is disabled
                let (displayed_content, effective_horizontal_offset) = if !config.word_wrap && wl.logical_line != usize::MAX {
                    // Apply horizontal scrolling with Unicode awareness
                    let chars: Vec<char> = wl.content.chars().collect();
                    if editor.horizontal_offset == 0 {
                        (wl.content.clone(), 0)
                    } else {
                        // Skip characters until we've scrolled past horizontal_offset visual columns
                        let mut visual_col = 0;
                        let mut char_idx = 0;
                        for (i, &ch) in chars.iter().enumerate() {
                            if visual_col >= editor.horizontal_offset {
                                char_idx = i;
                                break;
                            }
                            visual_col += char_display_width(ch);
                        }
                        if char_idx >= chars.len() {
                            (String::new(), editor.horizontal_offset)
                        } else {
                            (chars[char_idx..].iter().collect(), editor.horizontal_offset)
                        }
                    }
                } else {
                    (wl.content.clone(), 0)
                };
                
                let mut line = self.apply_syntax_highlighting_wrapped(
                    displayed_content, 
                    buffer, 
                    wl.logical_line, 
                    wl.line_start_col,
                    config,
                    editor,
                    effective_horizontal_offset
                );
                if wl.logical_line == usize::MAX {
                    let virtual_color = config.theme.parse_color(&config.theme.colors.virtual_line);
                    line = line.style(Style::default().fg(virtual_color));
                }
                line
            }).collect();

            let bg_color = config.theme.parse_color(&config.theme.colors.background);
            let fg_color = config.theme.parse_color(&config.theme.colors.foreground);

            // Apply current line highlighting if enabled
            let lines_with_current_line: Vec<Line> = if config.highlight_current_line {
                let current_line_bg = config.theme.parse_color(&config.theme.colors.current_line_bg);
                wrapped_lines.iter().zip(lines.into_iter()).map(|(wl, mut line)| {
                    if wl.logical_line == buffer.cursor.line && wl.logical_line != usize::MAX {
                        // Calculate the total visual width of existing content
                        let content_width: usize = line.spans.iter()
                            .map(|span| str_display_width(&span.content))
                            .sum();
                        let viewport_width = content_area.width as usize;
                        
                        // Apply current line background to all spans in the line
                        let mut spans: Vec<Span> = line.spans.into_iter().map(|span| {
                            // Only add background if the span doesn't already have one
                            if span.style.bg.is_none() {
                                span.patch_style(Style::default().bg(current_line_bg))
                            } else {
                                span
                            }
                        }).collect();
                        
                        // If the line is shorter than the viewport, add padding to fill the rest
                        if content_width < viewport_width {
                            let padding_width = viewport_width - content_width;
                            let padding = " ".repeat(padding_width);
                            spans.push(Span::styled(padding, Style::default().bg(current_line_bg)));
                        }
                        
                        Line::from(spans)
                    } else {
                        line
                    }
                }).collect()
            } else {
                lines
            };
            
            let paragraph = Paragraph::new(lines_with_current_line)
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
                Line::from("Press F1 for help and keybindings"),
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
                Line::from("• C/C++ (.c, .cpp, .h, .hpp)"),
                Line::from("• And 20+ more languages..."),
                Line::from(""),
                Line::from("Features:"),
                Line::from("• Syntax highlighting for 35+ languages"),
                Line::from("• Customizable color themes with live preview"),
                Line::from("• Word wrapping with proper cursor handling"),
                Line::from("• Configurable margins and keybindings"),
                Line::from("• Line numbers (absolute/relative)"),
                Line::from("• Undo/Redo with intelligent grouping"),
                Line::from("• UTF-8 support"),
                Line::from(""),
                Line::from("Quick Start:"),
                Line::from("• F1: Help"),
                Line::from("• Ctrl+S: Save"),
                Line::from("• Ctrl+Q: Quit"),
                Line::from("• F1: Help"),
                Line::from("• Ctrl+L: Change language"),
                Line::from("• Ctrl+T: Change theme"),
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
                        // Calculate visual column considering Unicode width and indentation
                        let chars_before_cursor = cursor_col - start_col;
                        
                        // Check if this is a continuation line with indentation
                        let indent_offset = if *start_col > 0 && config.word_wrap {
                            // Count leading spaces/tabs in the wrapped content
                            let wrapped_chars: Vec<char> = wrapped_content.chars().collect();
                            wrapped_chars.iter().take_while(|&&c| c == ' ' || c == '\t').count()
                        } else {
                            0
                        };
                        
                        // The cursor position in the wrapped line includes the indentation
                        let cursor_pos_in_wrapped = indent_offset + chars_before_cursor;
                        let line_before_cursor: String = wrapped_content.chars().take(cursor_pos_in_wrapped).collect();
                        let visual_col = str_display_width(&line_before_cursor);
                        
                        // Apply horizontal offset to cursor position when word wrap is disabled
                        if !config.word_wrap && editor.horizontal_offset > 0 {
                            if visual_col >= editor.horizontal_offset {
                                cursor_visual_pos = Some((visual_col - editor.horizontal_offset, visual_line_idx));
                            }
                            // Cursor is scrolled off to the left, don't show it
                        } else {
                            cursor_visual_pos = Some((visual_col, visual_line_idx));
                        }
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

    // New method for word-wrap aware content preparation
    fn prepare_wrapped_content_visual(
        &self,
        buffer: &Buffer,
        editor: &Editor,
        config: &Config,
        content_width: usize,
        content_height: usize,
    ) -> (Vec<WrappedLine>, Option<(usize, usize)>) {
        let mut wrapped_lines = Vec::new();
        let mut cursor_visual_pos = None;
         let _scrolloff = config.scrolloff as usize;
        let total_file_lines = buffer.rope.len_lines();
        
        // In word-wrap mode, viewport_line IS the visual line directly
        let viewport_visual_line = editor.viewport_line;
        
        // Add virtual lines at the start if needed
        let virtual_lines_before = if viewport_visual_line < 0 {
            (-viewport_visual_line) as usize
        } else {
            0
        };
        
        for _ in 0..virtual_lines_before.min(content_height) {
            wrapped_lines.push(WrappedLine {
                content: "~".to_string(),
                logical_line: usize::MAX,
                line_start_col: 0,
                line_end_col: 1,
            });
        }
        
        // Now we need to find which logical line corresponds to our visual viewport start
        let mut current_visual_line = 0;
        let start_visual_line = if viewport_visual_line >= 0 {
            viewport_visual_line as usize
        } else {
            0
        };
        
        // Iterate through all logical lines, counting visual lines
        for logical_line in 0..total_file_lines {
            let line_text = buffer.get_line_text(logical_line);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let line_wrapped = self.wrap_line(line_text_for_display, content_width);
            
            // Check if any of this logical line's visual lines are in our viewport
             for (_, (wrapped_content, start_col)) in line_wrapped.iter().enumerate() {
                // Skip visual lines before our viewport
                if current_visual_line < start_visual_line {
                    current_visual_line += 1;
                    continue;
                }
                
                // We've filled the screen
                if wrapped_lines.len() >= content_height {
                    return (wrapped_lines, cursor_visual_pos);
                }
                
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
                    if cursor_col >= *start_col && cursor_col <= end_col {
                        // Calculate visual column considering Unicode width and indentation
                        let chars_before_cursor = cursor_col - start_col;
                        
                        // Check if this is a continuation line with indentation
                        let indent_offset = if *start_col > 0 {
                            // Count leading spaces/tabs in the wrapped content
                            let wrapped_chars: Vec<char> = wrapped_content.chars().collect();
                            wrapped_chars.iter().take_while(|&&c| c == ' ' || c == '\t').count()
                        } else {
                            0
                        };
                        
                        // The cursor position in the wrapped line includes the indentation
                        let cursor_pos_in_wrapped = indent_offset + chars_before_cursor;
                        let line_before_cursor: String = wrapped_content.chars().take(cursor_pos_in_wrapped).collect();
                        let visual_col = str_display_width(&line_before_cursor);
                        
                        let visual_line_idx = wrapped_lines.len() - 1;
                        cursor_visual_pos = Some((visual_col, visual_line_idx));
                    }
                }
                
                current_visual_line += 1;
            }
            
            // Handle empty lines
            if line_wrapped.is_empty() {
                if current_visual_line >= start_visual_line && wrapped_lines.len() < content_height {
                    wrapped_lines.push(WrappedLine {
                        content: String::new(),
                        logical_line,
                        line_start_col: 0,
                        line_end_col: 0,
                    });
                    
                    if logical_line == buffer.cursor.line && buffer.cursor.column == 0 {
                        cursor_visual_pos = Some((0, wrapped_lines.len() - 1));
                    }
                }
                current_visual_line += 1;
            }
        }
        
        // Add virtual lines at the end if needed
        while wrapped_lines.len() < content_height {
            wrapped_lines.push(WrappedLine {
                content: "~".to_string(),
                logical_line: usize::MAX,
                line_start_col: 0,
                line_end_col: 1,
            });
        }
        
        (wrapped_lines, cursor_visual_pos)
    }

    // Convert a logical line number to visual line number
    fn logical_to_visual_line(&self, buffer: &Buffer, logical_line: isize, content_width: usize) -> isize {
        if logical_line < 0 {
            return logical_line; // Virtual lines before the file
        }
        
        let mut visual_line = 0;
        let target_logical = logical_line as usize;
        
        for line_idx in 0..target_logical.min(buffer.rope.len_lines()) {
            let line_text = buffer.get_line_text(line_idx);
            let line_text_for_display = if line_text.ends_with('\n') {
                &line_text[..line_text.len()-1]
            } else {
                &line_text
            };
            
            let wrapped = self.wrap_line(line_text_for_display, content_width);
            visual_line += wrapped.len().max(1) as isize;
        }
        
        visual_line
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
        config: &Config,
        editor: &Editor,
        horizontal_offset: usize
    ) -> Line<'static> {
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();
        
        // Calculate indentation offset for continuation lines
        // If segment_start > 0, this is a continuation line with added indentation
        let indent_offset = if segment_start > 0 {
            // Count leading spaces/tabs in the displayed text
            text_chars.iter().take_while(|&&c| c == ' ' || c == '\t').count()
        } else {
            0
        };
        
        // The actual content length in the original text (without added indentation)
        let original_content_len = text_len - indent_offset;
        let segment_end = segment_start + original_content_len;
        
        // Get selection range if any
        let selection_range = buffer.cursor.get_selection_range();
        
        // Check if we have find matches to highlight
        let has_find_matches = editor.find_replace_mode && !editor.find_query.is_empty() && 
            editor.find_matches.iter().any(|&(match_line, _, _)| match_line == line_idx);
        
        // For character-precise selection or find matches, we need to split text at boundaries
        if let Some((sel_start, sel_end)) = selection_range {
            return self.apply_highlighting_with_precise_selection(
                text, buffer, line_idx, segment_start, sel_start, sel_end, config, editor, horizontal_offset
            );
        } else if has_find_matches {
            // No selection but we have find matches - create a dummy selection range to trigger precise highlighting
            let dummy_start = Position { line: usize::MAX, column: usize::MAX };
            let dummy_end = Position { line: usize::MAX, column: usize::MAX };
            return self.apply_highlighting_with_precise_selection(
                text, buffer, line_idx, segment_start, dummy_start, dummy_end, config, editor, horizontal_offset
            );
        }
        
        // No selection - apply normal syntax highlighting
        if let Some(tokens) = buffer.syntax_highlighter.get_line_tokens(line_idx) {
            let mut spans = Vec::new();
            let mut last_end = 0;
            
            // If this is a continuation line with indentation, add the indentation as unstyled text first
            if indent_offset > 0 {
                let indent_text: String = text_chars[0..indent_offset].iter().collect();
                let normal_color = config.theme.parse_color(&config.theme.colors.normal);
                spans.push(Span::styled(indent_text, Style::default().fg(normal_color)));
                last_end = indent_offset;
            }

            // Sort tokens by start position to ensure proper rendering order
            let mut sorted_tokens = tokens.clone();
            sorted_tokens.sort_by_key(|token| token.start);

            for token in &sorted_tokens {
                // When horizontal scrolling is applied, we need to adjust token positions
                // The tokens are positioned relative to the full line, but we're displaying
                // a substring that starts at horizontal_offset
                let adjusted_token_start = token.start.saturating_sub(horizontal_offset);
                let adjusted_token_end = token.end.saturating_sub(horizontal_offset);
                
                // Skip tokens that are entirely outside this segment (accounting for horizontal scroll)
                if adjusted_token_end <= segment_start || adjusted_token_start >= segment_end {
                    continue;
                }

                let token_start_in_segment = if adjusted_token_start >= segment_start {
                    // Token starts within or after this segment
                    // Subtract segment_start to get position relative to displayed text
                    (adjusted_token_start - segment_start) + indent_offset
                } else {
                    // Token starts before this segment, clamp to beginning
                    indent_offset
                };
                
                let token_end_in_segment = if adjusted_token_end <= segment_end {
                    // Token ends within this segment
                    // Subtract segment_start to get position relative to displayed text
                    (adjusted_token_end - segment_start) + indent_offset
                } else {
                    // Token extends beyond segment, clamp to end
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
    // Draw find/replace bar
    fn draw_find_replace_bar(&self, f: &mut ratatui::Frame, area: Rect, editor: &Editor, config: &Config) {
        use crate::editor::FindReplaceFocus;
        
        let border_color = config.theme.parse_color(&config.theme.colors.border_active);
        let bg_color = config.theme.parse_color(&config.theme.colors.modal_bg);
        let fg_color = config.theme.parse_color(&config.theme.colors.modal_fg);
        let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
        let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
        
        let block = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(bg_color));
        
        f.render_widget(block, area);
        
        let inner_area = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
        
        let sections = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),  // Find section
                Constraint::Percentage(50),  // Replace section
            ])
            .split(inner_area);
        
        // Draw find field
        let find_label = "Find: ";
        let find_text = &editor.find_query;
        let find_style = if editor.find_replace_focus == FindReplaceFocus::FindField {
            Style::default().bg(selection_bg).fg(selection_fg)
        } else {
            Style::default().fg(fg_color)
        };
        
        // Create spans for find field with selection
        let mut find_spans = vec![Span::styled(find_label, find_style)];
        
        if let Some(sel_start) = editor.find_selection_start {
            let sel_end = editor.find_cursor_pos;
            let (start, end) = if sel_start <= sel_end { (sel_start, sel_end) } else { (sel_end, sel_start) };
            
            // Text before selection
            if start > 0 {
                find_spans.push(Span::styled(&find_text[..start], find_style));
            }
            // Selected text
            if start < end && end <= find_text.len() {
                find_spans.push(Span::styled(&find_text[start..end], Style::default().bg(selection_bg).fg(selection_fg)));
            }
            // Text after selection
            if end < find_text.len() {
                find_spans.push(Span::styled(&find_text[end..], find_style));
            }
        } else {
            // No selection, just render the text
            find_spans.push(Span::styled(find_text, find_style));
        }
        
        let find_paragraph = Paragraph::new(Line::from(find_spans))
            .alignment(Alignment::Left);
        
        f.render_widget(find_paragraph, sections[0]);
        
        // Draw replace field
        let replace_label = "Replace: ";
        let replace_text = &editor.replace_text;
        let replace_style = if editor.find_replace_focus == FindReplaceFocus::ReplaceField {
            Style::default().bg(selection_bg).fg(selection_fg)
        } else {
            Style::default().fg(fg_color)
        };
        
        // Create spans for replace field with selection
        let mut replace_spans = vec![Span::styled(replace_label, replace_style)];
        
        if let Some(sel_start) = editor.replace_selection_start {
            let sel_end = editor.replace_cursor_pos;
            let (start, end) = if sel_start <= sel_end { (sel_start, sel_end) } else { (sel_end, sel_start) };
            
            // Text before selection
            if start > 0 {
                replace_spans.push(Span::styled(&replace_text[..start], replace_style));
            }
            // Selected text
            if start < end && end <= replace_text.len() {
                replace_spans.push(Span::styled(&replace_text[start..end], Style::default().bg(selection_bg).fg(selection_fg)));
            }
            // Text after selection
            if end < replace_text.len() {
                replace_spans.push(Span::styled(&replace_text[end..], replace_style));
            }
        } else {
            // No selection, just render the text
            replace_spans.push(Span::styled(replace_text, replace_style));
        }
        
        let replace_paragraph = Paragraph::new(Line::from(replace_spans))
            .alignment(Alignment::Left);
        
        f.render_widget(replace_paragraph, sections[1]);
        
        // Show match count
        if let Some((current, total)) = editor.get_find_status() {
            let match_info = if total > 0 {
                format!(" {}/{} ", current, total)
            } else {
                " No matches ".to_string()
            };
            
            let match_info_width = match_info.len() as u16;
            let match_info_area = Rect {
                x: area.x + area.width - match_info_width - 1,
                y: inner_area.y,
                width: match_info_width,
                height: 1,
            };
            
            let match_paragraph = Paragraph::new(match_info)
                .style(Style::default().fg(fg_color))
                .alignment(Alignment::Right);
            
            f.render_widget(match_paragraph, match_info_area);
        }
        
        // Set cursor position based on focus
        match editor.find_replace_focus {
            FindReplaceFocus::FindField => {
                let cursor_x = sections[0].x + find_label.len() as u16 + editor.find_cursor_pos as u16;
                if cursor_x < sections[0].x + sections[0].width {
                    f.set_cursor_position((cursor_x, sections[0].y));
                }
            }
            FindReplaceFocus::ReplaceField => {
                let cursor_x = sections[1].x + replace_label.len() as u16 + editor.replace_cursor_pos as u16;
                if cursor_x < sections[1].x + sections[1].width {
                    f.set_cursor_position((cursor_x, sections[1].y));
                }
            }
            FindReplaceFocus::Editor => {
                // Cursor will be set by the editor drawing code
            }
        }
    }

    fn draw_help_modal(&self, f: &mut ratatui::Frame, editor: &Editor, config: &Config) {
        let area = f.area();
        let modal_width = 70;
        let modal_height = 30;
        
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

        // Full help content
        let help_content = vec![
            Line::from(""),
            Line::from("⌨️  HELP NAVIGATION"),
            Line::from("  ↑/↓ or j/k     Scroll help content"),
            Line::from("  Page Up/Down   Scroll by page"),
            Line::from("  Home/End       Jump to top/bottom"),
            Line::from(""),
            Line::from("📝 EDITOR COMMANDS"),
            Line::from("  Ctrl+S         Save file"),
            Line::from("  Ctrl+Alt+S     Save as (with new filename)"),
            Line::from("  Ctrl+O         Open file (TODO)"),
            Line::from("  Ctrl+Q         Quit editor"),
            Line::from("  Ctrl+Z         Undo"),
            Line::from("  Ctrl+Y         Redo"),
            Line::from(""),
            Line::from("🔍 FIND & REPLACE"),
            Line::from("  Ctrl+F         Open find/replace (or next match)"),
            Line::from("  Ctrl+Alt+F     Previous match"),
            Line::from("  Ctrl+H         Replace current match"),
            Line::from("  Ctrl+Alt+H     Replace all matches"),
            Line::from("  Tab            Toggle between find/replace/editor"),
            Line::from("  Ctrl+A         Select all text in current field"),
            Line::from("  Ctrl+C         Copy selected text"),
            Line::from("  Ctrl+V         Paste text"),
            Line::from("  Esc            Close find/replace"),
            Line::from(""),
            Line::from("🔤 CURSOR MOVEMENT"),
            Line::from("  Arrow Keys     Move cursor"),
            Line::from("  Home           Move to beginning of line"),
            Line::from("  End            Move to end of line"),
            Line::from("  Page Up/Down   Move by page"),
            Line::from("  Ctrl+PgUp      Move to previous paragraph"),
            Line::from("  Ctrl+PgDown    Move to next paragraph"),
            Line::from(""),
            Line::from("✏️  TEXT EDITING"),
            Line::from("  Enter          Insert new line"),
            Line::from("  Tab            Indent line/selection (4 spaces)"),
            Line::from("  Shift+Tab      Dedent line/selection"),
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
            Line::from("  Scroll Wheel   Scroll viewport up/down"),
            Line::from("  Shift+Scroll   Scroll horizontally (when word wrap off)"),
            Line::from(""),
            Line::from("🎨 CUSTOMIZATION"),
            Line::from("  F1             Show this help"),
            Line::from("  F2/F3          Increase/Decrease vertical margins"),
            Line::from("  F4/F5          Increase/Decrease horizontal margins"),
            Line::from("  F6             Toggle word wrap"),
            Line::from("  F7             Toggle gutter (None/Absolute/Relative)"),
            Line::from("  Ctrl+L         Change language/syntax highlighting"),
            Line::from("  Ctrl+T         Change color theme"),
            Line::from(""),
            Line::from("📋 BULLET JOURNAL"),
            Line::from("  Ctrl+Left      Insert todo bullet (□)"),
            Line::from("  Ctrl+Down      Insert in-progress bullet (◪)"),
            Line::from("  Ctrl+Right     Insert done bullet (■)"),
            Line::from(""),
            Line::from("💡 FEATURES"),
            Line::from("  • Syntax highlighting for 35+ languages"),
            Line::from("  • Word wrapping with smart cursor movement"),
            Line::from("  • Manual save only (Ctrl+S) - no auto-save by default"),
            Line::from("  • Configurable margins (0 to any size)"),
            Line::from("  • Multiple color themes with live preview"),
            Line::from("  • Line numbers (absolute/relative)"),
            Line::from("  • Undo/Redo with intelligent grouping"),
            Line::from("  • Paragraph navigation"),
            Line::from("  • UTF-8 support with proper character handling"),
            Line::from(""),
            Line::from(Span::styled("Press ESC, F1, or Q to close this help", Style::default().add_modifier(Modifier::BOLD))),
        ];

        // Calculate visible content area (account for borders and title)
        let content_height = modal_area.height.saturating_sub(2) as usize;
        let total_lines = help_content.len();
        
        // Clamp scroll offset
        let max_scroll = total_lines.saturating_sub(content_height);
        let scroll_offset = editor.help_scroll_offset.min(max_scroll);
        
        // Get visible lines
        let visible_end = (scroll_offset + content_height).min(total_lines);
        let visible_lines: Vec<Line> = help_content[scroll_offset..visible_end].to_vec();

        // Create scroll indicator
        let scroll_indicator = if total_lines > content_height {
            let scroll_percentage = if max_scroll > 0 {
                (scroll_offset as f32 / max_scroll as f32 * 100.0) as u32
            } else {
                0
            };
            format!(" [{}% - Line {}/{}]", scroll_percentage, scroll_offset + 1, total_lines)
        } else {
            String::new()
        };

        let help_paragraph = Paragraph::new(visible_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(format!("Thyme Editor - Help{}", scroll_indicator))
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
        editor: &Editor,
        horizontal_offset: usize,
    ) -> Line<'static> {
        let text_chars: Vec<char> = text.chars().collect();
        let text_len = text_chars.len();
        
        // Calculate indentation offset for continuation lines
        let indent_offset = if segment_start > 0 {
            // Count leading spaces/tabs in the displayed text
            text_chars.iter().take_while(|&&c| c == ' ' || c == '\t').count()
        } else {
            0
        };
        
        // The actual content length in the original text (without added indentation)
        let original_content_len = text_len - indent_offset;
        let segment_end = segment_start + original_content_len;
        
        let mut spans = Vec::new();
        
        // Calculate selection boundaries within this segment
        // Adjust for horizontal offset when not in word wrap mode
        let selection_boundaries = if horizontal_offset > 0 {
            self.calculate_selection_boundaries(
                line_idx, segment_start, segment_end, sel_start, sel_end, horizontal_offset
            )
        } else {
            self.calculate_selection_boundaries(
                line_idx, segment_start, segment_end, sel_start, sel_end, 0
            )
        };
        
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
        
        // Add indentation offset as first boundary if present
        if indent_offset > 0 {
            all_boundaries.push(indent_offset);
        }
        
        // Add selection boundaries (adjusted for display)
        for boundary in selection_boundaries {
            if boundary >= segment_start && boundary <= segment_end {
                all_boundaries.push(boundary - segment_start + indent_offset);
            }
        }
        
        // Add find match boundaries if in find mode
        if editor.find_replace_mode && !editor.find_query.is_empty() {
            for &(match_line, match_start, match_end) in &editor.find_matches {
                if match_line == line_idx {
                    if match_start >= segment_start && match_start <= segment_end {
                        all_boundaries.push(match_start - segment_start + indent_offset);
                    }
                    if match_end >= segment_start && match_end <= segment_end {
                        all_boundaries.push(match_end - segment_start + indent_offset);
                    }
                }
            }
        }
        
        // Add token boundaries (adjusted for display and horizontal offset)
        for token in &tokens {
            // Adjust token positions for horizontal scrolling
            let adjusted_token_start = token.start.saturating_sub(horizontal_offset);
            let adjusted_token_end = token.end.saturating_sub(horizontal_offset);
            
            if adjusted_token_start >= segment_start && adjusted_token_start <= segment_end {
                all_boundaries.push(adjusted_token_start - segment_start + indent_offset);
            }
            if adjusted_token_end >= segment_start && adjusted_token_end <= segment_end {
                all_boundaries.push(adjusted_token_end - segment_start + indent_offset);
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
                // Map display positions back to original text positions
                let original_start = if start >= indent_offset {
                    segment_start + start - indent_offset
                } else {
                    segment_start
                };
                let original_end = if end > indent_offset {
                    segment_start + end - indent_offset
                } else {
                    segment_start
                };
                
                let is_selected = self.is_segment_selected(
                    line_idx, original_start, original_end,
                    sel_start, sel_end, horizontal_offset
                );
                
                // Find the appropriate token style for this segment
                let style = self.get_segment_style(
                    &tokens, original_start, original_end,
                    is_selected, config, line_idx, editor, horizontal_offset
                );
                
                spans.push(Span::styled(segment_text, style));
            }
        }
        
        if spans.is_empty() {
            let is_selected = self.is_segment_selected(
                line_idx, segment_start, segment_end, sel_start, sel_end, horizontal_offset
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
        horizontal_offset: usize,
    ) -> Vec<usize> {
        let mut boundaries = Vec::new();
        
        // Adjust selection columns for horizontal offset
        let adjusted_sel_start_col = sel_start.column.saturating_sub(horizontal_offset);
        let adjusted_sel_end_col = sel_end.column.saturating_sub(horizontal_offset);
        
        // Single line selection
        if sel_start.line == sel_end.line && sel_start.line == line_idx {
            if adjusted_sel_start_col >= segment_start && adjusted_sel_start_col <= segment_end {
                boundaries.push(adjusted_sel_start_col);
            }
            if adjusted_sel_end_col >= segment_start && adjusted_sel_end_col <= segment_end {
                boundaries.push(adjusted_sel_end_col);
            }
        }
        // Multi-line selection
        else {
            if line_idx == sel_start.line && adjusted_sel_start_col >= segment_start && adjusted_sel_start_col <= segment_end {
                boundaries.push(adjusted_sel_start_col);
            }
            if line_idx == sel_end.line && adjusted_sel_end_col >= segment_start && adjusted_sel_end_col <= segment_end {
                boundaries.push(adjusted_sel_end_col);
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
        horizontal_offset: usize,
    ) -> bool {
        // Check for dummy selection (used when we only have find matches)
        if sel_start.line == usize::MAX && sel_end.line == usize::MAX {
            return false;
        }
        
        // Check if line is within selection range
        if line_idx < sel_start.line || line_idx > sel_end.line {
            return false;
        }
        
        // Adjust selection columns for horizontal offset when needed
        let adjusted_sel_start_col = if horizontal_offset > 0 {
            sel_start.column.saturating_sub(horizontal_offset)
        } else {
            sel_start.column
        };
        let adjusted_sel_end_col = if horizontal_offset > 0 {
            sel_end.column.saturating_sub(horizontal_offset)
        } else {
            sel_end.column
        };
        
        // Single line selection
        if sel_start.line == sel_end.line {
            if line_idx == sel_start.line {
                // Segment must be entirely within selection bounds
                return start_col >= adjusted_sel_start_col && end_col <= adjusted_sel_end_col;
            }
            return false;
        }
        
        // Multi-line selection
        if line_idx == sel_start.line {
            // First line: segment must start at or after selection start
            return start_col >= adjusted_sel_start_col;
        } else if line_idx == sel_end.line {
            // Last line: segment must end at or before selection end
            return end_col <= adjusted_sel_end_col;
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
        line_idx: usize,
        editor: &Editor,
        horizontal_offset: usize,
    ) -> Style {
        // Check if this segment is a find match
        let is_find_match = if editor.find_replace_mode && !editor.find_query.is_empty() {
            editor.find_matches.iter().any(|&(match_line, match_start, match_end)| {
                match_line == line_idx && 
                start_col >= match_start && 
                end_col <= match_end
            })
        } else {
            false
        };
        
        // Check if this segment is the current find match
        let is_current_match = if let Some(current_idx) = editor.current_match_index {
            editor.find_matches.get(current_idx)
                .map(|&(match_line, match_start, match_end)| {
                    match_line == line_idx && 
                    start_col >= match_start && 
                    end_col <= match_end
                })
                .unwrap_or(false)
        } else {
            false
        };
        
        // Find the token that contains this segment
        // Need to adjust for horizontal offset when matching tokens
        let token_style = tokens.iter()
            .find(|token| {
                // Adjust token positions for horizontal scrolling
                let adjusted_start = token.start.saturating_sub(horizontal_offset);
                let adjusted_end = token.end.saturating_sub(horizontal_offset);
                adjusted_start <= start_col && adjusted_end >= end_col
            })
            .map(|token| self.get_token_style(&token.token_type, config))
            .unwrap_or_else(|| {
                let normal_color = config.theme.parse_color(&config.theme.colors.normal);
                Style::default().fg(normal_color)
            });
        
        // Apply styles in order of priority: selection > current match > match > token
        if is_selected {
            let selection_bg = config.theme.parse_color(&config.theme.colors.selection_bg);
            let selection_fg = config.theme.parse_color(&config.theme.colors.selection_fg);
            Style::default().bg(selection_bg).fg(selection_fg)
        } else if is_current_match {
            let match_bg = config.theme.parse_color(&config.theme.colors.find_current_match_bg);
            let match_fg = config.theme.parse_color(&config.theme.colors.find_current_match_fg);
            Style::default().bg(match_bg).fg(match_fg)
        } else if is_find_match {
            let match_bg = config.theme.parse_color(&config.theme.colors.find_match_bg);
            let match_fg = config.theme.parse_color(&config.theme.colors.find_match_fg);
            Style::default().bg(match_bg).fg(match_fg)
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
         _editor: &Editor,
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
    
    fn draw_filename_prompt_modal(&self, f: &mut ratatui::Frame, editor: &Editor, config: &Config) {
        let area = f.area();
        let modal_width = 50;
        let modal_height = 5;
        
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

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Save As ")
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(modal_bg));

        let inner_area = block.inner(modal_area);

        // Use consistent formatting without extra spaces
        let prompt_text = vec![
            Line::from(""),
            Line::from(format!("Filename: {}", editor.filename_prompt_text)),
        ];

        let paragraph = Paragraph::new(prompt_text)
            .style(Style::default().fg(modal_fg))
            .alignment(Alignment::Left);

        f.render_widget(block, modal_area);
        f.render_widget(paragraph, inner_area);

        // Position cursor after "Filename: " (10 characters)
        if inner_area.height >= 2 {
            f.set_cursor_position((
                inner_area.x + 10 + editor.filename_cursor_pos as u16,
                inner_area.y + 1
            ));
        }
    }

    fn draw_save_prompt_overlay(&self, f: &mut ratatui::Frame, config: &Config) {
        let area = f.area();
        let modal_width = 50;
        let modal_height = 10;
        
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
        let keyword_color = config.theme.parse_color(&config.theme.colors.keyword);

        // Create the prompt content
        let prompt_text = vec![
            Line::from(""),
            Line::from("You have unsaved changes."),
            Line::from(""),
            Line::from("Save before quitting?"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Y", Style::default().fg(keyword_color).add_modifier(Modifier::BOLD)),
                Span::raw(" - Save and quit"),
            ]),
            Line::from(vec![
                Span::styled("N", Style::default().fg(keyword_color).add_modifier(Modifier::BOLD)),
                Span::raw(" - Quit without saving"),
            ]),
            Line::from(vec![
                Span::styled("Esc", Style::default().fg(keyword_color).add_modifier(Modifier::BOLD)),
                Span::raw(" - Cancel"),
            ]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Save Changes? ")
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(modal_bg));

        let inner_area = block.inner(modal_area);
        
        let paragraph = Paragraph::new(prompt_text)
            .style(Style::default().fg(modal_fg))
            .alignment(Alignment::Center);

        f.render_widget(block, modal_area);
        f.render_widget(paragraph, inner_area);
    }
}
