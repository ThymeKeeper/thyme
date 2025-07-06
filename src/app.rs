// src/app.rs

use crate::{
    config::{Config, Theme},
    editor::Editor,
    events::{Event, EventHandler},
    ui::Ui,
    buffer::Buffer,
};
use anyhow::Result;
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton},
    execute,
    terminal::SetTitle,
};
use ratatui::{backend::Backend, Terminal, widgets::{Paragraph}, layout::Alignment};
use std::{io::stdout, path::PathBuf, time::Instant};

pub struct App {
    pub editor: Editor,
    pub config: Config,
    pub ui: Ui,
    pub event_handler: EventHandler,
    pub running: bool,
    pub last_save_check: Instant,
    pub last_terminal_size: (u16, u16),
    pub saved_theme: Option<Theme>, // For theme preview
    pub mouse_dragging: bool, // Track mouse drag state
    pub needs_full_redraw: bool, // Track when full terminal redraw is needed
}

impl App {
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let editor = Editor::new();
        let ui = Ui::new();
        let event_handler = EventHandler::new()?;

        let last_terminal_size = crossterm::terminal::size().unwrap_or((80, 24));

        Ok(Self {
            editor,
            config,
            ui,
            event_handler,
            running: true,
            last_save_check: Instant::now(),
            last_terminal_size,
            saved_theme: None,
            mouse_dragging: false,
            needs_full_redraw: false,
        })
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Load a file if provided as argument
        if let Some(arg) = std::env::args().nth(1) {
            self.editor.open_file(&PathBuf::from(arg)).await?;
            // Adjust viewport to respect scrolloff setting after loading
            self.editor.adjust_viewport_initial(&self.config, self.calculate_visible_lines());
        } else {
            self.editor.new_buffer();
            // Adjust viewport to respect scrolloff setting for new buffer
            self.editor.adjust_viewport_initial(&self.config, self.calculate_visible_lines());
        }
        
        // Set initial terminal title
        self.update_terminal_title();

        while self.running {
            // Check for terminal resize
            self.check_terminal_resize();
            // Force full clear and redraw if needed (e.g., after paragraph jumps)
            if self.needs_full_redraw {
                terminal.clear()?;
                self.needs_full_redraw = false;
            }

            // Draw UI
            if !self.editor.paste_in_progress {
                terminal.draw(|f| self.ui.draw(f, &self.editor, &self.config))?;
            } else if let Some(ref progress) = self.editor.paste_progress {
                // Show simple progress message
                terminal.draw(|f| {
                    let area = f.area();
                    let msg = Paragraph::new(progress.as_str())
                        .alignment(Alignment::Center);
                    f.render_widget(msg, area);
                })?;
            }

            // Handle events
            if let Some(event) = self.event_handler.next().await? {
                self.handle_event(event).await?;
            }

            // Check for auto-save
            if self.last_save_check.elapsed().as_secs() >= 1 {
                self.check_auto_save().await?;
                self.last_save_check = Instant::now();
            }
        }

