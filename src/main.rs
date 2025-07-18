// src/main.rs

mod app;
mod buffer;
mod config;
mod cursor;
mod editor;
mod events;
mod syntax;
mod text_utils;
mod ui;
mod unicode_utils;

use anyhow::Result;
use app::App;
use crossterm::{
    execute,
    event::{EnableMouseCapture, DisableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // Create app and run
    let mut app = App::new().await?;
    let result = app.run(&mut terminal).await;
    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(), 
        SetTitle(""),  // Restore original terminal title
        LeaveAlternateScreen, 
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}
