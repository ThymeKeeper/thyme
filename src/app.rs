// src/app.rs

use crate::{
    config::Config,
    editor::Editor,
    events::{Event, EventHandler},
    ui::Ui,
    buffer::Buffer,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::{path::PathBuf, time::Instant};

pub struct App {
    pub editor: Editor,
    pub config: Config,
    pub ui: Ui,
    pub event_handler: EventHandler,
    pub running: bool,
    pub last_save_check: Instant,
    pub last_terminal_size: (u16, u16),
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
        })
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Load a file if provided as argument
        if let Some(arg) = std::env::args().nth(1) {
            self.editor.open_file(&PathBuf::from(arg)).await?;
        } else {
            self.editor.new_buffer();
        }

        while self.running {
            // Check for terminal resize
            self.check_terminal_resize();

            // Draw UI
            terminal.draw(|f| self.ui.draw(f, &self.editor, &self.config))?;

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
        // NEW: Handle language selection mode first
        if self.editor.language_selection_mode {
            return self.handle_language_selection_key(key).await;
        }

        // Handle custom keybindings first
        if self.handle_custom_keybindings(key).await? {
            return Ok(());
        }

        // Calculate content width for word-wrap-aware movement
        let content_width = self.calculate_content_width();
        
        // Handle standard editor keys
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.editor.save_current_buffer().await?;
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // TODO: Implement file open dialog
            }
            // Language selection mode trigger is handled in custom keybindings
            KeyCode::Left => self.editor.move_cursor_left(content_width),
            KeyCode::Right => self.editor.move_cursor_right(content_width),
            KeyCode::Up => self.editor.move_cursor_up(self.config.word_wrap, content_width),
            KeyCode::Down => self.editor.move_cursor_down(self.config.word_wrap, content_width),
            KeyCode::Home => self.editor.move_cursor_home(),
            KeyCode::End => self.editor.move_cursor_end(),
            KeyCode::PageUp => self.editor.move_cursor_page_up(),
            KeyCode::PageDown => self.editor.move_cursor_page_down(),
            KeyCode::Backspace => self.editor.delete_char_backwards(content_width),
            KeyCode::Delete => self.editor.delete_char_forwards(),
            KeyCode::Enter => self.editor.insert_newline(),
            KeyCode::Tab => self.editor.insert_tab(content_width),
            KeyCode::Char(c) => self.editor.insert_char(c, content_width),
            _ => {}
        }

        Ok(())
    }

    // NEW: Handle keys when in language selection mode
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
            }
            KeyCode::Esc => {
                self.editor.exit_language_selection_mode();
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Allow quit even in language selection mode
                self.running = false;
            }
            // NEW: Quick language selection with number keys
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let digit = c.to_digit(10).unwrap() as usize;
                let languages = Buffer::get_supported_languages();
                if digit > 0 && digit <= languages.len() {
                    self.editor.language_selection_index = digit - 1;
                    self.editor.apply_selected_language();
                }
            }
            _ => {
                // Ignore other keys in language selection mode
            }
        }
        Ok(())
    }

    fn calculate_content_width(&self) -> usize {
        let terminal_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);
        
        let content_width = terminal_width
            .saturating_sub(2) // outer margins
            .saturating_sub((self.config.margins.horizontal * 2) as usize) // editor margins
            .saturating_sub(2); // editor borders
            
        // Ensure minimum width to prevent issues
        content_width.max(10)
    }

    async fn handle_custom_keybindings(&mut self, key: KeyEvent) -> Result<bool> {
        let keybindings = &self.config.keybindings;

        if key == keybindings.increase_vertical_margin {
            self.config.margins.vertical = self.config.margins.vertical.saturating_add(1);
            return Ok(true);
        }

        if key == keybindings.decrease_vertical_margin {
            self.config.margins.vertical = self.config.margins.vertical.saturating_sub(1);
            return Ok(true);
        }

        if key == keybindings.increase_horizontal_margin {
            self.config.margins.horizontal = self.config.margins.horizontal.saturating_add(1);
            self.reset_preferred_column();
            return Ok(true);
        }

        if key == keybindings.decrease_horizontal_margin {
            self.config.margins.horizontal = self.config.margins.horizontal.saturating_sub(1);
            self.reset_preferred_column();
            return Ok(true);
        }

        if key == keybindings.toggle_word_wrap {
            self.config.word_wrap = !self.config.word_wrap;
            self.reset_preferred_column();
            return Ok(true);
        }

        // NEW: Language selection keybinding
        if key == keybindings.language_selection {
            self.editor.enter_language_selection_mode();
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
            }
        }
    }

    fn reset_preferred_column(&mut self) {
        if let Some(buffer) = self.editor.current_buffer_mut() {
            buffer.reset_preferred_column();
        }
    }
}