        Ok(())
    }

    async fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => self.handle_key_event(key).await?,
            Event::Mouse(mouse) => self.handle_mouse_event(mouse).await?,
            Event::Tick => {
                // Update syntax highlighting if needed
                if let Some(buffer) = self.editor.current_buffer_mut() {
                    buffer.update_syntax_if_needed();
                }
            }
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle find/replace mode first
        if self.editor.find_replace_mode {
            return self.handle_find_replace_key(key).await;
        }
        
        // Handle help mode first
        if self.editor.help_mode {
            return self.handle_help_key(key).await;
        }

        // Handle language selection mode
        if self.editor.language_selection_mode {
            return self.handle_language_selection_key(key).await;
        }

        // Handle theme selection mode
        if self.editor.theme_selection_mode {
            return self.handle_theme_selection_key(key).await;
        }

        // Handle custom keybindings first
        if self.handle_custom_keybindings(key).await? {
            return Ok(());
        }

        // Calculate content width for word-wrap-aware movement
        let content_width = self.calculate_content_width();
        let visible_lines = self.calculate_visible_lines();
        
        // Handle standard editor keys
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.save_current_buffer().await?;
                self.update_terminal_title();
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // TODO: Implement file open dialog
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.select_all();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Err(e) = self.editor.copy_selection() {
                    eprintln!("Failed to copy: {}", e);
                }
            }
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Err(e) = self.editor.cut_selection() {
                    eprintln!("Failed to cut: {}", e);
                }
            }
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Err(e) = self.editor.paste_from_clipboard() {
                    eprintln!("Failed to paste: {}", e);
                }
            }
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.undo();
            }
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.redo();
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.enter_find_replace_mode();
            }
            // Arrow keys with optional Shift for selection
            KeyCode::Left => {
                let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                self.handle_cursor_movement_left(extend_selection, content_width);
            }
            KeyCode::Right => {
                let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                self.handle_cursor_movement_right(extend_selection, content_width);
            }
            KeyCode::Up => {
                let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                self.handle_cursor_movement_up(extend_selection, content_width);
            }
            KeyCode::Down => {
                let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                self.handle_cursor_movement_down(extend_selection, content_width);
            }
            KeyCode::Home => self.editor.move_cursor_home(),
            KeyCode::End => self.editor.move_cursor_end(),
            KeyCode::PageUp => {
                self.editor.move_cursor_page_up(&self.config, visible_lines);
                self.needs_full_redraw = true; // Force redraw after large jump
            }
            KeyCode::PageDown => {
                self.editor.move_cursor_page_down(&self.config, visible_lines);
                self.needs_full_redraw = true; // Force redraw after large jump
            }
            KeyCode::Backspace => self.editor.delete_char_backwards(content_width, &self.config, visible_lines),
            KeyCode::Delete => self.editor.delete_char_forwards(),
            KeyCode::Enter => self.editor.insert_newline(&self.config, visible_lines),
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.editor.dedent_lines();
                } else {
                    self.editor.indent_lines();
                }
            }
            KeyCode::BackTab => {
                // Some terminals send BackTab instead of Shift+Tab
                self.editor.dedent_lines();
            }
            KeyCode::Char(c) => self.editor.insert_char(c, content_width),
            _ => {}
        }

        Ok(())
    }

    // Handle keys when in find/replace mode
    async fn handle_find_replace_key(&mut self, key: KeyEvent) -> Result<()> {
        use crate::editor::FindReplaceFocus;
        
        match key.code {
            KeyCode::Esc => {
                self.editor.exit_find_replace_mode();
                self.needs_full_redraw = true;
            }
            
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    self.editor.find_previous(&self.config, self.calculate_visible_lines());
                } else {
                    self.editor.find_next(&self.config, self.calculate_visible_lines());
                }
            }
            
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    self.editor.replace_all_matches();
                } else {
                    self.editor.replace_current_match();
                    // DON'T call find_next here - replace_current_match already positions us correctly
                }
            }
            
            KeyCode::Tab => {
                self.editor.toggle_find_replace_focus();
            }
            
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.undo();
            }
            
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.redo();
            }
            
            KeyCode::Char(c) => {
                if self.editor.find_replace_focus != FindReplaceFocus::Editor {
                    self.editor.add_char_to_find_query(c);
                } else {
                    self.editor.insert_char(c, self.calculate_content_width());
                }
            }
            
            KeyCode::Backspace => {
                if self.editor.find_replace_focus != FindReplaceFocus::Editor {
                    self.editor.backspace_find_replace_field();
                } else {
                    self.editor.delete_char_backwards(
                        self.calculate_content_width(), 
                        &self.config, 
                        self.calculate_visible_lines()
                    );
                }
            }
            
            KeyCode::Enter => {
                if self.editor.find_replace_focus == FindReplaceFocus::FindField {
                    self.editor.find_next(&self.config, self.calculate_visible_lines());
                    self.editor.find_replace_focus = FindReplaceFocus::Editor;
                } else if self.editor.find_replace_focus == FindReplaceFocus::ReplaceField {
                    self.editor.replace_current_match();
                    // DON'T call find_next here either - replace_current_match positions us correctly
                } else {
                    self.editor.insert_newline(&self.config, self.calculate_visible_lines());
                }
            }
            
            KeyCode::Left => {
                if self.editor.find_replace_focus == FindReplaceFocus::Editor {
                    self.handle_cursor_movement_left(key.modifiers.contains(KeyModifiers::SHIFT), self.calculate_content_width());
                } else {
                    self.editor.move_find_replace_cursor_left();
                }
            }
            
            KeyCode::Right => {
                if self.editor.find_replace_focus == FindReplaceFocus::Editor {
                    self.handle_cursor_movement_right(key.modifiers.contains(KeyModifiers::SHIFT), self.calculate_content_width());
                } else {
                    self.editor.move_find_replace_cursor_right();
                }
            }
            
            KeyCode::Home => {
                if self.editor.find_replace_focus == FindReplaceFocus::Editor {
                    self.editor.move_cursor_home();
                } else {
                    self.editor.move_find_replace_cursor_home();
                }
            }
            
            KeyCode::End => {
                if self.editor.find_replace_focus == FindReplaceFocus::Editor {
                    self.editor.move_cursor_end();
                } else {
                    self.editor.move_find_replace_cursor_end();
                }
            }
            
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                if self.editor.find_replace_focus == FindReplaceFocus::Editor {
                    match key.code {
                        KeyCode::Up => self.handle_cursor_movement_up(key.modifiers.contains(KeyModifiers::SHIFT), self.calculate_content_width()),
                        KeyCode::Down => self.handle_cursor_movement_down(key.modifiers.contains(KeyModifiers::SHIFT), self.calculate_content_width()),
                        KeyCode::PageUp => {
                            self.editor.move_cursor_page_up(&self.config, self.calculate_visible_lines());
                            self.needs_full_redraw = true;
                        }
                        KeyCode::PageDown => {
                            self.editor.move_cursor_page_down(&self.config, self.calculate_visible_lines());
                            self.needs_full_redraw = true;
                        }
                        _ => {}
                    }
                }
            }
            
            _ => {}
        }
        
        Ok(())
    }

    // Handle keys when in language selection mode
    async fn handle_language_selection_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.editor.language_selection_up();
            }
            KeyCode::Down => {
                self.editor.language_selection_down();
            }
            KeyCode::Enter => {
                self.editor.apply_selected_language();
                self.needs_full_redraw = true; // Clear any artifacts from modal
            }
            KeyCode::Esc => {
                self.editor.exit_language_selection_mode();
                self.needs_full_redraw = true; // Clear any artifacts from modal
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Allow quit even in language selection mode
                self.running = false;
            }
            // Quick language selection with number keys (for visible items)
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let digit = c.to_digit(10).unwrap() as usize;
                let languages = Buffer::get_supported_languages();
                let scroll_offset = self.editor.language_selection_scroll_offset;
                let max_visible_items = 15;
                let visible_end = (scroll_offset + max_visible_items).min(languages.len());
                let visible_count = visible_end - scroll_offset;
                
                if digit > 0 && digit <= visible_count && !languages.is_empty() {
                    let visible_index = digit - 1;
                    let actual_index = scroll_offset + visible_index;
                    if actual_index < languages.len() {
                        self.editor.language_selection_index = actual_index;
                        self.editor.apply_selected_language();
                        self.needs_full_redraw = true; // Clear any artifacts from modal
                    }
                }
            }
            _ => {
                // Ignore other keys in language selection mode
            }
        }
        Ok(())
    }

    // Handle keys when in theme selection mode
    async fn handle_theme_selection_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.editor.theme_selection_up();
                self.preview_selected_theme();
            }
            KeyCode::Down => {
                self.editor.theme_selection_down();
                self.preview_selected_theme();
            }
            KeyCode::Enter => {
                if let Some(theme_filename) = self.editor.get_selected_theme() {
                    if theme_filename == "_default" {
                        // Reset to default theme
                        self.config.theme = Theme::default();
                        self.config.theme_name = Some("Default Dark".to_string());
                    } else {
                        // Load the selected theme
                        if let Err(e) = self.config.load_theme(theme_filename) {
                            eprintln!("Failed to load theme: {}", e);
                        }
                    }
                    self.editor.exit_theme_selection_mode();
                    self.saved_theme = None; // Clear saved theme
                    self.needs_full_redraw = true; // Clear any artifacts from modal
                    
                    // Save config with new theme
                    if let Err(e) = self.config.save() {
                        eprintln!("Failed to save config: {}", e);
                    }
                }
            }
            KeyCode::Esc => {
                // Restore saved theme
                if let Some(saved_theme) = self.saved_theme.take() {
                    self.config.theme = saved_theme;
                }
                self.editor.exit_theme_selection_mode();
                self.needs_full_redraw = true; // Clear any artifacts from modal
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Allow quit even in theme selection mode
                self.running = false;
            }
            // Quick theme selection with number keys (for visible items)
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let digit = c.to_digit(10).unwrap() as usize;
                let scroll_offset = self.editor.theme_selection_scroll_offset;
                let max_visible_items = 15;
                let visible_end = (scroll_offset + max_visible_items).min(self.editor.available_themes.len());
                let visible_count = visible_end - scroll_offset;
                
                if digit > 0 && digit <= visible_count && !self.editor.available_themes.is_empty() {
                    let visible_index = digit - 1;
                    let actual_index = scroll_offset + visible_index;
                    if actual_index < self.editor.available_themes.len() {
                        self.editor.theme_selection_index = actual_index;
                        self.preview_selected_theme();
                    
                        if let Some(theme_filename) = self.editor.get_selected_theme() {
                            if theme_filename == "_default" {
                                self.config.theme = Theme::default();
                                self.config.theme_name = Some("Default Dark".to_string());
                            } else {
                                if let Err(e) = self.config.load_theme(theme_filename) {
                                    eprintln!("Failed to load theme: {}", e);
                                }
                            }
                            self.editor.exit_theme_selection_mode();
                            self.saved_theme = None; // Clear saved theme
                            self.needs_full_redraw = true; // Clear any artifacts from modal
                            
                            // Save config with new theme
                            if let Err(e) = self.config.save() {
                                eprintln!("Failed to save config: {}", e);
                            }
                        }
                    }
                }
            }
            _ => {
                // Ignore other keys in theme selection mode
            }
        }
        Ok(())
    }

    // Handle keys when in help mode
    async fn handle_help_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Allow quit even in help mode
                self.running = false;
            }
            KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('q') => {
                self.editor.exit_help_mode();
                self.needs_full_redraw = true; // Clear any artifacts from modal
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // Scroll up
                if self.editor.help_scroll_offset > 0 {
                    self.editor.help_scroll_offset -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Scroll down - the actual limit will be checked in the UI drawing code
                self.editor.help_scroll_offset += 1;
            }
            KeyCode::PageUp => {
                // Scroll up by page
                self.editor.help_scroll_offset = self.editor.help_scroll_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                // Scroll down by page
                self.editor.help_scroll_offset += 10;
            }
            KeyCode::Home => {
                // Go to top
                self.editor.help_scroll_offset = 0;
            }
            KeyCode::End => {
                // Go to bottom - will be clamped in UI code
                self.editor.help_scroll_offset = usize::MAX;
            }
            _ => {
                // Ignore other keys in help mode
            }
        }
        Ok(())
    }

    fn preview_selected_theme(&mut self) {
        if let Some(theme_filename) = self.editor.get_selected_theme() {
            if theme_filename == "_default" {
                self.config.theme = Theme::default();
            } else {
                // Try to load the theme for preview
                let mut temp_config = Config::default();
                if let Err(e) = temp_config.load_theme(theme_filename) {
                    eprintln!("Failed to preview theme: {}", e);
                } else {
                    self.config.theme = temp_config.theme;
                }
            }
        }
    }

    fn calculate_content_width(&self) -> usize {
        let terminal_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);
        
        let content_width = terminal_width
            .saturating_sub((self.config.margins.horizontal * 2) as usize); // editor margins only
            // No outer layout margin or border subtraction
            
        // Ensure minimum width to prevent issues
        content_width.max(10)
    }
    
    fn calculate_visible_lines(&self) -> usize {
        let terminal_height = crossterm::terminal::size()
            .map(|(_, h)| h as usize)
            .unwrap_or(24);
        
        // Account for status line (1 line) and vertical margins
        let visible_lines = terminal_height
            .saturating_sub(1) // status line
            .saturating_sub((self.config.margins.vertical * 2) as usize); // vertical margins
            
        // Ensure minimum height
        visible_lines.max(1)
    }

    async fn handle_custom_keybindings(&mut self, key: KeyEvent) -> Result<bool> {
        let keybindings = &self.config.keybindings;

        if key == keybindings.increase_vertical_margin {
            self.config.margins.vertical = self.config.margins.vertical.saturating_add(1);
            // Recalculate viewport to maintain scrolloff with new margins
            let visible_lines = self.calculate_visible_lines();
            self.editor.adjust_viewport(&self.config, visible_lines);
            return Ok(true);
        }

        if key == keybindings.decrease_vertical_margin {
            self.config.margins.vertical = self.config.margins.vertical.saturating_sub(1);
            // Recalculate viewport to maintain scrolloff with new margins
            let visible_lines = self.calculate_visible_lines();
            self.editor.adjust_viewport(&self.config, visible_lines);
            return Ok(true);
        }

        if key == keybindings.increase_horizontal_margin {
            self.config.margins.horizontal = self.config.margins.horizontal.saturating_add(1);
            self.reset_preferred_column();
            // Recalculate viewport in case content width changes affect wrapped lines
            let visible_lines = self.calculate_visible_lines();
            self.editor.adjust_viewport(&self.config, visible_lines);
            return Ok(true);
        }

        if key == keybindings.decrease_horizontal_margin {
            self.config.margins.horizontal = self.config.margins.horizontal.saturating_sub(1);
            self.reset_preferred_column();
            // Recalculate viewport in case content width changes affect wrapped lines
            let visible_lines = self.calculate_visible_lines();
            self.editor.adjust_viewport(&self.config, visible_lines);
            return Ok(true);
        }

        if key == keybindings.toggle_word_wrap {
            self.config.word_wrap = !self.config.word_wrap;
            self.reset_preferred_column();
            return Ok(true);
        }

        if key == keybindings.toggle_gutter {
            self.config.toggle_gutter();
            return Ok(true);
        }

        // Language selection keybinding
        if key == keybindings.language_selection {
            self.editor.enter_language_selection_mode();
            self.needs_full_redraw = true; // Clear screen before showing modal
            return Ok(true);
        }

        // Theme selection keybinding
        if key == keybindings.theme_selection {
            // Save current theme before entering selection mode
            self.saved_theme = Some(self.config.theme.clone());
            let current_theme_name = &self.config.theme.name;
            if let Err(e) = self.editor.enter_theme_selection_mode(current_theme_name) {
                eprintln!("Failed to enter theme selection mode: {}", e);
            }
            self.needs_full_redraw = true; // Clear screen before showing modal
            return Ok(true);
        }

        // Help keybinding
        if key == keybindings.help {
            self.editor.enter_help_mode();
            self.needs_full_redraw = true; // Clear screen before showing modal
            return Ok(true);
        }

        // Bullet journal hotkeys
        if key == keybindings.bullet_todo {
            self.editor.insert_char('□', self.calculate_content_width());
            return Ok(true);
        }

        if key == keybindings.bullet_in_progress {
            self.editor.insert_char('◪', self.calculate_content_width());
            return Ok(true);
        }

        if key == keybindings.bullet_done {
            self.editor.insert_char('■', self.calculate_content_width());
            return Ok(true);
        }

        // Paragraph navigation
        if key == keybindings.paragraph_up {
            self.editor.move_to_paragraph_up(&self.config, self.calculate_visible_lines());
            self.needs_full_redraw = true; // Force full redraw after viewport jump
            return Ok(true);
        }

        if key == keybindings.paragraph_down {
            self.editor.move_to_paragraph_down(&self.config, self.calculate_visible_lines());
            self.needs_full_redraw = true; // Force full redraw after viewport jump
            return Ok(true);
        }

        // Line movement
        if key == keybindings.move_line_up {
            self.editor.move_lines_up();
            self.editor.adjust_viewport(&self.config, self.calculate_visible_lines());
            return Ok(true);
        }

        if key == keybindings.move_line_down {
            self.editor.move_lines_down();
            self.editor.adjust_viewport(&self.config, self.calculate_visible_lines());
            return Ok(true);
        }

        Ok(false)
    }

    async fn check_auto_save(&mut self) -> Result<()> {
        let should_save = if let Some(buffer) = self.editor.current_buffer() {
            buffer.should_auto_save(&self.config)
        } else {
            false
        };
        
        if should_save {
            self.editor.save_current_buffer().await?;
            if let Some(buffer) = self.editor.current_buffer_mut() {
                buffer.mark_auto_saved();
            }
        }
        Ok(())
    }

    fn check_terminal_resize(&mut self) {
        if let Ok(current_size) = crossterm::terminal::size() {
            if current_size != self.last_terminal_size {
                self.reset_preferred_column();
                self.last_terminal_size = current_size;
                self.needs_full_redraw = true; // Force full redraw after resize
            }
        }
    }

    fn reset_preferred_column(&mut self) {
        if let Some(buffer) = self.editor.current_buffer_mut() {
            buffer.reset_preferred_column();
        }
    }
    
    fn update_terminal_title(&self) {
        if let Some(buffer) = self.editor.current_buffer() {
            let title = if let Some(path) = &buffer.file_path {
                if let Some(filename) = path.file_name() {
                    filename.to_string_lossy().to_string()
                } else {
                    "[No Name]".to_string()
                }
            } else {
                "[No Name]".to_string()
            };
            
            // Set terminal title
            let _ = execute!(stdout(), SetTitle(title));
        } else {
            let _ = execute!(stdout(), SetTitle(""));
        }
    }
    
    async fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Left click - move cursor or extend selection
                let extend_selection = mouse.modifiers.contains(KeyModifiers::SHIFT);
                
                if extend_selection {
                    // Shift+click - extend selection to mouse position
                    self.editor.handle_shift_click(mouse.column, mouse.row, &self.config, self.calculate_visible_lines());
                } else {
                    // Regular click - move cursor and clear selection
                    self.editor.handle_regular_click(mouse.column, mouse.row, &self.config, self.calculate_visible_lines());
                    // Prepare for potential drag selection
                    self.mouse_dragging = true;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Mouse drag - create/extend selection
                if self.mouse_dragging {
                    self.editor.handle_mouse_drag(mouse.column, mouse.row, &self.config, self.calculate_visible_lines());
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                // Mouse button up - stop dragging
                self.mouse_dragging = false;
            }
            MouseEventKind::ScrollDown => {
                // Mouse wheel scroll down - scroll viewport down
                self.editor.scroll_viewport_down(3, &self.config, self.calculate_visible_lines());
            }
            MouseEventKind::ScrollUp => {
                // Mouse wheel scroll up - scroll viewport up  
                self.editor.scroll_viewport_up(3, &self.config, self.calculate_visible_lines());
            }
            MouseEventKind::ScrollLeft => {
                // Trackpad horizontal scroll left - scroll content left
                if !self.config.word_wrap {
                    self.editor.scroll_left(3);
                }
            }
            MouseEventKind::ScrollRight => {
                // Trackpad horizontal scroll right - scroll content right  
                if !self.config.word_wrap {
                    self.editor.scroll_right(3, self.calculate_content_width());
                }
            }
            _ => {
                // Ignore other mouse events
            }
        }
        Ok(())
    }
    
    fn handle_cursor_movement_left(&mut self, extend_selection: bool, content_width: usize) {
        if let Some(buffer) = self.editor.current_buffer_mut() {
            if extend_selection && !buffer.cursor.has_selection() {
                buffer.cursor.start_selection();
            } else if !extend_selection {
                buffer.cursor.clear_selection();
            }
        }
        self.editor.move_cursor_left(content_width, &self.config, self.calculate_visible_lines());
    }
    
    fn handle_cursor_movement_right(&mut self, extend_selection: bool, content_width: usize) {
        if let Some(buffer) = self.editor.current_buffer_mut() {
            if extend_selection && !buffer.cursor.has_selection() {
                buffer.cursor.start_selection();
            } else if !extend_selection {
                buffer.cursor.clear_selection();
            }
        }
        self.editor.move_cursor_right(content_width, &self.config, self.calculate_visible_lines());
    }
    
    fn handle_cursor_movement_up(&mut self, extend_selection: bool, content_width: usize) {
        if let Some(buffer) = self.editor.current_buffer_mut() {
            if extend_selection && !buffer.cursor.has_selection() {
                buffer.cursor.start_selection();
            } else if !extend_selection {
                buffer.cursor.clear_selection();
            }
        }
        self.editor.move_cursor_up(self.config.word_wrap, content_width, &self.config, self.calculate_visible_lines());
    }
    
    fn handle_cursor_movement_down(&mut self, extend_selection: bool, content_width: usize) {
        if let Some(buffer) = self.editor.current_buffer_mut() {
            if extend_selection && !buffer.cursor.has_selection() {
                buffer.cursor.start_selection();
            } else if !extend_selection {
                buffer.cursor.clear_selection();
            }
        }
        self.editor.move_cursor_down(self.config.word_wrap, content_width, &self.config, self.calculate_visible_lines());
    }
}