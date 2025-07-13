// src/events.rs

use anyhow::Result;
use crossterm::event::{poll, read, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent, KeyCode};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
}

pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<Event>,
    _handles: Vec<tokio::task::JoinHandle<()>>,
}

impl EventHandler {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let event_sender = sender.clone();
        let event_handle = tokio::spawn(async move {
            loop {
                if let Ok(true) = poll(Duration::from_millis(16)) {
                    if let Ok(event) = read() {
                        match event {
                            CrosstermEvent::Key(key) => {
                                // Determine if this key event should be sent through
                                if should_process_key_event(&key) {
                                    if event_sender.send(Event::Key(key)).is_err() {
                                        break;
                                    }
                                }
                            }
                            CrosstermEvent::Mouse(mouse) => {
                                if event_sender.send(Event::Mouse(mouse)).is_err() {
                                    break;
                                }
                            }
                            _ => {} // Ignore other events
                        }
                    }
                } else {
                    // Send tick event when no input is available
                    if event_sender.send(Event::Tick).is_err() {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(16)).await;
            }
        });
        
        Ok(Self {
            receiver,
            _handles: vec![event_handle],
        })
    }
    
    pub async fn next(&mut self) -> Result<Option<Event>> {
        Ok(self.receiver.recv().await)
    }
}

/// Determines if a key event should be processed
/// This allows home row mods to work while preventing duplicate character insertion
fn should_process_key_event(key: &KeyEvent) -> bool {
    match key.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => {
            // Always process Press and Repeat events
            true
        }
        KeyEventKind::Release => {
            // For Release events, only process special keys that might be used by home row mods
            // or that need release events for proper functionality
            matches!(
                key.code,
                KeyCode::Modifier(_) |  // Any modifier key
                KeyCode::Esc |          // Often used in modal interfaces
                KeyCode::CapsLock |     // State-based key
                KeyCode::NumLock |      // State-based key
                KeyCode::ScrollLock     // State-based key
            )
        }
    }
}

// Optional: If you need more fine-grained control for specific home row mod implementations
#[allow(dead_code)]
fn should_process_key_event_advanced(key: &KeyEvent) -> bool {
    match key.kind {
        KeyEventKind::Press => true,
        KeyEventKind::Repeat => true,
        KeyEventKind::Release => {
            // Process release events for:
            match key.code {
                // Modifier keys (needed for home row mods)
                KeyCode::Modifier(_) => true,
                
                // State keys
                KeyCode::CapsLock | KeyCode::NumLock | KeyCode::ScrollLock => true,
                
                // Special keys that might be part of home row mod combos
                KeyCode::Esc => true,
                
                // Function keys (some home row mods use these)
                KeyCode::F(_) => true,
                
                // Don't process release events for regular characters
                // This prevents duplicate character insertion
                KeyCode::Char(_) => false,
                
                // Don't process release for other keys either
                _ => false,
            }
        }
    }
}