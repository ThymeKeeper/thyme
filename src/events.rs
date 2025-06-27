// src/events.rs

use anyhow::Result;
use crossterm::event::{poll, read, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<Event>,
    _handles: Vec<tokio::task::JoinHandle<()>>,
}

impl EventHandler {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Keyboard event handler using polling instead of EventStream
        let keyboard_sender = sender.clone();
        let keyboard_handle = tokio::spawn(async move {
            loop {
                // Poll for events with a short timeout
                if let Ok(true) = poll(Duration::from_millis(16)) {
                    if let Ok(event) = read() {
                        if let CrosstermEvent::Key(key) = event {
                            if keyboard_sender.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                    }
                } else {
                    // Send tick event when no input is available
                    if keyboard_sender.send(Event::Tick).is_err() {
                        break;
                    }
                }
                
                // Small delay to prevent busy waiting
                tokio::time::sleep(Duration::from_millis(16)).await;
            }
        });

        Ok(Self {
            receiver,
            _handles: vec![keyboard_handle],
        })
    }

    pub async fn next(&mut self) -> Result<Option<Event>> {
        Ok(self.receiver.recv().await)
    }
}
