// src/events.rs

use anyhow::Result;
use crossterm::event::{poll, read, Event as CrosstermEvent, KeyEvent, MouseEvent};
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
        // Event handler for both keyboard and mouse events
        let event_sender = sender.clone();
        let event_handle = tokio::spawn(async move {
            loop {
                // Poll for events with a short timeout
                if let Ok(true) = poll(Duration::from_millis(16)) {
                    if let Ok(event) = read() {
                        match event {
                            CrosstermEvent::Key(key) => {
                                if event_sender.send(Event::Key(key)).is_err() {
                                    break;
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
                // Small delay to prevent busy waiting
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
